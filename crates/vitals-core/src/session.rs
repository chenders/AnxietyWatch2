//! Session identity, sequence numbers, and node provenance.
//!
//! # Why `session_id` uniqueness across restarts is load-bearing
//!
//! Identity is `(source, type, session_id, seq)` and ingest is
//! `ON CONFLICT DO NOTHING`. A per-process counter starting at zero — the
//! obvious implementation — therefore *destroys data silently*: a collector
//! restarts, re-mints `session_id = 0`, and its next reading lands on the same
//! key as the first reading of a previous run. The insert is discarded with no
//! error, no log line, and no gap that any health metric can see.
//!
//! The blast radius is asymmetric. For RR intervals `seq` is a beat index that
//! climbs into the thousands, so only the first few beats of a session collide.
//! For blood pressure `seq` is 0–3 *every single time*, so collision is close to
//! total — on the one source that is reconstructible from nothing, because the
//! cuff deletes its copy on the kernel's ATT confirmation.
//!
//! So the counter is persisted and fsynced **before** the id is handed out.
//! Doing it after would reuse an id across a crash, which is the same bug with
//! a narrower window.
//!
//! Wall-clock time is not used either: the RTC can be wrong at boot, which is
//! why the hardware floor pins a watchdog and a governor in the first place.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

/// 16 bytes identifying this node. Generated once, never regenerated: it is
/// provenance, and it must survive a store rebuild so restored rows stay
/// attributable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeId([u8; 16]);

impl NodeId {
    /// Wraps raw bytes. For tests and for reading a value back out of storage.
    #[must_use]
    pub const fn from_bytes(b: [u8; 16]) -> Self {
        Self(b)
    }

    /// The raw bytes, for the `BYTEA` column.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    /// Reads `<dir>/node_id`, creating it on first call.
    ///
    /// Written via a temp file and rename so a crash mid-write cannot leave a
    /// truncated id that a later run would read as valid. A silently changed
    /// `node_id` re-attributes history to a node that never produced it.
    ///
    /// # Errors
    /// I/O failure, or an existing file that is not exactly 16 bytes — which is
    /// reported rather than regenerated, because regenerating would discard the
    /// provenance of everything already stored.
    pub fn load_or_create(dir: &Path) -> io::Result<Self> {
        let path = dir.join("node_id");
        match fs::read(&path) {
            Ok(bytes) => {
                let b: [u8; 16] = bytes.as_slice().try_into().map_err(|_| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "{}: node_id is {} bytes, expected 16 — refusing to regenerate, \
                             that would re-attribute every stored row",
                            path.display(),
                            bytes.len()
                        ),
                    )
                })?;
                Ok(Self(b))
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                fs::create_dir_all(dir)?;
                let mut b = [0u8; 16];
                getrandom::fill(&mut b).map_err(|e| io::Error::other(format!("getrandom: {e}")))?;
                let tmp = dir.join("node_id.tmp");
                {
                    let mut f = File::create(&tmp)?;
                    f.write_all(&b)?;
                    f.sync_all()?;
                }
                fs::rename(&tmp, &path)?;
                sync_dir(dir)?;
                Ok(Self(b))
            }
            Err(e) => Err(e),
        }
    }
}

/// fsync the directory so a rename is itself durable. Without this the rename
/// can be lost on power failure even though the file's contents were synced.
fn sync_dir(dir: &Path) -> io::Result<()> {
    File::open(dir)?.sync_all()
}

/// Hands out `session_id`s that are unique across process restarts.
#[derive(Debug)]
pub struct SessionMinter {
    path: PathBuf,
    dir: PathBuf,
}

impl SessionMinter {
    /// Opens (or prepares) the counter at `<dir>/session_seq`.
    ///
    /// # Errors
    /// I/O failure creating `dir`.
    pub fn open(dir: &Path) -> io::Result<Self> {
        fs::create_dir_all(dir)?;
        Ok(Self {
            path: dir.join("session_seq"),
            dir: dir.to_path_buf(),
        })
    }

    /// Mints the next `session_id`.
    ///
    /// The counter is incremented and **fsynced before the value is returned**.
    /// Returning first and persisting later would reuse an id across a crash —
    /// the same silent-drop bug this type exists to prevent, with a smaller
    /// window.
    ///
    /// # Errors
    /// I/O failure, or a counter file that is corrupt. A corrupt counter is an
    /// error rather than a reset to zero: resetting would hand out ids that are
    /// already in the store.
    pub fn mint(&mut self) -> io::Result<i64> {
        let mut f = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&self.path)?;

        let mut buf = Vec::new();
        f.read_to_end(&mut buf)?;
        let current: i64 = if buf.is_empty() {
            0
        } else {
            let s = std::str::from_utf8(&buf)
                .map_err(|_| self.corrupt("not UTF-8"))?
                .trim();
            s.parse()
                .map_err(|_| self.corrupt(&format!("{s:?} is not an integer")))?
        };

        let next = current
            .checked_add(1)
            .ok_or_else(|| self.corrupt("counter overflowed i64"))?;

        f.seek(SeekFrom::Start(0))?;
        f.set_len(0)?;
        write!(f, "{next}")?;
        // Durable BEFORE the caller sees it. This ordering is the whole point.
        f.sync_all()?;
        sync_dir(&self.dir)?;
        Ok(next)
    }

    fn corrupt(&self, why: &str) -> io::Error {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{}: session counter corrupt ({why}) — refusing to reset, that would \
                 re-issue ids already present in the store",
                self.path.display()
            ),
        )
    }
}

/// One upstream connection's identity: for `bp-collector` the BLE connection to
/// the cuff, not the collector's socket connection to `vitalsd`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Session {
    id: i64,
    next_seq: i64,
}

impl Session {
    /// Starts a session with `seq` counting from zero.
    #[must_use]
    pub const fn new(id: i64) -> Self {
        Self { id, next_seq: 0 }
    }

    /// Resumes at a known `seq` — for a source whose sequence derives from its
    /// own data (RR intervals from the cumulative sum) rather than from the
    /// order this process happened to see things.
    #[must_use]
    pub const fn resuming_at(id: i64, next_seq: i64) -> Self {
        Self { id, next_seq }
    }

    /// This session's id.
    #[must_use]
    pub const fn id(self) -> i64 {
        self.id
    }

    /// The next `seq`, advancing the counter.
    ///
    /// Saturates rather than wrapping. A wrapped `seq` would collide with the
    /// start of the same session and silently drop rows; a stuck one drops only
    /// the samples past the ceiling, and `max(seq) - count(*)` still reports the
    /// loss. Neither is good, but only one is invisible.
    pub const fn next_seq(&mut self) -> i64 {
        let s = self.next_seq;
        self.next_seq = self.next_seq.saturating_add(1);
        s
    }

    /// The `seq` that would be issued next, without advancing.
    #[must_use]
    pub const fn peek_seq(self) -> i64 {
        self.next_seq
    }
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    reason = "a failing assertion in a test should panic loudly"
)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn node_id_is_stable_across_reloads() {
        let d = tempdir().expect("tempdir");
        let a = NodeId::load_or_create(d.path()).expect("create");
        let b = NodeId::load_or_create(d.path()).expect("reload");
        assert_eq!(a, b, "node_id must never change once written");
        assert_ne!(a.as_bytes(), &[0u8; 16], "node_id must not be all zeros");
    }

    #[test]
    fn node_id_of_wrong_length_is_an_error_not_a_regeneration() {
        let d = tempdir().expect("tempdir");
        fs::write(d.path().join("node_id"), b"short").expect("write");
        let err = NodeId::load_or_create(d.path()).expect_err("must refuse");
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("refusing to regenerate"));

        let io_error_dir = tempdir().expect("I/O error tempdir");
        fs::create_dir(io_error_dir.path().join("node_id")).expect("node id directory");
        assert!(NodeId::load_or_create(io_error_dir.path()).is_err());
    }

    /// The defect this module exists to prevent. A fresh `SessionMinter` on the
    /// same directory is exactly what a process restart looks like.
    #[test]
    fn session_ids_never_repeat_across_restarts() {
        let d = tempdir().expect("tempdir");
        let mut seen = Vec::new();
        for _restart in 0..5 {
            let mut m = SessionMinter::open(d.path()).expect("open");
            for _connection in 0..3 {
                seen.push(m.mint().expect("mint"));
            }
        }
        let mut sorted = seen.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(
            sorted.len(),
            seen.len(),
            "a session_id was reused: {seen:?}"
        );
        assert_eq!(
            seen.first(),
            Some(&1),
            "ids start at 1, so 0 is never a valid session"
        );
    }

    #[test]
    fn session_ids_increase_monotonically() {
        let d = tempdir().expect("tempdir");
        let mut m = SessionMinter::open(d.path()).expect("open");
        let a = m.mint().expect("mint");
        let b = m.mint().expect("mint");
        assert!(b > a);
    }

    #[test]
    fn corrupt_counter_is_an_error_not_a_reset() {
        let d = tempdir().expect("tempdir");
        fs::write(d.path().join("session_seq"), b"garbage").expect("write");
        let mut m = SessionMinter::open(d.path()).expect("open");
        let err = m.mint().expect_err("must refuse");
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("refusing to reset"));
    }

    /// The counter must be durable before the id is used, or a crash re-issues
    /// it. Asserted by reading the file back immediately after minting.
    #[test]
    fn counter_is_persisted_before_the_id_is_returned() {
        let d = tempdir().expect("tempdir");
        let mut m = SessionMinter::open(d.path()).expect("open");
        let id = m.mint().expect("mint");
        let on_disk = fs::read_to_string(d.path().join("session_seq")).expect("read");
        assert_eq!(on_disk.trim().parse::<i64>().ok(), Some(id));
    }

    #[test]
    fn seq_starts_at_zero_and_advances() {
        let mut s = Session::new(7);
        assert_eq!(s.id(), 7);
        assert_eq!(s.next_seq(), 0);
        assert_eq!(s.next_seq(), 1);
        assert_eq!(s.peek_seq(), 2);
    }

    #[test]
    fn seq_saturates_rather_than_wrapping() {
        let mut s = Session::resuming_at(1, i64::MAX);
        assert_eq!(s.next_seq(), i64::MAX);
        // Wrapping to i64::MIN, then eventually back through 0, would collide
        // with the start of this same session and drop rows silently.
        assert_eq!(s.next_seq(), i64::MAX);
    }
}
