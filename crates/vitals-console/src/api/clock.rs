use crate::api::ops::SysOps;
use crate::handlers::AppState;
use axum::{Json, extract::State, http::StatusCode};
use serde::Serialize;

/// Clock synchronization state as reported by `timedatectl`.
#[derive(Serialize)]
pub struct ClockInfo {
    /// Whether the system reports NTP synchronization active.
    pub ntp_synchronized: bool,
    /// System clock time, microseconds since the Unix epoch.
    pub time_usec: Option<u64>,
    /// Hardware RTC time, microseconds since the Unix epoch.
    pub rtc_time_usec: Option<u64>,
    /// RTC minus system clock, microseconds; positive means the RTC runs ahead.
    pub drift_usec: Option<i64>,
}

/// Collect clock state from `timedatectl show`.
///
/// Missing or unparseable output degrades to `false`/`None` rather than
/// failing, so a broken `timedatectl` never hides the other fields.
///
/// # Errors
/// Never returns an error: command and parse failures are swallowed by
/// `unwrap_or_default`/`ok()` and reported as absent fields.
pub async fn collect_clock_info(ops: &SysOps) -> Result<ClockInfo, String> {
    let out = (ops.cmd)("timedatectl", &["show"])
        .await
        .unwrap_or_default();

    let mut ntp_synchronized = false;
    let mut time_usec = None;
    let mut rtc_time_usec = None;

    for line in out.lines() {
        if let Some(val) = line.strip_prefix("NTPSynchronized=") {
            ntp_synchronized = val == "yes";
        } else if let Some(val) = line.strip_prefix("TimeUSec=") {
            time_usec = val.parse().ok();
        } else if let Some(val) = line.strip_prefix("RTCTimeUSec=") {
            rtc_time_usec = val.parse().ok();
        }
    }

    let mut drift_usec = None;
    if let (Some(sys), Some(rtc)) = (time_usec, rtc_time_usec) {
        drift_usec =
            Some(i64::try_from(sys).unwrap_or(i64::MAX) - i64::try_from(rtc).unwrap_or(i64::MAX));
    }

    Ok(ClockInfo {
        ntp_synchronized,
        time_usec,
        rtc_time_usec,
        drift_usec,
    })
}

/// GET /api/clock
///
/// Serves the clock synchronization state.
///
/// # Errors
/// 500 if collection fails; unreachable today because `collect_clock_info`
/// degrades instead of failing.
pub async fn clock_handler(
    State(state): State<AppState>,
) -> Result<Json<ClockInfo>, (StatusCode, String)> {
    collect_clock_info(&state.sys_ops)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))
}
