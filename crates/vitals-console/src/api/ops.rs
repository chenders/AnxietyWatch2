use axum::Json;
use axum::extract::{Path as AxumPath, State};
use axum::http::{HeaderMap, Method, StatusCode};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::path::Path;
use std::time::Duration;
use subtle::ConstantTimeEq;

use crate::db;
use crate::handlers::AppState;
use crate::models::{Effect, ErrorResponse, error_reply};
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
                        .kill_on_drop(true)
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

/// Header carrying the second, write-only service-operation capability.
pub const OPS_TOKEN_HEADER: &str = "x-anxietywatch-ops-token";
/// Attempt duration used for the bounded systemd operation window.
pub const SERVICE_OPERATION_EXPECTED_DURATION_S: i32 = 30;
const MAX_DETAIL_BYTES: usize = 512;
const SERVICE_OPERATION_TIMEOUT: Duration =
    Duration::from_secs(SERVICE_OPERATION_EXPECTED_DURATION_S as u64);

/// The only systemd actions exposed by the console.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ServiceAction {
    /// Ask systemd to restart one exact deployed unit.
    Restart,
    /// Ask systemd to stop one exact deployed unit.
    Stop,
}

impl ServiceAction {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "restart" => Ok(Self::Restart),
            "stop" => Ok(Self::Stop),
            _ => Err(format!(
                "service operation blocked: action={value} is not one of restart|stop"
            )),
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::Restart => "restart",
            Self::Stop => "stop",
        }
    }
}

/// Confirmation level supplied by the admin UI for a destructive operation.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConfirmationClass {
    /// Normal confirmation for a routine restart.
    Standard,
    /// Explicit confirmation for a high-consequence operation.
    Strong,
}

/// JSON body of a service operation request.
#[derive(Debug, Deserialize)]
pub struct ServiceOperationRequest {
    /// Confirmation class selected after the UI displays the consequence.
    pub confirmation_class: ConfirmationClass,
}

/// JSON response after a service operation has been recorded.
#[derive(Debug, Serialize)]
pub struct ServiceOperationResponse {
    /// Exact deployment-manifest unit acted on.
    pub unit: String,
    /// The allowlisted action executed.
    pub action: ServiceAction,
    /// Database attempt id for this operation.
    pub attempt_id: i64,
    /// Final attempt outcome.
    pub outcome: &'static str,
    /// Sanitised executor detail, if any.
    pub detail: Option<String>,
    /// Effect breakdown recorded with the attempt.
    pub effect: Effect,
}

/// Return exact service-unit basenames from the deployment manifests.
///
/// # Errors
///
/// Returns a named glob failure; no service command is run on this path.
pub fn deployment_unit_names(ops: &SysOps, deploy_dir: &str) -> Result<Vec<String>, String> {
    let pattern = format!("{deploy_dir}/*.service");
    let paths = (ops.glob_paths)(&pattern).map_err(|error| {
        format!("service operation blocked: cannot inspect deployment manifests {pattern}: {error}")
    })?;
    let mut names = paths
        .into_iter()
        .filter_map(|path| Path::new(&path).file_name()?.to_str().map(str::to_owned))
        .collect::<Vec<_>>();
    names.sort();
    names.dedup();
    Ok(names)
}

fn validate_unit_name(unit: &str, allowed_units: &[String]) -> Result<(), String> {
    if unit.is_empty()
        || !unit.ends_with(".service")
        || unit.contains('/')
        || unit.contains('\\')
        || unit.contains("..")
        || unit.contains('@')
        || unit.contains('*')
        || unit.contains('?')
    {
        return Err(format!(
            "service operation blocked: unit={unit} is not an exact non-template deployment unit"
        ));
    }
    if allowed_units.iter().all(|allowed| allowed != unit) {
        return Err(format!(
            "service operation blocked: unit={unit} is not present in the deployment manifests"
        ));
    }
    Ok(())
}

fn requires_strong_confirmation(unit: &str, action: ServiceAction) -> bool {
    matches!(action, ServiceAction::Stop)
        || matches!(
            unit,
            "vitalsd.service"
                | "vitals-spool-drain.service"
                | "vitals-h10.service"
                | "vitals-bp.service"
        )
}

fn token_matches(expected: Option<&str>, supplied: Option<&str>) -> bool {
    let Some(expected) = expected.filter(|value| !value.is_empty()) else {
        return false;
    };
    let Some(supplied) = supplied.filter(|value| !value.is_empty()) else {
        return false;
    };
    expected.as_bytes().ct_eq(supplied.as_bytes()).into()
}

fn validate_capability(
    expected: Option<&str>,
    headers: &HeaderMap,
    unit: &str,
    action: ServiceAction,
) -> Result<(), String> {
    let supplied = headers
        .get(OPS_TOKEN_HEADER)
        .and_then(|value| value.to_str().ok());
    if token_matches(expected, supplied) {
        Ok(())
    } else if expected.is_none() {
        Err(format!(
            "service operation blocked: {unit}/{} has no configured VITALS_CONSOLE_OPS_TOKEN",
            action.as_str()
        ))
    } else if supplied.is_none() {
        Err(format!(
            "service operation blocked: {unit}/{} is missing {OPS_TOKEN_HEADER}",
            action.as_str()
        ))
    } else {
        Err(format!(
            "service operation blocked: {unit}/{} supplied an invalid {OPS_TOKEN_HEADER}",
            action.as_str()
        ))
    }
}

/// Sanitize executor detail before storing or returning it.
#[must_use]
pub fn sanitize_detail(detail: &str, sensitive: Option<&str>) -> String {
    let redacted = sensitive.filter(|value| !value.is_empty()).map_or_else(
        || detail.to_string(),
        |value| detail.replace(value, "[redacted]"),
    );
    let mut sanitized = String::new();
    for character in redacted.chars() {
        let character = if character.is_control() && !matches!(character, '\n' | '\r' | '\t') {
            ' '
        } else {
            character
        };
        if sanitized.len().saturating_add(character.len_utf8()) > MAX_DETAIL_BYTES {
            break;
        }
        sanitized.push(character);
    }
    sanitized.trim().to_string()
}

/// Execute one already-validated service action without a shell.
///
/// # Errors
///
/// Returns the executor's stderr/stdout detail; callers must sanitize it
/// before persistence or rendering.
pub async fn execute_service_action(
    ops: &SysOps,
    unit: &str,
    action: ServiceAction,
) -> Result<(), String> {
    let args = [action.as_str(), unit];
    (ops.cmd)("systemctl", &args).await.map(|_| ())
}

/// POST `/api/services/{unit}/{action}`.
///
/// Authentication and CSRF are enforced by T-M2a's route middleware. This
/// handler adds the second capability, exact manifest allowlist, confirmation,
/// attempt lifecycle, executor call, and immutable audit record.
///
/// # Errors
///
/// Returns a named 400/403/409/502/503 blocker or database failure.
#[allow(
    clippy::too_many_lines,
    reason = "the guarded operation keeps validation, execution, attempt, and audit ordering together"
)]
pub async fn service_operation_handler(
    State(state): State<AppState>,
    AxumPath((unit, action_value)): AxumPath<(String, String)>,
    headers: HeaderMap,
    Json(request): Json<ServiceOperationRequest>,
) -> Result<(StatusCode, Json<ServiceOperationResponse>), (StatusCode, Json<ErrorResponse>)> {
    state
        .security
        .authenticate(&headers, &Method::POST, crate::security::unix_now())
        .map_err(|error| crate::security::security_reply(&error))?;
    let action = ServiceAction::parse(&action_value)
        .map_err(|reason| error_reply(StatusCode::BAD_REQUEST, reason))?;
    let deploy_dir = std::env::var("DEPLOY_DIR").unwrap_or_else(|_| "deploy".to_string());
    let allowed_units = deployment_unit_names(&state.sys_ops, &deploy_dir)
        .map_err(|reason| error_reply(StatusCode::INTERNAL_SERVER_ERROR, reason))?;
    validate_unit_name(&unit, &allowed_units)
        .map_err(|reason| error_reply(StatusCode::BAD_REQUEST, reason))?;
    if unit == "vitals-console.service" && matches!(action, ServiceAction::Stop) {
        return Err(error_reply(
            StatusCode::CONFLICT,
            "service operation blocked: action=stop unit=vitals-console.service requires a separate local-only maintenance contract",
        ));
    }
    if requires_strong_confirmation(&unit, action)
        && request.confirmation_class != ConfirmationClass::Strong
    {
        return Err(error_reply(
            StatusCode::BAD_REQUEST,
            format!(
                "service operation blocked: {unit}/{} requires confirmation_class=strong, got {:?}",
                action.as_str(),
                request.confirmation_class
            ),
        ));
    }
    validate_capability(state.ops_token.as_deref(), &headers, &unit, action)
        .map_err(|reason| error_reply(StatusCode::FORBIDDEN, reason))?;

    let db = state.db.as_ref().ok_or_else(|| {
        error_reply(
            StatusCode::SERVICE_UNAVAILABLE,
            "service operation blocked: database is unavailable before attempt creation",
        )
    })?;
    let subject = format!("{unit}/{}", action.as_str());
    let attempt_id = db::create_attempt(
        db,
        "service_operation",
        &subject,
        SERVICE_OPERATION_EXPECTED_DURATION_S,
    )
    .await?;

    let execution = tokio::time::timeout(
        SERVICE_OPERATION_TIMEOUT,
        execute_service_action(&state.sys_ops, &unit, action),
    )
    .await;
    let (outcome, effect, detail) = match execution {
        Ok(Ok(())) => (
            "succeeded",
            Effect {
                updated: 1,
                ..Effect::default()
            },
            None,
        ),
        Ok(Err(error)) => (
            "failed",
            Effect::default(),
            Some(sanitize_detail(&error, state.ops_token.as_deref())),
        ),
        Err(_) => (
            "failed",
            Effect::default(),
            Some(format!(
                "systemd operation exceeded {SERVICE_OPERATION_EXPECTED_DURATION_S}s; result is unconfirmed and the systemd job may still be running"
            )),
        ),
    };
    let confirmation_class = match request.confirmation_class {
        ConfirmationClass::Standard => "standard",
        ConfirmationClass::Strong => "strong",
    };
    // T-M2a deliberately has one Argon2id operator identity and no username
    // field; this stable actor id is the authenticated principal for now.
    let audit = db::ServiceOperationAudit {
        actor: crate::security::AUTHENTICATED_ACTOR,
        target: &unit,
        action: action.as_str(),
        confirmation_class,
        attempt_id,
        outcome,
        detail: detail.as_deref(),
    };
    db::finish_service_operation_attempt(db, &audit, &effect).await?;

    if outcome != "succeeded" {
        return Err(error_reply(
            StatusCode::BAD_GATEWAY,
            format!(
                "service operation {outcome}: unit={unit} action={} detail={}",
                action.as_str(),
                detail.as_deref().unwrap_or("executor returned no detail")
            ),
        ));
    }
    Ok((
        StatusCode::OK,
        Json(ServiceOperationResponse {
            unit,
            action,
            attempt_id,
            outcome,
            detail,
            effect,
        }),
    ))
}

#[cfg(test)]
mod service_operation_tests {
    #![allow(
        clippy::expect_used,
        clippy::unwrap_used,
        reason = "service-operation tests use fixed manifest and header fixtures"
    )]

    use super::*;

    #[test]
    fn service_operation_action_allowlist_is_exact() {
        assert_eq!(ServiceAction::parse("restart"), Ok(ServiceAction::Restart));
        assert_eq!(ServiceAction::parse("stop"), Ok(ServiceAction::Stop));
        assert!(ServiceAction::parse("status").is_err());
    }

    #[test]
    fn service_operation_unit_allowlist_rejects_paths_aliases_and_templates() {
        let allowed = vec!["vitalsd.service".to_string()];
        assert!(validate_unit_name("vitalsd.service", &allowed).is_ok());
        assert!(validate_unit_name("../vitalsd.service", &allowed).is_err());
        assert!(validate_unit_name("vitalsd@.service", &allowed).is_err());
        assert!(validate_unit_name("vitalsd.service", &[]).is_err());
    }

    #[test]
    fn service_operation_token_comparison_is_constant_time_api() {
        assert!(token_matches(Some("secret"), Some("secret")));
        assert!(!token_matches(Some("secret"), Some("secreT")));
        assert!(!token_matches(Some("secret"), None));
        assert!(!token_matches(None, Some("secret")));
    }

    #[test]
    fn service_operation_confirmation_scales_with_consequence() {
        assert!(!requires_strong_confirmation(
            "lyrics.service",
            ServiceAction::Restart
        ));
        assert!(requires_strong_confirmation(
            "lyrics.service",
            ServiceAction::Stop
        ));
        assert!(requires_strong_confirmation(
            "vitalsd.service",
            ServiceAction::Restart
        ));
    }

    #[tokio::test]
    async fn service_operation_executor_passes_only_allowlisted_systemctl_args() {
        fn fake_command(
            prog: &str,
            args: &[&str],
        ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send>> {
            assert_eq!(prog, "systemctl");
            assert_eq!(args, ["restart", "lyrics.service"]);
            Box::pin(async { Ok(String::new()) })
        }

        let ops = SysOps {
            cmd: fake_command,
            read_string: |_| Box::pin(async { Ok(String::new()) }),
            path_exists: |_| false,
            stat_vfs: |_| Ok((0, 0)),
            glob_exists: |_| false,
            glob_paths: |_| Ok(Vec::new()),
            file_mtime: |_| Err(String::new()),
        };
        assert!(
            execute_service_action(&ops, "lyrics.service", ServiceAction::Restart)
                .await
                .is_ok()
        );
    }

    #[test]
    fn service_operation_detail_is_bounded_and_redacts_capability() {
        let detail = sanitize_detail("token=secret\u{0}\n/path/to/user", Some("secret"));
        assert!(!detail.contains("secret"));
        assert!(!detail.contains('\u{0}'));
        assert!(detail.len() <= MAX_DETAIL_BYTES);
    }
}
