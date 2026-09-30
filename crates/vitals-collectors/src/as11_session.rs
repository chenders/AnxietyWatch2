//! Conservative `AirSense` therapy-session boundary policy.
//!
//! This module distinguishes device therapy lifecycle evidence from the BLE
//! transport session. It keeps a stopped session provisional while post-stop
//! stream activity may still represent hose clearing or device shutdown, and
//! appends small derived snapshots to a private durable journal.

use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

/// Minimum post-stop settling interval before a quiet stream may finalize.
pub const POST_STOP_SETTLE_MICROS: i64 = 30_000_000;
/// Maximum post-stop settling interval before finalization is allowed.
pub const POST_STOP_SETTLE_MAX_MICROS: i64 = 180_000_000;
const JOURNAL_NAME: &str = "therapy-sessions.jsonl";
const MAX_JOURNAL_BYTES: u64 = 4 * 1024 * 1024;
const MAX_SNAPSHOT_BYTES: usize = 2048;
const SNAPSHOT_VERSION: u8 = 1;

/// Conservative therapy-session state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TherapyState {
    /// No lifecycle boundary has been observed.
    #[default]
    Unknown,
    /// Explicit start evidence has been observed.
    Active,
    /// Stop evidence has been observed but settling is incomplete.
    Provisional,
    /// The post-stop boundary passed without a new start.
    Complete,
}

/// An error from the durable therapy-session journal or lifecycle policy.
#[derive(Debug)]
pub enum TherapyError {
    /// The journal filesystem operation failed.
    Io(io::Error),
    /// The journal path is not a private regular file.
    InsecureJournal,
    /// The journal exceeded its finite bound.
    JournalTooLarge {
        /// Observed journal length.
        len: u64,
        /// Maximum journal length.
        max: u64,
    },
    /// A snapshot exceeded its finite bound.
    SnapshotTooLarge {
        /// Observed snapshot length.
        len: usize,
        /// Maximum snapshot length.
        max: usize,
    },
    /// A journal snapshot was malformed or used an unknown version.
    SnapshotInvalid,
    /// The durable session identity cannot advance.
    SessionIdExhausted,
}

impl core::fmt::Display for TherapyError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "therapy session journal I/O failed: {error}"),
            Self::InsecureJournal => f.write_str("therapy session journal is not private"),
            Self::JournalTooLarge { len, max } => {
                write!(f, "therapy session journal bytes={len} exceeds limit={max}")
            }
            Self::SnapshotTooLarge { len, max } => {
                write!(
                    f,
                    "therapy session snapshot bytes={len} exceeds limit={max}"
                )
            }
            Self::SnapshotInvalid => f.write_str("therapy session snapshot is invalid"),
            Self::SessionIdExhausted => f.write_str("therapy session identity exhausted"),
        }
    }
}

impl std::error::Error for TherapyError {}

impl From<io::Error> for TherapyError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// Host-receipt lifecycle tracker for one current therapy session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TherapyLifecycle {
    session_id: u64,
    state: TherapyState,
    start_host_unix_micros: Option<i64>,
    stop_host_unix_micros: Option<i64>,
    finalized_host_unix_micros: Option<i64>,
    stream_after_stop: bool,
}

impl TherapyLifecycle {
    /// Current durable therapy-session identity, or zero before a start.
    #[must_use]
    pub const fn session_id(self) -> u64 {
        self.session_id
    }

    /// Current lifecycle state.
    #[must_use]
    pub const fn state(self) -> TherapyState {
        self.state
    }

    /// Host receipt of the first start evidence.
    #[must_use]
    pub const fn start_host_unix_micros(self) -> Option<i64> {
        self.start_host_unix_micros
    }

    /// Host receipt of stop evidence.
    #[must_use]
    pub const fn stop_host_unix_micros(self) -> Option<i64> {
        self.stop_host_unix_micros
    }

    /// Host receipt at which the conservative boundary was finalized.
    #[must_use]
    pub const fn finalized_host_unix_micros(self) -> Option<i64> {
        self.finalized_host_unix_micros
    }

    /// Whether stream evidence arrived after stop evidence.
    #[must_use]
    pub const fn stream_after_stop(self) -> bool {
        self.stream_after_stop
    }

    /// Records one lifecycle event by its protocol name.
    ///
    /// `TherapyStart` and `MaskOn` establish activity. `TherapyStop`,
    /// `MaskOff`, and `StandbyStarted` begin the provisional post-stop window.
    /// A start during that window resumes the same current session rather than
    /// silently creating a second one.
    ///
    /// # Errors
    /// Returns [`TherapyError::SessionIdExhausted`] if a new session cannot be
    /// assigned a durable identity.
    pub fn observe_event(
        &mut self,
        name: &[u8],
        host_unix_micros: i64,
    ) -> Result<(), TherapyError> {
        if matches!(name, b"TherapyStart" | b"MaskOn") {
            if self.state == TherapyState::Complete || self.start_host_unix_micros.is_none() {
                self.session_id = self
                    .session_id
                    .checked_add(1)
                    .ok_or(TherapyError::SessionIdExhausted)?;
                self.start_host_unix_micros = Some(host_unix_micros);
            }
            self.state = TherapyState::Active;
            self.stop_host_unix_micros = None;
            self.finalized_host_unix_micros = None;
            self.stream_after_stop = false;
        } else if matches!(name, b"TherapyStop" | b"MaskOff" | b"StandbyStarted")
            && self.state != TherapyState::Complete
        {
            if self.start_host_unix_micros.is_none() {
                self.session_id = self
                    .session_id
                    .checked_add(1)
                    .ok_or(TherapyError::SessionIdExhausted)?;
                self.start_host_unix_micros = Some(host_unix_micros);
            }
            self.stop_host_unix_micros = Some(host_unix_micros);
            self.finalized_host_unix_micros = None;
            self.stream_after_stop = false;
            self.state = TherapyState::Provisional;
        }
        Ok(())
    }

    /// Marks an active session as unknown after the collector process restarts.
    ///
    /// The stop event may have occurred while the Pi was down, so restoring
    /// `Active` would claim current therapy without evidence. The durable
    /// session identity and its observed start are retained; a fresh lifecycle
    /// event must establish the next state.
    pub fn mark_process_restart(&mut self) -> bool {
        if self.state != TherapyState::Active {
            return false;
        }
        self.state = TherapyState::Unknown;
        self.stop_host_unix_micros = None;
        self.finalized_host_unix_micros = None;
        self.stream_after_stop = false;
        true
    }

    /// Records a stream envelope received after a stop boundary.
    pub fn observe_stream(&mut self) {
        if self.state == TherapyState::Provisional {
            self.stream_after_stop = true;
        }
    }

    /// Advances the conservative post-stop boundary.
    ///
    /// Returns `true` when the public state changed. A quiet post-stop period
    /// may finalize after 30 seconds; any stream activity after the stop keeps
    /// the state provisional until the 180-second maximum.
    pub fn advance(&mut self, now_unix_micros: i64) -> bool {
        if self.state != TherapyState::Provisional {
            return false;
        }
        let Some(stop) = self.stop_host_unix_micros else {
            return false;
        };
        let elapsed = now_unix_micros.saturating_sub(stop);
        let can_finalize = if self.stream_after_stop {
            elapsed >= POST_STOP_SETTLE_MAX_MICROS
        } else {
            elapsed >= POST_STOP_SETTLE_MICROS
        };
        if !can_finalize {
            return false;
        }
        self.state = TherapyState::Complete;
        self.finalized_host_unix_micros = Some(now_unix_micros);
        true
    }

    fn snapshot(self) -> TherapySnapshot {
        TherapySnapshot {
            version: SNAPSHOT_VERSION,
            session_id: self.session_id,
            state: self.state,
            start_host_unix_micros: self.start_host_unix_micros,
            stop_host_unix_micros: self.stop_host_unix_micros,
            finalized_host_unix_micros: self.finalized_host_unix_micros,
            stream_after_stop: self.stream_after_stop,
        }
    }

    fn restore(&mut self, snapshot: TherapySnapshot) -> Result<(), TherapyError> {
        if snapshot.version != SNAPSHOT_VERSION {
            return Err(TherapyError::SnapshotInvalid);
        }
        self.session_id = snapshot.session_id;
        self.state = snapshot.state;
        self.start_host_unix_micros = snapshot.start_host_unix_micros;
        self.stop_host_unix_micros = snapshot.stop_host_unix_micros;
        self.finalized_host_unix_micros = snapshot.finalized_host_unix_micros;
        self.stream_after_stop = snapshot.stream_after_stop;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
struct TherapySnapshot {
    version: u8,
    session_id: u64,
    state: TherapyState,
    start_host_unix_micros: Option<i64>,
    stop_host_unix_micros: Option<i64>,
    finalized_host_unix_micros: Option<i64>,
    stream_after_stop: bool,
}

/// Append-only durable therapy lifecycle snapshots.
pub struct TherapyJournal {
    file: File,
    path: PathBuf,
    lifecycle: TherapyLifecycle,
}

impl TherapyJournal {
    /// Opens and replays the private therapy journal in `dir`.
    ///
    /// # Errors
    /// Returns [`TherapyError`] when the journal is inaccessible, malformed,
    /// non-private, or outside its finite bounds.
    pub fn open(dir: &Path) -> Result<Self, TherapyError> {
        let path = dir.join(JOURNAL_NAME);
        let existing = match fs::symlink_metadata(&path) {
            Ok(metadata) => {
                if !metadata.is_file() || is_insecure(&metadata) {
                    return Err(TherapyError::InsecureJournal);
                }
                if metadata.len() > MAX_JOURNAL_BYTES {
                    return Err(TherapyError::JournalTooLarge {
                        len: metadata.len(),
                        max: MAX_JOURNAL_BYTES,
                    });
                }
                Some(fs::read(&path)?)
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => return Err(TherapyError::Io(error)),
        };
        let file_was_existing = existing.is_some();
        let mut lifecycle = TherapyLifecycle::default();
        if let Some(bytes) = existing {
            for line in bytes
                .split(|byte| *byte == b'\n')
                .filter(|line| !line.is_empty())
            {
                if line.len() > MAX_SNAPSHOT_BYTES {
                    return Err(TherapyError::SnapshotTooLarge {
                        len: line.len(),
                        max: MAX_SNAPSHOT_BYTES,
                    });
                }
                let snapshot = serde_json::from_slice::<TherapySnapshot>(line)
                    .map_err(|_| TherapyError::SnapshotInvalid)?;
                lifecycle.restore(snapshot)?;
            }
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&path)?;
        if !file_was_existing {
            set_private(&file)?;
        }
        if is_insecure(&file.metadata()?) {
            return Err(TherapyError::InsecureJournal);
        }
        Ok(Self {
            file,
            path,
            lifecycle,
        })
    }

    /// Restored current lifecycle state.
    #[must_use]
    pub const fn lifecycle(&self) -> TherapyLifecycle {
        self.lifecycle
    }

    /// Appends and fsyncs the latest derived lifecycle snapshot.
    ///
    /// # Errors
    /// Returns [`TherapyError`] when the snapshot cannot be serialized or the
    /// private journal cannot be extended and synced within its bound.
    pub fn append(&mut self, lifecycle: TherapyLifecycle) -> Result<(), TherapyError> {
        let mut line =
            serde_json::to_vec(&lifecycle.snapshot()).map_err(|_| TherapyError::SnapshotInvalid)?;
        line.push(b'\n');
        if line.len() > MAX_SNAPSHOT_BYTES {
            return Err(TherapyError::SnapshotTooLarge {
                len: line.len(),
                max: MAX_SNAPSHOT_BYTES,
            });
        }
        let next_len = self
            .file
            .metadata()?
            .len()
            .checked_add(
                u64::try_from(line.len()).map_err(|_| TherapyError::SnapshotTooLarge {
                    len: line.len(),
                    max: MAX_SNAPSHOT_BYTES,
                })?,
            )
            .ok_or(TherapyError::JournalTooLarge {
                len: u64::MAX,
                max: MAX_JOURNAL_BYTES,
            })?;
        if next_len > MAX_JOURNAL_BYTES {
            return Err(TherapyError::JournalTooLarge {
                len: next_len,
                max: MAX_JOURNAL_BYTES,
            });
        }
        self.file.write_all(&line)?;
        self.file.sync_all()?;
        self.lifecycle = lifecycle;
        Ok(())
    }

    /// Journal path, for diagnostics that do not include payload data.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(unix)]
fn set_private(file: &File) -> Result<(), TherapyError> {
    use std::os::unix::fs::PermissionsExt;
    file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_private(_file: &File) -> Result<(), TherapyError> {
    Ok(())
}

#[cfg(unix)]
fn is_insecure(metadata: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o077 != 0
}

#[cfg(not(unix))]
fn is_insecure(_metadata: &std::fs::Metadata) -> bool {
    false
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    reason = "therapy journal tests use fixed local fixtures and assertion-only access"
)]
mod tests {
    use super::*;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn temp_dir() -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("aw2-therapy-{suffix}"));
        fs::create_dir(&path).expect("directory");
        path
    }

    #[test]
    fn explicit_start_and_stop_are_distinct_from_transport_state() {
        let mut lifecycle = TherapyLifecycle::default();
        lifecycle
            .observe_event(b"TherapyStart", 1_000_000)
            .expect("start");
        assert_eq!(lifecycle.state(), TherapyState::Active);
        assert_eq!(lifecycle.session_id(), 1);
        lifecycle
            .observe_event(b"TherapyStop", 2_000_000)
            .expect("stop");
        assert_eq!(lifecycle.state(), TherapyState::Provisional);
        assert!(!lifecycle.advance(2_000_000 + POST_STOP_SETTLE_MICROS - 1));
        assert!(lifecycle.advance(2_000_000 + POST_STOP_SETTLE_MICROS));
        assert_eq!(lifecycle.state(), TherapyState::Complete);
    }

    #[test]
    fn post_stop_stream_activity_uses_the_maximum_settle_window() {
        let mut lifecycle = TherapyLifecycle::default();
        lifecycle
            .observe_event(b"MaskOn", 10_000_000)
            .expect("start");
        lifecycle
            .observe_event(b"MaskOff", 20_000_000)
            .expect("stop");
        lifecycle.observe_stream();
        assert!(!lifecycle.advance(20_000_000 + POST_STOP_SETTLE_MICROS));
        assert!(lifecycle.advance(20_000_000 + POST_STOP_SETTLE_MAX_MICROS));
        assert_eq!(lifecycle.state(), TherapyState::Complete);
    }

    #[test]
    fn active_session_becomes_unknown_after_process_restart() {
        let mut lifecycle = TherapyLifecycle::default();
        lifecycle
            .observe_event(b"TherapyStart", 100)
            .expect("start");
        assert!(lifecycle.mark_process_restart());
        assert_eq!(lifecycle.state(), TherapyState::Unknown);
        assert_eq!(lifecycle.session_id(), 1);
        assert_eq!(lifecycle.start_host_unix_micros(), Some(100));
        assert!(!lifecycle.mark_process_restart());
        lifecycle.observe_event(b"TherapyStop", 200).expect("stop");
        assert_eq!(lifecycle.state(), TherapyState::Provisional);
    }

    #[test]
    fn a_new_start_during_settling_resumes_without_duplication() {
        let mut lifecycle = TherapyLifecycle::default();
        lifecycle
            .observe_event(b"TherapyStart", 100)
            .expect("start");
        lifecycle.observe_event(b"TherapyStop", 200).expect("stop");
        lifecycle
            .observe_event(b"TherapyStart", 300)
            .expect("resume");
        assert_eq!(lifecycle.state(), TherapyState::Active);
        assert_eq!(lifecycle.session_id(), 1);
        assert_eq!(lifecycle.start_host_unix_micros(), Some(100));
        assert_eq!(lifecycle.stop_host_unix_micros(), None);
    }

    #[test]
    fn journal_replay_restores_the_latest_snapshot() {
        let dir = temp_dir();
        let mut journal = TherapyJournal::open(&dir).expect("open");
        let mut lifecycle = TherapyLifecycle::default();
        lifecycle
            .observe_event(b"TherapyStart", 100)
            .expect("start");
        journal.append(lifecycle).expect("append");
        lifecycle.observe_event(b"TherapyStop", 200).expect("stop");
        journal.append(lifecycle).expect("append");
        drop(journal);
        let restored = TherapyJournal::open(&dir).expect("reopen");
        assert_eq!(restored.lifecycle(), lifecycle);
        fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn unrelated_events_do_not_create_a_session_boundary() {
        let mut lifecycle = TherapyLifecycle::default();
        lifecycle.observe_event(b"ValueChange", 100).expect("value");
        lifecycle.observe_event(b"Leak", 200).expect("leak");
        assert_eq!(lifecycle.state(), TherapyState::Unknown);
    }
}
