//! Crash-safe draining of COPY-binary spool segments into `PostgreSQL`.

use fs4::fs_std::FileExt;
use postgres::{Client, NoTls};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use vitals_core::copybin;
use vitals_core::spool::{
    FIRST_ROW_OFFSET, SAMPLES_LAYOUT, SEGMENT_FORMAT_VERSION, SEGMENT_HEADER_BYTES, SEGMENT_MAGIC,
    SchemaFingerprint, samples_fingerprint,
};
const ADVISORY_LOCK: i64 = 0x5641_4C53;

/// A validated segment and its durable local offset file.
#[derive(Debug)]
pub struct Segment {
    /// Segment path.
    pub path: PathBuf,
    /// Stable segment identifier stored in `PostgreSQL`.
    pub name: String,
    /// Segment schema fingerprint.
    pub fingerprint: SchemaFingerprint,
    /// Segment size in bytes.
    pub length: u64,
    /// Last locally recorded drained offset.
    pub offset: u64,
    file: File,
    offset_file: File,
    offset_path: PathBuf,
}

/// A named drain failure.
#[derive(Debug)]
pub enum DrainError {
    /// Local spool I/O or framing failure.
    Io(io::Error),
    /// `PostgreSQL` operation failed.
    Database(postgres::Error),
    /// Another drainer owns the database advisory lock.
    LockHeld,
    /// Spool and live table layouts differ.
    FingerprintMismatch {
        /// Fingerprint required by the live table.
        expected: u64,
        /// Fingerprint recorded in the segment.
        actual: u64,
        /// Operator-actionable source of the incompatible schema.
        context: String,
    },
    /// Live `samples` differs from the canonical positional COPY layout.
    SchemaMismatch(String),
    /// `PostgreSQL` reported an impossible offset.
    InvalidOffset {
        /// Segment whose status is invalid.
        segment: String,
        /// Invalid database offset.
        offset: i64,
    },
}

impl std::fmt::Display for DrainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "spool I/O blocked drain: {e}"),
            Self::Database(e) => write!(f, "PostgreSQL blocked drain: {e}"),
            Self::LockHeld => write!(
                f,
                "drain blocked: PostgreSQL advisory lock 0x56414c53 is held"
            ),
            Self::FingerprintMismatch {
                expected,
                actual,
                context,
            } => write!(
                f,
                "drain blocked: {context}: expected samples fingerprint {expected:016x}, found {actual:016x}; drain with the matching pre-migration schema or migrate the segment explicitly"
            ),
            Self::SchemaMismatch(detail) => {
                write!(f, "drain blocked: live samples schema mismatch: {detail}")
            }
            Self::InvalidOffset { segment, offset } => write!(
                f,
                "drain blocked: spool_status offset {offset} is invalid for segment {segment}"
            ),
        }
    }
}

impl std::error::Error for DrainError {}
impl From<io::Error> for DrainError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}
impl From<postgres::Error> for DrainError {
    fn from(value: postgres::Error) -> Self {
        Self::Database(value)
    }
}

/// Connects and obtains the process-wide single-drainer lock.
///
/// # Errors
/// Returns the named connection/query error or [`DrainError::LockHeld`].
pub fn connect_locked(dsn: &str) -> Result<Client, DrainError> {
    let mut client = Client::connect(dsn, NoTls)?;
    let locked: bool = client
        .query_one("SELECT pg_try_advisory_lock($1)", &[&ADVISORY_LOCK])?
        .get(0);
    if !locked {
        return Err(DrainError::LockHeld);
    }
    Ok(client)
}

/// Computes the live `samples` fingerprint in declaration order.
///
/// A mismatch names the first changed column (or column count), rather than
/// reporting only an opaque hash that an operator cannot act on.
///
/// # Errors
/// Returns a database error or names the first field mismatch.
pub fn live_fingerprint(client: &mut Client) -> Result<SchemaFingerprint, DrainError> {
    let rows = client.query(
        "SELECT a.attname, a.atttypid::bigint FROM pg_attribute a WHERE a.attrelid='samples'::regclass AND a.attnum>0 AND NOT a.attisdropped ORDER BY a.attnum",
        &[],
    )?;
    let mut owned = Vec::with_capacity(rows.len());
    for row in rows {
        let name: String = row.get(0);
        let oid: i64 = row.get(1);
        let oid = u32::try_from(oid).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("live type OID {oid} does not fit u32"),
            )
        })?;
        owned.push((name, oid));
    }
    for (index, (expected_name, expected_oid)) in SAMPLES_LAYOUT.iter().enumerate() {
        match owned.get(index) {
            Some((actual_name, actual_oid))
                if actual_name == expected_name && actual_oid == expected_oid => {}
            Some((actual_name, actual_oid)) => {
                return Err(DrainError::SchemaMismatch(format!(
                    "field {} expected {expected_name} oid {expected_oid}, found {actual_name} oid {actual_oid}",
                    index + 1
                )));
            }
            None => {
                return Err(DrainError::SchemaMismatch(format!(
                    "field {} expected {expected_name} oid {expected_oid}, found end of table",
                    index + 1
                )));
            }
        }
    }
    if owned.len() != SAMPLES_LAYOUT.len() {
        let (name, oid) = owned
            .get(SAMPLES_LAYOUT.len())
            .ok_or_else(|| DrainError::SchemaMismatch("samples column count changed".to_owned()))?;
        return Err(DrainError::SchemaMismatch(format!(
            "field {} is unexpected: {name} oid {oid}",
            SAMPLES_LAYOUT.len() + 1
        )));
    }
    let borrowed: Vec<_> = owned.iter().map(|(n, o)| (n.as_str(), *o)).collect();
    Ok(SchemaFingerprint::of(&borrowed))
}

/// Discovers published segments in creation/lexical order.
///
/// # Errors
/// Returns an I/O error if the spool directory cannot be enumerated.
pub fn discover(dir: &Path) -> Result<Vec<PathBuf>, DrainError> {
    let mut paths = Vec::new();
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.extension().and_then(|x| x.to_str()) == Some("spool") {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

/// Opens and exclusively locks one segment and its offset file.
///
/// # Errors
/// Returns a framing, offset, lock, or filesystem error.
#[expect(
    clippy::too_many_lines,
    reason = "validation keeps the segment and its paired offset lock in one auditable operation"
)]
pub fn open_segment(path: &Path) -> Result<Segment, DrainError> {
    let mut file = OpenOptions::new().read(true).open(path)?;
    if !file.try_lock_exclusive()? {
        return Err(io::Error::new(
            io::ErrorKind::WouldBlock,
            format!("{}: segment lock held by writer or drainer", path.display()),
        )
        .into());
    }
    let mut header = [0_u8; SEGMENT_HEADER_BYTES];
    file.read_exact(&mut header)?;
    if header.get(..8) != Some(SEGMENT_MAGIC) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{}: VITSPOOL magic missing", path.display()),
        )
        .into());
    }
    let version = u16::from_be_bytes([header[8], header[9]]);
    if version != SEGMENT_FORMAT_VERSION {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{}: spool format version {version}, expected {SEGMENT_FORMAT_VERSION}",
                path.display()
            ),
        )
        .into());
    }
    let raw: [u8; 8] = header[10..18]
        .try_into()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "fingerprint header truncated"))?;
    let actual = u64::from_be_bytes(raw);
    let fingerprint = SchemaFingerprint::from_value(actual);
    let length = file.metadata()?.len();
    if length < FIRST_ROW_OFFSET {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            format!(
                "{}: segment is {length} bytes, shorter than framing",
                path.display()
            ),
        )
        .into());
    }
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{}: non-UTF8 segment name", path.display()),
            )
        })?
        .to_owned();
    let offset_path = path.with_extension("offset");
    let mut offset_file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(&offset_path)?;
    if !offset_file.try_lock_exclusive()? {
        return Err(io::Error::new(
            io::ErrorKind::WouldBlock,
            format!(
                "{}: offset lock held by another drainer",
                offset_path.display()
            ),
        )
        .into());
    }
    let mut text = String::new();
    offset_file.read_to_string(&mut text)?;
    let offset = if text.trim().is_empty() {
        FIRST_ROW_OFFSET
    } else {
        text.trim().parse().map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{}: invalid offset {:?}",
                    offset_path.display(),
                    text.trim()
                ),
            )
        })?
    };
    if !(FIRST_ROW_OFFSET..=length).contains(&offset) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{}: offset {offset} outside {FIRST_ROW_OFFSET}..={length}",
                path.display()
            ),
        )
        .into());
    }
    Ok(Segment {
        path: path.to_path_buf(),
        name,
        fingerprint,
        length,
        offset,
        file,
        offset_file,
        offset_path,
    })
}

impl Segment {
    /// Drains the undrained suffix and advances DB + local status crash-safely.
    ///
    /// # Errors
    /// Returns without advancing local status on any I/O or database failure.
    pub fn drain(
        &mut self,
        client: &mut Client,
        expected: SchemaFingerprint,
    ) -> Result<u64, DrainError> {
        if self.fingerprint != expected {
            return Err(DrainError::FingerprintMismatch {
                expected: expected.value(),
                actual: self.fingerprint.value(),
                context: format!(
                    "segment {} predates or differs from the live schema",
                    self.name
                ),
            });
        }
        let db_offset = client
            .query_opt(
                "SELECT offset_bytes FROM spool_status WHERE segment=$1",
                &[&self.name],
            )?
            .map(|r| r.get::<_, i64>(0));
        let start = match db_offset {
            Some(v) => u64::try_from(v).map_err(|_| DrainError::InvalidOffset {
                segment: self.name.clone(),
                offset: v,
            })?,
            // The database row is the transactional authority. If it is absent
            // (for example after restoring an older database), replay from the
            // first row; ON CONFLICT makes replay safe. Trusting a newer local
            // offset here could silently discard rows missing from the restore.
            None => FIRST_ROW_OFFSET,
        };
        if !(FIRST_ROW_OFFSET..=self.length).contains(&start) {
            return Err(DrainError::InvalidOffset {
                segment: self.name.clone(),
                offset: i64::try_from(start).unwrap_or(i64::MAX),
            });
        }
        if start == self.length {
            self.persist_offset(start)?;
            self.persist_progress()?;
            return Ok(0);
        }
        self.file.seek(SeekFrom::Start(start))?;

        let mut tx = client.transaction()?;
        tx.batch_execute(
            "CREATE TEMP TABLE staging (LIKE samples INCLUDING DEFAULTS) ON COMMIT DROP",
        )?;
        let mut sink = tx.copy_in("COPY staging FROM STDIN (FORMAT binary)")?;
        sink.write_all(&copybin::copy_header())?;
        // Bound memory independently of segment size. `take` also prevents a
        // concurrently extended file from moving the committed offset beyond
        // the length validated while holding its lock.
        let mut suffix = (&mut self.file).take(self.length - start);
        io::copy(&mut suffix, &mut sink)?;
        sink.write_all(&copybin::copy_trailer())?;
        sink.finish()?;
        // Test-only crash window used by the Pi acceptance script. The
        // transaction remains uncommitted, proving replay after SIGKILL.
        if let Ok(raw) = std::env::var("VITALS_DRAIN_TEST_PAUSE_BEFORE_COMMIT") {
            let seconds = raw.parse::<u64>().map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("VITALS_DRAIN_TEST_PAUSE_BEFORE_COMMIT={raw:?} is not seconds"),
                )
            })?;
            std::thread::sleep(std::time::Duration::from_secs(seconds));
        }
        let inserted = tx.execute("INSERT INTO samples SELECT * FROM staging ON CONFLICT (source,type,session_id,seq) DO NOTHING", &[])?;
        let end = i64::try_from(self.length).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidData, "segment length exceeds BIGINT")
        })?;
        let fp = self.fingerprint.value().to_be_bytes().to_vec();
        tx.execute("INSERT INTO spool_status(segment,offset_bytes,drained,fingerprint,last_advanced) VALUES($1,$2,TRUE,$3,clock_timestamp()) ON CONFLICT(segment) DO UPDATE SET offset_bytes=EXCLUDED.offset_bytes,drained=TRUE,fingerprint=EXCLUDED.fingerprint,last_advanced=EXCLUDED.last_advanced", &[&self.name, &end, &fp])?;
        tx.commit()?;
        self.persist_offset(self.length)?;
        self.persist_progress()?;
        Ok(inserted)
    }

    fn persist_progress(&self) -> Result<(), DrainError> {
        let dir = self.path.parent().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "segment has no parent directory",
            )
        })?;
        let target = dir.join(".drain-status");
        let temporary = dir.join(format!(".drain-status.{}.tmp", std::process::id()));
        let advanced = epoch_seconds();
        match fs::remove_file(&temporary) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)?;
        writeln!(file, "{advanced}")?;
        file.sync_all()?;
        fs::rename(&temporary, &target)?;
        File::open(dir)?.sync_all()?;
        Ok(())
    }

    fn persist_offset(&mut self, offset: u64) -> Result<(), DrainError> {
        self.offset_file.set_len(0)?;
        self.offset_file.seek(SeekFrom::Start(0))?;
        writeln!(self.offset_file, "{offset}")?;
        self.offset_file.sync_all()?;
        self.offset = offset;
        Ok(())
    }

    /// Removes only a fully drained, still-locked segment and its exact offset file.
    ///
    /// # Errors
    /// Returns an I/O error; an undrained segment is never removed.
    pub fn remove_if_drained(self) -> Result<bool, DrainError> {
        if self.offset != self.length {
            return Ok(false);
        }
        fs::remove_file(&self.path)?;
        fs::remove_file(&self.offset_path)?;
        if let Some(parent) = self.path.parent() {
            File::open(parent)?.sync_all()?;
        }
        Ok(true)
    }
}

/// Current epoch seconds for liveness reporting.
#[must_use]
pub fn epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Canonical expected fingerprint.
#[must_use]
pub fn expected_fingerprint() -> SchemaFingerprint {
    samples_fingerprint()
}

#[cfg(test)]
#[expect(clippy::expect_used, reason = "test setup must fail loudly")]
mod tests {
    use super::*;
    use vitals_core::{NodeId, Sample, Spool};

    #[test]
    fn discovers_only_published_segments_in_order() {
        let d = tempfile::tempdir().expect("tempdir");
        File::create(d.path().join("segment-z-000.spool")).expect("z");
        File::create(d.path().join(".segment-a.init.tmp")).expect("tmp");
        File::create(d.path().join("segment-a-000.spool")).expect("a");
        let names: Vec<_> = discover(d.path())
            .expect("discover")
            .into_iter()
            .filter_map(|p| p.file_name().map(std::borrow::ToOwned::to_owned))
            .collect();
        assert_eq!(names, ["segment-a-000.spool", "segment-z-000.spool"]);
    }

    #[test]
    fn writer_lock_prevents_opening_active_segment() {
        let d = tempfile::tempdir().expect("tempdir");
        let mut spool = Spool::open(d.path()).expect("spool");
        let sample = Sample {
            source: 1,
            r#type: 1,
            session_id: 1,
            seq: 1,
            timestamp: UNIX_EPOCH,
            value: 1.0,
            extra: None,
            batch_id: None,
        };
        spool
            .append(&[sample], &NodeId::from_bytes([1; 16]), true)
            .expect("append");
        let error =
            open_segment(spool.segment_path()).expect_err("active segment must remain locked");
        assert!(error.to_string().contains("segment lock held"));
    }

    fn published_segment(dir: &Path) -> PathBuf {
        let mut spool = Spool::open(dir).expect("spool");
        let sample = Sample {
            source: 1,
            r#type: 1,
            session_id: 1,
            seq: 1,
            timestamp: UNIX_EPOCH,
            value: 1.0,
            extra: None,
            batch_id: None,
        };
        spool
            .append(&[sample], &NodeId::from_bytes([1; 16]), true)
            .expect("append");
        let path = spool.segment_path().to_path_buf();
        drop(spool);
        path
    }

    #[test]
    fn rejects_bad_magic_version_and_truncation() {
        let d = tempfile::tempdir().expect("tempdir");
        let path = published_segment(d.path());
        let original = fs::read(&path).expect("read");

        let mut bad = original.clone();
        *bad.first_mut().expect("magic byte") ^= 1;
        fs::write(&path, &bad).expect("bad magic");
        assert!(
            open_segment(&path)
                .expect_err("magic")
                .to_string()
                .contains("magic")
        );

        let mut bad = original.clone();
        bad.get_mut(8..10)
            .expect("version bytes")
            .copy_from_slice(&99_u16.to_be_bytes());
        fs::write(&path, &bad).expect("bad version");
        assert!(
            open_segment(&path)
                .expect_err("version")
                .to_string()
                .contains("version")
        );

        fs::write(&path, original.get(..10).expect("truncated prefix")).expect("truncate");
        assert!(
            open_segment(&path)
                .expect_err("truncated")
                .to_string()
                .contains("I/O")
        );
    }

    #[test]
    fn rejects_invalid_and_out_of_range_offsets() {
        let d = tempfile::tempdir().expect("tempdir");
        let path = published_segment(d.path());
        let offset = path.with_extension("offset");
        fs::write(&offset, "not-a-number\n").expect("invalid offset");
        assert!(
            open_segment(&path)
                .expect_err("invalid")
                .to_string()
                .contains("invalid offset")
        );
        fs::write(&offset, "0\n").expect("small offset");
        assert!(
            open_segment(&path)
                .expect_err("range")
                .to_string()
                .contains("outside")
        );
    }

    #[test]
    fn undrained_segment_is_never_removed() {
        let d = tempfile::tempdir().expect("tempdir");
        let path = published_segment(d.path());
        let segment = open_segment(&path).expect("open");
        assert!(!segment.remove_if_drained().expect("refuse removal"));
        assert!(path.exists());
    }
}
