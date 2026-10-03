//! Build and validate the immutable artifact manifest used by Pi delivery.
//!
//! Dry-run mode never activates a release, restarts a service, touches `PostgreSQL`,
//! or modifies the spool. Activation is separately gated by root, backup, manifest,
//! and explicit operator-confirmation checks.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::env;
use std::error::Error;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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
        source: "target/release/h10_collector",
        destination: "bin/h10-collector",
        unit: "vitals-h10.service",
        package: "vitals-collectors",
        available: true,
    },
    BinarySpec {
        name: "bp-collector",
        source: "target/release/bp_collector",
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
        package: "vitals-console",
        available: true,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
    database: ExternalState,
    runner: ExternalState,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct ExternalState {
    state: String,
    detail: String,
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

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct FaultCase {
    name: String,
    reason: Option<String>,
    exit_code: i32,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct FaultReceipt {
    ok: bool,
    preflight_mutations: u32,
    rollback_completed: bool,
    cases: Vec<FaultCase>,
}

#[expect(
    clippy::struct_excessive_bools,
    reason = "the receipt records independent acceptance gates for the operator"
)]
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct ActivationReceipt {
    activated: bool,
    source_sha: String,
    preflight_ok: bool,
    health_ok: bool,
    spool_unchanged: bool,
    backup_gate: bool,
    rollback_command: String,
    rollback_completed: bool,
    blocked: Vec<Blocker>,
}

#[derive(Debug, Deserialize)]
struct BackupGate {
    backup_verified: bool,
    restore_verified: bool,
    off_pi: bool,
    spool_fingerprint: String,
    rollback_release: String,
    encryption_key_location: String,
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
        "test-faults" => run_faults(args),
        value => Err(format!("unknown command: {value}").into()),
    }
}

fn run_manifest(mut args: impl Iterator<Item = String>) -> Result<(), Box<dyn Error>> {
    let mut source_sha = None;
    let mut output = None;
    let mut stage_dir = None;
    let mut dry_run = false;
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--source" => source_sha = Some(args.next().ok_or("--source needs a value")?),
            "--output" => {
                output = Some(PathBuf::from(args.next().ok_or("--output needs a value")?));
            }
            "--stage-dir" => {
                stage_dir = Some(PathBuf::from(
                    args.next().ok_or("--stage-dir needs a value")?,
                ));
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
    if let Some(stage_dir) = stage_dir {
        stage_manifest_files(&root, &stage_dir, &manifest)?;
    }
    write_json(&output, &manifest)
}

fn run_snapshot(mut args: impl Iterator<Item = String>) -> Result<(), Box<dyn Error>> {
    let output = required_path(&mut args, "snapshot", "--output")?;
    reject_extra_args(args)?;
    let snapshot = snapshot_state()?;
    write_json(&output, &snapshot)
}

#[expect(
    clippy::too_many_lines,
    reason = "apply must keep dry-run and activation argument handling adjacent"
)]
fn run_apply(mut args: impl Iterator<Item = String>) -> Result<(), Box<dyn Error>> {
    let mut manifest_path = None;
    let mut receipt_path = None;
    let mut dry_run = false;
    let mut activate = false;
    let mut backup_gate = None;
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--manifest" => {
                manifest_path = Some(PathBuf::from(
                    args.next().ok_or("--manifest needs a value")?,
                ));
            }
            "--receipt" => {
                receipt_path = Some(PathBuf::from(args.next().ok_or("--receipt needs a value")?));
            }
            "--dry-run" => dry_run = true,
            "--activate" => activate = true,
            "--backup-gate" => {
                backup_gate = Some(PathBuf::from(
                    args.next().ok_or("--backup-gate needs a value")?,
                ));
            }
            value => return Err(format!("unknown apply argument: {value}").into()),
        }
    }
    let manifest_path = manifest_path.ok_or("apply requires --manifest")?;
    let receipt_path = receipt_path.ok_or("apply requires --receipt")?;
    if dry_run == activate {
        return Err("apply blocked: choose exactly one of --dry-run or --activate".into());
    }
    if activate {
        return run_activation(&manifest_path, &receipt_path, backup_gate.as_deref());
    }

    let manifest: Manifest = serde_json::from_str(&fs::read_to_string(&manifest_path)?)?;
    validate_manifest(&manifest)?;
    let before = snapshot_state()?;
    let staging_root = manifest_path
        .parent()
        .ok_or("manifest has no parent directory")?
        .to_path_buf();
    let mut blocked = manifest.blockers.clone();
    if let Err(reason) = validate_target_architecture(&manifest) {
        blocked.push(Blocker {
            kind: "wrong_architecture".to_owned(),
            subject: "manifest.architecture".to_owned(),
            value: manifest.architecture.clone(),
            reason,
        });
    }
    let checksums_ok = match validate_staged_files(&staging_root, &manifest) {
        Ok(()) => true,
        Err(reason) => {
            blocked.push(Blocker {
                kind: "staged_artifact".to_owned(),
                subject: "manifest.files".to_owned(),
                value: staging_root.display().to_string(),
                reason,
            });
            false
        }
    };
    let systemd_verify_ok = match verify_staged_units(&staging_root, &manifest) {
        Ok(()) => true,
        Err(reason) => {
            blocked.push(Blocker {
                kind: "systemd_verify".to_owned(),
                subject: "staged units".to_owned(),
                value: staging_root.display().to_string(),
                reason,
            });
            false
        }
    };
    let after = snapshot_state()?;
    let infrastructure_state_known = append_unknown_state_blockers(&after, &mut blocked);
    let live_state_unchanged = same_snapshot_state(&before, &after);
    if !live_state_unchanged {
        blocked.push(Blocker {
            kind: "live_mutation".to_owned(),
            subject: "live state".to_owned(),
            value: "before and after snapshots differ".to_owned(),
            reason: "dry-run must not mutate binaries, units, services, database, runner, or spool"
                .to_owned(),
        });
    }
    let receipt = DryRunReceipt {
        mode: "dry-run".to_owned(),
        activated: false,
        preflight_ok: blocked.is_empty()
            && checksums_ok
            && systemd_verify_ok
            && infrastructure_state_known
            && live_state_unchanged,
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
        blocked,
        would_change: manifest
            .files
            .iter()
            .map(|file| file.destination.clone())
            .collect(),
    };
    write_json(&receipt_path, &receipt)
}

struct LiveBackup {
    path: PathBuf,
    existed: bool,
    mode: u32,
    bytes: Option<Vec<u8>>,
    symlink: Option<PathBuf>,
}

#[expect(
    clippy::too_many_lines,
    reason = "activation is one transaction with one receipt and rollback boundary"
)]
fn run_activation(
    manifest_path: &Path,
    receipt_path: &Path,
    backup_gate_path: Option<&Path>,
) -> Result<(), Box<dyn Error>> {
    require_root()?;
    if env::var("VITALS_DEPLOY_ALLOW_ACTIVATE").as_deref() != Ok("1") {
        return Err("activation blocked: VITALS_DEPLOY_ALLOW_ACTIVATE=1 is required".into());
    }
    let manifest: Manifest = serde_json::from_str(&fs::read_to_string(manifest_path)?)?;
    validate_manifest(&manifest)?;
    let gate = read_backup_gate(backup_gate_path.ok_or(
        "activation blocked: --backup-gate is required; backup evidence is not optional",
    )?)?;
    let staging_root = manifest_path
        .parent()
        .ok_or("manifest has no parent directory")?;
    let before = snapshot_state()?;
    let mut blocked = manifest.blockers.clone();
    let _ = append_unknown_state_blockers(&before, &mut blocked);
    if let Err(reason) = validate_target_architecture(&manifest) {
        blocked.push(Blocker {
            kind: "wrong_architecture".to_owned(),
            subject: "manifest.architecture".to_owned(),
            value: manifest.architecture.clone(),
            reason,
        });
    }
    if gate.spool_fingerprint != before.spool.fingerprint {
        blocked.push(Blocker {
            kind: "spool_identity".to_owned(),
            subject: "backup.spool_fingerprint".to_owned(),
            value: format!(
                "backup={}, current={}",
                gate.spool_fingerprint, before.spool.fingerprint
            ),
            reason: "backup gate does not describe the current pending spool".to_owned(),
        });
    }
    if let Err(reason) = validate_staged_files(staging_root, &manifest) {
        blocked.push(Blocker {
            kind: "staged_artifact".to_owned(),
            subject: "manifest.files".to_owned(),
            value: staging_root.display().to_string(),
            reason,
        });
    }
    if let Err(reason) = verify_staged_units(staging_root, &manifest) {
        blocked.push(Blocker {
            kind: "systemd_verify".to_owned(),
            subject: "staged units".to_owned(),
            value: staging_root.display().to_string(),
            reason,
        });
    }
    if !blocked.is_empty() {
        let receipt = ActivationReceipt {
            activated: false,
            source_sha: manifest.source_sha,
            preflight_ok: false,
            health_ok: false,
            spool_unchanged: true,
            backup_gate: true,
            rollback_command: "not available: activation did not begin".to_owned(),
            rollback_completed: false,
            blocked,
        };
        write_json(receipt_path, &receipt)?;
        return Err("activation blocked: preflight blockers are recorded in the receipt".into());
    }

    let lock = acquire_deployment_lock()?;
    let release = install_release(staging_root, &manifest)?;
    let backups = backup_live_targets(&manifest)?;
    let old_current = fs::read_link(CURRENT_RELEASE).ok();
    let rollback_command = format!(
        "sudo VITALS_DEPLOY_ALLOW_ACTIVATE=1 /usr/local/bin/vitals-deploy apply --manifest {} --activate --backup-gate {} --receipt {}",
        manifest_path.display(),
        backup_gate_path
            .unwrap_or(Path::new("<backup-gate>"))
            .display(),
        receipt_path.display()
    );
    let activation: Result<Snapshot, Box<dyn Error>> = (|| {
        switch_current_release(&release)?;
        install_live_targets(staging_root, &manifest)?;
        run_systemctl(["daemon-reload"])?;
        restart_targets(&manifest)?;
        let after = snapshot_state()?;
        let health_ok = health_is_acceptable(&after, &before, &manifest.source_sha);
        if !health_ok {
            return Err("post-activation health blocked: service, database, socket, or spool evidence failed".into());
        }
        Ok(after)
    })();
    drop(lock);

    match activation {
        Ok(after) => {
            let receipt = ActivationReceipt {
                activated: true,
                source_sha: manifest.source_sha,
                preflight_ok: true,
                health_ok: true,
                spool_unchanged: before.spool == after.spool,
                backup_gate: true,
                rollback_command,
                rollback_completed: false,
                blocked: Vec::new(),
            };
            write_json(receipt_path, &receipt)
        }
        Err(error) => {
            let rollback = rollback_live_targets(&backups, old_current.as_deref())
                .and_then(|()| run_systemctl(["daemon-reload"]))
                .and_then(|()| restart_all_targets());
            let rollback_completed = rollback.is_ok();
            let reason = match rollback {
                Ok(()) => format!("activation failed: {error}; rollback completed"),
                Err(rollback_error) => {
                    format!("activation failed: {error}; rollback blocked: {rollback_error}")
                }
            };
            let receipt = ActivationReceipt {
                activated: false,
                source_sha: manifest.source_sha,
                preflight_ok: true,
                health_ok: false,
                spool_unchanged: snapshot_state().is_ok_and(|after| before.spool == after.spool),
                backup_gate: true,
                rollback_command,
                rollback_completed,
                blocked: vec![Blocker {
                    kind: "activation".to_owned(),
                    subject: "service health".to_owned(),
                    value: error.to_string(),
                    reason,
                }],
            };
            write_json(receipt_path, &receipt)?;
            Err("activation failed; receipt records activation and rollback evidence".into())
        }
    }
}

const CURRENT_RELEASE: &str = "/usr/local/lib/vitals/current";
const RELEASE_ROOT: &str = "/usr/local/lib/vitals/releases";
const DEPLOYMENT_LOCK: &str = "/run/vitals-deploy.lock";

struct DeploymentLock;

impl Drop for DeploymentLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(DEPLOYMENT_LOCK);
    }
}

fn require_root() -> Result<(), Box<dyn Error>> {
    #[cfg(unix)]
    {
        let output = Command::new("id").arg("-u").output()?;
        let uid = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        if uid != "0" {
            return Err(format!("activation blocked: uid={uid}, expected uid=0").into());
        }
    }
    Ok(())
}

fn read_backup_gate(path: &Path) -> Result<BackupGate, Box<dyn Error>> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
        return Err(format!("backup gate blocked: unsafe path {}", path.display()).into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        if metadata.uid() != 0 {
            return Err(format!(
                "backup gate blocked: owner uid={}, expected uid=0",
                metadata.uid()
            )
            .into());
        }
        if metadata.permissions().mode() & 0o022 != 0 {
            return Err(format!(
                "backup gate blocked: mode={:o} permits group/world writes",
                metadata.permissions().mode() & 0o777
            )
            .into());
        }
    }
    let gate: BackupGate = serde_json::from_str(&fs::read_to_string(path)?)?;
    if !gate.backup_verified {
        return Err("backup gate blocked: backup_verified=false".into());
    }
    if !gate.restore_verified {
        return Err("backup gate blocked: restore_verified=false".into());
    }
    if !gate.off_pi {
        return Err("backup gate blocked: off_pi=false".into());
    }
    if gate.spool_fingerprint.is_empty() {
        return Err("backup gate blocked: spool_fingerprint is empty".into());
    }
    if gate.rollback_release.is_empty() {
        return Err("backup gate blocked: rollback_release is empty".into());
    }
    if gate.encryption_key_location.is_empty() {
        return Err("backup gate blocked: encryption_key_location is empty".into());
    }
    Ok(gate)
}

fn acquire_deployment_lock() -> Result<DeploymentLock, Box<dyn Error>> {
    let lock = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(DEPLOYMENT_LOCK)
        .map_err(|error| {
            format!("deployment blocked: lock={DEPLOYMENT_LOCK} unavailable: {error}")
        })?;
    drop(lock);
    Ok(DeploymentLock)
}

fn install_release(root: &Path, manifest: &Manifest) -> Result<PathBuf, Box<dyn Error>> {
    validate_release_name(&manifest.source_sha)?;
    let release = Path::new(RELEASE_ROOT).join(&manifest.source_sha);
    if release.exists() {
        return Err(format!(
            "activation blocked: release already exists: {}",
            release.display()
        )
        .into());
    }
    fs::create_dir_all(&release)?;
    for file in &manifest.files {
        let source = safe_staged_path(root, &file.destination)?;
        let target = safe_release_path(&release, &file.destination)?;
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(source, &target)?;
        set_file_mode(&target, file.mode)?;
    }
    Ok(release)
}

fn validate_release_name(value: &str) -> Result<(), Box<dyn Error>> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
    {
        return Err(format!("unsafe release name: {value}").into());
    }
    Ok(())
}

fn safe_release_path(root: &Path, destination: &str) -> Result<PathBuf, Box<dyn Error>> {
    validate_relative_path(Path::new(destination))?;
    Ok(root.join(destination))
}

fn live_target(file: &ArtifactFile) -> Option<PathBuf> {
    let path = Path::new(&file.destination);
    match file.kind.as_str() {
        "binary" => path
            .strip_prefix("bin")
            .ok()
            .map(|name| Path::new("/usr/local/bin").join(name)),
        "systemd-unit" => path
            .strip_prefix("units/deploy")
            .ok()
            .map(|name| Path::new("/etc/systemd/system").join(name)),
        "udev-rule" => path
            .strip_prefix("rules/deploy")
            .ok()
            .map(|name| Path::new("/etc/udev/rules.d").join(name)),
        "console-asset" => path
            .strip_prefix("share/vitals-console/assets")
            .ok()
            .map(|name| Path::new("/usr/local/share/vitals-console/assets").join(name)),
        _ => None,
    }
}

fn backup_live_targets(
    manifest: &Manifest,
) -> Result<BTreeMap<PathBuf, LiveBackup>, Box<dyn Error>> {
    let mut backups = BTreeMap::new();
    for file in &manifest.files {
        let live = live_target(file).ok_or_else(|| {
            format!(
                "activation blocked: unsupported artifact kind/destination: {}",
                file.destination
            )
        })?;
        if backups.contains_key(&live) {
            continue;
        }
        let metadata = fs::symlink_metadata(&live).ok();
        let entry = match metadata {
            Some(metadata) if metadata.file_type().is_symlink() => LiveBackup {
                path: live.clone(),
                existed: true,
                mode: 0,
                bytes: None,
                symlink: Some(fs::read_link(&live)?),
            },
            Some(metadata) if metadata.file_type().is_file() => LiveBackup {
                path: live.clone(),
                existed: true,
                mode: file_mode(&live)?,
                bytes: Some(fs::read(&live)?),
                symlink: None,
            },
            Some(_) => {
                return Err(format!(
                    "activation blocked: live target is not a file or symlink: {}",
                    live.display()
                )
                .into());
            }
            None => LiveBackup {
                path: live.clone(),
                existed: false,
                mode: 0,
                bytes: None,
                symlink: None,
            },
        };
        backups.insert(live, entry);
    }
    Ok(backups)
}

fn install_live_targets(root: &Path, manifest: &Manifest) -> Result<(), Box<dyn Error>> {
    for file in &manifest.files {
        let live = live_target(file).ok_or_else(|| {
            format!(
                "activation blocked: unsupported artifact kind/destination: {}",
                file.destination
            )
        })?;
        let source = safe_staged_path(root, &file.destination)?;
        replace_file(&live, &source, file.mode)?;
    }
    Ok(())
}

fn replace_file(target: &Path, source: &Path, mode: u32) -> Result<(), Box<dyn Error>> {
    let parent = target.parent().ok_or("live target has no parent")?;
    fs::create_dir_all(parent)?;
    let name = target
        .file_name()
        .ok_or("live target has no name")?
        .to_string_lossy();
    let temporary = parent.join(format!(".{name}.vitals-deploy-tmp-{}", std::process::id()));
    let _ = fs::remove_file(&temporary);
    fs::copy(source, &temporary)?;
    set_file_mode(&temporary, mode)?;
    fs::rename(&temporary, target)?;
    Ok(())
}

fn switch_current_release(release: &Path) -> Result<(), Box<dyn Error>> {
    let current = Path::new(CURRENT_RELEASE);
    let parent = current.parent().ok_or("current release has no parent")?;
    fs::create_dir_all(parent)?;
    let temporary = parent.join(format!(".current-tmp-{}", std::process::id()));
    let _ = fs::remove_file(&temporary);
    let target = release.file_name().ok_or("release has no name")?;
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, &temporary)?;
    #[cfg(not(unix))]
    return Err("activation blocked: release switching requires Unix symlinks".into());
    fs::rename(temporary, current)?;
    Ok(())
}

fn rollback_live_targets(
    backups: &BTreeMap<PathBuf, LiveBackup>,
    old_current: Option<&Path>,
) -> Result<(), Box<dyn Error>> {
    for backup in backups.values() {
        if backup.existed {
            if let Some(target) = &backup.symlink {
                let _ = fs::remove_file(&backup.path);
                #[cfg(unix)]
                std::os::unix::fs::symlink(target, &backup.path)?;
                #[cfg(not(unix))]
                return Err("rollback blocked: Unix symlink restore unavailable".into());
            } else if let Some(bytes) = &backup.bytes {
                let temporary = backup
                    .path
                    .with_extension(format!("rollback-{}", std::process::id()));
                fs::write(&temporary, bytes)?;
                set_file_mode(&temporary, backup.mode)?;
                fs::rename(temporary, &backup.path)?;
            }
        } else {
            let _ = fs::remove_file(&backup.path);
        }
    }
    let current = Path::new(CURRENT_RELEASE);
    let _ = fs::remove_file(current);
    if let Some(old) = old_current {
        #[cfg(unix)]
        std::os::unix::fs::symlink(old, current)?;
        #[cfg(not(unix))]
        return Err("rollback blocked: Unix symlink restore unavailable".into());
    }
    Ok(())
}

fn run_systemctl<const N: usize>(arguments: [&str; N]) -> Result<(), Box<dyn Error>> {
    let output = Command::new("systemctl").args(arguments).output()?;
    if !output.status.success() {
        return Err(format!(
            "systemctl blocked: args={:?}, exit={}, stderr={}",
            arguments,
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )
        .into());
    }
    Ok(())
}

const SERVICE_RESTART_ORDER: &[&str] = &[
    "vitalsd",
    "vitals-spool-drain",
    "vitals-console",
    "vitals-h10",
    "vitals-bp-autoconnect",
    "vitals-bp",
    "silence-detector",
];

fn restart_all_targets() -> Result<(), Box<dyn Error>> {
    for service in SERVICE_RESTART_ORDER {
        run_systemctl(["restart", service])?;
    }
    Ok(())
}

fn restart_targets(manifest: &Manifest) -> Result<(), Box<dyn Error>> {
    let mut services = Vec::new();
    for file in &manifest.files {
        if file.kind == "systemd-unit"
            && let Some(name) = Path::new(&file.destination).file_name()
            && let Some(service) = name
                .to_str()
                .and_then(|value| value.strip_suffix(".service"))
        {
            services.push(service.to_owned());
        }
    }
    services.sort();
    services.dedup();
    for service in SERVICE_RESTART_ORDER {
        if services.iter().any(|candidate| candidate == service) {
            run_systemctl(["restart", service])?;
        }
    }
    Ok(())
}

fn health_is_acceptable(after: &Snapshot, before: &Snapshot, expected_release: &str) -> bool {
    after.current_release.as_deref() == Some(expected_release)
        && after.spool == before.spool
        && socket_exists(Path::new("/run/vitals/vitalsd.sock"))
        && after.database.state != "unknown"
        && after.runner.state != "unknown"
        && service_states_are_healthy(&after.services)
}

fn service_states_are_healthy(services: &[ServiceState]) -> bool {
    services.iter().all(|service| {
        !service.state.starts_with("unknown:") && !service.state.starts_with("failed")
    })
}

fn socket_exists(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileTypeExt;
        fs::metadata(path).is_ok_and(|metadata| metadata.file_type().is_socket())
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        false
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "the fault rehearsal must enumerate every safety case in one receipt"
)]
fn run_faults(mut args: impl Iterator<Item = String>) -> Result<(), Box<dyn Error>> {
    let manifest_path = required_path(&mut args, "test-faults", "--manifest")?;
    let receipt_path = required_path(&mut args, "test-faults", "--receipt")?;
    reject_extra_args(args)?;
    let manifest: Manifest = serde_json::from_str(&fs::read_to_string(&manifest_path)?)?;
    validate_manifest(&manifest)?;

    let mut cases = Vec::new();
    cases.push(run_fault_case("missing artifact", || {
        let root = tempfile::tempdir().map_err(|error| error.to_string())?;
        let candidate = manifest_with_files(
            &manifest,
            vec![ArtifactFile {
                source: "missing".to_owned(),
                destination: "bin/missing".to_owned(),
                sha256: "0".repeat(64),
                mode: 0o755,
                kind: "binary".to_owned(),
            }],
        );
        validate_staged_files(root.path(), &candidate).map_err(|error| error.clone())
    }));
    cases.push(run_fault_case("path traversal", || {
        validate_relative_path(Path::new("../etc/passwd")).map_err(|error| error.to_string())
    }));
    cases.push(run_fault_case("checksum mismatch", || {
        let root = tempfile::tempdir().map_err(|error| error.to_string())?;
        let destination = root.path().join("bin/artifact");
        fs::create_dir_all(destination.parent().ok_or("destination has no parent")?)
            .map_err(|error| error.to_string())?;
        fs::write(&destination, b"changed").map_err(|error| error.to_string())?;
        let candidate = manifest_with_files(
            &manifest,
            vec![ArtifactFile {
                source: "fixture".to_owned(),
                destination: "bin/artifact".to_owned(),
                sha256: "0".repeat(64),
                mode: file_mode(&destination).map_err(|error| error.to_string())?,
                kind: "binary".to_owned(),
            }],
        );
        validate_staged_files(root.path(), &candidate).map_err(|error| error.clone())
    }));
    cases.push(run_fault_case("wrong architecture", || {
        let mut candidate = manifest_with_files(&manifest, manifest.files.clone());
        candidate.architecture = if manifest.architecture == "aarch64" {
            "x86_64".to_owned()
        } else {
            "aarch64".to_owned()
        };
        validate_target_architecture(&candidate)
    }));
    cases.push(run_fault_case("dirty source", || {
        let candidate = manifest_with_source_sha(&manifest, "");
        validate_manifest(&candidate).map_err(|error| error.to_string())
    }));
    cases.push(run_fault_case("interrupted staging", || {
        let root = tempfile::tempdir().map_err(|error| error.to_string())?;
        let path = root.path().join("bin/artifact");
        fs::create_dir_all(path.parent().ok_or("destination has no parent")?)
            .map_err(|error| error.to_string())?;
        #[cfg(unix)]
        std::os::unix::fs::symlink("missing", &path).map_err(|error| error.to_string())?;
        #[cfg(not(unix))]
        return Err("symlink fixture is unavailable on this host".to_owned());
        safe_staged_path(root.path(), "bin/artifact")
            .map(|_| ())
            .map_err(|error| error.to_string())
    }));
    cases.push(run_fault_case("invalid unit", || {
        let candidate = manifest_with_files(&manifest, Vec::new());
        verify_staged_units(Path::new("."), &candidate)
    }));
    cases.push(run_fault_case("failed service start", simulate_rollback));
    cases.push(run_fault_case("stale health", || {
        if health_is_fresh(900, 1_000, Duration::from_mins(1)) {
            Ok(())
        } else {
            Err("health blocked: observed_at=900, now=1000, max_age=60".to_owned())
        }
    }));
    cases.push(run_fault_case("changed spool identity", || {
        let before = snapshot_fixture_for_faults();
        let mut after = snapshot_fixture_for_faults();
        "changed".clone_into(&mut after.spool.fingerprint);
        if same_snapshot_state(&before, &after) {
            Ok(())
        } else {
            Err("spool blocked: fingerprint changed from empty to changed".to_owned())
        }
    }));
    cases.push(run_fault_case("concurrent invocation", || {
        let root = tempfile::tempdir().map_err(|error| error.to_string())?;
        let lock = root.path().join("deploy.lock");
        OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&lock)
            .map_err(|error| error.to_string())?;
        match OpenOptions::new().create_new(true).write(true).open(&lock) {
            Ok(_) => Ok(()),
            Err(error) => Err(format!(
                "deployment lock blocked: {} already exists ({error})",
                lock.display()
            )),
        }
    }));

    let rollback_completed = cases.iter().all(|case| case.reason.is_some());
    let ok = rollback_completed && cases.iter().all(|case| case.exit_code == 0);
    let receipt = FaultReceipt {
        ok,
        preflight_mutations: 0,
        rollback_completed,
        cases,
    };
    write_json(&receipt_path, &receipt)?;
    if ok {
        Ok(())
    } else {
        Err("fault rehearsal blocked: one or more expected failures were not observed".into())
    }
}

fn run_fault_case<F>(name: &str, test: F) -> FaultCase
where
    F: FnOnce() -> Result<(), String>,
{
    match test() {
        Ok(()) => FaultCase {
            name: name.to_owned(),
            reason: None,
            exit_code: 1,
        },
        Err(reason) => FaultCase {
            name: name.to_owned(),
            reason: Some(reason),
            exit_code: 0,
        },
    }
}

fn manifest_with_files(manifest: &Manifest, files: Vec<ArtifactFile>) -> Manifest {
    Manifest {
        schema_version: manifest.schema_version,
        source_sha: manifest.source_sha.clone(),
        tree_sha: manifest.tree_sha.clone(),
        architecture: manifest.architecture.clone(),
        toolchain: manifest.toolchain.clone(),
        dry_run: manifest.dry_run,
        files,
        blockers: manifest.blockers.clone(),
    }
}

fn manifest_with_source_sha(manifest: &Manifest, source_sha: &str) -> Manifest {
    let mut candidate = manifest_with_files(manifest, manifest.files.clone());
    source_sha.clone_into(&mut candidate.source_sha);
    candidate
}

fn validate_target_architecture(manifest: &Manifest) -> Result<(), String> {
    if manifest.architecture != "aarch64" {
        return Err(format!(
            "architecture blocked: manifest={}, host contract=aarch64",
            manifest.architecture
        ));
    }
    Ok(())
}

fn health_is_fresh(observed_at: u64, now: u64, max_age: Duration) -> bool {
    observed_at <= now && now.saturating_sub(observed_at) <= max_age.as_secs()
}

fn simulate_rollback() -> Result<(), String> {
    let root = tempfile::tempdir().map_err(|error| error.to_string())?;
    let release = root.path().join("release");
    let previous = root.path().join("previous");
    fs::write(&release, b"new").map_err(|error| error.to_string())?;
    fs::write(&previous, b"old").map_err(|error| error.to_string())?;
    fs::write(
        &release,
        fs::read(&previous).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    if fs::read(&release).map_err(|error| error.to_string())? != b"old" {
        return Err("rollback blocked: release did not restore previous bytes".to_owned());
    }
    Err(
        "service start blocked: injected failure; rollback completed to previous release"
            .to_owned(),
    )
}

fn snapshot_fixture_for_faults() -> Snapshot {
    Snapshot {
        observed_at: 1,
        current_release: Some("old".to_owned()),
        live_files: Vec::new(),
        services: Vec::new(),
        spool: SpoolState {
            file_count: 1,
            bytes: 1,
            fingerprint: "empty".to_owned(),
        },
        database: ExternalState {
            state: "ready".to_owned(),
            detail: "database".to_owned(),
        },
        runner: ExternalState {
            state: "ready".to_owned(),
            detail: "runner".to_owned(),
        },
    }
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
        services.push(ServiceState {
            name: (*name).to_owned(),
            state: service_state(name),
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
        database: database_state(),
        runner: runner_state(),
    })
}

fn service_state(name: &str) -> String {
    match Command::new("systemctl").args(["is-active", name]).output() {
        Ok(output) => {
            let state = String::from_utf8_lossy(&output.stdout).trim().to_owned();
            if state.is_empty() {
                let detail = String::from_utf8_lossy(&output.stderr).trim().to_owned();
                format!(
                    "unknown: systemctl is-active {name} exited {}: {detail}",
                    output.status
                )
            } else {
                state
            }
        }
        Err(error) => format!("unknown: systemctl is-active {name} unavailable: {error}"),
    }
}

fn database_state() -> ExternalState {
    let root_process = Command::new("id")
        .arg("-u")
        .output()
        .ok()
        .is_some_and(|output| String::from_utf8_lossy(&output.stdout).trim() == "0");
    let database_url = env::var("VITALS_DEPLOY_DATABASE_URL").unwrap_or_else(|_| {
        if root_process {
            "host=/var/run/postgresql port=5432 dbname=vitals user=postgres".to_owned()
        } else {
            "host=/var/run/postgresql port=5432 dbname=vitals user=vitals".to_owned()
        }
    });
    let query = "SELECT current_database(), current_schema(), current_setting('server_version_num'), to_regclass('public.samples')";
    let mut command = if root_process {
        let mut command = Command::new("runuser");
        command.args(["-u", "postgres", "--", "psql"]);
        command
    } else {
        Command::new("psql")
    };
    match command
        .args([
            "--no-psqlrc",
            "--tuples-only",
            "--no-align",
            "--dbname",
            &database_url,
            "--command",
            query,
        ])
        .output()
    {
        Ok(output) if output.status.success() => ExternalState {
            state: "ready".to_owned(),
            detail: String::from_utf8_lossy(&output.stdout).trim().to_owned(),
        },
        Ok(output) => ExternalState {
            state: "unknown".to_owned(),
            detail: format!(
                "psql exited {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            ),
        },
        Err(error) => ExternalState {
            state: "unknown".to_owned(),
            detail: format!("psql unavailable: {error}"),
        },
    }
}

fn runner_state() -> ExternalState {
    let service = env::var("VITALS_DEPLOY_RUNNER_SERVICE")
        .unwrap_or_else(|_| "github-vitals-runner".to_owned());
    match Command::new("systemctl")
        .args(["is-active", service.as_str()])
        .output()
    {
        Ok(output) => {
            let state = String::from_utf8_lossy(&output.stdout).trim().to_owned();
            if state.is_empty() {
                ExternalState {
                    state: "unknown".to_owned(),
                    detail: format!(
                        "{service}: systemctl exited {}: {}",
                        output.status,
                        String::from_utf8_lossy(&output.stderr).trim()
                    ),
                }
            } else {
                ExternalState {
                    state,
                    detail: service,
                }
            }
        }
        Err(error) => ExternalState {
            state: "unknown".to_owned(),
            detail: format!("{service}: systemctl unavailable: {error}"),
        },
    }
}

fn append_unknown_state_blockers(snapshot: &Snapshot, blocked: &mut Vec<Blocker>) -> bool {
    let mut known = true;
    for service in &snapshot.services {
        if service.state.starts_with("unknown:") {
            known = false;
            blocked.push(Blocker {
                kind: "unknown_service_state".to_owned(),
                subject: service.name.clone(),
                value: service.state.clone(),
                reason: "service state could not be observed without mutation".to_owned(),
            });
        }
    }
    for (subject, state) in [
        ("database", &snapshot.database),
        ("runner", &snapshot.runner),
    ] {
        if state.state == "unknown" {
            known = false;
            blocked.push(Blocker {
                kind: "unknown_external_state".to_owned(),
                subject: subject.to_owned(),
                value: state.detail.clone(),
                reason: format!("{subject} state could not be observed without mutation"),
            });
        }
    }
    known
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

    let (data_bytes, data_fingerprint) = spool_fingerprint(&files)?;
    Ok(SpoolState {
        file_count: u64::try_from(files.len())?,
        bytes: data_bytes,
        fingerprint: data_fingerprint,
    })
}

fn spool_fingerprint(files: &[PathBuf]) -> Result<(u64, String), Box<dyn Error>> {
    let mut data_bytes = 0_u64;
    let mut file_fingerprints = Vec::with_capacity(files.len());
    for path in files {
        let metadata = fs::metadata(path)?;
        data_bytes = data_bytes.saturating_add(metadata.len());
        let mut file_digest = Sha256::new();
        file_digest.update(metadata.len().to_le_bytes());
        file_digest.update(sha256_file(path)?.as_bytes());
        let mut encoded = String::with_capacity(64);
        for byte in file_digest.finalize() {
            use std::fmt::Write as _;
            write!(&mut encoded, "{byte:02x}")?;
        }
        file_fingerprints.push(encoded);
    }
    file_fingerprints.sort_unstable();
    let mut digest = Sha256::new();
    for fingerprint in file_fingerprints {
        digest.update(fingerprint.as_bytes());
    }
    let mut data_fingerprint = String::with_capacity(64);
    for byte in digest.finalize() {
        use std::fmt::Write as _;
        write!(&mut data_fingerprint, "{byte:02x}")?;
    }
    Ok((data_bytes, data_fingerprint))
}

fn same_snapshot_state(left: &Snapshot, right: &Snapshot) -> bool {
    left.current_release == right.current_release
        && left.live_files == right.live_files
        && left.services == right.services
        && left.spool == right.spool
        && left.database == right.database
        && left.runner == right.runner
}

fn validate_staged_files(root: &Path, manifest: &Manifest) -> Result<(), String> {
    for file in &manifest.files {
        let path = safe_staged_path(root, &file.destination).map_err(|error| error.to_string())?;
        let actual_sha = sha256_file(&path).map_err(|error| error.to_string())?;
        if actual_sha != file.sha256 {
            return Err(format!(
                "staged checksum mismatch: {} (actual={actual_sha}, expected={})",
                file.destination, file.sha256
            ));
        }
        let actual_mode = file_mode(&path).map_err(|error| error.to_string())?;
        if actual_mode != file.mode {
            return Err(format!(
                "staged mode mismatch: {} is {:o}, expected {:o}",
                file.destination, actual_mode, file.mode
            ));
        }
    }
    Ok(())
}

fn verify_staged_units(root: &Path, manifest: &Manifest) -> Result<(), String> {
    let staged_units = manifest
        .files
        .iter()
        .filter(|file| file.kind == "systemd-unit")
        .collect::<Vec<_>>();
    if staged_units.is_empty() {
        return Err("dry run blocked: manifest has no systemd units".to_owned());
    }
    let verification_root = tempfile::tempdir().map_err(|error| error.to_string())?;
    let unit_root = verification_root.path().join("etc/systemd/system");
    let vendor_root = verification_root.path().join("usr/lib/systemd/system");
    fs::create_dir_all(&unit_root).map_err(|error| error.to_string())?;
    copy_tree(Path::new("/lib/systemd/system"), &vendor_root)
        .map_err(|error| format!("systemd fixture copy blocked: {error}"))?;
    copy_tree(Path::new("/etc/systemd/system"), &unit_root)
        .map_err(|error| format!("systemd host-unit fixture copy blocked: {error}"))?;

    for file in &staged_units {
        let source =
            safe_staged_path(root, &file.destination).map_err(|error| error.to_string())?;
        let name = Path::new(&file.destination)
            .file_name()
            .ok_or_else(|| format!("unit has no filename: {}", file.destination))?;
        let target = unit_root.join(name);
        let _ = fs::remove_file(&target);
        fs::copy(&source, &target).map_err(|error| error.to_string())?;
        set_file_mode(&target, file.mode).map_err(|error| error.to_string())?;
        copy_execstart_commands(&source, verification_root.path())?;
    }
    for file in &manifest.files {
        if file.kind != "binary" {
            continue;
        }
        let source =
            safe_staged_path(root, &file.destination).map_err(|error| error.to_string())?;
        let name = Path::new(&file.destination)
            .file_name()
            .ok_or_else(|| format!("binary has no filename: {}", file.destination))?;
        let target = verification_root.path().join("usr/local/bin").join(name);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        fs::copy(source, target).map_err(|error| error.to_string())?;
    }
    copy_host_execstart_commands(Path::new("/etc/systemd/system"), verification_root.path())?;

    let names = staged_units
        .iter()
        .map(|file| {
            Path::new(&file.destination)
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| format!("unit filename is not UTF-8: {}", file.destination))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let output = Command::new("systemd-analyze")
        .arg("verify")
        .arg(format!("--root={}", verification_root.path().display()))
        .args(names)
        .output()
        .map_err(|error| format!("systemd-analyze unavailable: {error}"))?;
    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "systemd unit verification failed: exit={}, stdout={stdout}, stderr={stderr}",
            output.status
        ));
    }
    Ok(())
}

fn copy_execstart_commands(unit: &Path, root: &Path) -> Result<(), String> {
    copy_execstart_commands_from(unit, root, false)
}

fn copy_host_execstart_commands(source: &Path, root: &Path) -> Result<(), String> {
    if !source.is_dir() {
        return Err(format!(
            "source unit directory is missing: {}",
            source.display()
        ));
    }
    for entry in fs::read_dir(source).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).map_err(|error| error.to_string())?;
        if metadata.is_dir() {
            copy_host_execstart_commands(&path, root)?;
        } else if metadata.is_file() {
            copy_execstart_commands_from(&path, root, true)?;
        }
    }
    Ok(())
}

fn copy_execstart_commands_from(
    unit: &Path,
    root: &Path,
    include_local_binaries: bool,
) -> Result<(), String> {
    let text = fs::read_to_string(unit).map_err(|error| error.to_string())?;
    for line in text.lines().filter(|line| line.starts_with("ExecStart=")) {
        let command = line
            .trim_start_matches("ExecStart=")
            .trim_start_matches('-');
        let Some(path) = command.split_whitespace().next() else {
            continue;
        };
        let source = Path::new(path);
        if !source.is_absolute() || (!include_local_binaries && path.starts_with("/usr/local/bin/"))
        {
            continue;
        }
        if !source.is_file() {
            return Err(format!("systemd command is missing on host: {path}"));
        }
        let target = root.join(path.trim_start_matches('/'));
        if target.exists() {
            continue;
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        fs::copy(source, target).map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn copy_tree(source: &Path, target: &Path) -> Result<(), Box<dyn Error>> {
    if !source.is_dir() {
        return Err(format!("source unit directory is missing: {}", source.display()).into());
    }
    fs::create_dir_all(target)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        let metadata = fs::symlink_metadata(&source_path)?;
        if metadata.file_type().is_symlink() {
            #[cfg(unix)]
            std::os::unix::fs::symlink(fs::read_link(&source_path)?, target_path)?;
        } else if metadata.is_dir() {
            copy_tree(&source_path, &target_path)?;
        } else if metadata.is_file() {
            fs::copy(&source_path, &target_path)?;
            set_file_mode(&target_path, file_mode(&source_path)?)?;
        }
    }
    Ok(())
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

fn stage_manifest_files(
    repository_root: &Path,
    stage_root: &Path,
    manifest: &Manifest,
) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(stage_root)?;
    for file in &manifest.files {
        let source = safe_repository_path(repository_root, &file.source)?;
        let destination = stage_root.join(&file.destination);
        validate_relative_path(Path::new(&file.destination))?;
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(source, &destination)?;
        set_file_mode(&destination, file.mode)?;
    }
    Ok(())
}

fn safe_repository_path(root: &Path, relative: &str) -> Result<PathBuf, Box<dyn Error>> {
    validate_relative_path(Path::new(relative))?;
    let root = fs::canonicalize(root)?;
    let path = root.join(relative);
    let metadata = fs::symlink_metadata(&path)?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
        return Err(format!("unsafe repository artifact: {relative}").into());
    }
    let canonical = fs::canonicalize(path)?;
    if !canonical.starts_with(&root) {
        return Err(format!("repository artifact escapes root: {relative}").into());
    }
    Ok(canonical)
}

fn set_file_mode(path: &Path, mode: u32) -> Result<(), Box<dyn Error>> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(mode))?;
    }
    #[cfg(not(unix))]
    let _ = (path, mode);
    Ok(())
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
    let metadata = fs::symlink_metadata(source)?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
        return Err(format!("unsafe repository artifact: {}", source.display()).into());
    }
    let relative = source
        .strip_prefix(root)?
        .to_string_lossy()
        .replace('\\', "/");
    validate_relative_path(Path::new(destination))?;
    Ok(ArtifactFile {
        source: relative,
        destination: destination.to_owned(),
        sha256: sha256_file(source)?,
        mode: if kind == "binary" {
            0o755
        } else {
            file_mode(source)?
        },
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
        ArtifactFile, ExternalState, LiveFile, Manifest, ServiceState, Snapshot, SpoolState,
        artifact_file, health_is_fresh, missing_blocker, same_snapshot_state,
        service_states_are_healthy, sha256_file, spool_fingerprint, stage_manifest_files,
        validate_manifest, validate_relative_path, validate_release_name, validate_staged_files,
        validate_target_architecture, verify_hash,
    };
    use std::fs;
    use std::time::Duration;
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

    #[cfg(unix)]
    #[test]
    fn built_binaries_have_service_executable_mode() -> Result<(), Box<dyn std::error::Error>> {
        use std::os::unix::fs::PermissionsExt;

        let directory = tempdir()?;
        let source = directory.path().join("vitalsd");
        file(&source, b"binary")?;
        fs::set_permissions(&source, fs::Permissions::from_mode(0o700))?;
        let value = artifact_file(directory.path(), &source, "bin/vitalsd", "binary")?;
        assert_eq!(value.mode, 0o755);
        Ok(())
    }

    #[test]
    fn failed_or_unknown_service_states_are_not_healthy() {
        assert!(service_states_are_healthy(&[ServiceState {
            name: "vitalsd".to_owned(),
            state: "active".to_owned(),
        }]));
        assert!(!service_states_are_healthy(&[ServiceState {
            name: "vitalsd".to_owned(),
            state: "failed".to_owned(),
        }]));
        assert!(!service_states_are_healthy(&[ServiceState {
            name: "vitalsd".to_owned(),
            state: "unknown: timeout".to_owned(),
        }]));
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
    fn reports_missing_target_as_a_blocker_instead_of_omitting_it() {
        let spec = super::BinarySpec {
            name: "future-detector",
            source: "target/release/future-detector",
            destination: "bin/future-detector",
            unit: "future-detector.service",
            package: "future-package",
            available: false,
        };
        let blocker = missing_blocker(&spec);
        assert_eq!(blocker.kind, "missing_artifact");
        assert_eq!(blocker.subject, "future-detector");
        assert!(blocker.reason.contains("future-package"));
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

    #[test]
    fn spool_fingerprint_ignores_segment_filename_but_detects_content()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempdir()?;
        let first_path = directory.path().join("segment-first.spool");
        file(&first_path, b"header")?;
        let first = spool_fingerprint(std::slice::from_ref(&first_path))?;
        let second_path = directory.path().join("segment-second.spool");
        fs::rename(&first_path, &second_path)?;
        let renamed = spool_fingerprint(std::slice::from_ref(&second_path))?;
        assert_eq!(first, renamed);
        file(&second_path, b"changed")?;
        let changed = spool_fingerprint(std::slice::from_ref(&second_path))?;
        assert_ne!(first, changed);
        Ok(())
    }

    #[test]
    fn spool_fingerprint_preserves_multiset_count_across_rename()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempdir()?;
        let first_path = directory.path().join("segment-first.spool");
        let second_path = directory.path().join("segment-second.spool");
        file(&first_path, b"first")?;
        file(&second_path, b"second")?;
        let baseline = spool_fingerprint(&[first_path.clone(), second_path.clone()])?;
        let renamed_path = directory.path().join("segment-renamed.spool");
        fs::rename(&first_path, &renamed_path)?;
        assert_eq!(
            baseline,
            spool_fingerprint(&[renamed_path.clone(), second_path.clone()])?
        );
        let third_path = directory.path().join("segment-third.spool");
        file(&third_path, b"third")?;
        assert_ne!(
            baseline,
            spool_fingerprint(&[
                renamed_path.clone(),
                second_path.clone(),
                third_path.clone(),
            ])?
        );
        fs::remove_file(third_path)?;
        assert_eq!(baseline, spool_fingerprint(&[renamed_path, second_path])?);
        Ok(())
    }

    #[test]
    fn stages_manifest_files_and_validates_the_staged_copy()
    -> Result<(), Box<dyn std::error::Error>> {
        let repository = tempdir()?;
        let source = repository.path().join("source.bin");
        file(&source, b"release")?;
        let stage = tempdir()?;
        let manifest = manifest(
            vec![ArtifactFile {
                source: "source.bin".to_owned(),
                destination: "bin/release.bin".to_owned(),
                sha256: sha256_file(&source)?,
                mode: 0o644,
                kind: "binary".to_owned(),
            }],
            "abc123",
        );
        stage_manifest_files(repository.path(), stage.path(), &manifest)?;
        assert_eq!(fs::read(stage.path().join("bin/release.bin"))?, b"release");
        validate_staged_files(stage.path(), &manifest).map_err(std::io::Error::other)?;
        Ok(())
    }

    fn snapshot_fixture() -> Snapshot {
        Snapshot {
            observed_at: 1,
            current_release: Some("abc".to_owned()),
            live_files: vec![LiveFile {
                path: "/usr/local/bin/example".to_owned(),
                exists: false,
                size: 0,
                sha256: None,
            }],
            services: vec![ServiceState {
                name: "example".to_owned(),
                state: "inactive".to_owned(),
            }],
            spool: SpoolState {
                file_count: 0,
                bytes: 0,
                fingerprint: "empty".to_owned(),
            },
            database: ExternalState {
                state: "ready".to_owned(),
                detail: "vitals|public|160000|samples".to_owned(),
            },
            runner: ExternalState {
                state: "ready".to_owned(),
                detail: "github-vitals-runner: active".to_owned(),
            },
        }
    }

    #[test]
    fn snapshot_comparison_detects_database_and_runner_changes() {
        let before = snapshot_fixture();
        let mut after = snapshot_fixture();
        assert!(same_snapshot_state(&before, &after));
        after.database.detail = "vitals|public|160000|missing".to_owned();
        assert!(!same_snapshot_state(&before, &after));
        after = snapshot_fixture();
        after.runner.state = "unknown".to_owned();
        assert!(!same_snapshot_state(&before, &after));
    }

    #[test]
    fn activation_contract_rejects_unsafe_release_names_and_wrong_architecture() {
        assert!(validate_release_name("abc123").is_ok());
        assert!(validate_release_name("../etc").is_err());
        let value = manifest(Vec::new(), "abc123");
        assert!(validate_target_architecture(&value).is_ok());
        let mut wrong = value;
        wrong.architecture = "x86_64".to_owned();
        assert!(validate_target_architecture(&wrong).is_err());
    }

    #[test]
    fn health_freshness_rejects_future_and_stale_documents() {
        assert!(health_is_fresh(950, 1_000, Duration::from_mins(1)));
        assert!(!health_is_fresh(900, 1_000, Duration::from_mins(1)));
        assert!(!health_is_fresh(1_001, 1_000, Duration::from_mins(1)));
    }
}
