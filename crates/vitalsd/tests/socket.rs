//! End-to-end over a real Unix socket.
//!
//! The unit tests cover the rules; this covers the wiring. Several of the
//! properties that matter here — a slow subscriber not stalling ingest, a
//! malformed frame not killing the daemon — cannot be observed without two
//! parties and a real socket.
#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::zombie_processes,
    clippy::match_wild_err_arm,
    clippy::cast_possible_truncation,
    reason = "test harness: assertions should panic loudly, the daemon child is \
              reaped in Drop, and the frame sizes here are literals"
)]

use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use vitals_proto::{
    ErrorKind, Request, Response, WireSample, check_frame_len, decode_payload, encode_frame,
};

struct Daemon {
    child: Child,
    socket: std::path::PathBuf,
    dir: tempfile::TempDir,
}

impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn start() -> Daemon {
    let dir = tempfile::tempdir().expect("tempdir");
    let socket = dir.path().join("vitalsd.sock");
    let child = Command::new(env!("CARGO_BIN_EXE_vitalsd"))
        .args([
            "--socket",
            socket.to_str().expect("utf8"),
            "--spool",
            dir.path().join("spool").to_str().expect("utf8"),
            "--state",
            dir.path().to_str().expect("utf8"),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn vitalsd");

    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if socket.exists() {
            std::thread::sleep(Duration::from_millis(50));
            return Daemon { child, socket, dir };
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    panic!("vitalsd did not create its socket within 10s");
}

async fn send(stream: &mut UnixStream, req: &Request) {
    let bytes = encode_frame(req).expect("encode");
    stream.write_all(&bytes).await.expect("write");
}

async fn recv(stream: &mut UnixStream) -> Response {
    let mut len = [0u8; 4];
    stream.read_exact(&mut len).await.expect("read len");
    let n = check_frame_len(u32::from_le_bytes(len)).expect("len ok");
    let mut payload = vec![0u8; n];
    stream.read_exact(&mut payload).await.expect("read body");
    decode_payload(&payload).expect("decode")
}

fn sample(source: i32, seq: i64) -> WireSample {
    WireSample {
        source,
        r#type: 6,
        session_id: 1,
        seq,
        timestamp_unix_micros: 1_767_225_600_000_000,
        value: 118.0,
        extra: None,
        batch_id: None,
    }
}

#[tokio::test]
async fn append_then_health_reports_what_landed() {
    let d = start();
    let mut c = UnixStream::connect(&d.socket).await.expect("connect");

    send(
        &mut c,
        &Request::Append {
            samples: vec![sample(0, 0), sample(0, 1)],
            sync: false,
        },
    )
    .await;
    match recv(&mut c).await {
        Response::Appended { count, durable } => {
            assert_eq!(count, 2);
            assert!(!durable, "sync was not requested");
        }
        other => panic!("expected Appended, got {other:?}"),
    }

    send(&mut c, &Request::Health).await;
    match recv(&mut c).await {
        Response::Health { health } => {
            assert_eq!(health.total_samples, 2);
            assert_eq!(health.sources.len(), 1);
            assert_eq!(health.sources[0].samples, 2);
            assert!(health.spool_bytes > 0);
        }
        other => panic!("expected Health, got {other:?}"),
    }
}

/// A caller that asks for durability must be able to tell it got it.
#[tokio::test]
async fn sync_true_is_reported_as_durable() {
    let d = start();
    let mut c = UnixStream::connect(&d.socket).await.expect("connect");
    send(
        &mut c,
        &Request::Append {
            samples: vec![sample(0, 0)],
            sync: true,
        },
    )
    .await;
    match recv(&mut c).await {
        Response::Appended { durable, .. } => assert!(durable),
        other => panic!("expected Appended, got {other:?}"),
    }
}

/// A sync append must leave a *sealed* segment behind. The drainer cannot
/// consume the active segment, so a durable-but-unsealed reading would sit
/// invisible until the age/size rotation fired.
#[tokio::test]
async fn sync_append_seals_the_segment_for_the_drainer() {
    let d = start();
    let mut c = UnixStream::connect(&d.socket).await.expect("connect");
    send(
        &mut c,
        &Request::Append {
            samples: vec![sample(0, 0)],
            sync: true,
        },
    )
    .await;
    match recv(&mut c).await {
        Response::Appended { durable, .. } => assert!(durable),
        other => panic!("expected Appended, got {other:?}"),
    }
    let segments = std::fs::read_dir(d.dir.path().join("spool"))
        .expect("spool dir")
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|x| x == "spool"))
        .count();
    assert!(
        segments >= 2,
        "expected a sealed segment plus a fresh active one, found {segments}"
    );
}

/// An empty result set and "there is no store yet" are different claims.
#[tokio::test]
async fn query_says_what_is_missing_instead_of_returning_nothing() {
    let d = start();
    let mut c = UnixStream::connect(&d.socket).await.expect("connect");
    send(
        &mut c,
        &Request::Query {
            source: None,
            r#type: None,
            since_unix_micros: 0,
            until_unix_micros: i64::MAX,
        },
    )
    .await;
    match recv(&mut c).await {
        Response::Error { kind, detail } => {
            assert_eq!(kind, ErrorKind::Unavailable);
            assert!(detail.contains("T-C3"), "must name the blocker: {detail}");
        }
        other => panic!("expected Unavailable, got {other:?}"),
    }
}

/// Fan-out reaches a matching subscriber.
#[tokio::test]
async fn a_subscriber_receives_matching_samples() {
    let d = start();
    let mut sub = UnixStream::connect(&d.socket).await.expect("connect sub");
    send(
        &mut sub,
        &Request::Subscribe {
            source: Some(0),
            r#type: None,
        },
    )
    .await;
    // Let the subscribe register before appending.
    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut pusher = UnixStream::connect(&d.socket)
        .await
        .expect("connect pusher");
    send(
        &mut pusher,
        &Request::Append {
            samples: vec![sample(0, 7)],
            sync: false,
        },
    )
    .await;
    let _ = recv(&mut pusher).await;

    let got = tokio::time::timeout(Duration::from_secs(5), recv(&mut sub))
        .await
        .expect("a sample should arrive");
    match got {
        Response::Sample { sample } => assert_eq!(sample.seq, 7),
        other => panic!("expected Sample, got {other:?}"),
    }
}

/// A non-matching selector must not receive anything.
#[tokio::test]
async fn a_selector_that_does_not_match_receives_nothing() {
    let d = start();
    let mut sub = UnixStream::connect(&d.socket).await.expect("connect sub");
    send(
        &mut sub,
        &Request::Subscribe {
            source: Some(99),
            r#type: None,
        },
    )
    .await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut pusher = UnixStream::connect(&d.socket)
        .await
        .expect("connect pusher");
    send(
        &mut pusher,
        &Request::Append {
            samples: vec![sample(0, 1)],
            sync: false,
        },
    )
    .await;
    let _ = recv(&mut pusher).await;

    let r = tokio::time::timeout(Duration::from_millis(500), recv(&mut sub)).await;
    assert!(r.is_err(), "a non-matching subscriber must receive nothing");
}

/// The property amendment 05 §4.11 exists for: a subscriber that never reads
/// must not stall ingest for everyone else.
#[tokio::test]
async fn a_subscriber_that_never_reads_does_not_stall_ingest() {
    let d = start();
    let mut deadbeat = UnixStream::connect(&d.socket).await.expect("connect");
    send(
        &mut deadbeat,
        &Request::Subscribe {
            source: None,
            r#type: None,
        },
    )
    .await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    // Deliberately never call recv on `deadbeat` again.

    let mut pusher = UnixStream::connect(&d.socket)
        .await
        .expect("connect pusher");
    let batch: Vec<WireSample> = (0..64).map(|i| sample(0, i)).collect();

    let started = Instant::now();
    for round in 0..80 {
        send(
            &mut pusher,
            &Request::Append {
                samples: batch.clone(),
                sync: false,
            },
        )
        .await;
        match tokio::time::timeout(Duration::from_secs(5), recv(&mut pusher)).await {
            Ok(Response::Appended { count, .. }) => assert_eq!(count, 64),
            Ok(other) => panic!("round {round}: expected Appended, got {other:?}"),
            Err(_) => panic!("round {round}: ingest stalled behind a slow subscriber"),
        }
    }
    assert!(
        started.elapsed() < Duration::from_secs(30),
        "5120 samples took {:?} with a deadbeat subscriber attached",
        started.elapsed()
    );

    // And the drops are counted and attributed, not silently swallowed.
    let mut probe = UnixStream::connect(&d.socket).await.expect("connect probe");
    send(&mut probe, &Request::Health).await;
    match recv(&mut probe).await {
        Response::Health { health } => {
            let dropped: u64 = health.subscribers.iter().map(|s| s.dropped).sum();
            assert!(dropped > 0, "a deadbeat subscriber must accumulate drops");
            assert!(
                health.degraded.iter().any(|d| d.contains("dropped")),
                "drops must surface as a named degraded state: {:?}",
                health.degraded
            );
        }
        other => panic!("expected Health, got {other:?}"),
    }
}

/// A malformed payload must not take the daemon down, and must not take the
/// connection down either — the framing is still intact.
#[tokio::test]
async fn a_malformed_payload_is_answered_and_the_connection_survives() {
    let d = start();
    let mut c = UnixStream::connect(&d.socket).await.expect("connect");

    let junk = [0xFFu8, 0x00, 0x13, 0x37];
    let mut frame = (junk.len() as u32).to_le_bytes().to_vec();
    frame.extend_from_slice(&junk);
    c.write_all(&frame).await.expect("write");

    match recv(&mut c).await {
        Response::Error { kind, .. } => assert_eq!(kind, ErrorKind::Malformed),
        other => panic!("expected Malformed, got {other:?}"),
    }

    // Still usable.
    send(&mut c, &Request::Health).await;
    assert!(matches!(recv(&mut c).await, Response::Health { .. }));
}

/// An oversized length prefix must be refused before anything is allocated for
/// it, and the daemon must survive.
#[tokio::test]
async fn an_oversized_frame_is_refused_without_killing_the_daemon() {
    let d = start();
    {
        let mut bad = UnixStream::connect(&d.socket).await.expect("connect");
        bad.write_all(&u32::MAX.to_le_bytes()).await.expect("write");
        match tokio::time::timeout(Duration::from_secs(5), recv(&mut bad)).await {
            Ok(Response::Error { kind, .. }) => assert_eq!(kind, ErrorKind::Malformed),
            Ok(other) => panic!("expected Malformed, got {other:?}"),
            Err(_) => panic!("no answer to an oversized frame"),
        }
    }

    let mut good = UnixStream::connect(&d.socket)
        .await
        .expect("daemon still alive");
    send(&mut good, &Request::Health).await;
    assert!(matches!(recv(&mut good).await, Response::Health { .. }));
}

/// A subscriber that disconnects must stop being tracked, or its entry leaks
/// and its stale drop count keeps the daemon looking degraded forever.
#[tokio::test]
async fn a_disconnected_subscriber_is_forgotten() {
    let d = start();
    {
        let mut sub = UnixStream::connect(&d.socket).await.expect("connect");
        send(
            &mut sub,
            &Request::Subscribe {
                source: None,
                r#type: None,
            },
        )
        .await;
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    tokio::time::sleep(Duration::from_millis(300)).await;

    let mut probe = UnixStream::connect(&d.socket).await.expect("connect probe");
    send(&mut probe, &Request::Health).await;
    match recv(&mut probe).await {
        Response::Health { health } => {
            assert!(
                health.subscribers.is_empty(),
                "leaked: {:?}",
                health.subscribers
            );
        }
        other => panic!("expected Health, got {other:?}"),
    }
}

/// Two collectors at once is the normal case, not an edge case.
#[tokio::test]
async fn concurrent_clients_all_get_served() {
    let d = start();
    let mut handles = Vec::new();
    for source in 0..4 {
        let socket = d.socket.clone();
        handles.push(tokio::spawn(async move {
            let mut c = UnixStream::connect(&socket).await.expect("connect");
            for seq in 0..25 {
                send(
                    &mut c,
                    &Request::Append {
                        samples: vec![sample(source, seq)],
                        sync: false,
                    },
                )
                .await;
                match recv(&mut c).await {
                    Response::Appended { count, .. } => assert_eq!(count, 1),
                    other => panic!("expected Appended, got {other:?}"),
                }
            }
        }));
    }
    for h in handles {
        h.await.expect("client task");
    }

    let mut probe = UnixStream::connect(&d.socket).await.expect("connect probe");
    send(&mut probe, &Request::Health).await;
    match recv(&mut probe).await {
        Response::Health { health } => {
            assert_eq!(health.total_samples, 100);
            assert_eq!(health.sources.len(), 4);
        }
        other => panic!("expected Health, got {other:?}"),
    }
}
