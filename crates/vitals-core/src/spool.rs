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

/// Magic at the head of every segment, before the fingerprint.
const SEGMENT_MAGIC: &[u8; 8] = b"VITSPOOL";

/// Bumped when the segment framing itself changes — distinct from the schema
/// fingerprint, which tracks the *table*.
const SEGMENT_FORMAT_VERSION: u16 = 1;

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
}

/// An append-only spool directory holding one open segment.
#[derive(Debug)]
pub struct Spool {
    dir: PathBuf,
    file: File,
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
    /// Takes an exclusive advisory lock on the segment. A second writer fails
    /// here rather than interleaving partial rows into the same file, where the
    /// damage would be undetectable — binary `COPY` has no record boundaries a
    /// reader can resynchronise on.
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
        fs::create_dir_all(dir)?;
        let path = dir.join(Self::segment_name());
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&path)?;
        // `try_lock_exclusive` returns io::Result<bool>: Ok(false) means the
        // lock was NOT acquired. An earlier version only handled Err, so
        // Ok(false) passed and the spool ran unlocked — a guard against
        // undetectable interleaved writes that silently did not guard. Caught
        // by `a_second_writer_is_refused`, which is why that test exists.
        let locked = file.try_lock_exclusive().map_err(|e| {
            io::Error::other(format!("{}: locking the spool failed: {e}", path.display()))
        })?;
        if !locked {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                format!(
                    "{}: another writer holds the spool lock — two writers would \
                     interleave partial rows with no way to resynchronise",
                    path.display()
                ),
            ));
        }

        let mut bytes = file.metadata()?.len();
        if bytes == 0 {
            let header = Self::segment_header(fingerprint);
            file.write_all(&header)?;
            file.write_all(&copybin::copy_header())?;
            file.sync_all()?;
            bytes = header.len() as u64 + 19;
        }

        let now = Instant::now();
        Ok(Self {
            dir: dir.to_path_buf(),
            file,
            path,
            bytes,
            opened: now,
            last_sync: now,
            dirty: false,
            fingerprint,
        })
    }

    fn segment_header(fp: SchemaFingerprint) -> Vec<u8> {
        let mut h = Vec::with_capacity(18);
        h.extend_from_slice(SEGMENT_MAGIC);
        h.extend_from_slice(&SEGMENT_FORMAT_VERSION.to_be_bytes());
        h.extend_from_slice(&fp.value().to_be_bytes());
        h
    }

    fn segment_name() -> String {
        // Monotonic within a boot is not enough — a name must not be reused
        // after a restart, or a new segment appends into an undrained one.
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        format!("segment-{secs:010}.spool")
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

        let mut buf = Vec::with_capacity(rows.len() * 96);
        for s in rows {
            copybin::encode_row(&mut buf, s, node_id)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
        }

        self.file.write_all(&buf)?;
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
            self.file.sync_all()?;
            self.dirty = false;
        }
        self.last_sync = Instant::now();
        Ok(())
    }

    /// Whether the current segment is due for rotation.
    #[must_use]
    pub fn rotation_due(&self) -> bool {
        self.bytes >= MAX_SEGMENT_BYTES || self.opened.elapsed() >= MAX_SEGMENT_AGE
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
        self.sync()?;
        FileExt::unlock(&self.file)?;
        let next = Self::open_with(&self.dir, self.fingerprint)?;
        *self = next;
        Ok(true)
    }
}

impl Drop for Spool {
    fn drop(&mut self) {
        // Best effort. A failure here cannot be reported, but leaving bytes
        // unsynced on a clean drop would lose data no error ever mentioned.
        let _ = self.sync();
        let _ = FileExt::unlock(&self.file);
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
}
