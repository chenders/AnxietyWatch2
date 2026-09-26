use crate::api::ops::{SysOps, sanitize_detail};
use crate::handlers::AppState;
use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;
use std::time::Duration;

/// Restart count at which a unit is considered to be in a crash loop.
pub const RESTART_STORM_THRESHOLD: u32 = 5;
const MAX_JOURNAL_LINES: usize = 50;
const MAX_JOURNAL_OUTPUT_BYTES: usize = 256 * 1024;
const SERVICE_QUERY_TIMEOUT: Duration = Duration::from_secs(5);

/// One sanitised journal line for a deployed unit.
#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
pub struct JournalLine {
    /// Journal timestamp as returned by journald.
    pub timestamp: String,
    /// Normalised journald severity name.
    pub severity: String,
    /// Sanitised message text.
    pub message: String,
    #[serde(skip)]
    systemd_origin: bool,
}

/// One systemd unit's process state and computed health.
#[derive(Debug, Serialize)]
pub struct ServiceState {
    /// Unit file name, e.g. `vitalsd.service`.
    pub name: String,
    /// systemd `ActiveState` property.
    pub active_state: String,
    /// systemd `SubState` property.
    pub sub_state: String,
    /// Independent computed process-state label.
    pub process_state: String,
    /// Health assertion separate from process state.
    pub health_state: String,
    /// Why the health assertion has its current value.
    pub health_reason: String,
    /// systemd `NRestarts` counter, absent when unmeasured.
    pub n_restarts: Option<u32>,
    /// systemd `ExecMainStartTimestamp` property, absent when unmeasured.
    pub exec_main_start_timestamp: Option<String>,
    /// Seconds since systemd's monotonic start timestamp, when measurable.
    pub uptime_seconds: Option<u64>,
    /// systemd `MemoryCurrent` in bytes, when measurable.
    pub memory_bytes: Option<u64>,
    /// Most recent error-level journal message, if one exists.
    pub last_error: Option<String>,
    /// Bounded, sanitised journal lines.
    pub journal: Vec<JournalLine>,
    /// Why journal evidence could not be measured, if applicable.
    pub journal_unmeasured_reason: Option<String>,
}

/// Filters for unit and severity in `GET /api/services`.
#[derive(Debug, Deserialize, Default)]
pub struct ServicesQuery {
    /// Exact deployment-manifest unit to return.
    pub unit: Option<String>,
    /// Exact severity name to retain in each unit's journal.
    pub severity: Option<String>,
}

#[derive(Debug, Default)]
struct ParsedProperties {
    active_state: Option<String>,
    sub_state: Option<String>,
    n_restarts: Option<u32>,
    exec_main_start_timestamp: Option<String>,
    active_enter_timestamp_monotonic: Option<u64>,
    memory_bytes: Option<u64>,
}

async fn run_service_command(ops: &SysOps, program: &str, args: &[&str]) -> Result<String, String> {
    match tokio::time::timeout(SERVICE_QUERY_TIMEOUT, (ops.cmd)(program, args)).await {
        Ok(result) => result,
        Err(_) => Err(format!("{program} command timed out after 5s")),
    }
}

fn parse_show_output(output: &str) -> Result<ParsedProperties, String> {
    let mut properties = ParsedProperties::default();
    for line in output.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key {
            "ActiveState" => properties.active_state = Some(value.to_string()),
            "SubState" => properties.sub_state = Some(value.to_string()),
            "NRestarts" => properties.n_restarts = value.parse().ok(),
            "ExecMainStartTimestamp" => {
                if !value.is_empty() {
                    properties.exec_main_start_timestamp = Some(value.to_string());
                }
            }
            "ActiveEnterTimestampMonotonic" => {
                properties.active_enter_timestamp_monotonic = value.parse().ok();
            }
            "MemoryCurrent" => {
                properties.memory_bytes = value.parse().ok();
            }
            _ => {}
        }
    }
    if properties.active_state.is_none() || properties.sub_state.is_none() {
        return Err("systemctl show omitted ActiveState or SubState".to_string());
    }
    Ok(properties)
}

fn parse_decimal_scaled(value: &str, fractional_digits: usize) -> Option<u64> {
    let (whole, fraction) = value.split_once('.').map_or((value, ""), |parts| parts);
    if value.contains('.')
        && (fraction.is_empty()
            || fraction.len() > fractional_digits
            || fraction.bytes().any(|byte| !byte.is_ascii_digit()))
    {
        return None;
    }
    let whole = whole.parse::<u64>().ok()?;
    let mut fraction_value = 0_u64;
    for byte in fraction.bytes() {
        fraction_value = fraction_value
            .checked_mul(10)?
            .checked_add(u64::from(byte - b'0'))?;
    }
    for _ in fraction.len()..fractional_digits {
        fraction_value = fraction_value.checked_mul(10)?;
    }
    whole
        .checked_mul(10_u64.checked_pow(u32::try_from(fractional_digits).ok()?)?)?
        .checked_add(fraction_value)
}

fn health_for(
    properties: &ParsedProperties,
    recent_restart_count: Option<u32>,
) -> (String, String) {
    let active_state = properties.active_state.as_deref().unwrap_or("unknown");
    if active_state == "failed" {
        return (
            "Failing".to_string(),
            format!("ActiveState=failed, NRestarts={:?}", properties.n_restarts),
        );
    }
    if active_state == "inactive" {
        return (
            "Failing".to_string(),
            "ActiveState=inactive, expected supervised unit is not running".to_string(),
        );
    }
    if recent_restart_count.is_some_and(|count| count >= RESTART_STORM_THRESHOLD) {
        return (
            "Failing".to_string(),
            format!(
                "restart storm: recent_restart_count={}, threshold={RESTART_STORM_THRESHOLD}",
                recent_restart_count.unwrap_or(0)
            ),
        );
    }
    let Some(n_restarts) = properties.n_restarts else {
        return (
            "Unknown".to_string(),
            "NRestarts is missing; restart-storm health is unmeasurable".to_string(),
        );
    };
    if n_restarts >= RESTART_STORM_THRESHOLD && recent_restart_count.is_none() {
        return (
            "Unknown".to_string(),
            format!(
                "recent restart window is unmeasured; cumulative NRestarts={n_restarts}, threshold={RESTART_STORM_THRESHOLD}"
            ),
        );
    }
    match active_state {
        "active" => (
            "Live".to_string(),
            format!(
                "ActiveState=active, NRestarts={n_restarts}; recent_restart_count={}",
                recent_restart_count
                    .map_or_else(|| "unmeasured".to_string(), |count| count.to_string())
            ),
        ),
        "activating" | "deactivating" => (
            "Unknown".to_string(),
            format!("ActiveState={active_state}, completion is unmeasured"),
        ),
        other => (
            "Unknown".to_string(),
            format!("ActiveState={other} is not a recognised process state"),
        ),
    }
}

fn parse_priority(value: &str) -> Option<&'static str> {
    match value {
        "0" => Some("emerg"),
        "1" => Some("alert"),
        "2" => Some("crit"),
        "3" => Some("err"),
        "4" => Some("warning"),
        "5" => Some("notice"),
        "6" => Some("info"),
        "7" => Some("debug"),
        _ => None,
    }
}

fn parse_journal(output: &str) -> Result<Vec<JournalLine>, String> {
    if output.len() > MAX_JOURNAL_OUTPUT_BYTES {
        return Err(format!(
            "journal output is {} bytes, maximum is {MAX_JOURNAL_OUTPUT_BYTES}",
            output.len()
        ));
    }
    let mut lines = Vec::new();
    for (index, line) in output.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(line)
            .map_err(|error| format!("journal line {index} is not JSON: {error}"))?;
        let timestamp = value
            .get("__REALTIME_TIMESTAMP")
            .and_then(Value::as_str)
            .map_or_else(
                || "unmeasured".to_string(),
                |value| sanitize_detail(value, None),
            );
        let severity = value
            .get("PRIORITY")
            .and_then(Value::as_str)
            .and_then(parse_priority)
            .ok_or_else(|| format!("journal line {index} has invalid PRIORITY"))?
            .to_string();
        let message = value
            .get("MESSAGE")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("journal line {index} has no MESSAGE"))?;
        let systemd_origin = value
            .get("SYSLOG_IDENTIFIER")
            .and_then(Value::as_str)
            .is_some_and(|identifier| identifier == "systemd");
        lines.push(JournalLine {
            timestamp,
            severity,
            message: sanitize_detail(message, None),
            systemd_origin,
        });
        if lines.len() == MAX_JOURNAL_LINES {
            break;
        }
    }
    Ok(lines)
}

fn uptime_seconds(properties: &ParsedProperties, uptime: Option<u64>) -> Option<u64> {
    let current = uptime?;
    let started = properties.active_enter_timestamp_monotonic?;
    current
        .checked_sub(started)
        .map(|elapsed_microseconds| elapsed_microseconds / 1_000_000)
}

fn state_from_properties(
    name: String,
    properties: ParsedProperties,
    uptime: Option<u64>,
    journal: Result<Vec<JournalLine>, String>,
) -> ServiceState {
    let active_state = properties
        .active_state
        .clone()
        .unwrap_or_else(|| "unknown".to_string());
    let sub_state = properties
        .sub_state
        .clone()
        .unwrap_or_else(|| "unknown".to_string());
    let uptime_seconds = uptime_seconds(&properties, uptime);
    let recent_restart_count = journal.as_ref().ok().and_then(|lines| {
        let count = lines
            .iter()
            .filter(|line| {
                line.systemd_origin
                    && (line.message.contains("Scheduled restart job")
                        || line.message.contains("restart counter is"))
            })
            .count();
        u32::try_from(count).ok()
    });
    let (health_state, health_reason) = health_for(&properties, recent_restart_count);
    let process_state = format!("{active_state}/{sub_state}");
    let (journal, journal_unmeasured_reason) = match journal {
        Ok(lines) => (lines, None),
        Err(reason) => (Vec::new(), Some(sanitize_detail(&reason, None))),
    };
    let last_error = journal
        .iter()
        .rev()
        .find(|line| matches!(line.severity.as_str(), "emerg" | "alert" | "crit" | "err"))
        .map(|line| line.message.clone());
    ServiceState {
        name,
        active_state,
        sub_state,
        process_state,
        health_state,
        health_reason,
        n_restarts: properties.n_restarts,
        exec_main_start_timestamp: properties.exec_main_start_timestamp,
        uptime_seconds,
        memory_bytes: properties.memory_bytes,
        last_error,
        journal,
        journal_unmeasured_reason,
    }
}

fn unknown_state(name: String, reason: &str) -> ServiceState {
    let reason = sanitize_detail(reason, None);
    ServiceState {
        name,
        active_state: "unknown".to_string(),
        sub_state: "unknown".to_string(),
        process_state: "unknown".to_string(),
        health_state: "Unknown".to_string(),
        health_reason: reason.clone(),
        n_restarts: None,
        exec_main_start_timestamp: None,
        uptime_seconds: None,
        memory_bytes: None,
        last_error: None,
        journal: Vec::new(),
        journal_unmeasured_reason: Some(reason),
    }
}

/// Collect unit states, computed health, bounded logs, and diagnostics.
///
/// # Errors
///
/// Returns an error when the deployment manifest cannot be inspected. A
/// per-unit command or parse failure is retained as explicit `Unknown` state.
pub async fn collect_service_states(
    ops: &SysOps,
    deploy_dir: &str,
) -> Result<Vec<ServiceState>, String> {
    let paths = (ops.glob_paths)(&format!("{deploy_dir}/*.service"))
        .map_err(|error| format!("glob {deploy_dir}: {error}"))?;
    let uptime = (ops.read_string)("/proc/uptime")
        .await
        .ok()
        .and_then(|value| value.split_whitespace().next().map(str::to_string))
        .and_then(|value| parse_decimal_scaled(&value, 6));
    let mut services = Vec::new();
    for path in paths {
        let Some(name) = Path::new(&path)
            .file_name()
            .and_then(|value| value.to_str())
        else {
            continue;
        };
        let name = name.to_string();
        let properties = match run_service_command(
            ops,
            "systemctl",
            &[
                "show",
                "-p",
                "ActiveState,SubState,NRestarts,ExecMainStartTimestamp,ActiveEnterTimestampMonotonic,MemoryCurrent",
                &name,
            ],
        )
        .await
        {
            Ok(output) => match parse_show_output(&output) {
                Ok(properties) => properties,
                Err(reason) => {
                    services.push(unknown_state(name, &reason));
                    continue;
                }
            },
            Err(error) => {
                let reason = format!("systemctl show failed: {error}");
                services.push(unknown_state(name, &reason));
                continue;
            }
        };
        let journal = run_service_command(
            ops,
            "journalctl",
            &[
                "-u",
                &name,
                "-n",
                "50",
                "--since=-1h",
                "--reverse",
                "--no-pager",
                "-o",
                "json",
            ],
        )
        .await
        .and_then(|output| parse_journal(&output));
        services.push(state_from_properties(name, properties, uptime, journal));
    }
    Ok(services)
}

/// GET `/api/services`, with optional exact unit and severity filters.
///
/// # Errors
///
/// 400 for an unsupported severity; 500 if the deployment directory cannot
/// be inspected.
pub async fn services_handler(
    State(state): State<AppState>,
    Query(query): Query<ServicesQuery>,
) -> Result<Json<Vec<ServiceState>>, (StatusCode, String)> {
    let severity_filter = query.severity.as_deref().map(str::to_ascii_lowercase);
    if let Some(severity) = &severity_filter
        && !matches!(
            severity.as_str(),
            "emerg" | "alert" | "crit" | "err" | "warning" | "notice" | "info" | "debug"
        )
    {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("services filter blocked: severity={severity} is unsupported"),
        ));
    }
    let deploy_dir = std::env::var("DEPLOY_DIR").unwrap_or_else(|_| "deploy".to_string());
    let states = collect_service_states(&state.sys_ops, &deploy_dir)
        .await
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error))?;
    let states = states
        .into_iter()
        .filter(|service| {
            query
                .unit
                .as_deref()
                .is_none_or(|unit| service.name == unit)
        })
        .map(|mut service| {
            if let Some(severity) = &severity_filter {
                service.journal.retain(|line| &line.severity == severity);
            }
            service
        })
        .collect();
    Ok(Json(states))
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::expect_used,
        clippy::unwrap_used,
        reason = "service tests use fixed systemd and journal fixtures"
    )]

    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    fn fixture_ops() -> SysOps {
        SysOps {
            cmd: |prog, args| {
                let prog = prog.to_string();
                let args = args.iter().map(ToString::to_string).collect::<Vec<_>>();
                Box::pin(async move {
                    if prog == "systemctl" {
                        Ok("ActiveState=active\nSubState=running\nNRestarts=1\nExecMainStartTimestamp=Mon 2021-01-01\nActiveEnterTimestampMonotonic=1000000\nMemoryCurrent=2048\n".to_string())
                    } else if prog == "journalctl" {
                        Ok(r#"{"__REALTIME_TIMESTAMP":"2021-01-01T00:00:00Z","PRIORITY":"3","MESSAGE":"failure token=secret"}"#.to_string())
                    } else {
                        Err(format!("unexpected command {prog} {args:?}"))
                    }
                })
            },
            read_string: |path| {
                let path = path.to_string();
                Box::pin(async move {
                    if path == "/proc/uptime" {
                        Ok("12.500 0.0".to_string())
                    } else {
                        Err("not found".to_string())
                    }
                })
            },
            path_exists: |_| false,
            stat_vfs: |_| Ok((0, 0)),
            glob_exists: |_| false,
            glob_paths: |pattern| {
                let pattern = pattern.to_string();
                Ok(glob::glob(&pattern)
                    .unwrap()
                    .map(|path| path.unwrap().to_string_lossy().to_string())
                    .collect())
            },
            file_mtime: |_| Err("not found".to_string()),
        }
    }

    #[tokio::test]
    async fn services_manifest_state_has_separate_health_and_process_axes() {
        let directory = tempdir().unwrap();
        File::create(directory.path().join("vitals-a.service")).unwrap();
        let states = collect_service_states(&fixture_ops(), directory.path().to_str().unwrap())
            .await
            .unwrap();
        assert_eq!(states.len(), 1);
        let state = states.first().expect("one manifest state");
        assert_eq!(state.process_state, "active/running");
        assert_eq!(state.health_state, "Live");
        assert_eq!(state.n_restarts, Some(1));
        assert_eq!(state.memory_bytes, Some(2048));
        assert_eq!(state.uptime_seconds, Some(11));
        assert_eq!(state.journal.len(), 1);
        assert_eq!(
            state.last_error.as_deref(),
            Some("failure token=[redacted]")
        );
    }

    #[test]
    fn services_restart_storm_is_failing() {
        let properties = parse_show_output(&format!(
            "ActiveState=active\nSubState=running\nNRestarts={RESTART_STORM_THRESHOLD}\n"
        ))
        .unwrap();
        let (state, reason) = health_for(&properties, Some(RESTART_STORM_THRESHOLD));
        assert_eq!(state, "Failing");
        assert!(reason.contains("threshold=5"));
    }

    #[test]
    fn services_old_cumulative_restarts_do_not_claim_a_current_storm() {
        let properties =
            parse_show_output("ActiveState=active\nSubState=running\nNRestarts=5\n").unwrap();
        let (state, _) = health_for(&properties, Some(0));
        assert_eq!(state, "Live");

        let failed = parse_show_output("ActiveState=failed\nSubState=failed\n").unwrap();
        let (failed_state, _) = health_for(&failed, None);
        assert_eq!(failed_state, "Failing");

        let recent_storm =
            parse_show_output("ActiveState=active\nSubState=running\nNRestarts=1\n").unwrap();
        let (storm_state, _) = health_for(&recent_storm, Some(RESTART_STORM_THRESHOLD));
        assert_eq!(storm_state, "Failing");
    }

    #[test]
    fn services_unknown_command_or_parse_is_unknown_not_empty_success() {
        let unknown = unknown_state("vitals-a.service".to_string(), "command failed");
        assert_eq!(unknown.health_state, "Unknown");
        assert_eq!(unknown.process_state, "unknown");
        assert!(unknown.journal.is_empty());
        assert!(unknown.journal_unmeasured_reason.is_some());
        assert!(parse_show_output("ActiveState=active\n").is_err());
    }

    #[test]
    fn sanitize_token_password_url_and_user_path() {
        let detail = sanitize_detail(
            "token=\"abc def\" password: \"abc def\" --url=https://example.test/path path=/home/user/private Authorization: Bearer secret api_key=key",
            None,
        );
        assert!(!detail.contains("abc"));
        assert!(!detail.contains("https://"));
        assert!(!detail.contains("/home/user"));
        assert!(!detail.contains("secret"));
        assert!(!detail.contains("=key"));
        assert!(detail.contains("api_key=[redacted]"));
        assert!(detail.contains("[redacted-url]"));
        assert!(detail.contains("[redacted-path]"));
    }

    #[test]
    fn services_uptime_parser_rejects_malformed_values() {
        assert_eq!(parse_decimal_scaled("12.500", 6), Some(12_500_000));
        assert_eq!(parse_decimal_scaled("12.", 6), None);
        assert_eq!(parse_decimal_scaled("12.1234567", 6), None);
        assert_eq!(parse_decimal_scaled("bad", 6), None);
    }
}
