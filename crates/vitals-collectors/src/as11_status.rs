//! Operator-safe, atomically published `AirSense` collector status.
//!
//! Status is deliberately a facts-and-blockers document. It contains no target
//! address, serial, client identifier, master key, session key, or raw payload.
//! A link can be authenticated without having delivered a usable measurement;
//! link and measurement states therefore remain separate.

use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

/// State of the BLE/session portion of the collector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkState {
    /// The collector is preparing a connection attempt.
    Starting,
    /// `BlueZ` connected and the exact GATT pipe was resolved.
    Connected,
    /// The `AirSense` session integrity exchange succeeded.
    Authenticated,
    /// The fixed stream subscription was accepted.
    Streaming,
    /// The link ended and a bounded reconnect is pending.
    Disconnected,
    /// A dependency or protocol failure stopped publication.
    Degraded,
}

/// State of the physiological evidence portion of the collector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeasurementState {
    /// No frame has supplied a usable numeric value.
    NeverMeasured,
    /// At least one validated numeric value has been published.
    Measured,
    /// Frames arrived but all values were null or unavailable.
    Unavailable,
    /// A frame or source position was rejected and retained for diagnosis.
    Quarantined,
    /// A previously measured stream has gone silent or disconnected.
    Stale,
}

/// State of the read-only `AirSense` event subscription.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum EventSubscriptionState {
    /// No event subscription attempt has completed.
    #[default]
    NotAttempted,
    /// The fixed event subscription was accepted by the device.
    Subscribed,
    /// The stream remains usable, but event coverage is unavailable.
    Unavailable,
}

/// One latest canonical `AirSense` value and its host receipt timestamp.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Air11LatestValue {
    /// Canonical numeric value.
    pub value: f64,
    /// Host receipt timestamp in Unix microseconds.
    pub sample_at_unix_micros: i64,
}

/// Latest canonical values retained for the local bedside projection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Air11Latest {
    /// Respiratory rate in breaths per minute.
    pub respiratory_rate: Option<Air11LatestValue>,
    /// Leak in litres per minute.
    pub leak_lpm: Option<Air11LatestValue>,
    /// Mask pressure in cmH2O.
    pub pressure_cmh2o: Option<Air11LatestValue>,
}

/// The status document written by the `AirSense` collector.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Air11Status {
    /// BLE/session state.
    pub link_state: LinkState,
    /// Numeric-evidence state; it is not inferred from connection state.
    pub measurement_state: MeasurementState,
    /// The persistent upstream session, when authentication has completed.
    pub session_id: Option<i64>,
    /// Number of reconnects attempted after the first connection.
    pub reconnect_count: u64,
    /// Number of bounded retry delays entered.
    pub retry_count: u64,
    /// Number of decrypted envelopes journaled before decode.
    pub received_envelopes: u64,
    /// Number of envelopes accepted by the semantic `StreamData` decoder.
    pub decoded_frames: u64,
    /// Number of numeric rows acknowledged by vitalsd.
    pub published_samples: u64,
    /// Number of null or unavailable values observed.
    pub unavailable: u64,
    /// Number of retransmitted source positions ignored.
    pub duplicates: u64,
    /// Number of malformed, out-of-range, or source-position failures.
    pub quarantined: u64,
    /// Number of envelopes dropped before journaling; normally zero.
    pub dropped: u64,
    /// Whether the last append failure left durability unknown.
    pub durability_unknown: bool,
    /// Host receipt time of the latest journaled envelope, if any.
    pub last_host_receipt_unix_micros: Option<i64>,
    /// Raw device `startTime` from the latest decoded frame, if valid UTF-8.
    pub last_device_start_time: Option<String>,
    /// The current degraded blocker, without private protocol material.
    pub blocker: Option<String>,
    /// State of the fixed lifecycle and respiratory event subscription.
    #[serde(default)]
    pub event_subscription_state: EventSubscriptionState,
    /// Number of event notifications journaled and decoded.
    #[serde(default)]
    pub event_notifications: u64,
    /// Number of event records retained from notifications.
    #[serde(default)]
    pub event_records: u64,
    /// Number of notifications whose event list exceeded the decoder bound.
    #[serde(default)]
    pub event_truncated_notifications: u64,
    /// Last event data id, when it is valid UTF-8.
    #[serde(default)]
    pub last_event_data_id: Option<String>,
    /// Why event coverage is unavailable, without private protocol material.
    #[serde(default)]
    pub event_blocker: Option<String>,
    /// Durable therapy-session identity, when lifecycle evidence exists.
    #[serde(default)]
    pub therapy_session_id: Option<u64>,
    /// Conservative therapy lifecycle state.
    #[serde(default)]
    pub therapy_state: crate::as11_session::TherapyState,
    /// Host receipt of the first observed therapy-start evidence.
    #[serde(default)]
    pub therapy_start_host_unix_micros: Option<i64>,
    /// Host receipt of therapy-stop evidence.
    #[serde(default)]
    pub therapy_stop_host_unix_micros: Option<i64>,
    /// Host receipt at which the post-stop boundary became complete.
    #[serde(default)]
    pub therapy_finalized_host_unix_micros: Option<i64>,
    /// Why a therapy boundary remains provisional, if applicable.
    #[serde(default)]
    pub therapy_blocker: Option<String>,
    /// Latest numeric values for same-host bedside projection.
    #[serde(default)]
    pub latest: Air11Latest,
}

impl Default for Air11Status {
    fn default() -> Self {
        Self {
            link_state: LinkState::Starting,
            measurement_state: MeasurementState::NeverMeasured,
            session_id: None,
            reconnect_count: 0,
            retry_count: 0,
            received_envelopes: 0,
            decoded_frames: 0,
            published_samples: 0,
            unavailable: 0,
            duplicates: 0,
            quarantined: 0,
            dropped: 0,
            durability_unknown: false,
            last_host_receipt_unix_micros: None,
            last_device_start_time: None,
            blocker: None,
            event_subscription_state: EventSubscriptionState::NotAttempted,
            event_notifications: 0,
            event_records: 0,
            event_truncated_notifications: 0,
            last_event_data_id: None,
            event_blocker: None,
            therapy_session_id: None,
            therapy_state: crate::as11_session::TherapyState::Unknown,
            therapy_start_host_unix_micros: None,
            therapy_stop_host_unix_micros: None,
            therapy_finalized_host_unix_micros: None,
            therapy_blocker: None,
            latest: Air11Latest::default(),
        }
    }
}

impl Air11Status {
    /// Publishes this document through a write/sync/rename sequence.
    ///
    /// The temporary file is created privately and with `create_new`, so a
    /// stale temporary symlink cannot redirect a status write. The published
    /// status has no credentials or device identifiers and is group-readable
    /// for the local bedside publisher. The directory is synced after rename
    /// so a power loss cannot silently lose the replacement.
    ///
    /// # Errors
    /// Returns a serialization or filesystem failure. Callers should stop
    /// numeric publication if status cannot be made truthful.
    pub fn write_atomic(&self, path: &Path) -> io::Result<()> {
        let bytes = serde_json::to_vec(self)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        let parent = path.parent().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "status path has no parent")
        })?;
        fs::create_dir_all(parent)?;
        let (temporary, mut file) = create_temporary(parent, path.file_name())?;
        let result = (|| {
            file.write_all(&bytes)?;
            file.sync_all()?;
            fs::rename(&temporary, path)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt as _;
                fs::set_permissions(path, fs::Permissions::from_mode(0o640))?;
            }
            File::open(parent)?.sync_all()
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result
    }
}

fn create_temporary(parent: &Path, name: Option<&std::ffi::OsStr>) -> io::Result<(PathBuf, File)> {
    let name = name
        .and_then(|value| value.to_str())
        .map_or("air11-status.json", |value| value);
    for suffix in 0_u32..16 {
        let temporary = parent.join(format!(".{name}.tmp-{suffix}"));
        let mut options = OpenOptions::new();
        options.write(true).read(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o640);
        }
        match options.open(&temporary) {
            Ok(file) => return Ok((temporary, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "air11 status temporary-file names exhausted",
    ))
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    reason = "status tests use fixed local paths and assertion-only access"
)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn status_is_atomically_written_without_private_fields() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("status.json");
        let status = Air11Status {
            link_state: LinkState::Streaming,
            measurement_state: MeasurementState::Unavailable,
            session_id: Some(17),
            blocker: Some("no usable signal: _HRT=null".to_owned()),
            ..Air11Status::default()
        };
        status.write_atomic(&path).expect("write status");
        let text = fs::read_to_string(&path).expect("read status");
        let decoded: Air11Status = serde_json::from_str(&text).expect("status JSON");
        assert_eq!(decoded, status);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&path)
                    .expect("status metadata")
                    .permissions()
                    .mode()
                    & 0o777,
                0o640
            );
        }
        assert!(text.contains("measurement_state"));
        assert!(!text.contains("AA:BB:CC:DD:EE:FF"));
        assert!(!text.contains("masterPairKey"));
        assert!(!text.contains("sessionKey"));
        assert!(
            fs::read_dir(dir.path())
                .expect("list status directory")
                .filter_map(Result::ok)
                .all(|entry| !entry.file_name().to_string_lossy().contains(".tmp-"))
        );
    }

    #[test]
    fn stale_is_distinct_from_never_measured() {
        assert_ne!(
            MeasurementState::Stale,
            MeasurementState::NeverMeasured,
            "silence after evidence must not look like no startup evidence"
        );
    }
}
