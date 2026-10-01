//! Low-cardinality broker telemetry.
//!
//! Metrics deliberately contain source and operation classes, never device
//! selectors, addresses, handles, reasons, or payload contents. The snapshot
//! is Prometheus text suitable for a local textfile collector.

use std::{
    fmt::Write as _,
    fs,
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

use crate::{OperationKind, Source};

const SOURCE_NAMES: [&str; 6] = ["bp", "h10", "sleepo2", "airsense", "wellue", "diagnostics"];
const OPERATION_NAMES: [&str; 6] = [
    "observe_device",
    "discover_target",
    "connect",
    "resolve",
    "subscribe",
    "write",
];
const SOURCE_COUNT: usize = SOURCE_NAMES.len();
const OPERATION_COUNT: usize = OPERATION_NAMES.len();

struct SourceCounters {
    connected_total: AtomicU64,
    active: AtomicU64,
    request_bytes_total: AtomicU64,
    operations_total: [AtomicU64; OPERATION_COUNT],
    operation_success_total: [AtomicU64; OPERATION_COUNT],
    operation_failure_total: [AtomicU64; OPERATION_COUNT],
    operation_cancelled_total: [AtomicU64; OPERATION_COUNT],
    operation_duration_ns_total: [AtomicU64; OPERATION_COUNT],
    leases_granted_total: AtomicU64,
    leases_queued_total: AtomicU64,
    leases_released_total: AtomicU64,
}

impl SourceCounters {
    fn new() -> Self {
        Self {
            connected_total: AtomicU64::new(0),
            active: AtomicU64::new(0),
            request_bytes_total: AtomicU64::new(0),
            operations_total: std::array::from_fn(|_| AtomicU64::new(0)),
            operation_success_total: std::array::from_fn(|_| AtomicU64::new(0)),
            operation_failure_total: std::array::from_fn(|_| AtomicU64::new(0)),
            operation_cancelled_total: std::array::from_fn(|_| AtomicU64::new(0)),
            operation_duration_ns_total: std::array::from_fn(|_| AtomicU64::new(0)),
            leases_granted_total: AtomicU64::new(0),
            leases_queued_total: AtomicU64::new(0),
            leases_released_total: AtomicU64::new(0),
        }
    }
}

/// Process-wide broker counters and resource snapshot.
pub struct Telemetry {
    started: Instant,
    requests_bytes_total: AtomicU64,
    responses_bytes_total: AtomicU64,
    notification_bytes_total: AtomicU64,
    clients_rejected_total: AtomicU64,
    counters: [SourceCounters; SOURCE_COUNT],
}

impl Telemetry {
    /// Construct an empty telemetry registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            started: Instant::now(),
            requests_bytes_total: AtomicU64::new(0),
            responses_bytes_total: AtomicU64::new(0),
            notification_bytes_total: AtomicU64::new(0),
            clients_rejected_total: AtomicU64::new(0),
            counters: std::array::from_fn(|_| SourceCounters::new()),
        }
    }

    /// Record an authenticated client connection.
    pub fn client_connected(&self, source: Source) {
        let counters = self.source_counters(source);
        counters.connected_total.fetch_add(1, Ordering::Relaxed);
        counters.active.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a client disconnect after cleanup.
    pub fn client_disconnected(&self, source: Source) {
        self.source_counters(source)
            .active
            .fetch_sub(1, Ordering::Relaxed);
    }

    /// Record a rejected handshake.
    pub fn client_rejected(&self) {
        self.clients_rejected_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Record the encoded size of one client request.
    pub fn request_bytes(&self, source: Source, bytes: usize) {
        let bytes = u64::try_from(bytes).unwrap_or(u64::MAX);
        self.requests_bytes_total
            .fetch_add(bytes, Ordering::Relaxed);
        self.source_counters(source)
            .request_bytes_total
            .fetch_add(bytes, Ordering::Relaxed);
    }

    /// Record the encoded size of one broker response.
    pub fn response_bytes(&self, bytes: usize) {
        self.responses_bytes_total
            .fetch_add(u64::try_from(bytes).unwrap_or(u64::MAX), Ordering::Relaxed);
    }

    /// Record notification payload throughput separately from protocol bytes.
    pub fn notification_bytes(&self, bytes: usize) {
        self.notification_bytes_total
            .fetch_add(u64::try_from(bytes).unwrap_or(u64::MAX), Ordering::Relaxed);
    }

    /// Record a lease scheduler result.
    pub fn lease_granted(&self, source: Source) {
        self.source_counters(source)
            .leases_granted_total
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Record a bounded request that was queued.
    pub fn lease_queued(&self, source: Source) {
        self.source_counters(source)
            .leases_queued_total
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Record a lease release.
    pub fn lease_released(&self, source: Source) {
        self.source_counters(source)
            .leases_released_total
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Record one typed operation and its terminal state.
    pub fn operation_finished(
        &self,
        source: Source,
        operation: OperationKind,
        duration: Duration,
        result: OperationResult,
    ) {
        let index = operation_index(operation);
        let counters = self.source_counters(source);
        operation_counter(&counters.operations_total, index).fetch_add(1, Ordering::Relaxed);
        operation_counter(&counters.operation_duration_ns_total, index).fetch_add(
            u64::try_from(duration.as_nanos().min(u128::from(u64::MAX))).unwrap_or(u64::MAX),
            Ordering::Relaxed,
        );
        match result {
            OperationResult::Success => {
                operation_counter(&counters.operation_success_total, index)
                    .fetch_add(1, Ordering::Relaxed);
            }
            OperationResult::Failure => {
                operation_counter(&counters.operation_failure_total, index)
                    .fetch_add(1, Ordering::Relaxed);
            }
            OperationResult::Cancelled => {
                operation_counter(&counters.operation_cancelled_total, index)
                    .fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    fn source_counters(&self, source: Source) -> &SourceCounters {
        self.counters
            .get(source_index(source))
            .unwrap_or_else(|| std::process::abort())
    }

    /// Render a stable Prometheus text snapshot.
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn render(&self) -> String {
        let mut output = String::new();
        output.push_str("# HELP vitals_ble_broker_uptime_seconds Broker process uptime.\n# TYPE vitals_ble_broker_uptime_seconds gauge\n");
        let _ = writeln!(
            output,
            "vitals_ble_broker_uptime_seconds {}",
            self.started.elapsed().as_secs_f64()
        );
        output.push_str("# HELP vitals_ble_broker_request_bytes_total Encoded request bytes received from clients.\n# TYPE vitals_ble_broker_request_bytes_total counter\n");
        let _ = writeln!(
            output,
            "vitals_ble_broker_request_bytes_total {}",
            self.requests_bytes_total.load(Ordering::Relaxed)
        );
        output.push_str("# HELP vitals_ble_broker_response_bytes_total Encoded response bytes sent to clients.\n# TYPE vitals_ble_broker_response_bytes_total counter\n");
        let _ = writeln!(
            output,
            "vitals_ble_broker_response_bytes_total {}",
            self.responses_bytes_total.load(Ordering::Relaxed)
        );
        output.push_str("# HELP vitals_ble_broker_notification_bytes_total GATT notification payload bytes sent.\n# TYPE vitals_ble_broker_notification_bytes_total counter\n");
        let _ = writeln!(
            output,
            "vitals_ble_broker_notification_bytes_total {}",
            self.notification_bytes_total.load(Ordering::Relaxed)
        );
        output.push_str("# HELP vitals_ble_broker_clients_rejected_total Handshakes rejected before authentication.\n# TYPE vitals_ble_broker_clients_rejected_total counter\n");
        let _ = writeln!(
            output,
            "vitals_ble_broker_clients_rejected_total {}",
            self.clients_rejected_total.load(Ordering::Relaxed)
        );
        for (source_index, source_name) in SOURCE_NAMES.iter().enumerate() {
            let counters = self
                .counters
                .get(source_index)
                .unwrap_or_else(|| std::process::abort());
            let label = format!("{{source=\"{source_name}\"}} ");
            metric_line(
                &mut output,
                "vitals_ble_broker_clients_connected_total",
                &label,
                counters.connected_total.load(Ordering::Relaxed),
            );
            metric_line(
                &mut output,
                "vitals_ble_broker_clients_active",
                &label,
                counters.active.load(Ordering::Relaxed),
            );
            metric_line(
                &mut output,
                "vitals_ble_broker_request_bytes_total",
                &label,
                counters.request_bytes_total.load(Ordering::Relaxed),
            );
            metric_line(
                &mut output,
                "vitals_ble_broker_leases_granted_total",
                &label,
                counters.leases_granted_total.load(Ordering::Relaxed),
            );
            metric_line(
                &mut output,
                "vitals_ble_broker_leases_queued_total",
                &label,
                counters.leases_queued_total.load(Ordering::Relaxed),
            );
            metric_line(
                &mut output,
                "vitals_ble_broker_leases_released_total",
                &label,
                counters.leases_released_total.load(Ordering::Relaxed),
            );
            for (operation_index, operation_name) in OPERATION_NAMES.iter().enumerate() {
                let operation_label =
                    format!("{{source=\"{source_name}\",operation=\"{operation_name}\"}} ");
                metric_line(
                    &mut output,
                    "vitals_ble_broker_operations_total",
                    &operation_label,
                    operation_counter(&counters.operations_total, operation_index)
                        .load(Ordering::Relaxed),
                );
                metric_line(
                    &mut output,
                    "vitals_ble_broker_operation_success_total",
                    &operation_label,
                    operation_counter(&counters.operation_success_total, operation_index)
                        .load(Ordering::Relaxed),
                );
                metric_line(
                    &mut output,
                    "vitals_ble_broker_operation_failure_total",
                    &operation_label,
                    operation_counter(&counters.operation_failure_total, operation_index)
                        .load(Ordering::Relaxed),
                );
                metric_line(
                    &mut output,
                    "vitals_ble_broker_operation_cancelled_total",
                    &operation_label,
                    operation_counter(&counters.operation_cancelled_total, operation_index)
                        .load(Ordering::Relaxed),
                );
                metric_line(
                    &mut output,
                    "vitals_ble_broker_operation_duration_nanoseconds_total",
                    &operation_label,
                    operation_counter(&counters.operation_duration_ns_total, operation_index)
                        .load(Ordering::Relaxed),
                );
            }
        }
        let (rss_bytes, cpu_jiffies) = process_resources();
        metric_line(
            &mut output,
            "vitals_ble_broker_process_rss_bytes",
            "",
            rss_bytes,
        );
        metric_line(
            &mut output,
            "vitals_ble_broker_process_cpu_jiffies_total",
            "",
            cpu_jiffies,
        );
        metric_line(
            &mut output,
            "vitals_ble_broker_estimated_direct_mode_processes_removed",
            "",
            1,
        );
        metric_line(
            &mut output,
            "vitals_ble_broker_estimated_direct_mode_rss_bytes_saved",
            "",
            rss_bytes,
        );
        metric_line(
            &mut output,
            "vitals_ble_broker_estimated_direct_mode_cpu_jiffies_saved",
            "",
            cpu_jiffies,
        );
        metric_line(
            &mut output,
            "vitals_ble_broker_estimated_direct_mode_ipc_bytes_avoided_total",
            "",
            self.requests_bytes_total
                .load(Ordering::Relaxed)
                .saturating_add(self.responses_bytes_total.load(Ordering::Relaxed)),
        );
        output
    }

    /// Atomically publish one snapshot for a local Prometheus textfile collector.
    pub fn export_once(&self, path: &Path) -> Result<(), String> {
        let temporary = path.with_extension("prom.tmp");
        fs::write(&temporary, self.render())
            .map_err(|error| format!("metrics write failed: {error}"))?;
        fs::rename(&temporary, path).map_err(|error| format!("metrics publish failed: {error}"))
    }

    /// Run the low-frequency local exporter.
    pub async fn export_loop(self: std::sync::Arc<Self>, path: std::path::PathBuf) {
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        loop {
            let _ = self.export_once(&path);
            interval.tick().await;
        }
    }
}

/// Terminal state of a typed broker operation.
#[derive(Clone, Copy)]
pub enum OperationResult {
    /// `BlueZ` completed the operation.
    Success,
    /// The operation returned a policy or `BlueZ` error.
    Failure,
    /// The client cancelled the operation or its lease disappeared.
    Cancelled,
}

fn metric_line(output: &mut String, name: &str, labels: &str, value: u64) {
    let _ = writeln!(output, "{name}{labels}{value}");
}

fn operation_counter(counters: &[AtomicU64; OPERATION_COUNT], index: usize) -> &AtomicU64 {
    counters.get(index).unwrap_or_else(|| std::process::abort())
}

fn source_index(source: Source) -> usize {
    match source {
        Source::Bp => 0,
        Source::H10 => 1,
        Source::SleepO2 => 2,
        Source::AirSense => 3,
        Source::Wellue => 4,
        Source::Diagnostics => 5,
    }
}

fn operation_index(operation: OperationKind) -> usize {
    match operation {
        OperationKind::ObserveDevice => 0,
        OperationKind::DiscoverTarget => 1,
        OperationKind::Connect => 2,
        OperationKind::Resolve => 3,
        OperationKind::Subscribe => 4,
        OperationKind::Write => 5,
    }
}

fn process_resources() -> (u64, u64) {
    let rss_bytes = fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|status| {
            status.lines().find_map(|line| {
                line.strip_prefix("VmRSS:")
                    .and_then(|value| value.split_whitespace().next())
                    .and_then(|value| value.parse::<u64>().ok())
            })
        })
        .unwrap_or_default()
        .saturating_mul(1024);
    let cpu_jiffies = fs::read_to_string("/proc/self/stat")
        .ok()
        .and_then(|stat| {
            let (_, fields) = stat.rsplit_once(") ")?;
            let fields: Vec<&str> = fields.split_whitespace().collect();
            let user = fields.get(11)?.parse::<u64>().ok()?;
            let system = fields.get(12)?.parse::<u64>().ok()?;
            Some(user.saturating_add(system))
        })
        .unwrap_or_default();
    (rss_bytes, cpu_jiffies)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::OperationKind;

    #[test]
    fn records_all_source_operation_and_lease_dimensions() {
        let telemetry = Telemetry::new();
        for source in [
            Source::Bp,
            Source::H10,
            Source::SleepO2,
            Source::AirSense,
            Source::Wellue,
            Source::Diagnostics,
        ] {
            telemetry.client_connected(source);
            telemetry.request_bytes(source, 7);
            telemetry.lease_granted(source);
            telemetry.lease_queued(source);
            telemetry.lease_released(source);
            telemetry.operation_finished(
                source,
                OperationKind::ObserveDevice,
                Duration::from_nanos(1),
                OperationResult::Success,
            );
            telemetry.operation_finished(
                source,
                OperationKind::Connect,
                Duration::from_nanos(2),
                OperationResult::Failure,
            );
            telemetry.operation_finished(
                source,
                OperationKind::Write,
                Duration::from_nanos(3),
                OperationResult::Cancelled,
            );
            telemetry.client_disconnected(source);
        }
        for operation in [
            OperationKind::ObserveDevice,
            OperationKind::DiscoverTarget,
            OperationKind::Connect,
            OperationKind::Resolve,
            OperationKind::Subscribe,
            OperationKind::Write,
        ] {
            telemetry.operation_finished(
                Source::Diagnostics,
                operation,
                Duration::from_nanos(1),
                OperationResult::Success,
            );
        }
        telemetry.client_rejected();
        telemetry.response_bytes(8);
        telemetry.notification_bytes(9);
        let snapshot = telemetry.render();
        assert!(snapshot.contains("source=\"airsense\""));
        assert!(snapshot.contains("operation=\"write\""));
        assert!(snapshot.contains("estimated_direct_mode_rss_bytes_saved"));
    }

    #[tokio::test]
    async fn exports_atomically() {
        let directory = std::env::temp_dir().join(format!("ble-telemetry-{}", std::process::id()));
        let _ = fs::create_dir_all(&directory);
        let path = directory.join("broker.prom");
        let telemetry = std::sync::Arc::new(Telemetry::new());
        assert!(telemetry.export_once(&path).is_ok());
        assert!(
            fs::read_to_string(&path)
                .unwrap_or_default()
                .contains("uptime_seconds")
        );
        let _ = fs::remove_dir_all(directory);
    }
}
