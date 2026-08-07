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
    /// Whether RTC time was reported. This is not battery-presence evidence.
    pub rtc_time_measured: bool,
    /// Why required RTC battery-presence evidence is unavailable.
    pub rtc_battery_unmeasured_reason: String,
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
        drift_usec = Some(
            i64::try_from(rtc)
                .unwrap_or(i64::MAX)
                .saturating_sub(i64::try_from(sys).unwrap_or(i64::MAX)),
        );
    }

    Ok(ClockInfo {
        ntp_synchronized,
        time_usec,
        rtc_time_usec,
        rtc_time_measured: rtc_time_usec.is_some(),
        rtc_battery_unmeasured_reason:
            "RTC battery presence is not exposed by the available timedatectl interface".to_string(),
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

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "unit tests on fixed fixtures")]
    use super::*;
    use crate::api::ops::SysOps;

    #[tokio::test]
    async fn test_drift_is_rtc_minus_sys() {
        let mut ops = SysOps::real();
        ops.cmd = |_cmd, _args| {
            Box::pin(async move { Ok("TimeUSec=1000\nRTCTimeUSec=1050\n".to_string()) })
        };

        let info = collect_clock_info(&ops).await.unwrap();
        assert_eq!(info.time_usec, Some(1000));
        assert_eq!(info.rtc_time_usec, Some(1050));
        assert!(info.rtc_time_measured);
        assert_eq!(
            info.rtc_battery_unmeasured_reason,
            "RTC battery presence is not exposed by the available timedatectl interface"
        );
        assert_eq!(info.drift_usec, Some(50));

        let mut ops2 = SysOps::real();
        ops2.cmd = |_cmd, _args| {
            Box::pin(async move { Ok("TimeUSec=2000\nRTCTimeUSec=1500\n".to_string()) })
        };

        let info2 = collect_clock_info(&ops2).await.unwrap();
        assert_eq!(info2.time_usec, Some(2000));
        assert_eq!(info2.rtc_time_usec, Some(1500));
        assert_eq!(info2.drift_usec, Some(-500));
    }

    #[tokio::test]
    async fn test_rtc_time_never_claims_battery_presence() {
        let mut ops = SysOps::real();
        ops.cmd = |_cmd, _args| {
            Box::pin(async move { Ok("NTPSynchronized=yes\nTimeUSec=1000\n".to_string()) })
        };

        let info = collect_clock_info(&ops).await.unwrap();
        assert!(info.ntp_synchronized);
        assert!(!info.rtc_time_measured);
        assert_eq!(info.rtc_time_usec, None);
        assert_eq!(info.drift_usec, None);
        assert_eq!(
            info.rtc_battery_unmeasured_reason,
            "RTC battery presence is not exposed by the available timedatectl interface"
        );
    }
}
