use crate::api::ops::SysOps;
use crate::handlers::AppState;
use axum::{Json, extract::State, http::StatusCode};
use serde::Serialize;

/// Throttling flags decoded from the `vcgencmd get_throttled` bitmask.
///
/// One boolean per bit of the bitmask, named after the Raspberry Pi firmware
/// documentation. Grouped here because the flags are one decodable unit;
/// `#[serde(flatten)]` keeps them flat in the JSON response.
///
/// `expect` rather than `allow`: if the struct ever drops below the lint's
/// threshold, the expectation must be removed rather than silently stale.
#[derive(Serialize)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "one bool per bit of the vcgencmd get_throttled bitmask; grouping further would obscure the bit-to-field mapping"
)]
pub struct ThrottleStatus {
    /// Under-voltage detected right now.
    pub undervoltage_now: bool,
    /// Under-voltage latched since boot.
    pub undervoltage_latched: bool,
    /// ARM frequency capped right now.
    pub freq_capped_now: bool,
    /// ARM frequency capped latched since boot.
    pub freq_capped_latched: bool,
    /// Throttled right now.
    pub throttled_now: bool,
    /// Throttled latched since boot.
    pub throttled_latched: bool,
    /// Soft temperature limit reached right now.
    pub soft_temp_limit_now: bool,
    /// Soft temperature limit latched since boot.
    pub soft_temp_limit_latched: bool,
}

/// Host health: disk, thermal, throttling, network, watchdog, load.
#[derive(Serialize)]
pub struct HostInfo {
    /// Free bytes on the root filesystem.
    pub disk_free_bytes: u64,
    /// Total bytes on the root filesystem.
    pub disk_total_bytes: u64,
    /// `SoC` temperature in degrees Celsius, or `None` if unmeasurable.
    pub thermal_c: Option<f64>,
    /// Throttling flags decoded from the firmware bitmask, or `None` when
    /// `vcgencmd get_throttled` failed or answered garbage — an unmeasured
    /// power rail must never render as a healthy one.
    pub throttle: Option<ThrottleStatus>,
    /// Why `throttle` is absent, when it is.
    pub throttle_unmeasured_reason: Option<String>,
    /// Name of the default-route network interface, when one exists.
    pub network_interface: Option<String>,
    /// Signal strength of the wireless interface in dBm, when present.
    pub wireless_signal: Option<i32>,
    /// Whether a `/dev/watchdog` device is present.
    pub watchdog_present: bool,
    /// Contents of `/proc/loadavg`.
    pub load_avg: String,
}

/// Collect host health from sysfs, procfs, and `vcgencmd`.
///
/// # Errors
/// Returns `"not available on this host"` on non-Linux hosts, or an error
/// naming the failure when the root filesystem cannot be stat'ed.
pub async fn collect_host_info(ops: &SysOps) -> Result<HostInfo, String> {
    let stat = (ops.stat_vfs)("/");
    let (disk_free_bytes, disk_total_bytes) = match stat {
        Ok(v) => v,
        Err(e) if e == "not available on this host" => return Err(e),
        Err(e) => return Err(format!("stat_vfs: {e}")),
    };

    let thermal_c = {
        if (ops.path_exists)("/sys/class/thermal/thermal_zone0/temp") {
            if let Ok(s) = (ops.read_string)("/sys/class/thermal/thermal_zone0/temp").await {
                s.trim().parse::<f64>().ok().map(|v| v / 1000.0)
            } else {
                None
            }
        } else if let Ok(out) = (ops.cmd)("vcgencmd", &["measure_temp"]).await {
            if let Some(val) = out.strip_prefix("temp=") {
                let stripped = val.strip_suffix("'C\n").unwrap_or(val);
                stripped.trim().parse::<f64>().ok()
            } else {
                None
            }
        } else {
            None
        }
    };

    let (throttled, throttle_unmeasured_reason) = {
        match (ops.cmd)("vcgencmd", &["get_throttled"]).await {
            Ok(out) => match out
                .strip_prefix("throttled=")
                .and_then(|v| u32::from_str_radix(v.trim().trim_start_matches("0x"), 16).ok())
            {
                Some(bits) => (Some(bits), None),
                None => (
                    None,
                    Some(format!("vcgencmd answered unparseable output: {out:?}")),
                ),
            },
            Err(e) => (None, Some(format!("vcgencmd get_throttled failed: {e}"))),
        }
    };

    let mut network_interface = None;
    let mut wireless_signal = None;

    let route = (ops.read_string)("/proc/net/route")
        .await
        .unwrap_or_default();
    for line in route.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() > 1 && parts.get(1).copied() == Some("00000000") {
            network_interface = Some(parts.first().unwrap_or(&"").to_string());
            break;
        }
    }

    if let Some(ref iface) = network_interface {
        let w = (ops.read_string)("/proc/net/wireless")
            .await
            .unwrap_or_default();
        for line in w.lines().skip(2) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() > 3 && parts.first().unwrap_or(&"").trim_end_matches(':') == iface {
                wireless_signal = parts.get(3).unwrap_or(&"").parse::<i32>().ok();
            }
        }
    }

    let watchdog_present = (ops.path_exists)("/dev/watchdog");
    let load_avg = (ops.read_string)("/proc/loadavg")
        .await
        .unwrap_or_default()
        .trim()
        .to_string();

    Ok(HostInfo {
        disk_free_bytes,
        disk_total_bytes,
        thermal_c,
        throttle: throttled.map(|bits| ThrottleStatus {
            undervoltage_now: (bits & 1) != 0,
            freq_capped_now: (bits & 2) != 0,
            throttled_now: (bits & 4) != 0,
            soft_temp_limit_now: (bits & 8) != 0,
            undervoltage_latched: (bits & 0x10000) != 0,
            freq_capped_latched: (bits & 0x20000) != 0,
            throttled_latched: (bits & 0x40000) != 0,
            soft_temp_limit_latched: (bits & 0x80000) != 0,
        }),
        throttle_unmeasured_reason,
        network_interface,
        wireless_signal,
        watchdog_present,
        load_avg,
    })
}

/// GET /api/host
///
/// Serves host health.
///
/// # Errors
/// 501 when the host cannot report host state (non-Linux); 500 on other
/// collection failures.
pub async fn host_handler(
    State(state): State<AppState>,
) -> Result<Json<HostInfo>, (StatusCode, String)> {
    collect_host_info(&state.sys_ops)
        .await
        .map(Json)
        .map_err(|e| {
            if e == "not available on this host" {
                (StatusCode::NOT_IMPLEMENTED, e)
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, e)
            }
        })
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "unit tests on fixed fixtures")]
    use super::*;

    #[test]
    fn test_throttled_bits_decoder() {
        let throttled = 0x50005;
        let info = HostInfo {
            disk_free_bytes: 0,
            disk_total_bytes: 0,
            thermal_c: None,
            throttle: Some(ThrottleStatus {
                undervoltage_now: (throttled & 1) != 0,
                freq_capped_now: (throttled & 2) != 0,
                throttled_now: (throttled & 4) != 0,
                soft_temp_limit_now: (throttled & 8) != 0,
                undervoltage_latched: (throttled & 0x10000) != 0,
                freq_capped_latched: (throttled & 0x20000) != 0,
                throttled_latched: (throttled & 0x40000) != 0,
                soft_temp_limit_latched: (throttled & 0x80000) != 0,
            }),
            throttle_unmeasured_reason: None,
            network_interface: None,
            wireless_signal: None,
            watchdog_present: false,
            load_avg: String::new(),
        };

        #[expect(clippy::expect_used, reason = "test fixture always decodes")]
        let t = info.throttle.expect("decoded");
        assert!(t.undervoltage_now);
        assert!(!t.freq_capped_now);
        assert!(t.throttled_now);
        assert!(!t.soft_temp_limit_now);

        assert!(t.undervoltage_latched);
        assert!(!t.freq_capped_latched);
        assert!(t.throttled_latched);
        assert!(!t.soft_temp_limit_latched);
    }

    #[tokio::test]
    async fn test_thermal_failure_is_none() {
        let mut ops = SysOps::real();
        ops.stat_vfs = |_path| Ok((100, 200));
        ops.path_exists = |_path| true;
        ops.read_string = |_path| Box::pin(async move { Ok("garbage\n".to_string()) });

        let info = collect_host_info(&ops).await.unwrap();
        assert_eq!(info.thermal_c, None);
    }

    #[tokio::test]
    async fn test_thermal_success() {
        let mut ops = SysOps::real();
        ops.stat_vfs = |_path| Ok((100, 200));
        ops.path_exists = |_path| true;
        ops.read_string = |_path| Box::pin(async move { Ok("42123\n".to_string()) });

        let info = collect_host_info(&ops).await.unwrap();
        assert_eq!(info.thermal_c, Some(42.123));
    }
}
