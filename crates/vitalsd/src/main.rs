//! `vitalsd` — the single writer.
//!
//! Every collector connects over a Unix socket; `vitalsd` alone touches the
//! spool. That is why the runtime is **current-thread**: a single writer needs
//! no locking, and the absence of locking is the design, not an optimisation.
//! A multi-threaded runtime here would silently make `&mut Spool` a lie.
//!
//! # What this daemon refuses to do
//!
//! - **Block a collector on disk unless it asked to.** `append` fsyncs before
//!   replying only when the caller sets `sync` (amendment 06 §2.1).
//! - **Block a collector on a slow subscriber.** Fan-out uses bounded queues;
//!   on overflow it drops, counts, and names the subscriber (amendment 05
//!   §4.11). A detector that missed samples must know it missed them.
//! - **Report silence as health.** A source past its cadence is `Stale`, never
//!   absent-and-therefore-fine.
//! - **Answer a query it cannot serve.** `query` returns `Unavailable` naming
//!   the missing dependency rather than an empty result set, because an empty
//!   result and "no store yet" are different claims.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc;
use vitals_core::{NodeId, Sample, Spool};
use vitals_proto::{
    DRAIN_HEALTH_SCHEMA, DrainHealth, ErrorKind, Request, Response, WireSample, check_frame_len,
    decode_payload, encode_frame,
};
use vitalsd::state::{SUBSCRIBER_QUEUE_DEPTH, Selector, State};

const DEFAULT_SOCKET: &str = "/run/vitals/vitalsd.sock";
const DEFAULT_SPOOL: &str = "/var/lib/vitals/spool";
const DEFAULT_STATE: &str = "/var/lib/vitals";

/// `SpO2` and respiratory rate. Only these can satisfy the heartbeat's primary
/// requirement; HR and HRV corroborate but must never launder a missing primary
/// into "all clear" (amendment 05 §2.1).
const PRIMARY_SOURCES: [i32; 1] = [0];

/// What a client task sends back to the single writer.
enum Cmd {
    Append {
        client_id: u64,
        request_id: u64,
        samples: Vec<WireSample>,
        sync: bool,
        reply: mpsc::Sender<Response>,
    },
    Subscribe {
        id: u64,
        selector: Selector,
        tx: mpsc::Sender<Response>,
    },
    Unsubscribe {
        id: u64,
    },
    Health {
        reply: mpsc::Sender<Response>,
    },
    Query {
        reply: mpsc::Sender<Response>,
    },
}

fn to_sample(w: &WireSample) -> Sample {
    let t = if w.timestamp_unix_micros >= 0 {
        UNIX_EPOCH + Duration::from_micros(w.timestamp_unix_micros.unsigned_abs())
    } else {
        UNIX_EPOCH - Duration::from_micros(w.timestamp_unix_micros.unsigned_abs())
    };
    Sample {
        source: w.source,
        r#type: w.r#type,
        session_id: w.session_id,
        seq: w.seq,
        timestamp: t,
        value: w.value,
        extra: w.extra.clone(),
        batch_id: w.batch_id,
    }
}

struct Args {
    socket: PathBuf,
    spool: PathBuf,
    state_dir: PathBuf,
}

fn parse_args() -> Args {
    let mut a = Args {
        socket: PathBuf::from(DEFAULT_SOCKET),
        spool: PathBuf::from(DEFAULT_SPOOL),
        state_dir: PathBuf::from(DEFAULT_STATE),
    };
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        match flag.as_str() {
            "--socket" => {
                if let Some(v) = it.next() {
                    a.socket = PathBuf::from(v);
                }
            }
            "--spool" => {
                if let Some(v) = it.next() {
                    a.spool = PathBuf::from(v);
                }
            }
            "--state" => {
                if let Some(v) = it.next() {
                    a.state_dir = PathBuf::from(v);
                }
            }
            _ => {}
        }
    }
    a
}

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

fn unavailable_drain_health(detail: String) -> DrainHealth {
    DrainHealth {
        schema: DRAIN_HEALTH_SCHEMA,
        available: false,
        generated_at_unix_secs: unix_seconds(),
        last_error: Some(detail),
        ..DrainHealth::default()
    }
}

fn read_drain_health(spool: &Path) -> DrainHealth {
    let path = spool.join(vitals_core::spool::DRAIN_HEALTH_FILENAME);
    match std::fs::read(&path) {
        Ok(bytes) => match serde_json::from_slice::<DrainHealth>(&bytes) {
            Ok(health) => health,
            Err(error) => unavailable_drain_health(format!(
                "{}: malformed drainer health JSON: {error}",
                path.display()
            )),
        },
        Err(error) => unavailable_drain_health(format!(
            "{}: drainer health read failed: {error}",
            path.display()
        )),
    }
}

fn refresh_drain_health(st: &mut State, spool: &Path, previous_blocker: &mut Option<String>) {
    let health = read_drain_health(spool);
    st.record_drain_health(health, unix_seconds());
    let current_blocker = st.drain_blocker().map(ToOwned::to_owned);
    if current_blocker != *previous_blocker {
        match &current_blocker {
            Some(blocker) => {
                eprintln!("event=vitalsd.health stage=drain result=degraded blocker={blocker:?}");
            }
            None => eprintln!("event=vitalsd.health stage=drain result=healthy"),
        }
        *previous_blocker = current_blocker;
    }
}

fn main() -> std::io::Result<()> {
    let args = parse_args();

    // Current-thread on purpose. See the module docs.
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let local = tokio::task::LocalSet::new();
    let result = local.block_on(&rt, run(args));
    if let Err(error) = &result {
        eprintln!(
            "event=vitalsd.failure stage=run result=error detail={:?}",
            error.to_string()
        );
    }
    result
}

async fn run(args: Args) -> std::io::Result<()> {
    std::fs::create_dir_all(&args.state_dir)?;
    let node_id = NodeId::load_or_create(&args.state_dir)?;
    let mut spool = Spool::open(&args.spool)?;

    if let Some(parent) = args.socket.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // A stale socket from a crashed run would make bind fail with EADDRINUSE.
    // Removing it is safe because the spool lock — not the socket — is what
    // prevents two daemons, and that lock has already been taken above.
    let _ = std::fs::remove_file(&args.socket);
    let listener = UnixListener::bind(&args.socket)?;
    set_socket_mode(&args.socket)?;

    let (cmd_tx, mut cmd_rx) = mpsc::channel::<Cmd>(256);
    let mut st = State::new_with_spool_bytes(PRIMARY_SOURCES.to_vec(), spool.segment_bytes());
    let mut subscribers: Vec<(u64, mpsc::Sender<Response>)> = Vec::new();
    let mut next_id: u64 = 0;
    let mut previous_drain_blocker = None;
    refresh_drain_health(&mut st, &args.spool, &mut previous_drain_blocker);

    eprintln!(
        "event=vitalsd.start socket={} spool={} node={}",
        args.socket.display(),
        args.spool.display(),
        hex16(node_id.as_bytes())
    );

    let mut tick = tokio::time::interval(Duration::from_millis(250));

    loop {
        tokio::select! {
            accepted = listener.accept() => {
                let (stream, _) = accepted?;
                next_id = next_id.saturating_add(1);
                let id = next_id;
                eprintln!("event=vitalsd.client stage=accepted client_id={id}");
                let tx = cmd_tx.clone();
                tokio::task::spawn_local(async move {
                    if let Err(e) = serve_client(stream, id, tx).await {
                        eprintln!(
                            "event=vitalsd.client stage=ended client_id={id} detail={:?}",
                            e.to_string()
                        );
                    }
                });
            }

            Some(cmd) = cmd_rx.recv() => {
                handle(cmd, &mut st, &mut spool, &node_id, &mut subscribers);
            }

            _ = tick.tick() => {
                // Bounds unsynced data at SYNC_INTERVAL even when no append
                // arrives to trigger the check.
                if let Err(e) = spool.sync() {
                    eprintln!(
                        "event=vitalsd.failure stage=fsync result=error detail={:?}",
                        e.to_string()
                    );
                    return Err(e);
                }
                let fsynced_rows = spool.take_fsynced_rows();
                if fsynced_rows > 0 {
                    st.record_fsynced(fsynced_rows);
                    eprintln!(
                        "event=vitalsd.stage stage=fsynced reason=periodic segment={} bytes={} count={fsynced_rows}",
                        spool.segment_path().display(),
                        spool.segment_bytes()
                    );
                }
                match spool.rotate_if_due() {
                    Ok(true) => eprintln!(
                        "event=vitalsd.stage stage=rotated segment={} bytes={}",
                        spool.segment_path().display(),
                        spool.segment_bytes()
                    ),
                    Ok(false) => {}
                    Err(e) => {
                        eprintln!(
                            "event=vitalsd.failure stage=rotation result=error detail={:?}",
                            e.to_string()
                        );
                        return Err(e);
                    }
                }
                refresh_drain_health(&mut st, &args.spool, &mut previous_drain_blocker);
            }
        }
    }
}

fn hex16(b: &[u8; 16]) -> String {
    use std::fmt::Write as _;
    b.iter().fold(String::with_capacity(32), |mut acc, x| {
        let _ = write!(acc, "{x:02x}");
        acc
    })
}

#[cfg(unix)]
fn set_socket_mode(path: &std::path::Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    // 0660: the vitals group only. Collectors run as members; nothing else on
    // the box can append samples.
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o660))
}

#[expect(
    clippy::too_many_lines,
    reason = "the command handler keeps each durability stage adjacent and auditable"
)]
fn handle(
    cmd: Cmd,
    st: &mut State,
    spool: &mut Spool,
    node_id: &NodeId,
    subscribers: &mut Vec<(u64, mpsc::Sender<Response>)>,
) {
    let now = Instant::now();
    match cmd {
        Cmd::Append {
            client_id,
            request_id,
            samples,
            sync,
            reply,
        } => {
            eprintln!(
                "event=vitalsd.append stage=received client_id={client_id} request_id={request_id} count={} sync={sync}",
                samples.len()
            );
            st.record_received(samples.len());
            let rows: Vec<Sample> = samples.iter().map(to_sample).collect();
            let resp = match spool.append(&rows, node_id, sync) {
                Ok(()) => {
                    st.record_appended(&samples, spool.segment_bytes(), now);
                    eprintln!(
                        "event=vitalsd.append stage=spooled client_id={client_id} request_id={request_id} count={} segment={} bytes={} result=ok",
                        samples.len(),
                        spool.segment_path().display(),
                        spool.segment_bytes()
                    );
                    let fsynced_rows = spool.take_fsynced_rows();
                    if fsynced_rows > 0 {
                        st.record_fsynced(fsynced_rows);
                        let reason = if sync { "explicit" } else { "append_interval" };
                        eprintln!(
                            "event=vitalsd.append stage=fsynced client_id={client_id} request_id={request_id} count={fsynced_rows} segment={} result=ok reason={reason}",
                            spool.segment_path().display()
                        );
                    }
                    let (published, dropped) = fan_out(&samples, st, subscribers);
                    st.record_published(published);
                    if published > 0 {
                        eprintln!(
                            "event=vitalsd.append stage=published client_id={client_id} request_id={request_id} count={published}"
                        );
                    }
                    if dropped > 0 {
                        eprintln!(
                            "event=vitalsd.append stage=published client_id={client_id} request_id={request_id} dropped={dropped} result=degraded"
                        );
                    }
                    Response::Appended {
                        count: samples.len(),
                        durable: sync,
                    }
                }
                Err(e) => {
                    eprintln!(
                        "event=vitalsd.append stage=spooled client_id={client_id} request_id={request_id} count={} result=error detail={:?}",
                        samples.len(),
                        e.to_string()
                    );
                    Response::Error {
                        kind: ErrorKind::Io,
                        detail: format!("spool append failed: {e}"),
                    }
                }
            };
            if reply.try_send(resp).is_err() {
                eprintln!(
                    "event=vitalsd.append stage=response client_id={client_id} request_id={request_id} result=not_delivered"
                );
            }
        }
        Cmd::Subscribe { id, selector, tx } => {
            st.add_subscriber(id, selector);
            subscribers.push((id, tx));
            eprintln!(
                "event=vitalsd.subscriber stage=registered subscriber_id={id} source={:?} type={:?}",
                selector.source, selector.r#type
            );
        }
        Cmd::Unsubscribe { id } => {
            st.remove_subscriber(id);
            subscribers.retain(|(sid, _)| *sid != id);
            eprintln!("event=vitalsd.subscriber stage=unregistered subscriber_id={id}");
        }
        Cmd::Health { reply } => {
            if reply
                .try_send(Response::Health {
                    health: Box::new(st.health(now)),
                })
                .is_err()
            {
                eprintln!("event=vitalsd.health stage=response result=not_delivered");
            }
        }
        Cmd::Query { reply } => {
            // An empty result set and "there is no store yet" are different
            // claims, and answering the first would be a lie.
            if reply
                .try_send(Response::Error {
                    kind: ErrorKind::Unavailable,
                    detail: "query needs the PostgreSQL store, which is gated on G3 \
                             (a verified restore). Blocked by: T-C1, T-C2, T-C3."
                        .to_owned(),
                })
                .is_err()
            {
                eprintln!("event=vitalsd.query stage=response result=not_delivered");
            }
        }
    }
}

/// Fan out to subscribers without ever blocking the writer.
fn fan_out(
    samples: &[WireSample],
    st: &mut State,
    subscribers: &mut [(u64, mpsc::Sender<Response>)],
) -> (u64, u64) {
    let mut published = 0_u64;
    let mut dropped = 0_u64;
    for s in samples {
        let ids: Vec<u64> = st.matching_subscribers(s).collect();
        for id in ids {
            let Some((_, tx)) = subscribers.iter().find(|(sid, _)| *sid == id) else {
                continue;
            };
            // try_send, never send: awaiting here would let one slow subscriber
            // stall ingest for every collector on the box.
            if tx
                .try_send(Response::Sample {
                    sample: Box::new(s.clone()),
                })
                .is_err()
            {
                dropped = dropped.saturating_add(1);
                st.record_drop(id, 1, SUBSCRIBER_QUEUE_DEPTH);
                eprintln!(
                    "event=vitalsd.subscriber stage=published subscriber_id={id} result=dropped reason=queue_full queue_depth={SUBSCRIBER_QUEUE_DEPTH}"
                );
            } else {
                published = published.saturating_add(1);
            }
        }
    }
    (published, dropped)
}

#[expect(
    clippy::too_many_lines,
    reason = "the socket loop keeps framing, protocol, and client lifecycle evidence together"
)]
async fn serve_client(
    stream: UnixStream,
    id: u64,
    cmd_tx: mpsc::Sender<Cmd>,
) -> std::io::Result<()> {
    let (mut rd, mut wr) = stream.into_split();
    let (out_tx, mut out_rx) = mpsc::channel::<Response>(SUBSCRIBER_QUEUE_DEPTH);

    let writer = tokio::task::spawn_local(async move {
        while let Some(resp) = out_rx.recv().await {
            match encode_frame(&resp) {
                Ok(bytes) => {
                    if let Err(error) = wr.write_all(&bytes).await {
                        eprintln!(
                            "event=vitalsd.client stage=write result=error detail={:?}",
                            error.to_string()
                        );
                        break;
                    }
                }
                Err(e) => eprintln!(
                    "event=vitalsd.client stage=encode result=error detail={:?}",
                    e.to_string()
                ),
            }
        }
    });

    let mut subscribed = false;
    let mut request_id = 0_u64;
    loop {
        let mut len_buf = [0u8; 4];
        if rd.read_exact(&mut len_buf).await.is_err() {
            eprintln!("event=vitalsd.client stage=disconnected client_id={id}");
            break; // clean disconnect
        }
        let len = match check_frame_len(u32::from_le_bytes(len_buf)) {
            Ok(n) => n,
            Err(e) => {
                eprintln!(
                    "event=vitalsd.client stage=frame result=malformed client_id={id} detail={:?}",
                    e.to_string()
                );
                let _ = out_tx.try_send(Response::Error {
                    kind: ErrorKind::Malformed,
                    detail: e.to_string(),
                });
                break; // the stream is desynchronised; there is no resync point
            }
        };
        let mut payload = vec![0u8; len];
        if rd.read_exact(&mut payload).await.is_err() {
            eprintln!(
                "event=vitalsd.client stage=frame result=truncated client_id={id} length={len}"
            );
            break;
        }

        let req: Request = match decode_payload(&payload) {
            Ok(r) => r,
            Err(e) => {
                eprintln!(
                    "event=vitalsd.client stage=decode result=malformed client_id={id} detail={:?}",
                    e.to_string()
                );
                let _ = out_tx.try_send(Response::Error {
                    kind: ErrorKind::Malformed,
                    detail: e.to_string(),
                });
                continue; // framing is still intact, so the connection survives
            }
        };

        let cmd = match req {
            Request::Append { samples, sync } => {
                request_id = request_id.saturating_add(1);
                Cmd::Append {
                    client_id: id,
                    request_id,
                    samples,
                    sync,
                    reply: out_tx.clone(),
                }
            }
            Request::Subscribe { source, r#type } => {
                subscribed = true;
                Cmd::Subscribe {
                    id,
                    selector: Selector { source, r#type },
                    tx: out_tx.clone(),
                }
            }
            Request::Health => Cmd::Health {
                reply: out_tx.clone(),
            },
            Request::Query { .. } => Cmd::Query {
                reply: out_tx.clone(),
            },
        };
        if cmd_tx.send(cmd).await.is_err() {
            eprintln!("event=vitalsd.client stage=command result=not_delivered client_id={id}");
            break; // the daemon is shutting down
        }
    }

    if subscribed {
        let _ = cmd_tx.send(Cmd::Unsubscribe { id }).await;
    }
    drop(out_tx);
    let _ = writer.await;
    Ok(())
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    reason = "a failing assertion in a test should panic loudly"
)]
mod tests {
    use super::*;
    use std::fs;

    fn wire_sample(seq: i64) -> WireSample {
        WireSample {
            source: 1,
            r#type: 1,
            session_id: 9,
            seq,
            timestamp_unix_micros: 1_767_225_600_000_000,
            value: 98.0,
            extra: None,
            batch_id: None,
        }
    }

    #[test]
    fn argument_defaults_and_hex_encoding_are_observable() {
        let defaults = parse_args();
        assert_eq!(defaults.socket, PathBuf::from(DEFAULT_SOCKET));
        assert_eq!(defaults.spool, PathBuf::from(DEFAULT_SPOOL));
        assert_eq!(defaults.state_dir, PathBuf::from(DEFAULT_STATE));
        assert_eq!(hex16(&[0xab; 16]), "abababababababababababababababab");
    }

    #[test]
    fn missing_and_malformed_drain_health_are_unavailable() {
        let dir = tempfile::tempdir().expect("tempdir");
        let missing = read_drain_health(dir.path());
        assert!(!missing.available);
        assert!(
            missing
                .last_error
                .as_deref()
                .is_some_and(|detail| detail.contains("read failed"))
        );

        fs::write(
            dir.path().join(vitals_core::spool::DRAIN_HEALTH_FILENAME),
            b"not-json",
        )
        .expect("malformed health");
        let malformed = read_drain_health(dir.path());
        assert!(!malformed.available);
        assert!(
            malformed
                .last_error
                .as_deref()
                .is_some_and(|detail| detail.contains("malformed"))
        );
    }

    #[test]
    fn valid_drain_health_clears_the_logged_blocker() {
        let dir = tempfile::tempdir().expect("tempdir");
        let health = DrainHealth {
            available: true,
            generated_at_unix_secs: unix_seconds(),
            ..DrainHealth::default()
        };
        fs::write(
            dir.path().join(vitals_core::spool::DRAIN_HEALTH_FILENAME),
            serde_json::to_vec(&health).expect("encode health"),
        )
        .expect("write health");
        let mut state = State::new(Vec::new());
        let mut previous = Some("old blocker".to_owned());
        refresh_drain_health(&mut state, dir.path(), &mut previous);
        assert!(state.drain.available);
        assert!(state.drain_blocker().is_none());
        assert_eq!(previous, None);
    }

    #[test]
    fn fan_out_reports_published_and_dropped_separately() {
        let mut state = State::new(Vec::new());
        let selector = Selector {
            source: None,
            r#type: None,
        };
        state.add_subscriber(7, selector);
        let (tx, mut rx) = mpsc::channel(SUBSCRIBER_QUEUE_DEPTH);
        let mut subscribers = vec![(7, tx)];
        let (published, dropped) = fan_out(&[wire_sample(1)], &mut state, &mut subscribers);
        assert_eq!((published, dropped), (1, 0));
        assert!(matches!(rx.try_recv(), Ok(Response::Sample { .. })));

        let (tx, _rx) = mpsc::channel(1);
        tx.try_send(Response::Error {
            kind: ErrorKind::Io,
            detail: "fill".to_owned(),
        })
        .expect("fill subscriber queue");
        let mut subscribers = vec![(7, tx)];
        let (published, dropped) = fan_out(&[wire_sample(2)], &mut state, &mut subscribers);
        assert_eq!((published, dropped), (0, 1));
        assert_eq!(
            state
                .health(Instant::now())
                .subscribers
                .first()
                .map(|subscriber| subscriber.dropped),
            Some(1)
        );
    }

    #[test]
    fn wire_time_survives_the_conversion_in_both_directions() {
        let w = WireSample {
            source: 1,
            r#type: 1,
            session_id: 1,
            seq: 1,
            timestamp_unix_micros: 1_767_225_600_123_456,
            value: 1.0,
            extra: None,
            batch_id: None,
        };
        let s = to_sample(&w);
        let back = s
            .timestamp
            .duration_since(UNIX_EPOCH)
            .expect("after epoch")
            .as_micros();
        assert_eq!(back, 1_767_225_600_123_456);
    }

    /// Times before 1970 are legal. `unsigned_abs` on a negative i64 plus the
    /// wrong branch would place them decades in the future instead.
    #[test]
    fn pre_epoch_wire_times_go_backwards_not_forwards() {
        let w = WireSample {
            source: 1,
            r#type: 1,
            session_id: 1,
            seq: 1,
            timestamp_unix_micros: -1_000_000,
            value: 1.0,
            extra: None,
            batch_id: None,
        };
        let s = to_sample(&w);
        assert!(s.timestamp < UNIX_EPOCH);
        assert_eq!(
            UNIX_EPOCH
                .duration_since(s.timestamp)
                .expect("before epoch")
                .as_micros(),
            1_000_000
        );
    }
}
