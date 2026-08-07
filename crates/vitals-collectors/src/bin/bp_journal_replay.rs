//! Reads the bp-collector raw-frame journal; optionally replays it into vitalsd.
//!
//! The journal exists so a decoder bug is survivable for the one source whose
//! readings are deleted at their origin on delivery. A journal without a
//! reader is a tombstone nobody can open — this tool is what makes the
//! recovery property real.
//!
//! Record format: arrival micros (`i64` LE) · payload length (`u32` LE) ·
//! payload bytes. There is no per-record checksum yet: a torn tail write ends
//! the readable prefix, and this tool stops there with the byte offset named
//! rather than guessing at resynchronisation.
//!
//! Replay identity: each `--apply` run mints a fresh session and assigns
//! drain-index seqs, exactly like a live connection. Replaying frames that
//! already landed therefore produces *visible duplicates*, never silent
//! drops — the failure direction amendment 06 §1.1 chose for this source.

use std::io::Read as _;
use std::path::{Path, PathBuf};
use vitals_decode::bp::bp_decode_2a35;

/// ATT payloads cannot exceed 512 bytes; anything larger is a torn record.
const MAX_FRAME: u32 = 512;

#[cfg_attr(test, derive(Debug))]
struct Record {
    offset: u64,
    arrival_micros: i64,
    raw: Vec<u8>,
}

fn read_journal(path: &Path) -> Result<Vec<Record>, String> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .and_then(|mut f| f.read_to_end(&mut bytes))
        .map_err(|e| format!("open {}: {e}", path.display()))?;
    let mut records = Vec::new();
    let mut off = 0usize;
    while off < bytes.len() {
        let start = off as u64;
        let Some(head) = bytes.get(off..off + 12) else {
            return Err(format!(
                "torn record header at byte {start}: {} trailing byte(s); \
                 records before this point are intact",
                bytes.len() - off
            ));
        };
        let arrival = head
            .get(..8)
            .and_then(|b| <[u8; 8]>::try_from(b).ok())
            .map(i64::from_le_bytes)
            .ok_or_else(|| format!("unreadable arrival at byte {start}"))?;
        let len = head
            .get(8..12)
            .and_then(|b| <[u8; 4]>::try_from(b).ok())
            .map(u32::from_le_bytes)
            .ok_or_else(|| format!("unreadable length at byte {start}"))?;
        if len > MAX_FRAME {
            return Err(format!(
                "implausible frame length {len} at byte {start} (max {MAX_FRAME}): \
                 journal is torn here; records before this point are intact"
            ));
        }
        off += 12;
        let Some(raw) = bytes.get(off..off + len as usize) else {
            return Err(format!(
                "torn payload at byte {start}: header claims {len} byte(s), \
                 file ends first; records before this point are intact"
            ));
        };
        records.push(Record {
            offset: start,
            arrival_micros: arrival,
            raw: raw.to_vec(),
        });
        off += len as usize;
    }
    Ok(records)
}

fn describe(records: &[Record]) {
    println!("{} journaled frame(s)", records.len());
    for r in records {
        let hex: String = r.raw.iter().fold(String::new(), |mut out, b| {
            use std::fmt::Write as _;
            let _ = write!(out, "{b:02x}");
            out
        });
        match bp_decode_2a35(&r.raw) {
            Some(m) => println!(
                "  @{:>8}  arrival={}  {}  sys={:?} dia={:?} map={:?} pulse={:?}",
                r.offset, r.arrival_micros, hex, m.systolic, m.diastolic, m.map, m.pulse_bpm
            ),
            None => println!(
                "  @{:>8}  arrival={}  {}  UNDECODABLE under current decoder",
                r.offset, r.arrival_micros, hex
            ),
        }
    }
}

#[cfg(unix)]
fn apply(records: &[Record], socket: &Path, state_dir: &Path) -> Result<(), String> {
    use std::io::Write as _;
    use vitals_core::{Session, SessionMinter};
    use vitals_proto::{Request, Response, check_frame_len, decode_payload, encode_frame};

    let mut minter =
        SessionMinter::open(state_dir).map_err(|e| format!("open session minter: {e}"))?;
    let id = minter.mint().map_err(|e| format!("mint session id: {e}"))?;
    let mut session = Session::new(id);
    let mut replayed = 0usize;
    for r in records {
        let seq = session.next_seq();
        let Some(m) = bp_decode_2a35(&r.raw) else {
            eprintln!(
                "bp-journal-replay: frame at byte {} still undecodable; skipped",
                r.offset
            );
            continue;
        };
        let built = vitals_collectors::bp_build::samples(&m, session.id(), seq, r.arrival_micros);
        for reason in &built.rejected {
            eprintln!("bp-journal-replay: {reason}");
        }
        if built.samples.is_empty() {
            continue;
        }
        let mut stream = std::os::unix::net::UnixStream::connect(socket)
            .map_err(|e| format!("connect {}: {e}", socket.display()))?;
        let frame = encode_frame(&Request::Append {
            samples: built.samples,
            sync: true,
        })
        .map_err(|e| format!("encode: {e}"))?;
        stream
            .write_all(&frame)
            .map_err(|e| format!("write: {e}"))?;
        let mut len = [0u8; 4];
        stream
            .read_exact(&mut len)
            .map_err(|e| format!("read reply length: {e}"))?;
        let n = check_frame_len(u32::from_le_bytes(len)).map_err(|e| format!("reply: {e}"))?;
        let mut body = vec![0; n];
        stream
            .read_exact(&mut body)
            .map_err(|e| format!("read reply: {e}"))?;
        match decode_payload::<Response>(&body).map_err(|e| format!("decode reply: {e}"))? {
            Response::Appended { durable: true, .. } => replayed += 1,
            other => return Err(format!("append not durable: {other:?}")),
        }
    }
    println!(
        "replayed {replayed} frame(s) under session {id} — duplicates, if any, are visible by design"
    );
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let Some(journal) = args.get(1).map(PathBuf::from) else {
        eprintln!(
            "usage: bp-journal-replay <journal> [--apply [--socket <path>] [--state-dir <dir>]]"
        );
        std::process::exit(64);
    };
    let records = match read_journal(&journal) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("bp-journal-replay: {e}");
            std::process::exit(65);
        }
    };
    describe(&records);
    if args.iter().any(|a| a == "--apply") {
        #[cfg(unix)]
        {
            let flag = |name: &str, default: &str| {
                args.iter()
                    .position(|a| a == name)
                    .and_then(|i| args.get(i + 1))
                    .cloned()
                    .unwrap_or_else(|| default.to_owned())
            };
            let socket = PathBuf::from(flag("--socket", "/run/vitals/vitalsd.sock"));
            let state_dir = PathBuf::from(flag("--state-dir", "/var/lib/vitals/bp"));
            if let Err(e) = apply(&records, &socket, &state_dir) {
                eprintln!("bp-journal-replay: {e}");
                std::process::exit(70);
            }
        }
        #[cfg(not(unix))]
        {
            eprintln!("bp-journal-replay: --apply needs a Unix socket host");
            std::process::exit(64);
        }
    }
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test assertions should panic loudly on fixed fixtures"
)]
mod tests {
    use super::*;
    use std::io::Write as _;

    fn journal_with(records: &[(i64, &[u8])], trailing: &[u8]) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().expect("tempfile");
        for (arrival, raw) in records {
            f.write_all(&arrival.to_le_bytes()).expect("write arrival");
            f.write_all(&u32::try_from(raw.len()).expect("len").to_le_bytes())
                .expect("write len");
            f.write_all(raw).expect("write payload");
        }
        f.write_all(trailing).expect("write trailing");
        f.flush().expect("flush");
        f
    }

    #[test]
    fn intact_records_parse_with_offsets_and_arrivals() {
        let f = journal_with(&[(1_000, b"\x14\x87\x00"), (2_000, b"\xaa")], &[]);
        let r = read_journal(f.path()).expect("parses");
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].arrival_micros, 1_000);
        assert_eq!(r[0].raw, b"\x14\x87\x00");
        assert_eq!(r[0].offset, 0);
        assert_eq!(r[1].offset, 12 + 3);
    }

    #[test]
    fn torn_header_names_the_offset_and_keeps_the_prefix_intact() {
        let f = journal_with(&[(1_000, b"\x01\x02")], &[0xEE; 5]);
        let e = read_journal(f.path()).expect_err("torn header must fail");
        assert!(e.contains("torn record header at byte 14"), "{e}");
        assert!(e.contains("records before this point are intact"), "{e}");
    }

    #[test]
    fn implausible_length_is_a_named_tear_not_a_giant_allocation() {
        let mut f = tempfile::NamedTempFile::new().expect("tempfile");
        f.write_all(&1_000_i64.to_le_bytes()).expect("arrival");
        f.write_all(&(MAX_FRAME + 1).to_le_bytes()).expect("len");
        f.flush().expect("flush");
        let e = read_journal(f.path()).expect_err("must fail");
        assert!(e.contains("implausible frame length 513 at byte 0"), "{e}");
    }

    #[test]
    fn torn_payload_names_the_claim_and_the_shortfall() {
        let mut f = tempfile::NamedTempFile::new().expect("tempfile");
        f.write_all(&1_000_i64.to_le_bytes()).expect("arrival");
        f.write_all(&8_u32.to_le_bytes()).expect("len");
        f.write_all(b"\x01\x02\x03").expect("short payload");
        f.flush().expect("flush");
        let e = read_journal(f.path()).expect_err("must fail");
        assert!(e.contains("torn payload at byte 0"), "{e}");
        assert!(e.contains("claims 8 byte(s)"), "{e}");
    }
}
