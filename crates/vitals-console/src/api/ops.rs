use std::future::Future;
use std::path::Path;
use std::pin::Pin;

type AsyncCommandFn =
    fn(&str, &[&str]) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send>>;
type AsyncReadStringFn = fn(&str) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send>>;
type SyncPathExistsFn = fn(&str) -> bool;
type StatVfsFn = fn(&str) -> Result<(u64, u64), String>;
type GlobExistsFn = fn(&str) -> bool;
type GlobPathsFn = fn(&str) -> Result<Vec<String>, String>;
type FileMtimeFn = fn(&str) -> Result<std::time::SystemTime, String>;

/// System primitives injected into the host-status collectors.
///
/// Every collector takes a `SysOps` instead of touching the host directly, so
/// a test can substitute fakes that never run a real command.
#[derive(Clone)]
pub struct SysOps {
    /// Run a program and return its stdout, or its stderr on a non-zero exit.
    pub cmd: AsyncCommandFn,
    /// Read a file's contents as UTF-8.
    pub read_string: AsyncReadStringFn,
    /// Whether a path exists.
    pub path_exists: SyncPathExistsFn,
    /// Free and total bytes of a filesystem; errors on non-Linux hosts.
    pub stat_vfs: StatVfsFn,
    /// Whether any path matches a glob pattern.
    pub glob_exists: GlobExistsFn,
    /// The paths matching a glob pattern.
    pub glob_paths: GlobPathsFn,
    /// Last modification time of a file.
    pub file_mtime: FileMtimeFn,
}

impl SysOps {
    /// The production implementation backed by the real system.
    #[must_use]
    pub fn real() -> Self {
        Self {
            cmd: |prog, args| {
                let prog = prog.to_string();
                let args = args
                    .iter()
                    .map(std::string::ToString::to_string)
                    .collect::<Vec<_>>();
                Box::pin(async move {
                    let out = tokio::process::Command::new(&prog)
                        .args(&args)
                        .output()
                        .await
                        .map_err(|e| e.to_string())?;
                    if out.status.success() {
                        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
                    } else {
                        Err(String::from_utf8_lossy(&out.stderr).into_owned())
                    }
                })
            },
            read_string: |path| {
                let path = path.to_string();
                Box::pin(async move {
                    tokio::fs::read_to_string(&path)
                        .await
                        .map_err(|e| e.to_string())
                })
            },
            path_exists: |path| Path::new(path).exists(),
            stat_vfs: |path| {
                #[cfg(target_os = "linux")]
                {
                    let stat = rustix::fs::statvfs(path).map_err(|e| e.to_string())?;
                    Ok((stat.f_bavail * stat.f_frsize, stat.f_blocks * stat.f_frsize))
                }
                #[cfg(not(target_os = "linux"))]
                {
                    let _ = path;
                    Err("not available on this host".to_string())
                }
            },
            glob_exists: |pattern| {
                glob::glob(pattern).is_ok_and(|mut paths| paths.next().is_some())
            },
            glob_paths: |pattern| {
                glob::glob(pattern)
                    .map(|paths| {
                        paths
                            .filter_map(Result::ok)
                            .map(|p| p.to_string_lossy().into_owned())
                            .collect()
                    })
                    .map_err(|e| e.to_string())
            },
            file_mtime: |path| {
                std::fs::metadata(path)
                    .and_then(|m| m.modified())
                    .map_err(|e| e.to_string())
            },
        }
    }
}
