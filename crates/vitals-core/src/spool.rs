//! Append-only spool of rows already encoded in `PostgreSQL` binary `COPY`
//! format.
//!
//! Collectors append here; `vitals-spool-drain` streams the bytes into
//! `COPY … FROM STDIN (FORMAT binary)`. There is no embedded database and no
//! WAL tail (amendment 01).
//!
//! # Durability is asymmetric on purpose
//!
//! The default path batches and fsyncs periodically, bounding loss at
//! [`SYNC_INTERVAL`]. A night of RR intervals is roughly 250 MB across ~100k
//! appends; fsyncing each would destroy the ingest path to protect data that
//! the sensor's own buffer can replay.
//!
//! `append(rows, sync = true)` opts a caller into durability-before-return
//! (amendment 06 §2.1). Only `bp-collector` sets it: the kernel confirms the
//! ATT indication before userspace sees the frame and the cuff deletes its copy
//! on that confirmation, so a BP reading is reconstructible from nothing.
//!
//! **A collector must never block on disk unless it asked to.**
//!
//! # The fingerprint
//!
//! Binary `COPY` is positional and freezes column count and order. A migration
//! that changes either while bytes are buffered orphans them unrecoverably —
//! the drainer cannot tell a stale row from a current one, because the bytes
//! carry no schema. Each segment therefore opens with a fingerprint of the
//! column names, order and types, and the drainer refuses a mismatch rather
//! than inserting garbage that `Postgres` would accept.

use crate::copybin;
use fs4::fs_std::FileExt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Rotate once a segment reaches this size.
pub const MAX_SEGMENT_BYTES: u64 = 64 * 1024 * 1024;

/// Rotate at least this often, so a quiet source still produces drainable
/// segments and the drain lag stays observable.
pub const MAX_SEGMENT_AGE: Duration = Duration::from_hours(1);

/// Upper bound on unsynced data in the default (non-`sync`) path.
pub const SYNC_INTERVAL: Duration = Duration::from_millis(250);

/// Disk usage at which opening a new segment is refused.
pub const REFUSE_SEGMENT_PERCENT: u64 = 95;
/// Disk usage at which rotation emits an operator warning.
pub const WARN_SEGMENT_PERCENT: u64 = 80;

/// Magic at the head of every segment, before the fingerprint.
pub const SEGMENT_MAGIC: &[u8; 8] = b"VITSPOOL";

/// Bumped when the segment framing itself changes — distinct from the schema
/// fingerprint, which tracks the *table*.
pub const SEGMENT_FORMAT_VERSION: u16 = 1;

/// Bytes before the first COPY row: framing plus `PostgreSQL`'s header.
pub const FIRST_ROW_OFFSET: u64 = SEGMENT_HEADER_BYTES as u64 + 19;

/// Bytes in the spool-specific magic/version/fingerprint header.
pub const SEGMENT_HEADER_BYTES: usize = 18;

/// Canonical `samples` field names and `PostgreSQL` type OIDs.
pub const SAMPLES_LAYOUT: [(&str, u32); 9] = [
    ("source", 23),
    ("type", 23),
    ("session_id", 20),
    ("seq", 20),
    ("timestamp", 1184),
    ("value", 701),
    ("extra", 17),
    ("node_id", 17),
    ("batch_id", 20),
];

/// Identifies the exact table layout a segment's rows were encoded against.
///
/// Not a hash of the DDL text: whitespace and comments would change it without
/// changing the layout, and the layout is what binary `COPY` depends on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchemaFingerprint(u64);

impl SchemaFingerprint {
    /// Computes a fingerprint from `(column_name, type_oid)` pairs **in
    /// declaration order**. Order is part of the identity because binary `COPY`
    /// is positional: two tables with the same columns in a different order are
    /// not interchangeable, and swapping `source` and `type` — both `INTEGER` —
    /// would otherwise be undetectable.
    #[must_use]
    pub fn of(columns: &[(&str, u32)]) -> Self {
        // FNV-1a: stable across runs and platforms, which a DefaultHasher is
        // explicitly not. A fingerprint that changes between processes would
        // reject every segment written by a different build.
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mut mix = |bytes: &[u8]| {
            for b in bytes {
                h ^= u64::from(*b);
                h = h.wrapping_mul(0x0000_0100_0000_01B3);
            }
        };
        for (name, oid) in columns {
            mix(name.as_bytes());
            mix(&oid.to_be_bytes());
            mix(b"\x1f"); // separator: ("ab","c") must not equal ("a","bc")
        }
        Self(h)
    }

    /// Reconstructs a fingerprint read from a validated segment header.
    #[must_use]
    pub const fn from_value(value: u64) -> Self {
        Self(value)
    }

    /// The raw value, for the segment header and for reporting a mismatch.
    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }
}

/// The `samples` layout, in `db/deploy/samples.sql` declaration order.
/// OIDs: int4 = 23, int8 = 20, timestamptz = 1184, float8 = 701, bytea = 17.
#[must_use]
pub fn samples_fingerprint() -> SchemaFingerprint {
    SchemaFingerprint::of(&SAMPLES_LAYOUT)
}

/// An append-only spool directory holding one open segment.
#[derive(Debug)]
pub struct Spool {
    dir: PathBuf,
    // The writer lock belongs to the directory, not the current segment. A
    // segment changes on rotation (and its name is time-derived), but the
    // single-writer invariant must not have either of those scopes.
    writer_lock: File,
    file: Option<File>,
    path: PathBuf,
    bytes: u64,
    opened: Instant,
    last_sync: Instant,
    dirty: bool,
    fingerprint: SchemaFingerprint,
}

impl Spool {
    /// Opens `dir`, creating it and a fresh segment if needed.
    ///
    /// Takes an exclusive advisory lock on the spool directory and on the new
    /// segment. A second writer fails even if it selects a differently named
    /// segment; otherwise both writers could publish independent, concurrent
    /// streams and violate the single-writer ingest invariant.
    ///
    /// # Errors
    /// I/O failure, or the lock already being held by another process.
    pub fn open(dir: &Path) -> io::Result<Self> {
        Self::open_with(dir, samples_fingerprint())
    }

    /// As [`Spool::open`], with an explicit fingerprint. For tests, and for a
    /// future table.
    ///
    /// # Errors
    /// I/O failure, or the lock already being held.
    pub fn open_with(dir: &Path, fingerprint: SchemaFingerprint) -> io::Result<Self> {
        Self::open_with_segment_name(dir, fingerprint, &Self::segment_name()?)
    }

    fn open_with_segment_name(
        dir: &Path,
        fingerprint: SchemaFingerprint,
        segment_name: &str,
    ) -> io::Result<Self> {
        fs::create_dir_all(dir)?;
        let lock_path = dir.join(".writer.lock");
        let writer_lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&lock_path)?;
        let locked = writer_lock.try_lock_exclusive().map_err(|e| {
            io::Error::other(format!(
                "{}: locking the spool directory failed: {e}",
                lock_path.display()
            ))
        })?;
        if !locked {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                format!(
                    "{}: another writer holds the spool lock — two writers would \
                     interleave partial rows with no way to resynchronise",
                    lock_path.display()
                ),
            ));
        }

        let created = Self::create_segment(dir, segment_name, fingerprint);
        let (file, path, bytes) = match created {
            Ok((file, path, bytes)) => (Some(file), path, bytes),
            Err(e) if e.kind() == io::ErrorKind::StorageFull => {
                // Keep the writer lock and start the socket service. Appends
                // remain available to in-memory subscribers but are explicitly
                // non-durable until space falls below the refusal watermark.
                (None, dir.join(".memory-only"), 0)
            }
            Err(e) => return Err(e),
        };
        let now = Instant::now();
        Ok(Self {
            dir: dir.to_path_buf(),
            writer_lock,
            file,
            path,
            bytes,
            opened: now,
            last_sync: now,
            dirty: false,
            fingerprint,
        })
    }

    fn create_segment(
        dir: &Path,
        segment_name: &str,
        fingerprint: SchemaFingerprint,
    ) -> io::Result<(File, PathBuf, u64)> {
        Self::check_segment_watermark(dir)?;
        // Initialise under an excluded suffix. A drainer that discovers only
        // `*.spool` can therefore never observe an empty or partial header.
        let mut staging_collision = 0_u64;
        let (mut file, staging_path) = loop {
            let staging_path = dir.join(format!(".{segment_name}.init-{staging_collision}.tmp"));
            match OpenOptions::new()
                .create_new(true)
                .append(true)
                .read(true)
                .open(&staging_path)
            {
                Ok(file) => break (file, staging_path),
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                    staging_collision = Self::next_collision(staging_collision, dir)?;
                }
                Err(e) => return Err(e),
            }
        };

        let initialized = (|| {
            let locked = file.try_lock_exclusive().map_err(|e| {
                io::Error::other(format!(
                    "{}: locking the staged segment failed: {e}",
                    staging_path.display()
                ))
            })?;
            if !locked {
                return Err(io::Error::new(
                    io::ErrorKind::WouldBlock,
                    format!(
                        "{}: the staged spool segment could not be locked",
                        staging_path.display()
                    ),
                ));
            }
            let header = Self::segment_header(fingerprint);
            file.write_all(&header)?;
            file.write_all(&copybin::copy_header())?;
            file.sync_all()?;
            Ok(header.len() as u64 + 19)
        })();
        let bytes = match initialized {
            Ok(bytes) => bytes,
            Err(e) => return Err(Self::remove_staging_after_error(&staging_path, e)),
        };

        // NOREPLACE is an atomic publication and collision check: unlike
        // ordinary rename, it cannot overwrite an undrained segment that
        // appeared at the target. Renaming also removes the staging entry in
        // the same metadata operation.
        let mut collision = 0_u64;
        let path = loop {
            let path = dir.join(Self::collision_name(segment_name, collision));
            match rustix::fs::renameat_with(
                rustix::fs::CWD,
                &staging_path,
                rustix::fs::CWD,
                &path,
                rustix::fs::RenameFlags::NOREPLACE,
            ) {
                Ok(()) => break path,
                Err(e) if e == rustix::io::Errno::EXIST => {
                    collision = match Self::next_collision(collision, dir) {
                        Ok(next) => next,
                        Err(exhausted) => {
                            return Err(Self::remove_staging_after_error(&staging_path, exhausted));
                        }
                    };
                }
                Err(e) => {
                    return Err(Self::remove_staging_after_error(
                        &staging_path,
                        io::Error::from_raw_os_error(e.raw_os_error()),
                    ));
                }
            }
        };

        // Persist both the final directory entry and removal of the staging
        // entry before reporting a usable segment. The inode was synced first.
        File::open(dir)
            .and_then(|directory| directory.sync_all())
            .map_err(|e| {
                io::Error::new(
                    e.kind(),
                    format!(
                        "{}: syncing published segment directory failed: {e}",
                        path.display()
                    ),
                )
            })?;
        Ok((file, path, bytes))
    }

    fn collision_name(segment_name: &str, collision: u64) -> String {
        // Every name, including the first, carries the same-width sequence so
        // lexical discovery order is creation order after collisions exceed 9.
        match segment_name.strip_suffix(".spool") {
            Some(stem) => format!("{stem}-{collision:020}.spool"),
            None => format!("{segment_name}-{collision:020}"),
        }
    }

    fn next_collision(collision: u64, dir: &Path) -> io::Result<u64> {
        collision.checked_add(1).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("{}: exhausted unique segment names", dir.display()),
            )
        })
    }

    fn remove_staging_after_error(staging_path: &Path, cause: io::Error) -> io::Error {
        match fs::remove_file(staging_path) {
            Ok(()) => cause,
            Err(cleanup) => io::Error::new(
                cause.kind(),
                format!(
                    "segment initialization failed: {cause}; cleanup of {} also failed: {cleanup}",
                    staging_path.display()
                ),
            ),
        }
    }

    fn segment_header(fp: SchemaFingerprint) -> Vec<u8> {
        let mut h = Vec::with_capacity(18);
        h.extend_from_slice(SEGMENT_MAGIC);
        h.extend_from_slice(&SEGMENT_FORMAT_VERSION.to_be_bytes());
        h.extend_from_slice(&fp.value().to_be_bytes());
        h
    }

    fn segment_name() -> io::Result<String> {
        // `create_new` below is the uniqueness guarantee; nanoseconds merely
        // keep collisions (including after a restart or RTC rollback) rare.
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        // Random provenance prevents an old spool_status row from ever being
        // mistaken for this segment after RTC rollback or restored files.
        let mut nonce = [0_u8; 8];
        getrandom::fill(&mut nonce)
            .map_err(|e| io::Error::other(format!("random segment identity failed: {e}")))?;
        Ok(format!(
            "segment-{nanos:020}-{:016x}.spool",
            u64::from_be_bytes(nonce)
        ))
    }

    /// This segment's path.
    #[must_use]
    pub fn segment_path(&self) -> &Path {
        &self.path
    }

    /// Bytes written to the current segment, header included.
    #[must_use]
    pub const fn segment_bytes(&self) -> u64 {
        self.bytes
    }

    /// Whether no durable segment is currently available.
    #[must_use]
    pub const fn is_memory_only(&self) -> bool {
        self.file.is_none()
    }

    /// The fingerprint this spool encodes against.
    #[must_use]
    pub const fn fingerprint(&self) -> SchemaFingerprint {
        self.fingerprint
    }

    /// Appends rows.
    ///
    /// With `sync = false` the write is buffered and fsynced at most
    /// [`SYNC_INTERVAL`] later. With `sync = true` the segment is fsynced before
    /// this returns.
    ///
    /// Encoding happens fully before any byte is written, so a row that fails to
    /// encode cannot leave a partial record in the file. A truncated row has no
    /// framing a reader could skip.
    ///
    /// # Errors
    /// I/O failure, or [`copybin::EncodeError`] surfaced as `InvalidData` —
    /// which names the offending field rather than silently dropping the row.
    pub fn append(
        &mut self,
        rows: &[crate::Sample],
        node_id: &crate::NodeId,
        sync: bool,
    ) -> io::Result<()> {
        if rows.is_empty() {
            if sync {
                self.sync()?;
            }
            return Ok(());
        }

        if self.file.is_none() {
            match Self::create_segment(&self.dir, &Self::segment_name()?, self.fingerprint) {
                Ok((file, path, bytes)) => {
                    self.file = Some(file);
                    self.path = path;
                    self.bytes = bytes;
                    self.opened = Instant::now();
                }
                Err(e) if e.kind() == io::ErrorKind::StorageFull => return Err(e),
                Err(e) => {
                    return Err(io::Error::new(
                        io::ErrorKind::StorageFull,
                        format!(
                            "spool append is memory-only: segment creation/capacity check failed: {e}"
                        ),
                    ));
                }
            }
        } else if self.rotation_due() {
            match self.disk_used_percent() {
                Ok(used) if used < REFUSE_SEGMENT_PERCENT => {}
                Ok(used) => {
                    return Err(io::Error::new(
                        io::ErrorKind::StorageFull,
                        format!("spool append is memory-only: filesystem is {used}% used"),
                    ));
                }
                Err(e) => {
                    return Err(io::Error::new(
                        io::ErrorKind::StorageFull,
                        format!(
                            "spool append is memory-only: filesystem capacity check failed: {e}"
                        ),
                    ));
                }
            }
        }

        let mut buf = Vec::with_capacity(rows.len() * 96);
        for s in rows {
            copybin::encode_row(&mut buf, s, node_id)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
        }

        let file = self.file.as_mut().ok_or_else(|| {
            io::Error::other("spool file unavailable after successful segment creation")
        })?;
        file.write_all(&buf)?;
        self.bytes += buf.len() as u64;
        self.dirty = true;

        if sync || self.last_sync.elapsed() >= SYNC_INTERVAL {
            self.sync()?;
        }
        Ok(())
    }

    /// Forces an fsync if there is anything unsynced.
    ///
    /// # Errors
    /// I/O failure.
    pub fn sync(&mut self) -> io::Result<()> {
        if self.dirty {
            let file = self
                .file
                .as_ref()
                .ok_or_else(|| io::Error::other("dirty spool has no backing segment"))?;
            file.sync_all()?;
            self.dirty = false;
        }
        self.last_sync = Instant::now();
        Ok(())
    }

    fn check_segment_watermark(dir: &Path) -> io::Result<u64> {
        let total = fs4::total_space(dir)?;
        if total == 0 {
            return Err(io::Error::other(format!(
                "{}: filesystem reports zero total bytes",
                dir.display()
            )));
        }
        let available = fs4::available_space(dir)?;
        let used = total.saturating_sub(available).saturating_mul(100) / total;
        if used >= REFUSE_SEGMENT_PERCENT {
            return Err(io::Error::new(
                io::ErrorKind::StorageFull,
                format!(
                    "spool segment creation refused: filesystem is {used}% used (limit {REFUSE_SEGMENT_PERCENT}%)"
                ),
            ));
        }
        if used >= WARN_SEGMENT_PERCENT {
            eprintln!(
                "vitals spool disk warning: filesystem is {used}% used (warning {WARN_SEGMENT_PERCENT}%)"
            );
        }
        Ok(used)
    }

    /// Filesystem use percentage for this spool.
    ///
    /// # Errors
    /// The filesystem statistics could not be read, or reported zero capacity.
    pub fn disk_used_percent(&self) -> io::Result<u64> {
        let total = fs4::total_space(&self.dir)?;
        if total == 0 {
            return Err(io::Error::other(format!(
                "{}: filesystem reports zero total bytes",
                self.dir.display()
            )));
        }
        let available = fs4::available_space(&self.dir)?;
        Ok(total.saturating_sub(available).saturating_mul(100) / total)
    }

    /// Whether the current segment is due for rotation.
    #[must_use]
    pub fn rotation_due(&self) -> bool {
        self.file.is_some()
            && (self.bytes >= MAX_SEGMENT_BYTES || self.opened.elapsed() >= MAX_SEGMENT_AGE)
    }

    /// Rotates if due. The finished segment is fsynced and its lock released
    /// before the new one opens, so a drainer can take it immediately.
    ///
    /// The `COPY` trailer is **not** written on rotation: a segment is a
    /// fragment of a stream, and the drainer appends the trailer when it feeds
    /// `Postgres`. Writing it here would make each segment claim to be a complete
    /// file and hide truncation.
    ///
    /// # Errors
    /// I/O failure, or the new segment's lock being unavailable.
    pub fn rotate_if_due(&mut self) -> io::Result<bool> {
        if !self.rotation_due() {
            return Ok(false);
        }
        self.seal_current()?;
        Ok(true)
    }

    /// Seals the current segment immediately and opens a fresh one.
    ///
    /// Durability-required, low-rate sources use this after their append so the
    /// drainer can lock and commit the finished segment without waiting for the
    /// normal age/size rotation threshold.
    ///
    /// # Errors
    /// I/O failure, or the replacement segment's lock being unavailable.
    pub fn seal_current(&mut self) -> io::Result<()> {
        self.rotate_with_segment_name(&Self::segment_name()?)
    }

    #[cfg(test)]
    fn rotate_if_due_with_segment_name(&mut self, segment_name: &str) -> io::Result<bool> {
        if !self.rotation_due() {
            return Ok(false);
        }
        self.rotate_with_segment_name(segment_name)?;
        Ok(true)
    }

    fn rotate_with_segment_name(&mut self, segment_name: &str) -> io::Result<()> {
        self.sync()?;
        let (file, path, bytes) = Self::create_segment(&self.dir, segment_name, self.fingerprint)?;
        // Keep the old segment locked until its replacement is fully created
        // and durable. The directory lock stays on the same file description:
        // rotation neither releases nor reacquires it. If creation fails, this
        // Spool therefore remains the sole locked writer.
        if let Some(old) = &self.file {
            FileExt::unlock(old)?;
        }
        self.file = Some(file);
        self.path = path;
        self.bytes = bytes;
        let now = Instant::now();
        self.opened = now;
        self.last_sync = now;
        self.dirty = false;
        Ok(())
    }
}

impl Drop for Spool {
    fn drop(&mut self) {
        // Best effort. A failure here cannot be reported, but leaving bytes
        // unsynced on a clean drop would lose data no error ever mentioned.
        let _ = self.sync();
        if let Some(file) = &self.file {
            let _ = FileExt::unlock(file);
        }
        let _ = FileExt::unlock(&self.writer_lock);
    }
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    reason = "a failing assertion in a test should panic loudly"
)]
#[expect(
    clippy::duration_suboptimal_units,
    reason = "epoch seconds; expressing them in hours would obscure the instant"
)]
mod tests {
    use super::*;
    use crate::{NodeId, Sample};
    use std::process::Command;
    use std::time::UNIX_EPOCH;
    use tempfile::tempdir;

    fn node() -> NodeId {
        NodeId::from_bytes([7u8; 16])
    }

    fn sample(seq: i64) -> Sample {
        Sample {
            source: 6,
            r#type: 6,
            session_id: 1,
            seq,
            timestamp: UNIX_EPOCH + Duration::from_secs(1_767_225_600),
            value: 120.0,
            extra: None,
            batch_id: None,
        }
    }

    #[test]
    fn a_new_segment_carries_magic_version_fingerprint_and_copy_header() {
        let d = tempdir().expect("tempdir");
        let s = Spool::open(d.path()).expect("open");
        let path = s.segment_path().to_path_buf();
        drop(s);

        let bytes = fs::read(&path).expect("read");
        assert_eq!(bytes.get(..8), Some(&SEGMENT_MAGIC[..]));
        assert_eq!(
            bytes.get(8..10),
            Some(&SEGMENT_FORMAT_VERSION.to_be_bytes()[..])
        );
        assert_eq!(
            bytes.get(10..18),
            Some(&samples_fingerprint().value().to_be_bytes()[..]),
            "segment must record the schema its rows were encoded against"
        );
        assert_eq!(bytes.get(18..29), Some(&b"PGCOPY\n\xff\r\n\x00"[..]));
    }

    /// Column ORDER is part of the identity. `source` and `type` are both
    /// `INTEGER`, so transposing them changes nothing a type check would catch
    /// and everything about what the rows mean.
    #[test]
    fn fingerprint_detects_reordering_and_retyping() {
        let base = SchemaFingerprint::of(&[("source", 23), ("type", 23)]);
        let swapped = SchemaFingerprint::of(&[("type", 23), ("source", 23)]);
        let retyped = SchemaFingerprint::of(&[("source", 20), ("type", 23)]);
        let renamed = SchemaFingerprint::of(&[("src", 23), ("type", 23)]);
        let added = SchemaFingerprint::of(&[("source", 23), ("type", 23), ("extra", 17)]);
        assert_ne!(base, swapped, "column order must change the fingerprint");
        assert_ne!(base, retyped);
        assert_ne!(base, renamed);
        assert_ne!(base, added);
    }

    /// A separator-free concatenation would make ("ab","c") and ("a","bc")
    /// identical — different tables with the same fingerprint.
    #[test]
    fn fingerprint_is_not_ambiguous_across_name_boundaries() {
        assert_ne!(
            SchemaFingerprint::of(&[("ab", 23), ("c", 23)]),
            SchemaFingerprint::of(&[("a", 23), ("bc", 23)])
        );
    }

    /// Stable across processes and builds, unlike `DefaultHasher`, whose seed is
    /// randomised — that would reject every segment written by another run.
    #[test]
    fn fingerprint_is_deterministic() {
        assert_eq!(samples_fingerprint(), samples_fingerprint());
        assert_eq!(
            samples_fingerprint().value(),
            SchemaFingerprint::of(&[
                ("source", 23),
                ("type", 23),
                ("session_id", 20),
                ("seq", 20),
                ("timestamp", 1184),
                ("value", 701),
                ("extra", 17),
                ("node_id", 17),
                ("batch_id", 20),
            ])
            .value()
        );
    }

    #[test]
    fn appended_rows_land_after_the_headers() {
        let d = tempdir().expect("tempdir");
        let mut s = Spool::open(d.path()).expect("open");
        let before = s.segment_bytes();
        s.append(&[sample(0), sample(1)], &node(), false)
            .expect("append");
        assert!(s.segment_bytes() > before);
        let path = s.segment_path().to_path_buf();
        drop(s);
        assert_eq!(fs::metadata(&path).expect("stat").len(), {
            // 18-byte segment header + 19-byte COPY header + two rows.
            let mut buf = Vec::new();
            copybin::encode_row(&mut buf, &sample(0), &node()).expect("enc");
            copybin::encode_row(&mut buf, &sample(1), &node()).expect("enc");
            18 + 19 + buf.len() as u64
        });
    }

    /// The reason `sync` is a parameter at all.
    #[test]
    fn sync_true_leaves_nothing_dirty() {
        let d = tempdir().expect("tempdir");
        let mut s = Spool::open(d.path()).expect("open");
        s.append(&[sample(0)], &node(), true).expect("append");
        assert!(!s.dirty, "sync: true must fsync before returning");
    }

    /// Durable low-rate sources seal after each append: the drainer cannot
    /// consume the active segment, so an unsealed reading would sit invisible
    /// until the age/size rotation.
    #[test]
    fn seal_current_retires_the_segment_and_opens_a_fresh_one() {
        let d = tempdir().expect("tempdir");
        let mut s = Spool::open(d.path()).expect("open");
        s.append(&[sample(0)], &node(), true).expect("append");
        let sealed = s.segment_path().to_path_buf();
        s.seal_current().expect("seal");
        assert_ne!(s.segment_path(), sealed.as_path());
        assert!(
            sealed.exists(),
            "sealed segment must remain for the drainer"
        );
        assert!(!s.dirty, "seal must leave nothing unsynced");
    }

    #[test]
    fn sync_false_defers() {
        let d = tempdir().expect("tempdir");
        let mut s = Spool::open(d.path()).expect("open");
        s.sync().expect("reset the timer");
        s.append(&[sample(0)], &node(), false).expect("append");
        assert!(s.dirty, "the default path must not block on disk");
    }

    /// A second writer must fail loudly. Interleaved partial rows in one binary
    /// COPY stream cannot be resynchronised by any reader.
    #[test]
    fn a_second_writer_is_refused() {
        let d = tempdir().expect("tempdir");
        let _first = Spool::open(d.path()).expect("first opens");
        let err = Spool::open(d.path()).expect_err("second must fail");
        assert_eq!(err.kind(), io::ErrorKind::WouldBlock);
        assert!(
            err.to_string()
                .contains("another writer holds the spool lock")
        );
    }

    /// The writer lock covers the spool directory, not the time-selected
    /// segment. The old segment lock let a second writer through whenever its
    /// call crossed a wall-clock second boundary and selected another file.
    #[test]
    fn a_second_writer_is_refused_even_when_it_selects_a_different_segment() {
        let d = tempdir().expect("tempdir");
        let _first =
            Spool::open_with_segment_name(d.path(), samples_fingerprint(), "segment-first.spool")
                .expect("first opens");
        let err = Spool::open_with_segment_name(
            d.path(),
            samples_fingerprint(),
            "segment-next-second.spool",
        )
        .expect_err("directory lock must refuse a writer targeting another segment");
        assert_eq!(err.kind(), io::ErrorKind::WouldBlock);
        assert!(!d.path().join("segment-next-second.spool").exists());
    }

    #[test]
    fn an_existing_segment_name_is_never_reopened_for_append() {
        let d = tempdir().expect("tempdir");
        let first_path = {
            let first = Spool::open_with_segment_name(
                d.path(),
                samples_fingerprint(),
                "segment-same-instant.spool",
            )
            .expect("first opens");
            first.segment_path().to_path_buf()
        };
        let second = Spool::open_with_segment_name(
            d.path(),
            samples_fingerprint(),
            "segment-same-instant.spool",
        )
        .expect("reopen after the writer exits");
        assert_ne!(second.segment_path(), first_path);
        assert_eq!(fs::metadata(&first_path).expect("stat first").len(), 37);
        assert_eq!(
            second.segment_path().extension().and_then(|x| x.to_str()),
            Some("spool")
        );
        let discovered = discoverable_segments(d.path());
        assert_eq!(discovered.len(), 2);
        assert!(discovered.contains(&first_path));
        assert!(discovered.contains(&second.segment_path().to_path_buf()));
        assert!(
            fs::read_dir(d.path())
                .expect("list")
                .filter_map(Result::ok)
                .all(|entry| !entry.file_name().to_string_lossy().ends_with(".tmp")),
            "successful publication must leave no staging files"
        );
    }

    fn discoverable_segments(dir: &Path) -> Vec<PathBuf> {
        let mut segments: Vec<_> = fs::read_dir(dir)
            .expect("list")
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|x| x == "spool"))
            .collect();
        segments.sort();
        segments
    }

    #[test]
    fn cross_process_lock_helper() {
        let Some(dir) = std::env::var_os("VITALS_TEST_LOCKED_SPOOL") else {
            return;
        };
        let err = Spool::open(Path::new(&dir)).expect_err("other process must hold lock");
        assert_eq!(err.kind(), io::ErrorKind::WouldBlock);
    }

    #[test]
    fn a_writer_in_another_process_is_refused() {
        let d = tempdir().expect("tempdir");
        let _first = Spool::open(d.path()).expect("first opens");
        let output = Command::new(std::env::current_exe().expect("test executable"))
            .args(["--exact", "spool::tests::cross_process_lock_helper"])
            .env("VITALS_TEST_LOCKED_SPOOL", d.path())
            .output()
            .expect("run lock helper");
        assert!(
            output.status.success(),
            "cross-process helper failed: stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn a_failed_row_writes_nothing() {
        let d = tempdir().expect("tempdir");
        let mut s = Spool::open(d.path()).expect("open");
        let before = s.segment_bytes();
        let bad = Sample {
            timestamp: UNIX_EPOCH + Duration::from_secs(u64::MAX / 2),
            ..sample(0)
        };
        let err = s
            .append(&[sample(0), bad], &node(), false)
            .expect_err("must fail");
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert_eq!(
            s.segment_bytes(),
            before,
            "a batch containing an unencodable row must write no bytes at all"
        );
    }

    #[test]
    fn empty_append_is_a_no_op_but_still_honours_sync() {
        let d = tempdir().expect("tempdir");
        let mut s = Spool::open(d.path()).expect("open");
        let before = s.segment_bytes();
        s.append(&[], &node(), true).expect("append");
        assert_eq!(s.segment_bytes(), before);
    }

    #[test]
    fn rotation_is_not_due_on_a_fresh_segment() {
        let d = tempdir().expect("tempdir");
        let mut s = Spool::open(d.path()).expect("open");
        assert!(!s.rotation_due());
        assert!(!s.rotate_if_due().expect("rotate"));
    }

    #[test]
    fn rotation_is_due_past_the_size_bound() {
        let d = tempdir().expect("tempdir");
        let mut s = Spool::open(d.path()).expect("open");
        s.bytes = MAX_SEGMENT_BYTES;
        assert!(s.rotation_due());
    }

    #[test]
    fn lexical_discovery_order_matches_creation_order_through_double_digits() {
        let d = tempdir().expect("tempdir");
        let mut created = Vec::new();
        let mut active = None;
        for collision in 0..=11 {
            let spool = Spool::open_with_segment_name(
                d.path(),
                samples_fingerprint(),
                "segment-lexical.spool",
            )
            .expect("open sequential writer");
            assert_eq!(
                spool.segment_path().file_name().and_then(|n| n.to_str()),
                Some(format!("segment-lexical-{collision:020}.spool").as_str())
            );
            created.push(spool.segment_path().to_path_buf());
            if collision == 11 {
                active = Some(spool);
            }
        }
        let active = active.expect("last segment remains active");
        let discovered = discoverable_segments(d.path());
        assert_eq!(discovered, created);
        assert_eq!(
            discovered.last(),
            Some(&active.segment_path().to_path_buf())
        );
    }

    #[test]
    fn rotation_sequences_a_colliding_name_and_retains_the_directory_lock() {
        let d = tempdir().expect("tempdir");
        let mut first =
            Spool::open_with_segment_name(d.path(), samples_fingerprint(), "segment-fixed.spool")
                .expect("first opens");
        let old_path = first.segment_path().to_path_buf();
        first.bytes = MAX_SEGMENT_BYTES;
        assert!(
            first
                .rotate_if_due_with_segment_name("segment-fixed.spool")
                .expect("rotate")
        );
        assert_ne!(first.segment_path(), old_path);
        assert_eq!(
            first.segment_path().file_name().and_then(|n| n.to_str()),
            Some("segment-fixed-00000000000000000001.spool")
        );
        let discovered = discoverable_segments(d.path());
        assert_eq!(discovered.len(), 2);
        assert!(discovered.contains(&old_path));
        assert!(discovered.contains(&first.segment_path().to_path_buf()));

        let err = Spool::open(d.path()).expect_err("rotation must retain the writer lock");
        assert_eq!(err.kind(), io::ErrorKind::WouldBlock);
    }
}
