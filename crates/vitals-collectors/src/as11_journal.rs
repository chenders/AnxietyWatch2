//! Bounded append-only journal for decrypted `AirSense` `StreamData` envelopes.
//!
//! The journal is deliberately separate from the `PostgreSQL` spool. It contains
//! only bounded stream envelopes and host receipt times, never authentication
//! requests, challenges, nonces, or keys. A collector writes a record before it
//! attempts semantic decoding so a decoder defect remains replayable.

use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use crate::as11_crypto::MAX_JSON_RESPONSE_BYTES;

/// Maximum bytes retained in one raw journal segment.
pub const MAX_JOURNAL_BYTES: u64 = 8 * 1024 * 1024;
/// Maximum decrypted JSON envelope length accepted in one journal record.
pub const MAX_JOURNAL_PAYLOAD_BYTES: usize = MAX_JSON_RESPONSE_BYTES;
/// Maximum records returned by one bounded replay operation.
pub const MAX_JOURNAL_RECORDS: usize = 4_096;
/// Maximum age of a non-empty active journal before the next append rotates it.
pub const MAX_JOURNAL_AGE: Duration = Duration::from_hours(1);
/// Maximum interval between journal fsyncs on the streaming path.
pub const JOURNAL_SYNC_INTERVAL: Duration = Duration::from_millis(250);
/// Bytes occupied by one record's receipt timestamp and length prefix.
pub const JOURNAL_RECORD_HEADER_BYTES: usize = 12;

const JOURNAL_ACTIVE_NAME: &str = "stream.current";
const JOURNAL_PREFIX: &str = "stream-";
const JOURNAL_SUFFIX: &str = ".journal";

/// A bounded journal failure. No variant contains an envelope or credential.
#[derive(Debug)]
pub enum JournalError {
    /// The journal file or directory could not be accessed.
    Io(io::Error),
    /// A journal path exists but is not a regular file.
    NotRegularFile,
    /// A journal file is readable by group or other users.
    InsecurePermissions,
    /// A record's payload exceeds the fixed envelope bound.
    PayloadTooLarge {
        /// Observed payload length.
        len: usize,
        /// Maximum accepted payload length.
        max: usize,
    },
    /// A journal segment is larger than its finite segment bound.
    SegmentTooLarge {
        /// Observed segment length.
        len: u64,
        /// Maximum segment length.
        max: u64,
    },
    /// A journal contains more complete records than one replay can retain.
    TooManyRecords {
        /// Observed record count.
        count: usize,
        /// Maximum replay record count.
        max: usize,
    },
    /// The generation number cannot advance without wrapping.
    GenerationExhausted,
    /// A complete record has an invalid length field.
    RecordLengthInvalid {
        /// Observed length field.
        len: u64,
        /// Maximum accepted payload length.
        max: usize,
    },
}

impl core::fmt::Display for JournalError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "air11 journal I/O failed: {error}"),
            Self::NotRegularFile => f.write_str("air11 journal path is not a regular file"),
            Self::InsecurePermissions => f.write_str("air11 journal permissions are not private"),
            Self::PayloadTooLarge { len, max } => {
                write!(f, "air11 journal payload length={len} exceeds limit={max}")
            }
            Self::SegmentTooLarge { len, max } => {
                write!(f, "air11 journal segment length={len} exceeds limit={max}")
            }
            Self::TooManyRecords { count, max } => {
                write!(f, "air11 journal records={count} exceeds limit={max}")
            }
            Self::GenerationExhausted => f.write_str("air11 journal generation exhausted"),
            Self::RecordLengthInvalid { len, max } => {
                write!(f, "air11 journal record length={len} exceeds limit={max}")
            }
        }
    }
}

impl std::error::Error for JournalError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<io::Error> for JournalError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// One replayable journal record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalRecord {
    /// Host receipt timestamp in Unix microseconds.
    pub receipt_unix_micros: i64,
    /// The bounded decrypted `StreamData` envelope.
    pub payload: Vec<u8>,
}

/// Result of reading a segment. A torn final record is reported, not treated as
/// a valid envelope; complete records before it remain replayable.
#[derive(Debug, PartialEq, Eq)]
pub struct JournalRead {
    /// Complete records in source order.
    pub records: Vec<JournalRecord>,
    /// Whether the segment ended in a partial header or payload.
    pub torn_tail: bool,
}

/// One active bounded journal segment.
#[derive(Debug)]
pub struct RawJournal {
    dir: PathBuf,
    file: File,
    path: PathBuf,
    bytes: u64,
    records: usize,
    opened: Instant,
    last_sync: Instant,
    dirty: bool,
    generation: u64,
}

impl RawJournal {
    /// Opens the active segment, repairing only a torn final record.
    ///
    /// # Errors
    /// Returns an error for inaccessible, non-private, oversized, or corrupt
    /// complete records. A power-loss tail is truncated to its last complete
    /// record and reported by [`read_records`] when replayed.
    pub fn open(dir: &Path) -> Result<Self, JournalError> {
        fs::create_dir_all(dir)?;
        let path = active_path(dir);
        let file = match private_file_metadata(&path) {
            Ok(_) => open_existing_private(&path)?,
            Err(JournalError::Io(error)) if error.kind() == io::ErrorKind::NotFound => {
                create_new_private(&path)?
            }
            Err(error) => return Err(error),
        };
        let length = file.metadata()?.len();
        if length > MAX_JOURNAL_BYTES {
            return Err(JournalError::SegmentTooLarge {
                len: length,
                max: MAX_JOURNAL_BYTES,
            });
        }
        let bytes_on_disk = fs::read(&path)?;
        let scan = scan_records(&bytes_on_disk)?;
        if scan.valid_bytes < bytes_on_disk.len() {
            file.set_len(u64::try_from(scan.valid_bytes).map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "journal offset overflow")
            })?)?;
            file.sync_all()?;
        }
        let generation = newest_generation(dir)?.map_or(0, |value| value);
        let bytes = file.metadata()?.len();
        let now = Instant::now();
        Ok(Self {
            dir: dir.to_path_buf(),
            file,
            path,
            bytes,
            records: scan.complete_records,
            opened: now,
            last_sync: now,
            dirty: false,
            generation,
        })
    }

    /// The active segment path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Bytes occupied by the active segment.
    #[must_use]
    pub const fn bytes(&self) -> u64 {
        self.bytes
    }

    /// Appends one bounded envelope before semantic decoding.
    ///
    /// The write is fsynced at most [`JOURNAL_SYNC_INTERVAL`] after the last
    /// sync. Call [`Self::sync`] before shutdown or after a fatal dependency
    /// failure when the caller needs the latest record durable immediately.
    ///
    /// # Errors
    /// Returns an error without writing when the payload exceeds the fixed
    /// bound or the segment cannot be written or rotated.
    pub fn append(&mut self, receipt_unix_micros: i64, payload: &[u8]) -> Result<(), JournalError> {
        if payload.len() > MAX_JOURNAL_PAYLOAD_BYTES {
            return Err(JournalError::PayloadTooLarge {
                len: payload.len(),
                max: MAX_JOURNAL_PAYLOAD_BYTES,
            });
        }
        let record_bytes = JOURNAL_RECORD_HEADER_BYTES
            .checked_add(payload.len())
            .ok_or(JournalError::PayloadTooLarge {
                len: payload.len(),
                max: MAX_JOURNAL_PAYLOAD_BYTES,
            })?;
        let record_bytes_u64 =
            u64::try_from(record_bytes).map_err(|_| JournalError::PayloadTooLarge {
                len: payload.len(),
                max: MAX_JOURNAL_PAYLOAD_BYTES,
            })?;
        if record_bytes_u64 > MAX_JOURNAL_BYTES {
            return Err(JournalError::PayloadTooLarge {
                len: payload.len(),
                max: MAX_JOURNAL_PAYLOAD_BYTES,
            });
        }
        if self.rotation_due()
            || self
                .bytes
                .checked_add(record_bytes_u64)
                .is_none_or(|next| next > MAX_JOURNAL_BYTES)
        {
            self.rotate()?;
        }

        self.file.write_all(&receipt_unix_micros.to_le_bytes())?;
        let length = u32::try_from(payload.len()).map_err(|_| JournalError::PayloadTooLarge {
            len: payload.len(),
            max: MAX_JOURNAL_PAYLOAD_BYTES,
        })?;
        self.file.write_all(&length.to_le_bytes())?;
        self.file.write_all(payload)?;
        self.bytes =
            self.bytes
                .checked_add(record_bytes_u64)
                .ok_or(JournalError::SegmentTooLarge {
                    len: u64::MAX,
                    max: MAX_JOURNAL_BYTES,
                })?;
        self.records = self.records.saturating_add(1);
        self.dirty = true;
        if self.last_sync.elapsed() >= JOURNAL_SYNC_INTERVAL {
            self.sync()?;
        }
        Ok(())
    }

    /// Whether the active segment should be rotated before another append.
    #[must_use]
    pub fn rotation_due(&self) -> bool {
        self.bytes >= MAX_JOURNAL_BYTES
            || self.records >= MAX_JOURNAL_RECORDS
            || (self.bytes > 0 && self.opened.elapsed() >= MAX_JOURNAL_AGE)
    }

    /// Fsyncs the active segment when it has changed.
    ///
    /// # Errors
    /// Returns the underlying fsync failure.
    pub fn sync(&mut self) -> Result<(), JournalError> {
        if self.dirty {
            self.file.sync_all()?;
            self.dirty = false;
        }
        self.last_sync = Instant::now();
        Ok(())
    }

    fn rotate(&mut self) -> Result<(), JournalError> {
        self.sync()?;
        let generation = self
            .generation
            .checked_add(1)
            .ok_or(JournalError::GenerationExhausted)?;
        let archive = journal_path(&self.dir, generation);
        fs::rename(&self.path, &archive)?;
        sync_directory(&self.dir)?;
        let file = create_new_private(&self.path)?;
        self.file = file;
        self.bytes = 0;
        self.records = 0;
        self.opened = Instant::now();
        self.last_sync = Instant::now();
        self.dirty = false;
        self.generation = generation;
        Ok(())
    }
}

impl Drop for RawJournal {
    fn drop(&mut self) {
        let _ = self.sync();
    }
}

/// Reads complete records from one segment without allocating from an
/// untrusted length field. A partial final record is a recoverable tail.
///
/// # Errors
/// Returns an error for a non-private/non-regular file, an oversized segment,
/// an invalid complete record length, or more than the replay record bound.
pub fn read_records(path: &Path) -> Result<JournalRead, JournalError> {
    let metadata = private_file_metadata(path)?;
    if metadata.len() > MAX_JOURNAL_BYTES {
        return Err(JournalError::SegmentTooLarge {
            len: metadata.len(),
            max: MAX_JOURNAL_BYTES,
        });
    }
    let bytes = fs::read(path)?;
    let scan = scan_records(&bytes)?;
    let mut records = Vec::with_capacity(MAX_JOURNAL_RECORDS.min(scan.complete_records));
    let mut offset = 0usize;
    while offset < scan.valid_bytes {
        let header_end = offset.checked_add(JOURNAL_RECORD_HEADER_BYTES).ok_or(
            JournalError::RecordLengthInvalid {
                len: u64::MAX,
                max: MAX_JOURNAL_PAYLOAD_BYTES,
            },
        )?;
        let header = bytes
            .get(offset..header_end)
            .ok_or(JournalError::RecordLengthInvalid {
                len: u64::MAX,
                max: MAX_JOURNAL_PAYLOAD_BYTES,
            })?;
        let timestamp_bytes: [u8; 8] = header
            .get(..8)
            .ok_or(JournalError::RecordLengthInvalid {
                len: u64::MAX,
                max: MAX_JOURNAL_PAYLOAD_BYTES,
            })?
            .try_into()
            .map_err(|_| JournalError::RecordLengthInvalid {
                len: u64::MAX,
                max: MAX_JOURNAL_PAYLOAD_BYTES,
            })?;
        let length_bytes: [u8; 4] = header
            .get(8..12)
            .ok_or(JournalError::RecordLengthInvalid {
                len: u64::MAX,
                max: MAX_JOURNAL_PAYLOAD_BYTES,
            })?
            .try_into()
            .map_err(|_| JournalError::RecordLengthInvalid {
                len: u64::MAX,
                max: MAX_JOURNAL_PAYLOAD_BYTES,
            })?;
        let payload_len = usize::try_from(u32::from_le_bytes(length_bytes)).map_err(|_| {
            JournalError::RecordLengthInvalid {
                len: u64::MAX,
                max: MAX_JOURNAL_PAYLOAD_BYTES,
            }
        })?;
        let payload_begin = header_end;
        let payload_end =
            payload_begin
                .checked_add(payload_len)
                .ok_or(JournalError::RecordLengthInvalid {
                    len: u64::MAX,
                    max: MAX_JOURNAL_PAYLOAD_BYTES,
                })?;
        let payload =
            bytes
                .get(payload_begin..payload_end)
                .ok_or(JournalError::RecordLengthInvalid {
                    len: u64::MAX,
                    max: MAX_JOURNAL_PAYLOAD_BYTES,
                })?;
        if records.len() >= MAX_JOURNAL_RECORDS {
            return Err(JournalError::TooManyRecords {
                count: records.len().saturating_add(1),
                max: MAX_JOURNAL_RECORDS,
            });
        }
        records.push(JournalRecord {
            receipt_unix_micros: i64::from_le_bytes(timestamp_bytes),
            payload: payload.to_vec(),
        });
        offset = payload_end;
    }
    Ok(JournalRead {
        records,
        torn_tail: scan.valid_bytes < bytes.len(),
    })
}

struct RecordScan {
    valid_bytes: usize,
    complete_records: usize,
}

fn scan_records(bytes: &[u8]) -> Result<RecordScan, JournalError> {
    let mut offset = 0usize;
    let mut complete_records = 0usize;
    while offset < bytes.len() {
        let remaining = bytes.len().saturating_sub(offset);
        if remaining < JOURNAL_RECORD_HEADER_BYTES {
            return Ok(RecordScan {
                valid_bytes: offset,
                complete_records,
            });
        }
        let header_end = offset + JOURNAL_RECORD_HEADER_BYTES;
        let header = bytes
            .get(offset..header_end)
            .ok_or(JournalError::RecordLengthInvalid {
                len: u64::MAX,
                max: MAX_JOURNAL_PAYLOAD_BYTES,
            })?;
        let length_bytes: [u8; 4] = header
            .get(8..12)
            .ok_or(JournalError::RecordLengthInvalid {
                len: u64::MAX,
                max: MAX_JOURNAL_PAYLOAD_BYTES,
            })?
            .try_into()
            .map_err(|_| JournalError::RecordLengthInvalid {
                len: u64::MAX,
                max: MAX_JOURNAL_PAYLOAD_BYTES,
            })?;
        let payload_len = usize::try_from(u32::from_le_bytes(length_bytes)).map_err(|_| {
            JournalError::RecordLengthInvalid {
                len: u64::MAX,
                max: MAX_JOURNAL_PAYLOAD_BYTES,
            }
        })?;
        if payload_len > MAX_JOURNAL_PAYLOAD_BYTES {
            return Err(JournalError::RecordLengthInvalid {
                len: u64::try_from(payload_len).map_or(u64::MAX, |value| value),
                max: MAX_JOURNAL_PAYLOAD_BYTES,
            });
        }
        let record_end =
            header_end
                .checked_add(payload_len)
                .ok_or(JournalError::RecordLengthInvalid {
                    len: u64::MAX,
                    max: MAX_JOURNAL_PAYLOAD_BYTES,
                })?;
        if record_end > bytes.len() {
            return Ok(RecordScan {
                valid_bytes: offset,
                complete_records,
            });
        }
        complete_records = complete_records.saturating_add(1);
        if complete_records > MAX_JOURNAL_RECORDS {
            return Err(JournalError::TooManyRecords {
                count: complete_records,
                max: MAX_JOURNAL_RECORDS,
            });
        }
        offset = record_end;
    }
    Ok(RecordScan {
        valid_bytes: offset,
        complete_records,
    })
}

fn newest_generation(dir: &Path) -> Result<Option<u64>, JournalError> {
    let mut newest = None;
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        let Some(number) = name
            .strip_prefix(JOURNAL_PREFIX)
            .and_then(|value| value.strip_suffix(JOURNAL_SUFFIX))
        else {
            continue;
        };
        let Ok(generation) = number.parse::<u64>() else {
            continue;
        };
        private_file_metadata(&entry.path())?;
        newest = Some(newest.map_or(generation, |oldest: u64| oldest.max(generation)));
    }
    Ok(newest)
}

fn active_path(dir: &Path) -> PathBuf {
    dir.join(JOURNAL_ACTIVE_NAME)
}

fn journal_path(dir: &Path, generation: u64) -> PathBuf {
    dir.join(format!("{JOURNAL_PREFIX}{generation:020}{JOURNAL_SUFFIX}"))
}

fn sync_directory(dir: &Path) -> Result<(), JournalError> {
    File::open(dir)?.sync_all()?;
    Ok(())
}

fn open_existing_private(path: &Path) -> Result<File, JournalError> {
    private_file_metadata(path)?;
    Ok(OpenOptions::new().append(true).read(true).open(path)?)
}

fn create_new_private(path: &Path) -> Result<File, JournalError> {
    let mut options = OpenOptions::new();
    options.write(true).read(true).append(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    Ok(options.open(path)?)
}

fn private_file_metadata(path: &Path) -> Result<std::fs::Metadata, JournalError> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() {
        return Err(JournalError::NotRegularFile);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(JournalError::InsecurePermissions);
        }
    }
    Ok(metadata)
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    reason = "journal tests use fixed local fixtures and assertion-only access"
)]
mod tests {
    use super::*;
    use crate::as11_build::Air11SampleBuilder;
    use std::{
        fs::OpenOptions,
        time::{Duration, SystemTime},
    };
    use tempfile::tempdir;
    use vitals_decode::stream::decode_stream;

    const RECEIPT: i64 = 1_800_000_000_123_456;

    #[test]
    fn complete_records_round_trip_in_source_order() {
        let dir = tempdir().expect("tempdir");
        let mut journal = RawJournal::open(dir.path()).expect("open");
        journal
            .append(RECEIPT, br#"{"method":"StreamData"}"#)
            .expect("first");
        journal.append(RECEIPT + 1, b"second").expect("second");
        journal.sync().expect("sync");
        let read = read_records(journal.path()).expect("read");
        assert!(!read.torn_tail);
        assert_eq!(read.records.len(), 2);
        assert_eq!(
            read.records.first().expect("first").receipt_unix_micros,
            RECEIPT
        );
        assert_eq!(read.records.get(1).expect("second").payload, b"second");
    }

    #[test]
    fn a_torn_tail_is_reported_and_reopen_repairs_it() {
        let dir = tempdir().expect("tempdir");
        let path = {
            let mut journal = RawJournal::open(dir.path()).expect("open");
            journal.append(RECEIPT, b"complete").expect("append");
            journal.sync().expect("sync");
            journal.path().to_path_buf()
        };
        let mut file = OpenOptions::new()
            .append(true)
            .open(&path)
            .expect("open tail");
        file.write_all(&RECEIPT.to_le_bytes())
            .expect("partial header");
        drop(file);
        let read = read_records(&path).expect("read torn");
        assert!(read.torn_tail);
        assert_eq!(read.records.len(), 1);

        let mut journal = RawJournal::open(dir.path()).expect("repair");
        journal
            .append(RECEIPT + 1, b"after-restart")
            .expect("append after repair");
        journal.sync().expect("sync");
        let read = read_records(journal.path()).expect("read repaired");
        assert_eq!(read.records.len(), 2);
        assert_eq!(
            read.records.get(1).expect("second").payload,
            b"after-restart"
        );
    }

    #[test]
    fn payload_bound_is_rejected_before_writing() {
        let dir = tempdir().expect("tempdir");
        let mut journal = RawJournal::open(dir.path()).expect("open");
        let before = journal.bytes();
        let error = journal
            .append(RECEIPT, &vec![0_u8; MAX_JOURNAL_PAYLOAD_BYTES + 1])
            .expect_err("oversized payload");
        assert!(matches!(error, JournalError::PayloadTooLarge { .. }));
        assert_eq!(journal.bytes(), before);
    }

    #[test]
    fn rotation_keeps_the_previous_segment_replayable() {
        let dir = tempdir().expect("tempdir");
        let mut journal = RawJournal::open(dir.path()).expect("open");
        journal.bytes = MAX_JOURNAL_BYTES;
        let old_path = journal.path().to_path_buf();
        journal.append(RECEIPT, b"new segment").expect("rotate");
        journal.sync().expect("sync");
        assert_eq!(journal.path(), old_path.as_path());
        let archive = journal_path(dir.path(), 1);
        assert!(archive.exists());
        assert_eq!(
            read_records(&archive).expect("archive read").records.len(),
            0
        );
        assert_eq!(
            read_records(journal.path())
                .expect("new read")
                .records
                .len(),
            1
        );
    }

    #[test]
    fn pre_epoch_receipt_is_data_not_a_parser_failure() {
        let dir = tempdir().expect("tempdir");
        let mut journal = RawJournal::open(dir.path()).expect("open");
        journal.append(-1, b"before epoch").expect("append");
        let read = read_records(journal.path()).expect("read");
        assert_eq!(
            read.records.first().expect("record").receipt_unix_micros,
            -1
        );
    }

    #[test]
    fn replay_keeps_the_same_source_identity_and_host_receipt() {
        let dir = tempdir().expect("tempdir");
        let mut journal = RawJournal::open(dir.path()).expect("open");
        let payload = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../vitals-decode/fixtures/air11-stream-null.json"
        ));
        journal.append(RECEIPT, payload).expect("journal");
        journal.sync().expect("sync");
        let records = read_records(journal.path()).expect("read").records;
        let frame = decode_stream(records.first().expect("journal record").payload.as_slice())
            .expect("stream frame");
        let receipt = SystemTime::UNIX_EPOCH + Duration::from_hours(500_000);
        let mut first = Air11SampleBuilder::new(77).expect("first builder");
        let original = first.build_frame(&frame, receipt).expect("first build");
        let mut replay = Air11SampleBuilder::new(77).expect("replay builder");
        let repeated = replay.build_frame(&frame, receipt).expect("replay build");
        assert_eq!(original.samples.len(), repeated.samples.len());
        for (left, right) in original.samples.iter().zip(repeated.samples.iter()) {
            assert_eq!(left.source, right.source);
            assert_eq!(left.r#type, right.r#type);
            assert_eq!(left.session_id, right.session_id);
            assert_eq!(left.seq, right.seq);
            assert_eq!(left.timestamp_unix_micros, right.timestamp_unix_micros);
            assert!((left.value - right.value).abs() < 1e-9);
            assert_eq!(left.extra, right.extra);
        }
    }
}
