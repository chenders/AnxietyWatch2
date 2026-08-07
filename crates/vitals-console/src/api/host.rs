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
    /// `SoC` temperature in degrees Celsius.
    pub thermal_c: f64,
    /// Throttling flags decoded from the firmware bitmask.
    #[serde(flatten)]
    pub throttle: ThrottleStatus,
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
            let s = (ops.read_string)("/sys/class/thermal/thermal_zone0/temp")
                .await
                .unwrap_or_default();
            s.trim().parse::<f64>().unwrap_or(0.0) / 1000.0
        } else {
            let out = (ops.cmd)("vcgencmd", &["measure_temp"])
                .await
                .unwrap_or_default();
            let mut t = 0.0;
            if let Some(val) = out.strip_prefix("temp=") {
                let stripped = val.strip_suffix("'C\n");
                if let Some(val2) = stripped {
                    t = val2.parse().unwrap_or(0.0);
                }
            }
            t
        }
    };

    let throttled = {
        let out = (ops.cmd)("vcgencmd", &["get_throttled"])
            .await
            .unwrap_or_default();
        if let Some(val) = out.strip_prefix("throttled=") {
            let hex = val.trim().trim_start_matches("0x");
            u32::from_str_radix(hex, 16).unwrap_or(0)
        } else {
            0
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
        throttle: ThrottleStatus {
            undervoltage_now: (throttled & 1) != 0,
            freq_capped_now: (throttled & 2) != 0,
            throttled_now: (throttled & 4) != 0,
            soft_temp_limit_now: (throttled & 8) != 0,
            undervoltage_latched: (throttled & 0x10000) != 0,
            freq_capped_latched: (throttled & 0x20000) != 0,
            throttled_latched: (throttled & 0x40000) != 0,
            soft_temp_limit_latched: (throttled & 0x80000) != 0,
        },
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
    use super::*;

    #[test]
    fn test_throttled_bits_decoder() {
        let throttled = 0x50005;
        let info = HostInfo {
            disk_free_bytes: 0,
            disk_total_bytes: 0,
            thermal_c: 0.0,
            throttle: ThrottleStatus {
                undervoltage_now: (throttled & 1) != 0,
                freq_capped_now: (throttled & 2) != 0,
                throttled_now: (throttled & 4) != 0,
                soft_temp_limit_now: (throttled & 8) != 0,
                undervoltage_latched: (throttled & 0x10000) != 0,
                freq_capped_latched: (throttled & 0x20000) != 0,
                throttled_latched: (throttled & 0x40000) != 0,
                soft_temp_limit_latched: (throttled & 0x80000) != 0,
            },
            network_interface: None,
            wireless_signal: None,
            watchdog_present: false,
            load_avg: String::new(),
        };

        assert!(info.throttle.undervoltage_now);
        assert!(!info.throttle.freq_capped_now);
        assert!(info.throttle.throttled_now);
        assert!(!info.throttle.soft_temp_limit_now);

        assert!(info.throttle.undervoltage_latched);
        assert!(!info.throttle.freq_capped_latched);
        assert!(info.throttle.throttled_latched);
        assert!(!info.throttle.soft_temp_limit_latched);
    }
}
