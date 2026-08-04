//! Round-trip against a real `PostgreSQL` server.
//!
//! Encoding correctly and *believing* you encode correctly are different
//! claims, and only a real server can tell them apart. The unit tests assert
//! against captured bytes; this asserts that a spool file the encoder produced
//! is actually loadable, and that every value survives.
//!
//! Skipped unless `VITALS_TEST_PG` is set to a connection the `psql` binary can
//! use, e.g.
//!
//! ```sh
//! docker run -d --rm --name aw2-refpg -e POSTGRES_PASSWORD=scratch \
//!     -p 55432:5432 postgres:16
//! VITALS_TEST_PG=postgres://postgres:scratch@localhost:55432/postgres \
//!     cargo test -p vitals-core --test roundtrip
//! ```
//!
//! **Skipping is announced, never silent.** A round-trip test that quietly
//! passes when no database is present is indistinguishable from one that ran —
//! which is the failure mode this project has produced five separate times.

#![expect(
    clippy::expect_used,
    reason = "a failing assertion in a test should panic loudly"
)]

use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{Duration, UNIX_EPOCH};
use vitals_core::{NodeId, Sample, Spool, copybin};

const ROWS: i64 = 1000;

fn psql(url: &str, sql: &str) -> Result<String, String> {
    let out = Command::new("psql")
        .args(["-X", "-q", "-At", url, "-c", sql])
        .output()
        .map_err(|e| format!("psql not runnable: {e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).into_owned());
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn sample(i: i64) -> Sample {
    Sample {
        source: 6,
        r#type: 6,
        session_id: 4242,
        seq: i,
        // Spread across a range wide enough that an epoch error cannot hide,
        // with sub-second components so precision loss shows up too.
        timestamp: UNIX_EPOCH
            + Duration::from_secs(1_700_000_000 + u64::try_from(i).unwrap_or(0) * 37)
            + Duration::from_micros(u64::try_from(i).unwrap_or(0) * 13 % 1_000_000),
        // Values that are not round in binary, so a truncation to f32 or a
        // decimal detour would show.
        value: 0.000_123_456_789f64.mul_add(f64::from(i32::try_from(i).unwrap_or(0)), 90.0),
        extra: if i % 7 == 0 {
            Some(vec![0xAB, 0xCD, u8::try_from(i % 251).unwrap_or(0)])
        } else {
            None
        },
        batch_id: if i % 3 == 0 { Some(i * 1_000) } else { None },
    }
}

#[test]
fn spool_bytes_load_into_postgres_and_every_value_survives() {
    let Ok(url) = std::env::var("VITALS_TEST_PG") else {
        // Loud, not silent. See the module docs.
        eprintln!(
            "SKIPPED: spool_bytes_load_into_postgres_and_every_value_survives \
             — set VITALS_TEST_PG to run it. This test has NOT verified anything."
        );
        return;
    };

    let node = NodeId::from_bytes([0x5A; 16]);
    let dir = tempfile::tempdir().expect("tempdir");

    // Write a real spool, through the real API, including a sync-true batch.
    {
        let mut spool = Spool::open(dir.path()).expect("open spool");
        let batch: Vec<Sample> = (0..ROWS).map(sample).collect();
        for chunk in batch.chunks(64) {
            spool.append(chunk, &node, false).expect("append");
        }
        // The bp-collector path.
        spool
            .append(&[sample(ROWS)], &node, true)
            .expect("append sync");
        spool.sync().expect("sync");
    }

    let seg = std::fs::read_dir(dir.path())
        .expect("readdir")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| p.extension().is_some_and(|x| x == "spool"))
        .expect("a segment exists");

    let bytes = std::fs::read(&seg).expect("read segment");
    // Strip the 18-byte segment framing; the COPY stream starts after it, and
    // append the trailer the drainer is responsible for.
    let mut stream = bytes.get(18..).expect("segment has a body").to_vec();
    stream.extend_from_slice(&copybin::copy_trailer());

    psql(
        &url,
        "DROP TABLE IF EXISTS rt_samples;
         CREATE TABLE rt_samples (
            source INTEGER NOT NULL, type INTEGER NOT NULL,
            session_id BIGINT NOT NULL, seq BIGINT NOT NULL,
            timestamp TIMESTAMPTZ NOT NULL, value DOUBLE PRECISION NOT NULL,
            extra BYTEA, node_id BYTEA NOT NULL, batch_id BIGINT,
            PRIMARY KEY (source, type, session_id, seq));",
    )
    .expect("create table");

    let mut child = Command::new("psql")
        .args([
            "-X",
            "-q",
            &url,
            "-c",
            "COPY rt_samples FROM STDIN (FORMAT binary)",
        ])
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn psql");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(&stream)
        .expect("write stream");
    let out = child.wait_with_output().expect("wait");
    assert!(
        out.status.success(),
        "COPY FROM STDIN rejected the spool bytes: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    assert_loaded_correctly(&url);
}

/// The claims, separately, so a failure names which one broke.
fn assert_loaded_correctly(url: &str) {
    let count = psql(url, "SELECT count(*) FROM rt_samples").expect("count");
    assert_eq!(count.trim(), (ROWS + 1).to_string(), "row count after load");

    // Every seq present exactly once — the identity that matters.
    let gaps = psql(
        url,
        "SELECT max(seq) - count(*) + 1 FROM rt_samples WHERE session_id = 4242",
    )
    .expect("gap query");
    assert_eq!(
        gaps.trim(),
        "0",
        "seq must be dense: max(seq) - count(*) + 1"
    );

    // Values within 1e-9, and timestamps to the microsecond. A Unix-epoch bug
    // would put every row in 1993 and this comparison would fail by ~30 years.
    let mismatches = psql(
        url,
        "SELECT count(*) FROM rt_samples
         WHERE abs(value - (90.0 + seq * 0.000123456789)) > 1e-9
            OR timestamp <> to_timestamp(1700000000 + seq * 37)
                         + (seq * 13 % 1000000) * interval '1 microsecond'",
    )
    .expect("mismatch query");
    assert_eq!(
        mismatches.trim(),
        "0",
        "values or timestamps did not survive the round trip"
    );

    // NULL and non-NULL must have stayed distinct in both nullable columns.
    let nulls = psql(
        url,
        "SELECT count(*) FILTER (WHERE extra IS NULL) || ',' ||
                count(*) FILTER (WHERE batch_id IS NULL) FROM rt_samples",
    )
    .expect("null query");
    let expected_extra_null = (0..=ROWS).filter(|i| i % 7 != 0).count();
    let expected_batch_null = (0..=ROWS).filter(|i| i % 3 != 0).count();
    assert_eq!(
        nulls.trim(),
        format!("{expected_extra_null},{expected_batch_null}"),
        "NULL and non-NULL must not have been conflated"
    );

    psql(url, "DROP TABLE rt_samples").expect("drop");
}
