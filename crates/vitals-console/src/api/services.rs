use crate::api::ops::SysOps;
use crate::handlers::AppState;
use axum::{Json, extract::State, http::StatusCode};
use serde::Serialize;

/// One systemd unit's state as reported by `systemctl show`.
#[derive(Serialize)]
pub struct ServiceState {
    /// Unit file name, e.g. `vitalsd.service`.
    pub name: String,
    /// systemd `ActiveState` property.
    pub active_state: String,
    /// systemd `SubState` property.
    pub sub_state: String,
    /// systemd `NRestarts` counter.
    pub n_restarts: u32,
    /// systemd `ExecMainStartTimestamp` property.
    pub exec_main_start_timestamp: String,
}

/// Collect unit states for every `*.service` file in the deploy directory.
///
/// # Errors
/// Returns an error string if the deploy directory cannot be globbed, naming
/// the directory and the underlying error.
pub async fn collect_service_states(
    ops: &SysOps,
    deploy_dir: &str,
) -> Result<Vec<ServiceState>, String> {
    let mut services = Vec::new();

    let paths = (ops.glob_paths)(&format!("{deploy_dir}/*.service"))
        .map_err(|e| format!("glob {deploy_dir}: {e}"))?;

    for path in paths {
        if let Some(name) = path.split('/').next_back() {
            let output = (ops.cmd)(
                "systemctl",
                &[
                    "show",
                    "-p",
                    "ActiveState,SubState,NRestarts,ExecMainStartTimestamp",
                    name,
                ],
            )
            .await
            .unwrap_or_default();

            let mut active_state = String::new();
            let mut sub_state = String::new();
            let mut n_restarts = 0;
            let mut exec_main_start_timestamp = String::new();

            for line in output.lines() {
                if let Some(val) = line.strip_prefix("ActiveState=") {
                    active_state = val.to_string();
                } else if let Some(val) = line.strip_prefix("SubState=") {
                    sub_state = val.to_string();
                } else if let Some(val) = line.strip_prefix("NRestarts=") {
                    n_restarts = val.parse().unwrap_or(0);
                } else if let Some(val) = line.strip_prefix("ExecMainStartTimestamp=") {
                    exec_main_start_timestamp = val.to_string();
                }
            }

            services.push(ServiceState {
                name: name.to_string(),
                active_state,
                sub_state,
                n_restarts,
                exec_main_start_timestamp,
            });
        }
    }

    Ok(services)
}

/// GET /api/services
///
/// Serves unit states for the units deployed under `DEPLOY_DIR` (default
/// `deploy`).
///
/// # Errors
/// 500 if the deploy directory cannot be globbed.
pub async fn services_handler(
    State(state): State<AppState>,
) -> Result<Json<Vec<ServiceState>>, (StatusCode, String)> {
    let deploy_dir = std::env::var("DEPLOY_DIR").unwrap_or_else(|_| "deploy".to_string());

    let states = collect_service_states(&state.sys_ops, &deploy_dir)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    Ok(Json(states))
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::unwrap_used,
        reason = "unit tests use fixed fixtures in a tempdir; unwrap is the assertion that setup and the handler call succeed"
    )]
    #![expect(
        clippy::indexing_slicing,
        reason = "tests index collected vectors and the stub systemctl output"
    )]

    use super::*;
    use crate::api::ops::SysOps;
    use std::fs::File;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_manifest_derived_unit_list() {
        let dir = tempdir().unwrap();
        let deploy_dir = dir.path().to_str().unwrap();

        File::create(dir.path().join("vitals-a.service")).unwrap();
        File::create(dir.path().join("vitals-b.service")).unwrap();
        File::create(dir.path().join("ignore.txt")).unwrap();

        let ops = SysOps {
            cmd: |prog, args| {
                let prog = prog.to_string();
                let args = args
                    .iter()
                    .map(std::string::ToString::to_string)
                    .collect::<Vec<_>>();
                Box::pin(async move {
                    if prog == "systemctl" && args.len() > 3 {
                        let name = &args[3];
                        if name == "vitals-a.service" {
                            Ok("ActiveState=active\nSubState=running\nNRestarts=0\nExecMainStartTimestamp=Mon 2021-01-01\n".to_string())
                        } else {
                            Ok("ActiveState=inactive\n".to_string())
                        }
                    } else {
                        Ok(String::new())
                    }
                })
            },
            read_string: |_| Box::pin(async move { Ok(String::new()) }),
            path_exists: |_| false,
            stat_vfs: |_| Ok((0, 0)),
            glob_exists: |_| false,
            glob_paths: |pattern| {
                let pattern = pattern.to_string();
                Ok(glob::glob(&pattern)
                    .unwrap()
                    .map(|p| p.unwrap().to_string_lossy().to_string())
                    .collect())
            },
            file_mtime: |_| Err(String::new()),
        };

        let mut states = collect_service_states(&ops, deploy_dir).await.unwrap();
        states.sort_by(|a, b| a.name.cmp(&b.name));

        assert_eq!(states.len(), 2);
        assert_eq!(states[0].name, "vitals-a.service");
        assert_eq!(states[0].active_state, "active");
        assert_eq!(states[1].name, "vitals-b.service");
        assert_eq!(states[1].active_state, "inactive");
    }
}
