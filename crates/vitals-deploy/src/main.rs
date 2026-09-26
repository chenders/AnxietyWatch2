//! Build and validate the immutable artifact manifest used by Pi delivery.
//!
//! This command does not activate a release, restart a service, touch `PostgreSQL`,
//! or modify the spool. Activation is a later, separately gated operation.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::env;
use std::error::Error;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const BINARY_SPECS: &[BinarySpec] = &[
    BinarySpec {
        name: "vitalsd",
        source: "target/release/vitalsd",
        destination: "bin/vitalsd",
        unit: "vitalsd.service",
        package: "vitalsd",
        available: true,
    },
    BinarySpec {
        name: "vitals-spool-drain",
        source: "target/release/vitals-spool-drain",
        destination: "bin/vitals-spool-drain",
        unit: "vitals-spool-drain.service",
        package: "vitals-drain",
        available: true,
    },
    BinarySpec {
        name: "vitals-console",
        source: "target/release/vitals-console",
        destination: "bin/vitals-console",
        unit: "vitals-console.service",
        package: "vitals-console",
        available: true,
    },
    BinarySpec {
        name: "h10-collector",
        source: "target/release/h10-collector",
        destination: "bin/h10-collector",
        unit: "vitals-h10.service",
        package: "vitals-collectors",
        available: true,
    },
    BinarySpec {
        name: "bp-collector",
        source: "target/release/bp-collector",
        destination: "bin/bp-collector",
        unit: "vitals-bp.service",
        package: "vitals-collectors",
        available: true,
    },
    BinarySpec {
        name: "silence-detector",
        source: "target/release/silence-detector",
        destination: "bin/silence-detector",
        unit: "silence-detector.service",
        package: "not present in workspace",
        available: false,
    },
];

const UNIT_FILES: &[&str] = &[
    "deploy/vitalsd.service",
    "deploy/vitals-spool-drain.service",
    "deploy/vitals-console.service",
    "deploy/vitals-h10.service",
    "deploy/vitals-bp.service",
    "deploy/vitals-bp-autoconnect.service",
    "deploy/silence-detector.service",
];

const RULE_FILES: &[&str] = &["deploy/99-vitals-bp-autoconnect.rules"];
const LIVE_BINARIES: &[&str] = &[
    "/usr/local/bin/vitalsd",
    "/usr/local/bin/vitals-spool-drain",
    "/usr/local/bin/vitals-console",
    "/usr/local/bin/h10-collector",
    "/usr/local/bin/bp-collector",
    "/usr/local/bin/silence-detector",
];
const LIVE_UNITS: &[&str] = &[
    "/etc/systemd/system/vitalsd.service",
    "/etc/systemd/system/vitals-spool-drain.service",
    "/etc/systemd/system/vitals-console.service",
    "/etc/systemd/system/vitals-h10.service",
    "/etc/systemd/system/vitals-bp.service",
    "/etc/systemd/system/vitals-bp-autoconnect.service",
    "/etc/systemd/system/silence-detector.service",
];
const LIVE_SERVICES: &[&str] = &[
    "vitalsd",
    "vitals-spool-drain",
    "vitals-console",
    "vitals-h10",
    "vitals-bp",
    "vitals-bp-autoconnect",
    "silence-detector",
];
const SPOOL_DIRECTORY: &str = "/var/lib/vitals/spool";

#[derive(Debug, Clone, Copy)]
struct BinarySpec {
    name: &'static str,
    source: &'static str,
    destination: &'static str,
    unit: &'static str,
    package: &'static str,
    available: bool,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Manifest {
    schema_version: u32,
    source_sha: String,
    tree_sha: Option<String>,
    architecture: String,
    toolchain: String,
    dry_run: bool,
    files: Vec<ArtifactFile>,
    blockers: Vec<Blocker>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ArtifactFile {
    source: String,
    destination: String,
    sha256: String,
    mode: u32,
    kind: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Blocker {
    kind: String,
    subject: String,
    value: String,
    reason: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Snapshot {
    observed_at: u64,
    current_release: Option<String>,
    live_files: Vec<LiveFile>,
    services: Vec<ServiceState>,
    spool: SpoolState,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct LiveFile {
    path: String,
    exists: bool,
    size: u64,
    sha256: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct ServiceState {
    name: String,
    state: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct SpoolState {
    file_count: u64,
    bytes: u64,
    fingerprint: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
enum CheckState {
    Passed,
    Failed,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct DryRunChecks {
    checksums: CheckState,
    systemd: CheckState,
    live_state: CheckState,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct DryRunReceipt {
    mode: String,
    activated: bool,
    preflight_ok: bool,
    checks: DryRunChecks,
    current_release: Option<String>,
    blocked: Vec<Blocker>,
    would_change: Vec<String>,
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("vitals-deploy: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = env::args();
    let _program = args.next();
    let command = args.next().ok_or("missing command")?;
    match command.as_str() {
        "manifest" => run_manifest(args),
        "snapshot" => run_snapshot(args),
        "apply" => run_apply(args),
        value => Err(format!("unknown command: {value}").into()),
    }
}

fn run_manifest(mut args: impl Iterator<Item = String>) -> Result<(), Box<dyn Error>> {
    let mut source_sha = None;
    let mut output = None;
    let mut dry_run = false;
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--source" => source_sha = Some(args.next().ok_or("--source needs a value")?),
            "--output" => {
                output = Some(PathBuf::from(args.next().ok_or("--output needs a value")?));
            }
            "--dry-run" => dry_run = true,
            value => return Err(format!("unknown argument: {value}").into()),
        }
    }

    let source_sha = source_sha.ok_or("manifest requires --source")?;
    let output = output.ok_or("manifest requires --output")?;
    let root = find_repository_root(env::current_dir()?)?;
    let manifest = build_manifest(&root, source_sha, dry_run)?;
    validate_manifest(&manifest)?;
    write_json(&output, &manifest)
}

fn run_snapshot(mut args: impl Iterator<Item = String>) -> Result<(), Box<dyn Error>> {
    let output = required_path(&mut args, "snapshot", "--output")?;
    reject_extra_args(args)?;
    let snapshot = snapshot_state()?;
    write_json(&output, &snapshot)
}

fn run_apply(mut args: impl Iterator<Item = String>) -> Result<(), Box<dyn Error>> {
    let manifest_path = required_path(&mut args, "apply", "--manifest")?;
    let receipt_path = required_path(&mut args, "apply", "--receipt")?;
    let mut dry_run = false;
    for argument in args {
        match argument.as_str() {
            "--dry-run" => dry_run = true,
            value => return Err(format!("unknown apply argument: {value}").into()),
        }
    }
    if !dry_run {
        return Err("apply blocked: activation is not implemented; pass --dry-run".into());
    }

    let manifest: Manifest = serde_json::from_str(&fs::read_to_string(&manifest_path)?)?;
    validate_manifest(&manifest)?;
    let before = snapshot_state()?;
    let staging_root = manifest_path
        .parent()
        .ok_or("manifest has no parent directory")?
        .to_path_buf();
    let checksums_ok = validate_staged_files(&staging_root, &manifest)?;
    let systemd_verify_ok = verify_staged_units(&staging_root, &manifest)?;
    let after = snapshot_state()?;
    let live_state_unchanged = same_snapshot_state(&before, &after);
    let receipt = DryRunReceipt {
        mode: "dry-run".to_owned(),
        activated: false,
        preflight_ok: checksums_ok && systemd_verify_ok && live_state_unchanged,
        checks: DryRunChecks {
            checksums: if checksums_ok {
                CheckState::Passed
            } else {
                CheckState::Failed
            },
            systemd: if systemd_verify_ok {
                CheckState::Passed
            } else {
                CheckState::Failed
            },
            live_state: if live_state_unchanged {
                CheckState::Passed
            } else {
                CheckState::Failed
            },
        },
        current_release: before.current_release,
        blocked: manifest.blockers,
        would_change: manifest
            .files
            .iter()
            .map(|file| file.destination.clone())
            .collect(),
    };
    write_json(&receipt_path, &receipt)
}

fn required_path(
    args: &mut impl Iterator<Item = String>,
    command: &str,
    option: &str,
) -> Result<PathBuf, Box<dyn Error>> {
    let argument = args.next().ok_or(format!("{command} requires {option}"))?;
    if argument != option {
        return Err(format!("unknown {command} argument: {argument}").into());
    }
    Ok(PathBuf::from(args.next().ok_or(format!(
        "{command} requires a value for {option}"
    ))?))
}

fn reject_extra_args(mut args: impl Iterator<Item = String>) -> Result<(), Box<dyn Error>> {
    if let Some(argument) = args.next() {
        return Err(format!("unexpected argument: {argument}").into());
    }
    Ok(())
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path
        .parent()
        .filter(|candidate| !candidate.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    let mut file = File::create(path)?;
    serde_json::to_writer_pretty(&mut file, value)?;
    file.write_all(b"\n")?;
    Ok(())
}

fn snapshot_state() -> Result<Snapshot, Box<dyn Error>> {
    let live_paths = LIVE_BINARIES.iter().chain(LIVE_UNITS.iter());
    let mut live_files = Vec::new();
    for path in live_paths {
        let file = Path::new(path);
        if file.is_file() {
            let metadata = fs::metadata(file)?;
            live_files.push(LiveFile {
                path: (*path).to_owned(),
                exists: true,
                size: metadata.len(),
                sha256: Some(sha256_file(file)?),
            });
        } else {
            live_files.push(LiveFile {
                path: (*path).to_owned(),
                exists: false,
                size: 0,
                sha256: None,
            });
        }
    }
    let mut services = Vec::new();
    for name in LIVE_SERVICES {
        let output = Command::new("systemctl").args(["is-active", name]).output();
        let state = match output {
            Ok(output) => String::from_utf8_lossy(&output.stdout).trim().to_owned(),
            Err(error) => format!("unknown: {error}"),
        };
        services.push(ServiceState {
            name: (*name).to_owned(),
            state,
        });
    }
    services.sort_by(|left, right| left.name.cmp(&right.name));
    let spool = spool_state()?;
    let current_release = fs::read_link("/usr/local/lib/vitals/current")
        .ok()
        .map(|path| path.to_string_lossy().into_owned());
    let observed_at = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    Ok(Snapshot {
        observed_at,
        current_release,
        live_files,
        services,
        spool,
    })
}

fn spool_state() -> Result<SpoolState, Box<dyn Error>> {
    let directory = Path::new(SPOOL_DIRECTORY);
    let mut files = Vec::new();
    if directory.is_dir() {
        collect_files(directory, &mut files)?;
    }
    files.retain(|path| {
        path.extension()
            .is_some_and(|extension| extension == "spool")
    });
    files.sort();

    let mut data_bytes = 0_u64;
    let mut digest = Sha256::new();
    for path in &files {
        let metadata = fs::metadata(path)?;
        data_bytes = data_bytes.saturating_add(metadata.len());
        let relative = path.strip_prefix(directory)?.to_string_lossy();
        digest.update(relative.as_bytes());
        digest.update([0_u8]);
        digest.update(metadata.len().to_le_bytes());
        digest.update(sha256_file(path)?.as_bytes());
    }
    let mut data_fingerprint = String::with_capacity(64);
    for byte in digest.finalize() {
        use std::fmt::Write as _;
        write!(&mut data_fingerprint, "{byte:02x}")?;
    }
    Ok(SpoolState {
        file_count: u64::try_from(files.len())?,
        bytes: data_bytes,
        fingerprint: data_fingerprint,
    })
}

fn same_snapshot_state(left: &Snapshot, right: &Snapshot) -> bool {
    left.current_release == right.current_release
        && left.live_files == right.live_files
        && left.services == right.services
        && left.spool == right.spool
}

fn validate_staged_files(root: &Path, manifest: &Manifest) -> Result<bool, Box<dyn Error>> {
    for file in &manifest.files {
        let path = safe_staged_path(root, &file.destination)?;
        if sha256_file(&path)? != file.sha256 {
            return Err(format!("staged checksum mismatch: {}", file.destination).into());
        }
        if file_mode(&path)? != file.mode {
            return Err(format!(
                "staged mode mismatch: {} is {:o}, expected {:o}",
                file.destination,
                file_mode(&path)?,
                file.mode
            )
            .into());
        }
    }
    Ok(true)
}

fn verify_staged_units(root: &Path, manifest: &Manifest) -> Result<bool, Box<dyn Error>> {
    let mut units = Vec::new();
    for file in &manifest.files {
        if file.kind == "systemd-unit" {
            units.push(safe_staged_path(root, &file.destination)?);
        }
    }
    if units.is_empty() {
        return Err("dry run blocked: manifest has no systemd units".into());
    }
    let output = Command::new("systemd-analyze")
        .arg("verify")
        .args(&units)
        .output()?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr);
        return Err(format!("systemd unit verification failed: {detail}").into());
    }
    Ok(true)
}

fn safe_staged_path(root: &Path, destination: &str) -> Result<PathBuf, Box<dyn Error>> {
    validate_relative_path(Path::new(destination))?;
    let root = fs::canonicalize(root)?;
    let path = root.join(destination);
    let metadata = fs::symlink_metadata(&path)?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
        return Err(format!("unsafe staged artifact: {destination}").into());
    }
    let canonical = fs::canonicalize(path)?;
    if !canonical.starts_with(&root) {
        return Err(format!("staged artifact escapes root: {destination}").into());
    }
    Ok(canonical)
}

fn build_manifest(
    root: &Path,
    source_sha: String,
    dry_run: bool,
) -> Result<Manifest, Box<dyn Error>> {
    if source_sha.is_empty() {
        return Err("source SHA is empty".into());
    }

    let mut files = Vec::new();
    let mut blockers = Vec::new();
    for spec in BINARY_SPECS {
        let path = root.join(spec.source);
        if spec.available && path.is_file() {
            files.push(artifact_file(root, &path, spec.destination, "binary")?);
        } else {
            blockers.push(missing_blocker(spec));
        }
    }

    for relative in UNIT_FILES {
        let path = root.join(relative);
        files.push(artifact_file(
            root,
            &path,
            &format!("units/{relative}"),
            "systemd-unit",
        )?);
    }
    for relative in RULE_FILES {
        let path = root.join(relative);
        files.push(artifact_file(
            root,
            &path,
            &format!("rules/{relative}"),
            "udev-rule",
        )?);
    }

    let assets = root.join("crates/vitals-console/assets");
    let mut asset_paths = Vec::new();
    collect_files(&assets, &mut asset_paths)?;
    for path in asset_paths {
        let relative = path
            .strip_prefix(&assets)?
            .to_string_lossy()
            .replace('\\', "/");
        let destination = format!("share/vitals-console/assets/{relative}");
        files.push(artifact_file(root, &path, &destination, "console-asset")?);
    }

    files.sort_by(|left, right| left.destination.cmp(&right.destination));
    blockers.sort_by(|left, right| left.subject.cmp(&right.subject));
    Ok(Manifest {
        schema_version: 1,
        source_sha,
        tree_sha: git_tree_sha(root),
        architecture: env::consts::ARCH.to_owned(),
        toolchain: rustc_version(),
        dry_run,
        files,
        blockers,
    })
}

fn missing_blocker(spec: &BinarySpec) -> Blocker {
    Blocker {
        kind: "missing_artifact".to_owned(),
        subject: spec.name.to_owned(),
        value: spec.source.to_owned(),
        reason: format!(
            "{} references {}, but the expected package {} did not produce it",
            spec.unit, spec.name, spec.package
        ),
    }
}

fn artifact_file(
    root: &Path,
    source: &Path,
    destination: &str,
    kind: &str,
) -> Result<ArtifactFile, Box<dyn Error>> {
    let relative = source
        .strip_prefix(root)?
        .to_string_lossy()
        .replace('\\', "/");
    validate_relative_path(Path::new(destination))?;
    Ok(ArtifactFile {
        source: relative,
        destination: destination.to_owned(),
        sha256: sha256_file(source)?,
        mode: file_mode(source)?,
        kind: kind.to_owned(),
    })
}

fn collect_files(directory: &Path, output: &mut Vec<PathBuf>) -> Result<(), Box<dyn Error>> {
    let mut entries = fs::read_dir(directory)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, output)?;
        } else if path.is_file() {
            output.push(path);
        }
    }
    Ok(())
}

fn validate_manifest(manifest: &Manifest) -> Result<(), Box<dyn Error>> {
    if manifest.schema_version != 1 {
        return Err(format!("unsupported manifest schema: {}", manifest.schema_version).into());
    }
    if manifest.source_sha.is_empty() {
        return Err("manifest source SHA is empty".into());
    }

    let mut destinations = Vec::with_capacity(manifest.files.len());
    for file in &manifest.files {
        validate_relative_path(Path::new(&file.destination))?;
        if file.sha256.len() != 64 || !file.sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(format!("invalid SHA-256 for {}", file.destination).into());
        }
        if destinations.iter().any(|seen| seen == &file.destination) {
            return Err(format!("duplicate destination: {}", file.destination).into());
        }
        destinations.push(file.destination.clone());
    }
    Ok(())
}

fn validate_relative_path(path: &Path) -> Result<(), Box<dyn Error>> {
    if path.as_os_str().is_empty() || path.is_absolute() {
        return Err(format!("unsafe artifact path: {}", path.display()).into());
    }
    for component in path.components() {
        match component {
            Component::Normal(_) => {}
            Component::CurDir
            | Component::ParentDir
            | Component::RootDir
            | Component::Prefix(_) => {
                return Err(format!("unsafe artifact path: {}", path.display()).into());
            }
        }
    }
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String, Box<dyn Error>> {
    let mut file = File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 8192];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        let chunk = buffer.get(..read).ok_or("read exceeded hash buffer")?;
        digest.update(chunk);
    }
    let mut result = String::with_capacity(64);
    for byte in digest.finalize() {
        use std::fmt::Write as _;
        write!(&mut result, "{byte:02x}")?;
    }
    Ok(result)
}

#[cfg(test)]
fn verify_hash(path: &Path, expected: &str) -> Result<(), Box<dyn Error>> {
    let actual = sha256_file(path)?;
    if actual != expected {
        return Err(format!(
            "checksum mismatch for {}: got {actual}, expected {expected}",
            path.display()
        )
        .into());
    }
    Ok(())
}

#[cfg(unix)]
fn file_mode(path: &Path) -> Result<u32, Box<dyn Error>> {
    use std::os::unix::fs::PermissionsExt;
    Ok(fs::metadata(path)?.permissions().mode() & 0o777)
}

#[cfg(not(unix))]
fn file_mode(path: &Path) -> Result<u32, Box<dyn Error>> {
    let _ = fs::metadata(path)?;
    Ok(0o644)
}

fn find_repository_root(start: PathBuf) -> Result<PathBuf, Box<dyn Error>> {
    let mut current = start;
    loop {
        if current.join("Cargo.toml").is_file() && current.join("vitals").is_dir() {
            return Ok(current);
        }
        if !current.pop() {
            return Err("could not find repository root".into());
        }
    }
}

fn git_tree_sha(root: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD^{tree}"])
        .current_dir(root)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8(output.stdout).ok()?;
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_owned())
    }
}

fn rustc_version() -> String {
    let output = Command::new("rustc").arg("--version").output();
    match output {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).trim().to_owned()
        }
        _ => "unknown".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ArtifactFile, BINARY_SPECS, Manifest, missing_blocker, sha256_file, validate_manifest,
        validate_relative_path, verify_hash,
    };
    use std::fs;
    use tempfile::tempdir;

    fn file(path: &std::path::Path, contents: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        fs::write(path, contents)?;
        Ok(())
    }

    fn artifact(destination: &str, sha256: &str) -> ArtifactFile {
        ArtifactFile {
            source: "fixture".to_owned(),
            destination: destination.to_owned(),
            sha256: sha256.to_owned(),
            mode: 0o644,
            kind: "test".to_owned(),
        }
    }

    fn manifest(files: Vec<ArtifactFile>, source_sha: &str) -> Manifest {
        Manifest {
            schema_version: 1,
            source_sha: source_sha.to_owned(),
            tree_sha: None,
            architecture: "aarch64".to_owned(),
            toolchain: "test".to_owned(),
            dry_run: true,
            files,
            blockers: Vec::new(),
        }
    }

    #[test]
    fn parses_and_validates_manifest() -> Result<(), Box<dyn std::error::Error>> {
        let value = manifest(vec![artifact("bin/example", &"a".repeat(64))], "abc123");
        let encoded = serde_json::to_string(&value)?;
        let decoded: Manifest = serde_json::from_str(&encoded)?;
        assert_eq!(decoded, value);
        assert!(validate_manifest(&decoded).is_ok());
        Ok(())
    }

    #[test]
    fn rejects_path_traversal() {
        assert!(validate_relative_path(std::path::Path::new("../etc/passwd")).is_err());
        assert!(validate_relative_path(std::path::Path::new("/etc/passwd")).is_err());
        assert!(validate_relative_path(std::path::Path::new("bin/../etc/passwd")).is_err());
    }

    #[test]
    fn rejects_checksum_mismatch() -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempdir()?;
        let path = directory.path().join("artifact");
        file(&path, b"artifact")?;
        assert!(verify_hash(&path, &"0".repeat(64)).is_err());
        Ok(())
    }

    #[test]
    fn reports_missing_target_as_a_blocker_instead_of_omitting_it()
    -> Result<(), Box<dyn std::error::Error>> {
        let spec = BINARY_SPECS
            .iter()
            .find(|candidate| candidate.name == "silence-detector")
            .ok_or("silence-detector spec missing")?;
        let blocker = missing_blocker(spec);
        assert_eq!(blocker.kind, "missing_artifact");
        assert_eq!(blocker.subject, "silence-detector");
        assert!(blocker.reason.contains("not present in workspace"));
        Ok(())
    }

    #[test]
    fn rejects_duplicate_destinations() {
        let duplicate = artifact("bin/example", &"a".repeat(64));
        let value = manifest(vec![duplicate.clone(), duplicate], "abc123");
        assert!(validate_manifest(&value).is_err());
    }

    #[test]
    fn rejects_empty_source_sha() {
        let value = manifest(vec![artifact("bin/example", &"a".repeat(64))], "");
        assert!(validate_manifest(&value).is_err());
    }

    #[test]
    fn produces_stable_file_hashes() -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempdir()?;
        let path = directory.path().join("artifact");
        file(&path, b"stable")?;
        let first = sha256_file(&path)?;
        let second = sha256_file(&path)?;
        assert_eq!(first, second);
        Ok(())
    }
}
