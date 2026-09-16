use crate::api::ops::{SysOps, sanitize_detail};
use crate::handlers::AppState;
use axum::{Json, extract::State, http::StatusCode};
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// Disk-free percentage below which the host needs operator attention.
pub const DISK_FREE_WARNING_PERCENT: f64 = 10.0;
/// `SoC` temperature above which host health is degraded.
pub const THERMAL_WARNING_C: f64 = 80.0;
const MAX_DISK_HISTORY: usize = 24;

/// Throttling flags decoded from the `vcgencmd get_throttled` bitmask.
#[derive(Serialize)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "one bool per bit of the vcgencmd get_throttled bitmask"
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

#[derive(Clone)]
struct DiskSample {
    measured_at_epoch_s: i64,
    free_bytes: u64,
    total_bytes: u64,
}

/// Bounded in-process disk history used to derive time-to-full.
#[derive(Clone, Default)]
pub struct HostHistory {
    samples: Arc<Mutex<VecDeque<DiskSample>>>,
}

impl HostHistory {
    /// Record one valid disk observation and retain only the bounded tail.
    pub fn record(&self, measured_at_epoch_s: i64, free_bytes: u64, total_bytes: u64) {
        if total_bytes == 0 {
            return;
        }
        if let Ok(mut samples) = self.samples.lock() {
            samples.push_back(DiskSample {
                measured_at_epoch_s,
                free_bytes,
                total_bytes,
            });
            while samples.len() > MAX_DISK_HISTORY {
                samples.pop_front();
            }
        }
    }

    fn snapshot(&self) -> Vec<DiskSample> {
        self.samples
            .lock()
            .map(|samples| samples.iter().cloned().collect())
            .unwrap_or_default()
    }
}

/// One bounded disk-history point rendered to the operator.
#[derive(Serialize)]
pub struct DiskHistoryPoint {
    /// UTC epoch timestamp of the observation.
    pub measured_at_epoch_s: i64,
    /// Free bytes at the observation.
    pub free_bytes: u64,
    /// Total bytes at the observation.
    pub total_bytes: u64,
}

/// Host health, diagnostics, and bounded history.
#[derive(Serialize)]
pub struct HostInfo {
    /// Free bytes on the root filesystem, or absent when unmeasured.
    pub disk_free_bytes: Option<u64>,
    /// Total bytes on the root filesystem, or absent when unmeasured.
    pub disk_total_bytes: Option<u64>,
    /// Percent free, or absent when the disk denominator is invalid.
    pub disk_free_percent: Option<f64>,
    /// Why root filesystem size/free space is absent.
    pub disk_unmeasured_reason: String,
    /// Disk wear percentage when the platform exposes it.
    pub disk_wear_percent: Option<f64>,
    /// Projected seconds until free space reaches zero, from bounded history.
    pub projected_time_to_full_s: Option<u64>,
    /// Bounded disk observations used by the projection.
    pub disk_history: Vec<DiskHistoryPoint>,
    /// Why disk wear is absent.
    pub disk_wear_unmeasured_reason: String,
    /// Why time-to-full is absent.
    pub time_to_full_unmeasured_reason: String,
    /// Disk threshold shown alongside the current value.
    pub disk_free_warning_percent: f64,
    /// `SoC` temperature in degrees Celsius, or absent if unmeasurable.
    pub thermal_c: Option<f64>,
    /// Why thermal state is absent.
    pub thermal_unmeasured_reason: String,
    /// Thermal threshold shown alongside the current value.
    pub thermal_warning_c: f64,
    /// Decoded power/throttling flags.
    pub throttle: Option<ThrottleStatus>,
    /// Why throttle state is absent.
    pub throttle_unmeasured_reason: String,
    /// Name of the default-route network interface.
    pub network_interface: Option<String>,
    /// Whether the default-route interface reports an up link.
    pub network_link_up: Option<bool>,
    /// Wireless signal strength in dBm.
    pub wireless_signal: Option<i32>,
    /// Whether a `/dev/watchdog` device is present.
    pub watchdog_present: bool,
    /// Whether watchdog state is reported armed.
    pub watchdog_armed: Option<bool>,
    /// Load average text, absent when `/proc/loadavg` is unmeasured.
    pub load_avg: Option<String>,
    /// Mains presence when a supported power-supply entry exists.
    pub mains_present: Option<bool>,
    /// UPS charge percentage when a supported battery entry exists.
    pub ups_charge_percent: Option<f64>,
    /// UPS status text when present.
    pub ups_status: Option<String>,
    /// Why mains/UPS state is absent.
    pub power_unmeasured_reason: String,
    /// UTC epoch time at which this snapshot was measured.
    pub measured_at_epoch_s: i64,
    /// Age of this response's measurements in seconds.
    pub freshness_age_s: u64,
}

fn now_epoch_s() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_secs()).ok())
        .unwrap_or(0)
}

fn parse_throttle(output: Result<String, String>) -> (Option<ThrottleStatus>, String) {
    let output = match output {
        Ok(output) => output,
        Err(error) => return (None, format!("vcgencmd get_throttled failed: {error}")),
    };
    let Some(value) = output.strip_prefix("throttled=") else {
        return (
            None,
            format!("vcgencmd answered without throttled= prefix: {output:?}"),
        );
    };
    let Some(bits) = u32::from_str_radix(value.trim().trim_start_matches("0x"), 16).ok() else {
        return (
            None,
            format!("vcgencmd returned an invalid throttled bitmask: {output:?}"),
        );
    };
    (
        Some(ThrottleStatus {
            undervoltage_now: bits & 1 != 0,
            freq_capped_now: bits & 2 != 0,
            throttled_now: bits & 4 != 0,
            soft_temp_limit_now: bits & 8 != 0,
            undervoltage_latched: bits & 0x10000 != 0,
            freq_capped_latched: bits & 0x20000 != 0,
            throttled_latched: bits & 0x40000 != 0,
            soft_temp_limit_latched: bits & 0x80000 != 0,
        }),
        String::new(),
    )
}

fn parse_wear(value: &str) -> Option<f64> {
    let wear = value.trim().parse::<f64>().ok()?;
    if wear.is_finite() && (0.0..=100.0).contains(&wear) {
        Some(wear)
    } else {
        None
    }
}

fn parse_binary(value: &str) -> Option<bool> {
    match value.trim() {
        "0" | "down" | "inactive" => Some(false),
        "1" | "up" | "active" => Some(true),
        _ => None,
    }
}

fn disk_projection(history: &HostHistory) -> (Vec<DiskHistoryPoint>, Option<u64>) {
    let samples = history.snapshot();
    let points = samples
        .iter()
        .map(|sample| DiskHistoryPoint {
            measured_at_epoch_s: sample.measured_at_epoch_s,
            free_bytes: sample.free_bytes,
            total_bytes: sample.total_bytes,
        })
        .collect::<Vec<_>>();
    let Some(first) = samples.first() else {
        return (points, None);
    };
    let Some(last) = samples.last() else {
        return (points, None);
    };
    let elapsed = last
        .measured_at_epoch_s
        .saturating_sub(first.measured_at_epoch_s);
    let loss = first.free_bytes.saturating_sub(last.free_bytes);
    if elapsed <= 0 || loss == 0 {
        return (points, None);
    }
    (
        points,
        Some(
            last.free_bytes
                .saturating_mul(u64::try_from(elapsed).unwrap_or(0))
                / loss,
        ),
    )
}

/// Collect host diagnostics with a bounded history snapshot.
///
/// Unsupported or malformed facts remain absent with an explicit reason;
/// they are never represented as zero or an empty healthy-looking string.
///
/// # Errors
/// This currently returns no errors, but retains `Result` for API compatibility
/// with platform-specific collectors.
#[expect(
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    reason = "one host snapshot keeps all independently measurable diagnostics together"
)]
pub async fn collect_host_info_at(
    ops: &SysOps,
    history: &HostHistory,
    measured_at_epoch_s: i64,
) -> Result<HostInfo, String> {
    // The percentage is presentation data; the source values remain exact u64s.

    let (disk_free_bytes, disk_total_bytes, disk_reason) = match (ops.stat_vfs)("/") {
        Ok((free, total)) if total > 0 && free <= total => {
            history.record(measured_at_epoch_s, free, total);
            (Some(free), Some(total), String::new())
        }
        Ok((free, total)) => (
            None,
            None,
            format!("root filesystem reported invalid free={free}, total={total}"),
        ),
        Err(error) => (None, None, format!("root filesystem stat failed: {error}")),
    };
    let disk_free_percent = disk_free_bytes
        .zip(disk_total_bytes)
        .and_then(|(free, total)| {
            if total == 0 {
                None
            } else {
                Some(free as f64 * 100.0 / total as f64)
            }
        });
    let (disk_history, projected_time_to_full_s) = disk_projection(history);

    let (thermal_c, thermal_unmeasured_reason) =
        if (ops.path_exists)("/sys/class/thermal/thermal_zone0/temp") {
            match (ops.read_string)("/sys/class/thermal/thermal_zone0/temp").await {
                Ok(value) => match value.trim().parse::<f64>().ok() {
                    Some(value) if value.is_finite() => (Some(value / 1000.0), String::new()),
                    _ => (
                        None,
                        "thermal sysfs value was not finite numeric millidegrees".to_string(),
                    ),
                },
                Err(error) => (None, format!("thermal sysfs read failed: {error}")),
            }
        } else {
            match (ops.cmd)("vcgencmd", &["measure_temp"]).await {
                Ok(output) => output
                    .strip_prefix("temp=")
                    .and_then(|value| value.strip_suffix("'C\n").or(Some(value)))
                    .and_then(|value| value.trim().parse::<f64>().ok())
                    .filter(|value| value.is_finite())
                    .map_or_else(
                        || {
                            (
                                None,
                                "vcgencmd temperature output was unmeasurable".to_string(),
                            )
                        },
                        |value| (Some(value), String::new()),
                    ),
                Err(error) => (None, format!("thermal command failed: {error}")),
            }
        };

    let (throttle, throttle_unmeasured_reason) =
        parse_throttle((ops.cmd)("vcgencmd", &["get_throttled"]).await);

    let mut network_interface = None;
    let route = (ops.read_string)("/proc/net/route")
        .await
        .unwrap_or_default();
    for line in route.lines().skip(1) {
        let mut parts = line.split_whitespace();
        let interface = parts.next();
        let destination = parts.next();
        if destination == Some("00000000") {
            network_interface = interface.map(str::to_string);
            break;
        }
    }
    let (network_link_up, wireless_signal) = if let Some(interface) = &network_interface {
        let link = (ops.read_string)(&format!("/sys/class/net/{interface}/operstate"))
            .await
            .ok()
            .and_then(|value| parse_binary(&value));
        let wireless = (ops.read_string)("/proc/net/wireless")
            .await
            .ok()
            .and_then(|value| {
                value.lines().skip(2).find_map(|line| {
                    let mut parts = line.split_whitespace();
                    let name = parts.next()?.trim_end_matches(':');
                    if name == interface {
                        parts
                            .nth(1)
                            .and_then(|signal| signal.split('.').next())
                            .and_then(|signal| signal.parse::<i32>().ok())
                    } else {
                        None
                    }
                })
            });
        (link, wireless)
    } else {
        (None, None)
    };

    let watchdog_present = (ops.path_exists)("/dev/watchdog");
    let watchdog_armed = (ops.read_string)("/sys/class/watchdog/watchdog0/state")
        .await
        .ok()
        .and_then(|value| parse_binary(&value));
    let load_avg = (ops.read_string)("/proc/loadavg")
        .await
        .ok()
        .map(|value| sanitize_detail(value.trim(), None))
        .filter(|value| !value.is_empty());

    let disk_wear_percent = (ops.read_string)("/sys/class/nvme/nvme0/device/life_used")
        .await
        .ok()
        .and_then(|value| parse_wear(&value));
    let disk_wear_unmeasured_reason = if disk_wear_percent.is_some() {
        String::new()
    } else {
        "NVMe life_used is absent or unmeasurable".to_string()
    };
    let time_to_full_unmeasured_reason = if projected_time_to_full_s.is_some() {
        String::new()
    } else {
        "insufficient decreasing disk history for a projection".to_string()
    };

    let mut mains_present = None;
    let mut ups_charge_percent = None;
    let mut ups_status = None;
    let power_paths = (ops.glob_paths)("/sys/class/power_supply/*").unwrap_or_default();
    for path in power_paths {
        let Some(name) = path.rsplit('/').next() else {
            continue;
        };
        let name_lower = name.to_ascii_lowercase();
        if name_lower.starts_with("ac") || name_lower.starts_with("adp") {
            mains_present = (ops.read_string)(&format!("{path}/online"))
                .await
                .ok()
                .and_then(|value| parse_binary(&value));
        } else if name_lower.contains("ups") || name_lower.contains("battery") {
            ups_charge_percent = (ops.read_string)(&format!("{path}/capacity"))
                .await
                .ok()
                .and_then(|value| parse_wear(&value));
            ups_status = (ops.read_string)(&format!("{path}/status"))
                .await
                .ok()
                .map(|value| sanitize_detail(value.trim(), None))
                .filter(|value| !value.is_empty());
        }
    }
    let power_unmeasured_reason = if mains_present.is_some() || ups_charge_percent.is_some() {
        String::new()
    } else {
        "no supported mains or UPS power-supply entries were measurable".to_string()
    };

    Ok(HostInfo {
        disk_free_bytes,
        disk_total_bytes,
        disk_free_percent,
        disk_unmeasured_reason: disk_reason,
        disk_wear_percent,
        projected_time_to_full_s,
        disk_history,
        disk_wear_unmeasured_reason,
        time_to_full_unmeasured_reason,
        disk_free_warning_percent: DISK_FREE_WARNING_PERCENT,
        thermal_c,
        thermal_unmeasured_reason,
        thermal_warning_c: THERMAL_WARNING_C,
        throttle,
        throttle_unmeasured_reason,
        network_interface,
        network_link_up,
        wireless_signal,
        watchdog_present,
        watchdog_armed,
        load_avg,
        mains_present,
        ups_charge_percent,
        ups_status,
        power_unmeasured_reason,
        measured_at_epoch_s,
        freshness_age_s: 0,
    })
}

/// Collect a one-shot host snapshot without retaining history.
///
/// # Errors
/// This currently returns no errors, but retains `Result` for API compatibility
/// with platform-specific collectors.
pub async fn collect_host_info(ops: &SysOps) -> Result<HostInfo, String> {
    collect_host_info_at(ops, &HostHistory::default(), now_epoch_s()).await
}

/// GET `/api/host`.
///
/// Serves host health, bounded history, diagnostics, and visible thresholds.
///
/// # Errors
/// Returns a 500 response if collection fails.
pub async fn host_handler(
    State(state): State<AppState>,
) -> Result<Json<HostInfo>, (StatusCode, String)> {
    collect_host_info_at(&state.sys_ops, &state.host_history, now_epoch_s())
        .await
        .map(Json)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error))
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::expect_used,
        clippy::unwrap_used,
        reason = "host tests use fixed fixtures"
    )]

    use super::*;

    #[tokio::test]
    async fn host_snapshot_preserves_unknowns_and_thresholds() {
        let mut ops = SysOps::real();
        ops.stat_vfs = |_| Ok((100, 200));
        ops.path_exists = |_| true;
        ops.read_string = |path| {
            let value = match path {
                "/sys/class/thermal/thermal_zone0/temp" => "42123\n",
                "/proc/loadavg" => "0.1 0.2 0.3 1/2 3\n",
                _ => "garbage\n",
            };
            Box::pin(async move { Ok(value.to_string()) })
        };
        ops.cmd = |_cmd, _args| Box::pin(async { Err("unsupported".to_string()) });
        ops.glob_paths = |_| Ok(Vec::new());
        let info = collect_host_info_at(&ops, &HostHistory::default(), 1_000)
            .await
            .unwrap();
        assert_eq!(info.disk_free_bytes, Some(100));
        assert!(
            info.disk_free_percent
                .is_some_and(|value| (value - 50.0).abs() < 1e-9)
        );
        assert!(
            info.thermal_c
                .is_some_and(|value| (value - 42.123).abs() < 1e-9)
        );
        assert_eq!(info.measured_at_epoch_s, 1_000);
        assert_eq!(info.freshness_age_s, 0);
        assert!((info.disk_free_warning_percent - DISK_FREE_WARNING_PERCENT).abs() < 1e-9);
        assert!((info.thermal_warning_c - THERMAL_WARNING_C).abs() < 1e-9);
        assert!(info.disk_wear_percent.is_none());
        assert!(info.power_unmeasured_reason.contains("no supported"));
    }

    #[tokio::test]
    async fn host_disk_history_projects_only_decreasing_free_space() {
        let history = HostHistory::default();
        history.record(1_000, 100, 200);
        history.record(1_100, 50, 200);
        let (points, projection) = disk_projection(&history);
        assert_eq!(points.len(), 2);
        assert_eq!(projection, Some(100));
    }

    #[test]
    fn host_wear_rejects_out_of_range_values() {
        assert_eq!(parse_wear("42\n"), Some(42.0));
        assert_eq!(parse_wear("101\n"), None);
        assert_eq!(parse_wear("NaN\n"), None);
    }
}
