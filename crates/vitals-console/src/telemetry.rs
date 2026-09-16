//! Opt-in, bounded, vendor-neutral OTLP telemetry.
//!
//! Only the operational counters and timings named by the admin-console
//! specification can be represented. There is no free-form metric or label
//! constructor, so sample values, journal words, and per-entry identifiers
//! cannot accidentally leave the appliance.

use crate::crypto;
use async_trait::async_trait;
use serde::Serialize;
use std::collections::VecDeque;
use std::fmt;

/// OTLP endpoint region. The endpoint is derived, never operator-entered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TelemetryRegion {
    /// New Relic US endpoint.
    Us,
    /// New Relic EU endpoint.
    Eu,
}

impl TelemetryRegion {
    /// Fixed OTLP HTTPS endpoint for this region.
    #[must_use]
    pub const fn endpoint(self) -> &'static str {
        match self {
            Self::Us => "https://otlp.nr-data.net",
            Self::Eu => "https://otlp.eu01.nr-data.net",
        }
    }
}

/// Allowed low-cardinality metric names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricName {
    /// Process uptime.
    DaemonUptimeSeconds,
    /// Process restart count.
    DaemonRestartCount,
    /// Per-unit error count.
    DaemonErrorCount,
    /// Sample rate by declared source.
    SampleRate,
    /// Spool depth.
    SpoolDepth,
    /// Drain latency.
    DrainLatencyMs,
    /// Rejected/quarantined batch count.
    RejectedBatchCount,
    /// Console request latency.
    ConsoleLatencyMs,
    /// Attempt duration.
    AttemptDurationMs,
}

/// Counter or timing only; physiological measurements are unrepresentable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum MetricValue {
    /// Monotonic event count.
    Counter(u64),
    /// Elapsed time in milliseconds.
    DurationMs(u64),
}

/// One safe metric point with no user-controlled labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct MetricPoint {
    /// Fixed allowlisted name.
    pub name: MetricName,
    /// Counter or duration value.
    pub value: MetricValue,
}

/// A bounded, non-blocking metric queue.
#[derive(Debug)]
pub struct TelemetryQueue {
    points: VecDeque<MetricPoint>,
    capacity: usize,
    dropped: u64,
}

impl TelemetryQueue {
    /// Construct a queue; zero capacity is rejected by using one slot.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            points: VecDeque::new(),
            capacity: capacity.max(1),
            dropped: 0,
        }
    }

    /// Enqueue without waiting; a full queue increments the drop counter.
    pub fn try_push(&mut self, point: MetricPoint) -> bool {
        if self.points.len() >= self.capacity {
            self.dropped = self.dropped.saturating_add(1);
            false
        } else {
            self.points.push_back(point);
            true
        }
    }

    /// Remove at most `limit` points for an exporter batch.
    pub fn drain(&mut self, limit: usize) -> Vec<MetricPoint> {
        let count = limit.min(self.points.len());
        self.points.drain(..count).collect()
    }

    /// Number of points dropped under backpressure.
    #[must_use]
    pub const fn dropped(&self) -> u64 {
        self.dropped
    }

    /// Current queue depth.
    #[must_use]
    pub fn len(&self) -> usize {
        self.points.len()
    }

    /// Whether the queue has no pending points.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }
}

/// Encrypted New Relic license key metadata.
pub struct EncryptedLicenseKey {
    nonce: [u8; 24],
    ciphertext: Vec<u8>,
    last_four: String,
}

impl fmt::Debug for EncryptedLicenseKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EncryptedLicenseKey")
            .field("configured", &true)
            .field("last_four", &self.last_four)
            .finish_non_exhaustive()
    }
}

impl EncryptedLicenseKey {
    /// Encrypt a non-empty license key; plaintext is not retained.
    ///
    /// # Errors
    /// Returns an error for empty input or random/encryption failure.
    pub fn seal(value: &str, key: &[u8; 32]) -> Result<Self, String> {
        if value.is_empty() {
            return Err("telemetry license key rejected: value is empty".to_string());
        }
        let mut nonce = [0_u8; 24];
        getrandom::getrandom(&mut nonce)
            .map_err(|error| format!("telemetry key nonce generation failed: {error}"))?;
        let ciphertext = crypto::encrypt(value.as_bytes(), key, &nonce)?;
        let last_four = value
            .chars()
            .rev()
            .take(4)
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        Ok(Self {
            nonce,
            ciphertext,
            last_four,
        })
    }

    fn open(&self, key: &[u8; 32]) -> Result<String, String> {
        String::from_utf8(crypto::decrypt(&self.ciphertext, key, &self.nonce)?)
            .map_err(|error| format!("stored telemetry key is not UTF-8: {error}"))
    }

    /// Non-secret last-four display value.
    #[must_use]
    pub fn last_four(&self) -> &str {
        &self.last_four
    }

    /// Ciphertext for persistence.
    #[must_use]
    pub fn ciphertext(&self) -> &[u8] {
        &self.ciphertext
    }
}

/// Opt-in telemetry configuration.
pub struct TelemetryConfiguration {
    /// Disabled by default.
    pub enabled: bool,
    /// Fixed US/EU endpoint selection.
    pub region: TelemetryRegion,
    /// Encrypted license key, if configured.
    pub license_key: Option<EncryptedLicenseKey>,
}

/// Export health state shown in the admin console.
#[derive(Debug, Clone, Serialize)]
pub struct TelemetryStatus {
    /// `Live`, `Failing`, or `Unknown`.
    pub state: String,
    /// Whether export is enabled.
    pub enabled: bool,
    /// Whether a key is present.
    pub key_configured: bool,
    /// Key last four, never the key.
    pub key_last_four: Option<String>,
    /// Fixed region and endpoint.
    pub region: TelemetryRegion,
    /// Last test result.
    pub last_test: Option<TelemetryTestResult>,
    /// Operator-facing reason.
    pub reason: String,
}

/// Result of a connection/export test.
#[derive(Debug, Clone, Serialize)]
pub struct TelemetryTestResult {
    /// Whether the export succeeded.
    pub succeeded: bool,
    /// Completion time in UTC epoch seconds.
    pub completed_at_epoch_s: i64,
    /// Sanitized detail.
    pub detail: String,
}

/// Vendor-neutral OTLP transport seam. Implementations must use HTTPS.
#[async_trait]
pub trait OtlpTransport: Send + Sync {
    /// Send a bounded batch to the supplied fixed regional endpoint.
    async fn export(
        &self,
        endpoint: &str,
        license_key: &str,
        points: &[MetricPoint],
    ) -> Result<(), String>;
}

/// Injectable exporter that never blocks callers enqueueing metrics.
pub struct TelemetryExporter<T> {
    transport: T,
    encryption_key: [u8; 32],
}

impl<T: OtlpTransport> TelemetryExporter<T> {
    /// Construct an exporter with a local encryption key.
    #[must_use]
    pub const fn new(transport: T, encryption_key: [u8; 32]) -> Self {
        Self {
            transport,
            encryption_key,
        }
    }

    /// Export a bounded batch. Disabled/unconfigured export fails closed.
    ///
    /// # Errors
    /// Returns an operator-safe failure for disabled/configuration/transport
    /// errors; it never panics or blocks the ingest caller.
    pub async fn export_batch(
        &self,
        configuration: &TelemetryConfiguration,
        points: &[MetricPoint],
        completed_at_epoch_s: i64,
    ) -> TelemetryTestResult {
        let result = async {
            if !configuration.enabled {
                return Err("telemetry export is disabled".to_string());
            }
            let key = configuration
                .license_key
                .as_ref()
                .ok_or_else(|| "telemetry license key is not configured".to_string())?
                .open(&self.encryption_key)?;
            self.transport
                .export(configuration.region.endpoint(), &key, points)
                .await
        }
        .await;
        match result {
            Ok(()) => TelemetryTestResult {
                succeeded: true,
                completed_at_epoch_s,
                detail: format!(
                    "OTLP export succeeded to {}",
                    configuration.region.endpoint()
                ),
            },
            Err(error) => TelemetryTestResult {
                succeeded: false,
                completed_at_epoch_s,
                detail: crate::api::ops::sanitize_detail(&error, None),
            },
        }
    }
}

/// Resolve status without exposing the license key.
#[must_use]
pub fn status(
    configuration: &TelemetryConfiguration,
    last_test: Option<TelemetryTestResult>,
) -> TelemetryStatus {
    let (state, reason) = if !configuration.enabled {
        ("Unknown", "telemetry export is disabled")
    } else if configuration.license_key.is_none() {
        ("Unknown", "telemetry license key is not configured")
    } else {
        match last_test.as_ref() {
            None => ("Unknown", "configured but never tested"),
            Some(test) if test.succeeded => ("Live", "last OTLP test succeeded"),
            Some(_) => ("Failing", "last OTLP test failed"),
        }
    };
    TelemetryStatus {
        state: state.to_string(),
        enabled: configuration.enabled,
        key_configured: configuration.license_key.is_some(),
        key_last_four: configuration
            .license_key
            .as_ref()
            .map(|key| key.last_four().to_string()),
        region: configuration.region,
        last_test,
        reason: reason.to_string(),
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "telemetry tests use fixed fixtures")]
    use super::*;
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    struct FixtureTransport {
        calls: Arc<Mutex<Vec<(String, String, usize)>>>,
        fail: bool,
    }

    #[async_trait]
    impl OtlpTransport for FixtureTransport {
        async fn export(
            &self,
            endpoint: &str,
            key: &str,
            points: &[MetricPoint],
        ) -> Result<(), String> {
            self.calls
                .lock()
                .unwrap()
                .push((endpoint.to_string(), key.to_string(), points.len()));
            if self.fail {
                Err("fixture OTLP refused".to_string())
            } else {
                Ok(())
            }
        }
    }

    fn point() -> MetricPoint {
        MetricPoint {
            name: MetricName::DrainLatencyMs,
            value: MetricValue::DurationMs(12),
        }
    }

    #[test]
    fn telemetry_queue_drops_without_blocking_and_counts_drops() {
        let mut queue = TelemetryQueue::new(1);
        assert!(queue.try_push(point()));
        assert!(!queue.try_push(point()));
        assert_eq!(queue.dropped(), 1);
        assert_eq!(queue.drain(8).len(), 1);
        assert!(queue.is_empty());
    }

    #[test]
    fn telemetry_rejects_free_form_sensitive_metrics_and_key_debug_is_safe() {
        let key = [5_u8; 32];
        let license = EncryptedLicenseKey::seal("license-secret-value", &key).unwrap();
        assert_eq!(license.last_four(), "alue");
        assert!(!format!("{license:?}").contains("license-secret-value"));
        assert!(EncryptedLicenseKey::seal("", &key).is_err());
    }

    #[tokio::test]
    async fn telemetry_export_uses_fixed_region_and_encrypted_key() {
        let key = [8_u8; 32];
        let transport = FixtureTransport::default();
        let calls = transport.calls.clone();
        let configuration = TelemetryConfiguration {
            enabled: true,
            region: TelemetryRegion::Eu,
            license_key: Some(EncryptedLicenseKey::seal("license-secret-value", &key).unwrap()),
        };
        let result = TelemetryExporter::new(transport, key)
            .export_batch(&configuration, &[point()], 44)
            .await;
        assert!(result.succeeded);
        let calls = calls.lock().unwrap();
        assert_eq!(
            calls.first().map(|call| call.0.as_str()),
            Some(TelemetryRegion::Eu.endpoint())
        );
        assert_eq!(
            calls.first().map(|call| call.1.as_str()),
            Some("license-secret-value")
        );
        assert_eq!(calls.first().map(|call| call.2), Some(1));
        assert!(!result.detail.contains("license-secret-value"));
    }

    #[test]
    fn telemetry_never_tested_and_failed_are_not_live() {
        let key = [1_u8; 32];
        let configuration = TelemetryConfiguration {
            enabled: true,
            region: TelemetryRegion::Us,
            license_key: Some(EncryptedLicenseKey::seal("license-secret-value", &key).unwrap()),
        };
        assert_eq!(status(&configuration, None).state, "Unknown");
        assert_eq!(
            status(
                &configuration,
                Some(TelemetryTestResult {
                    succeeded: false,
                    completed_at_epoch_s: 1,
                    detail: "failed".to_string(),
                }),
            )
            .state,
            "Failing"
        );
    }
}
