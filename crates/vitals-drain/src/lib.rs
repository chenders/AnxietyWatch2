//! Crash-safe draining of COPY-binary spool segments into `PostgreSQL`.

use fs4::fs_std::FileExt;
use postgres::{Client, NoTls};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use vitals_core::copybin;
pub use vitals_core::spool::DRAIN_HEALTH_FILENAME;
use vitals_core::spool::{
    FIRST_ROW_OFFSET, SAMPLES_LAYOUT, SEGMENT_FORMAT_VERSION, SEGMENT_HEADER_BYTES, SEGMENT_MAGIC,
    SchemaFingerprint, samples_fingerprint,
};
pub use vitals_proto::DRAIN_HEALTH_SCHEMA;
use vitals_proto::DrainHealth;
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

/// The result of one transactional segment drain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrainOutcome {
    /// Rows loaded into the private staging table.
    pub rows_read: u64,
    /// New durable identities inserted into `samples`.
    pub rows_inserted: u64,
    /// Rows replayed and ignored by the durable identity key.
    pub rows_duplicate: u64,
    /// Rows rejected during this successful drain (always zero on success).
    pub rows_rejected: u64,
    /// Byte offset committed for the segment.
    pub offset: u64,
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
    /// A transaction rejected rows after they were read from the spool.
    Rejected {
        /// Rows that could not become durable in this transaction.
        rows: u64,
        /// The database or validation error that rejected them.
        source: Box<DrainError>,
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
            Self::Rejected { rows, source } => {
                write!(f, "drain rejected {rows} row(s): {source}")
            }
        }
    }
}

impl std::error::Error for DrainError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Rejected { source, .. } => Some(source.as_ref()),
            _ => None,
        }
    }
}

impl DrainError {
    /// Number of rows known to have been rejected by the failed operation.
    #[must_use]
    pub fn rejected_rows(&self) -> u64 {
        match self {
            Self::Rejected { rows, .. } => *rows,
            _ => 0,
        }
    }
}
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

/// Validates a `PostgreSQL` type OID before it is used in a schema fingerprint.
fn checked_type_oid(oid: i64) -> Result<u32, io::Error> {
    u32::try_from(oid).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("live type OID {oid} does not fit u32"),
        )
    })
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
        let oid = checked_type_oid(oid)?;
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
    let version = header
        .get(8..10)
        .and_then(|bytes| bytes.try_into().ok())
        .map(u16::from_be_bytes)
        .ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "spool version header truncated")
        })?;
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
    let raw: [u8; 8] = header
        .get(10..18)
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "fingerprint header truncated")
        })?;
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
    ) -> Result<DrainOutcome, DrainError> {
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
            return Ok(DrainOutcome {
                rows_read: 0,
                rows_inserted: 0,
                rows_duplicate: 0,
                rows_rejected: 0,
                offset: start,
            });
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
        let rows_read_i64: i64 = tx.query_one("SELECT count(*) FROM staging", &[])?.get(0);
        let rows_read = u64::try_from(rows_read_i64).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("PostgreSQL returned a negative staging row count {rows_read_i64}"),
            )
        })?;
        // Test-only crash window used by the Pi acceptance script. The
        // transaction remains uncommitted, proving replay after SIGKILL.
        if let Ok(raw) = std::env::var("VITALS_DRAIN_TEST_PAUSE_BEFORE_COMMIT") {
            let seconds = parse_pause_seconds(&raw)?;
            std::thread::sleep(std::time::Duration::from_secs(seconds));
        }
        let rows_inserted = tx
            .execute(
                "INSERT INTO samples SELECT * FROM staging ON CONFLICT (source,type,session_id,seq) DO NOTHING",
                &[],
            )
            .map_err(|error| DrainError::Rejected {
                rows: rows_read,
                source: Box::new(error.into()),
            })?;
        let rows_duplicate = rows_read.checked_sub(rows_inserted).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "staging reported {rows_read} rows but PostgreSQL inserted {rows_inserted}"
                ),
            )
        })?;
        let end = i64::try_from(self.length).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidData, "segment length exceeds BIGINT")
        })?;
        let fp = self.fingerprint.value().to_be_bytes().to_vec();
        tx.execute("INSERT INTO spool_status(segment,offset_bytes,drained,fingerprint,last_advanced) VALUES($1,$2,TRUE,$3,clock_timestamp()) ON CONFLICT(segment) DO UPDATE SET offset_bytes=EXCLUDED.offset_bytes,drained=TRUE,fingerprint=EXCLUDED.fingerprint,last_advanced=EXCLUDED.last_advanced", &[&self.name, &end, &fp])
            .map_err(|error| DrainError::Rejected {
                rows: rows_read,
                source: Box::new(error.into()),
            })?;
        tx.commit()?;
        self.persist_offset(self.length)?;
        self.persist_progress()?;
        Ok(DrainOutcome {
            rows_read,
            rows_inserted,
            rows_duplicate,
            rows_rejected: 0,
            offset: self.length,
        })
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

/// Summary of rows that are still pending in the spool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingSpool {
    /// Number of published segments with an undrained suffix.
    pub segments: u64,
    /// Number of bytes after the locally recorded offset.
    pub bytes: u64,
    /// Age of the oldest pending segment.
    pub oldest_age_secs: Option<u64>,
}

fn local_offset(path: &Path, length: u64) -> Result<u64, DrainError> {
    let offset_path = path.with_extension("offset");
    let text = match fs::read_to_string(&offset_path) {
        Ok(text) => text,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(FIRST_ROW_OFFSET),
        Err(e) => return Err(e.into()),
    };
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(FIRST_ROW_OFFSET);
    }
    let offset = trimmed.parse::<u64>().map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{}: invalid offset {:?}", offset_path.display(), trimmed),
        )
    })?;
    if !(FIRST_ROW_OFFSET..=length).contains(&offset) {
        return Err(DrainError::InvalidOffset {
            segment: path.display().to_string(),
            offset: i64::try_from(offset).unwrap_or(i64::MAX),
        });
    }
    Ok(offset)
}

/// Inspects pending bytes without opening or locking the active segment.
///
/// The writer intentionally keeps the active segment locked, so a health
/// reader must use metadata and its offset sidecar rather than treating a lock
/// as an empty spool. A future modification time is an invalid clock/evidence
/// state, not an age of zero.
///
/// # Errors
/// Returns a named filesystem, offset, or clock error.
pub fn pending_spool(dir: &Path) -> Result<PendingSpool, DrainError> {
    let now = SystemTime::now();
    let mut summary = PendingSpool {
        segments: 0,
        bytes: 0,
        oldest_age_secs: None,
    };
    for path in discover(dir)? {
        let metadata = fs::metadata(&path)?;
        let length = metadata.len();
        if length <= FIRST_ROW_OFFSET {
            continue;
        }
        let offset = local_offset(&path, length)?;
        if offset == length {
            continue;
        }
        let modified = metadata.modified().map_err(|e| {
            io::Error::new(
                e.kind(),
                format!(
                    "{}: cannot establish pending-segment age: {e}",
                    path.display()
                ),
            )
        })?;
        let age = now.duration_since(modified).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{}: modification time is in the future; refusing to report pending age",
                    path.display()
                ),
            )
        })?;
        let pending = length.checked_sub(offset).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{}: offset {offset} exceeds length {length}",
                    path.display()
                ),
            )
        })?;
        summary.segments = summary.segments.saturating_add(1);
        summary.bytes = summary.bytes.saturating_add(pending);
        let age_secs = age.as_secs();
        summary.oldest_age_secs = Some(
            summary
                .oldest_age_secs
                .map_or(age_secs, |oldest| oldest.max(age_secs)),
        );
    }
    Ok(summary)
}

/// Atomically publishes a drainer health snapshot beside the spool.
///
/// The temporary file is written and fsynced before rename, followed by a
/// directory fsync. Readers therefore see either the previous complete JSON
/// document or the new complete document, never a partially written one.
///
/// # Errors
/// Returns serialization or filesystem errors. A failed publication is a
/// monitoring failure and must not be reported as a healthy snapshot.
pub fn publish_health(dir: &Path, health: &DrainHealth) -> Result<(), DrainError> {
    let payload = serde_json::to_vec(health).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("drainer health JSON serialization failed: {e}"),
        )
    })?;
    let target = dir.join(DRAIN_HEALTH_FILENAME);
    let temporary = dir.join(format!(
        ".{DRAIN_HEALTH_FILENAME}.{}.tmp",
        std::process::id()
    ));
    match fs::remove_file(&temporary) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)?;
    file.write_all(&payload)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    fs::rename(&temporary, &target)?;
    File::open(dir)?.sync_all()?;
    Ok(())
}

/// Refreshes pending counts and publishes a successful health snapshot.
///
/// The caller owns cumulative counters and last-error state; this function
/// changes only the current evidence fields and publication timestamp.
///
/// # Errors
/// Returns any pending-spool or publication failure.
pub fn refresh_health(dir: &Path, health: &mut DrainHealth) -> Result<(), DrainError> {
    let pending = pending_spool(dir)?;
    health.schema = DRAIN_HEALTH_SCHEMA;
    health.available = true;
    health.generated_at_unix_secs = epoch_seconds();
    health.pending_segments = pending.segments;
    health.pending_bytes = pending.bytes;
    health.oldest_pending_age_secs = pending.oldest_age_secs;
    health.last_error = None;
    publish_health(dir, health)
}

fn parse_pause_seconds(raw: &str) -> Result<u64, io::Error> {
    raw.parse::<u64>().map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("VITALS_DRAIN_TEST_PAUSE_BEFORE_COMMIT={raw:?} is not seconds"),
        )
    })
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
    use postgres::NoTls;
    use vitals_core::{NodeId, Sample, Spool};

    const TEST_PG_DEFAULT: &str = "host=/tmp port=5432 dbname=postgres";

    fn test_client() -> Option<Client> {
        let dsn = std::env::var("VITALS_TEST_PG").unwrap_or_else(|_| TEST_PG_DEFAULT.to_owned());
        match Client::connect(&dsn, NoTls) {
            Ok(client) => Some(client),
            Err(error) => {
                eprintln!(
                    "SKIPPED: PostgreSQL integration coverage — {error}; set VITALS_TEST_PG to a test database"
                );
                None
            }
        }
    }

    fn install_test_tables(client: &mut Client) {
        client
            .batch_execute(
                "CREATE TEMP TABLE samples (
                    source INTEGER NOT NULL,
                    type INTEGER NOT NULL,
                    session_id BIGINT NOT NULL,
                    seq BIGINT NOT NULL,
                    timestamp TIMESTAMPTZ NOT NULL,
                    value DOUBLE PRECISION NOT NULL,
                    extra BYTEA,
                    node_id BYTEA NOT NULL,
                    batch_id BIGINT,
                    PRIMARY KEY (source, type, session_id, seq)
                );
                CREATE TEMP TABLE spool_status (
                    segment TEXT PRIMARY KEY,
                    offset_bytes BIGINT NOT NULL,
                    drained BOOLEAN NOT NULL,
                    fingerprint BYTEA NOT NULL,
                    last_advanced TIMESTAMPTZ NOT NULL DEFAULT now()
                );",
            )
            .expect("install temporary drain tables");
    }

    fn dsn() -> String {
        std::env::var("VITALS_TEST_PG").unwrap_or_else(|_| TEST_PG_DEFAULT.to_owned())
    }

    fn sample() -> Sample {
        Sample {
            source: 1,
            r#type: 1,
            session_id: 1,
            seq: 1,
            timestamp: UNIX_EPOCH,
            value: 1.0,
            extra: None,
            batch_id: None,
        }
    }

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

    #[test]
    fn pending_inspection_includes_the_locked_active_segment() {
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
        let pending = pending_spool(d.path()).expect("inspect active segment");
        assert_eq!(pending.segments, 1);
        assert!(pending.bytes > 0);
        assert!(pending.oldest_age_secs.is_some());
    }

    #[test]
    fn empty_and_fully_drained_segments_are_not_pending() {
        let d = tempfile::tempdir().expect("tempdir");
        let path = published_segment(d.path());
        let length = fs::metadata(&path).expect("stat").len();
        fs::write(path.with_extension("offset"), format!("{length}\n")).expect("offset");
        let pending = pending_spool(d.path()).expect("inspect");
        assert_eq!(
            pending,
            PendingSpool {
                segments: 0,
                bytes: 0,
                oldest_age_secs: None,
            }
        );
    }

    #[test]
    fn health_snapshot_is_atomic_json() {
        let d = tempfile::tempdir().expect("tempdir");
        let health = DrainHealth {
            available: true,
            generated_at_unix_secs: 123,
            pending_segments: 2,
            pending_bytes: 456,
            ..DrainHealth::default()
        };
        publish_health(d.path(), &health).expect("publish");
        let bytes = fs::read(d.path().join(DRAIN_HEALTH_FILENAME)).expect("read snapshot");
        let decoded: DrainHealth = serde_json::from_slice(&bytes).expect("decode snapshot");
        assert_eq!(decoded, health);
        assert!(
            fs::read_dir(d.path())
                .expect("list")
                .filter_map(Result::ok)
                .all(|entry| !entry.file_name().to_string_lossy().ends_with(".tmp"))
        );
    }

    #[test]
    fn refresh_health_clears_old_error_and_records_pending_data() {
        let d = tempfile::tempdir().expect("tempdir");
        let mut spool = Spool::open(d.path()).expect("spool");
        spool
            .append(&[sample()], &NodeId::from_bytes([1; 16]), true)
            .expect("append");
        let mut health = DrainHealth {
            last_error: Some("old error".to_owned()),
            ..DrainHealth::default()
        };
        refresh_health(d.path(), &mut health).expect("refresh");
        assert!(health.available);
        assert_eq!(health.last_error, None);
        assert_eq!(health.pending_segments, 1);
        assert!(health.pending_bytes > 0);
    }

    #[test]
    fn live_schema_is_fingerprinted_and_lock_is_exclusive() {
        let Some(mut first) = test_client() else {
            return;
        };
        install_test_tables(&mut first);
        assert_eq!(
            live_fingerprint(&mut first).expect("fingerprint"),
            expected_fingerprint()
        );
        let first_locked: bool = first
            .query_one("SELECT pg_try_advisory_lock($1)", &[&ADVISORY_LOCK])
            .expect("acquire test advisory lock")
            .get(0);
        assert!(first_locked);

        assert!(matches!(connect_locked(&dsn()), Err(DrainError::LockHeld)));
        let released: bool = first
            .query_one("SELECT pg_advisory_unlock($1)", &[&ADVISORY_LOCK])
            .expect("release test advisory lock")
            .get(0);
        assert!(released);
        drop(first);

        let mut third = connect_locked(&dsn()).expect("lock is released with the client");
        install_test_tables(&mut third);
        third
            .batch_execute("ALTER TABLE samples ALTER COLUMN value TYPE REAL")
            .expect("change temporary schema");
        let error = live_fingerprint(&mut third).expect_err("wrong OID must be named");
        assert!(error.to_string().contains("value"));
    }

    #[test]
    fn schema_and_error_variants_name_the_blocker() {
        assert!(checked_type_oid(-1).is_err());
        assert!(connect_locked("host=/no/such/socket dbname=postgres connect_timeout=1").is_err());
        assert_eq!(parse_pause_seconds("0").expect("zero pause"), 0);
        assert!(parse_pause_seconds("not-seconds").is_err());

        let Some(mut client) = test_client() else {
            return;
        };
        install_test_tables(&mut client);
        client
            .batch_execute("ALTER TABLE samples ADD COLUMN unexpected INTEGER")
            .expect("add unexpected column");
        let extra = live_fingerprint(&mut client).expect_err("extra column must be rejected");
        assert!(extra.to_string().contains("unexpected"));

        client
            .batch_execute("DROP TABLE samples")
            .expect("drop temp table");
        client
            .batch_execute("CREATE TEMP TABLE samples (source INTEGER NOT NULL)")
            .expect("create short temp table");
        let short = live_fingerprint(&mut client).expect_err("short table must be rejected");
        assert!(short.to_string().contains("field 2"));

        client
            .batch_execute("DROP TABLE samples")
            .expect("drop temp table");
        let database =
            live_fingerprint(&mut client).expect_err("missing table is a database error");
        assert!(database.to_string().contains("PostgreSQL blocked drain"));

        let rejected = DrainError::Rejected {
            rows: 2,
            source: Box::new(DrainError::SchemaMismatch("constraint".to_owned())),
        };
        assert_eq!(rejected.rejected_rows(), 2);
        assert!(std::error::Error::source(&rejected).is_some());
        assert_eq!(DrainError::LockHeld.rejected_rows(), 0);

        let variants = [
            DrainError::Io(io::Error::other("disk")),
            DrainError::LockHeld,
            DrainError::FingerprintMismatch {
                expected: 1,
                actual: 2,
                context: "test segment".to_owned(),
            },
            DrainError::SchemaMismatch("test column".to_owned()),
            DrainError::InvalidOffset {
                segment: "test.spool".to_owned(),
                offset: -1,
            },
        ];
        for error in variants {
            let text = error.to_string();
            assert!(!text.is_empty(), "every drain error must be actionable");
        }
    }

    #[test]
    fn open_segment_rejects_short_framing_and_invalid_names() {
        let d = tempfile::tempdir().expect("tempdir");
        let short = d.path().join("short.spool");
        fs::write(&short, {
            let mut bytes = Vec::from(SEGMENT_MAGIC);
            bytes.extend_from_slice(&SEGMENT_FORMAT_VERSION.to_be_bytes());
            bytes.extend_from_slice(&expected_fingerprint().value().to_be_bytes());
            bytes
        })
        .expect("short segment");
        let error = open_segment(&short).expect_err("header without COPY bytes");
        assert!(error.to_string().contains("shorter than framing"));
    }

    #[test]
    fn open_segment_rejects_a_locked_offset_sidecar() {
        let d = tempfile::tempdir().expect("tempdir");
        let path = published_segment(d.path());
        let alias = d.path().join(
            path.file_stem()
                .expect("segment stem")
                .to_string_lossy()
                .to_string()
                + ".data",
        );
        fs::copy(&path, &alias).expect("copy segment alias");
        let held = open_segment(&path).expect("hold original offset");
        let error = open_segment(&alias).expect_err("offset sidecar must be exclusive");
        assert!(error.to_string().contains("offset lock held"));
        drop(held);
    }

    #[test]
    fn segment_drain_commits_checkpoint_and_replay_counts_duplicate() {
        let Some(mut client) = test_client() else {
            return;
        };
        install_test_tables(&mut client);
        let d = tempfile::tempdir().expect("tempdir");
        let path = published_segment(d.path());
        let mut segment = open_segment(&path).expect("open published segment");
        let outcome = segment
            .drain(&mut client, expected_fingerprint())
            .expect("drain");
        assert_eq!(outcome.rows_read, 1);
        assert_eq!(outcome.rows_inserted, 1);
        assert_eq!(outcome.rows_duplicate, 0);
        assert_eq!(outcome.offset, segment.length);
        assert!(d.path().join(".drain-status").exists());
        drop(segment);

        // Mimic a restored database without spool_status: replay is safe and
        // the durable identity reports the row as a duplicate.
        client
            .execute("DELETE FROM spool_status", &[])
            .expect("remove checkpoint");
        fs::write(
            path.with_extension("offset"),
            format!("{FIRST_ROW_OFFSET}\n"),
        )
        .expect("rewind local checkpoint");
        let mut replay = open_segment(&path).expect("reopen for replay");
        let replayed = replay
            .drain(&mut client, expected_fingerprint())
            .expect("replay");
        assert_eq!(replayed.rows_read, 1);
        assert_eq!(replayed.rows_inserted, 0);
        assert_eq!(replayed.rows_duplicate, 1);
        assert!(
            replay
                .remove_if_drained()
                .expect("remove committed segment")
        );
        assert!(!path.exists());
        assert!(!path.with_extension("offset").exists());
    }

    #[test]
    fn segment_drain_rejects_fingerprint_and_database_offsets() {
        let Some(mut client) = test_client() else {
            return;
        };
        install_test_tables(&mut client);
        let d = tempfile::tempdir().expect("tempdir");
        let path = published_segment(d.path());
        let mut segment = open_segment(&path).expect("open");
        let wrong = SchemaFingerprint::from_value(expected_fingerprint().value() ^ 1);
        let error = segment
            .drain(&mut client, wrong)
            .expect_err("fingerprint guard");
        assert!(matches!(error, DrainError::FingerprintMismatch { .. }));

        client
            .execute(
                "INSERT INTO spool_status(segment, offset_bytes, drained, fingerprint) VALUES($1, -1, TRUE, $2)",
                &[&segment.name, &expected_fingerprint().value().to_be_bytes().to_vec()],
            )
            .expect("insert invalid status");
        let error = segment
            .drain(&mut client, expected_fingerprint())
            .expect_err("negative database offset");
        assert!(matches!(
            error,
            DrainError::InvalidOffset { offset: -1, .. }
        ));

        client
            .execute(
                &[&segment.name],
            )
            .expect("update invalid status");
        let error = segment
            .drain(&mut client, expected_fingerprint())
            .expect_err("offset beyond segment must be rejected");
        assert!(matches!(error, DrainError::InvalidOffset { .. }));
    }

    #[test]
    fn segment_at_database_eof_persists_local_progress_without_copy() {
        let Some(mut client) = test_client() else {
            return;
        };
        install_test_tables(&mut client);
        let d = tempfile::tempdir().expect("tempdir");
        let path = published_segment(d.path());
        let length = fs::metadata(&path).expect("stat").len();
        client
            .execute(
                "INSERT INTO spool_status(segment, offset_bytes, drained, fingerprint) VALUES($1, $2, TRUE, $3)",
                &[
                    &path.file_name().expect("name").to_string_lossy().to_string(),
                    &i64::try_from(length).expect("length fits"),
                    &expected_fingerprint().value().to_be_bytes().to_vec(),
                ],
            )
            .expect("insert EOF status");
        let mut segment = open_segment(&path).expect("open");
        let outcome = segment
            .drain(&mut client, expected_fingerprint())
            .expect("EOF drain");
        assert_eq!(outcome.rows_read, 0);
        assert_eq!(outcome.offset, length);
    }

    #[test]
    fn transaction_rejection_reports_the_rows_that_were_read() {
        let Some(mut client) = test_client() else {
            return;
        };
        install_test_tables(&mut client);
        client
            .batch_execute("ALTER TABLE samples ADD CONSTRAINT reject_values CHECK (value < 0)")
            .expect("add rejecting constraint");
        let d = tempfile::tempdir().expect("tempdir");
        let path = published_segment(d.path());
        let mut segment = open_segment(&path).expect("open");
        let error = segment
            .drain(&mut client, expected_fingerprint())
            .expect_err("constraint must reject the row");
        assert_eq!(error.rejected_rows(), 1);
        assert!(error.to_string().contains("rejected 1 row"));
        assert_eq!(segment.offset, FIRST_ROW_OFFSET);
    }

    #[test]
    fn copy_and_status_failures_do_not_advance_checkpoint() {
        let Some(mut client) = test_client() else {
            return;
        };
        install_test_tables(&mut client);

        let truncated_dir = tempfile::tempdir().expect("truncated tempdir");
        let truncated_path = published_segment(truncated_dir.path());
        let truncated_length = fs::metadata(&truncated_path).expect("stat").len();
        OpenOptions::new()
            .write(true)
            .open(&truncated_path)
            .expect("open truncated segment")
            .set_len(truncated_length - 10)
            .expect("truncate copy row");
        let mut truncated = open_segment(&truncated_path).expect("open truncated segment");
        let copy_error = truncated
            .drain(&mut client, expected_fingerprint())
            .expect_err("truncated COPY must fail");
        assert_eq!(copy_error.rejected_rows(), 0);
        assert_eq!(truncated.offset, FIRST_ROW_OFFSET);

        client
            .batch_execute("DROP TABLE spool_status")
            .expect("drop temporary status table");
        let missing_status_dir = tempfile::tempdir().expect("missing-status tempdir");
        let missing_status_path = published_segment(missing_status_dir.path());
        let mut missing_status = open_segment(&missing_status_path).expect("open segment");
        let status_error = missing_status
            .drain(&mut client, expected_fingerprint())
            .expect_err("missing status table must fail closed");
        assert_eq!(status_error.rejected_rows(), 0);
        assert_eq!(missing_status.offset, FIRST_ROW_OFFSET);
    }

    #[test]
    fn progress_and_removal_failures_are_returned() {
        let d = tempfile::tempdir().expect("tempdir");
        let path = published_segment(d.path());
        let temporary = d
            .path()
            .join(format!(".drain-status.{}.tmp", std::process::id()));
        fs::write(&temporary, b"stale").expect("stale progress file");
        let segment = open_segment(&path).expect("open");
        segment.persist_progress().expect("replace progress file");
        assert!(d.path().join(".drain-status").exists());
        drop(segment);

        let error_dir = tempfile::tempdir().expect("progress-error tempdir");
        let error_path = published_segment(error_dir.path());
        let error_temporary = error_dir
            .path()
            .join(format!(".drain-status.{}.tmp", std::process::id()));
        fs::create_dir(&error_temporary).expect("progress temporary directory");
        let error_segment = open_segment(&error_path).expect("open error segment");
        assert!(matches!(
            error_segment.persist_progress(),
            Err(DrainError::Io(_))
        ));
        drop(error_segment);

        let removal_dir = tempfile::tempdir().expect("removal tempdir");
        let removal_path = published_segment(removal_dir.path());
        let length = fs::metadata(&removal_path).expect("stat").len();
        fs::write(removal_path.with_extension("offset"), format!("{length}\n"))
            .expect("drained offset");
        let removal = open_segment(&removal_path).expect("open drained segment");
        fs::remove_file(&removal_path).expect("remove segment before retry");
        assert!(matches!(
            removal.remove_if_drained(),
            Err(DrainError::Io(_))
        ));
    }

    #[test]
    fn health_publication_failures_are_not_reported_as_success() {
        let d = tempfile::tempdir().expect("tempdir");
        let missing = d.path().join("missing");
        let health = DrainHealth::default();
        assert!(matches!(
            publish_health(&missing, &health),
            Err(DrainError::Io(_))
        ));
        let temporary = d.path().join(format!(
            ".{DRAIN_HEALTH_FILENAME}.{}.tmp",
            std::process::id()
        ));
        fs::create_dir(&temporary).expect("health temporary directory");
        assert!(matches!(
            publish_health(d.path(), &health),
            Err(DrainError::Io(_))
        ));
        fs::remove_dir(temporary).expect("remove health temporary directory");
        let mut health = DrainHealth::default();
        assert!(matches!(
            refresh_health(&missing, &mut health),
            Err(DrainError::Io(_))
        ));
    }

    #[test]
    fn pending_inspection_rejects_sidecar_read_errors_and_uses_oldest_age() {
        let d = tempfile::tempdir().expect("tempdir");
        let first = published_segment(d.path());
        let second = d.path().join("second.spool");
        fs::copy(&first, &second).expect("copy second segment");
        fs::create_dir(first.with_extension("offset")).expect("offset directory");
        let error = pending_spool(d.path()).expect_err("offset directory is not readable text");
        assert!(error.to_string().contains("I/O") || error.to_string().contains("Is a directory"));
        fs::remove_dir(first.with_extension("offset")).expect("remove offset directory");
        fs::write(
            first.with_extension("offset"),
            format!("{FIRST_ROW_OFFSET}\n"),
        )
        .expect("first offset");
        fs::write(
            second.with_extension("offset"),
            format!("{FIRST_ROW_OFFSET}\n"),
        )
        .expect("second offset");
        let pending = pending_spool(d.path()).expect("two pending segments");
        assert_eq!(pending.segments, 2);
        assert!(pending.oldest_age_secs.is_some());
    }

    #[test]
    fn pending_inspection_names_missing_directory_and_bad_clock_data() {
        let d = tempfile::tempdir().expect("tempdir");
        let missing = d.path().join("missing");
        assert!(matches!(discover(&missing), Err(DrainError::Io(_))));
        assert!(matches!(pending_spool(&missing), Err(DrainError::Io(_))));

        let empty = d.path().join("empty.spool");
        File::create(&empty).expect("empty segment");
        assert_eq!(
            pending_spool(d.path()).expect("inspect empty"),
            PendingSpool {
                segments: 0,
                bytes: 0,
                oldest_age_secs: None,
            }
        );

        let path = d.path().join("future.spool");
        fs::write(
            &path,
            vec![0_u8; usize::try_from(FIRST_ROW_OFFSET).expect("offset fits") + 1],
        )
        .expect("segment");
        fs::write(
            path.with_extension("offset"),
            format!("{FIRST_ROW_OFFSET}\n"),
        )
        .expect("offset");
        let future = std::fs::FileTimes::new()
            .set_modified(SystemTime::now() + std::time::Duration::from_mins(1));
        File::open(&path)
            .expect("open segment for mtime")
            .set_times(future)
            .expect("future mtime");
        let error = pending_spool(d.path()).expect_err("future evidence");
        assert!(error.to_string().contains("in the future"));
    }
}
