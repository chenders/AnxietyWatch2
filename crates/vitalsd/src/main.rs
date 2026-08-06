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

mod state;

use state::{SUBSCRIBER_QUEUE_DEPTH, Selector, State};
use std::path::PathBuf;
use std::time::{Duration, Instant, UNIX_EPOCH};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc;
use vitals_core::{NodeId, Sample, Spool};
use vitals_proto::{
    ErrorKind, Request, Response, WireSample, check_frame_len, decode_payload, encode_frame,
};

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

fn main() -> std::io::Result<()> {
    let args = parse_args();

    // Current-thread on purpose. See the module docs.
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let local = tokio::task::LocalSet::new();
    local.block_on(&rt, run(args))
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

    eprintln!(
        "vitalsd: listening on {} · spool {} · node {}",
        args.socket.display(),
        args.spool.display(),
        hex16(node_id.as_bytes())
    );

    let mut tick = tokio::time::interval(Duration::from_millis(250));
    let mut rotation_blocker: Option<String> = None;

    loop {
        tokio::select! {
            accepted = listener.accept() => {
                let (stream, _) = accepted?;
                next_id += 1;
                let id = next_id;
                let tx = cmd_tx.clone();
                tokio::task::spawn_local(async move {
                    if let Err(e) = serve_client(stream, id, tx).await {
                        eprintln!("vitalsd: client {id} ended: {e}");
                    }
                });
            }

            Some(cmd) = cmd_rx.recv() => {
                handle(
                    cmd,
                    &mut st,
                    &mut spool,
                    &node_id,
                    &mut subscribers,
                    rotation_blocker.as_deref(),
                );
            }

            _ = tick.tick() => {
                // Bounds unsynced data at SYNC_INTERVAL even when no append
                // arrives to trigger the check.
                if let Err(e) = spool.sync() {
                    eprintln!("vitalsd: periodic sync failed: {e}");
                }
                match spool.rotate_if_due() {
                    Ok(true) => {
                        eprintln!("vitalsd: rotated to {}", spool.segment_path().display());
                        rotation_blocker = None;
                    }
                    Ok(false) => {}
                    Err(e) => {
                        let blocker = format!("spool rotation blocked: {e}");
                        if rotation_blocker.as_deref() != Some(&blocker) {
                            eprintln!("vitalsd: {blocker}");
                        }
                        rotation_blocker = Some(blocker);
                    },
                }
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

fn handle(
    cmd: Cmd,
    st: &mut State,
    spool: &mut Spool,
    node_id: &NodeId,
    subscribers: &mut Vec<(u64, mpsc::Sender<Response>)>,
    rotation_blocker: Option<&str>,
) {
    let now = Instant::now();
    match cmd {
        Cmd::Append {
            samples,
            sync,
            reply,
        } => {
            let rows: Vec<Sample> = samples.iter().map(to_sample).collect();
            let resp = match spool.append(&rows, node_id, sync) {
                Ok(()) => {
                    st.record_appended(&samples, spool.segment_bytes(), now);
                    fan_out(&samples, st, subscribers);
                    Response::Appended {
                        count: samples.len(),
                        durable: sync,
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::StorageFull && !sync => {
                    // Streaming sources may continue to the in-memory alarm
                    // path. Durability-required BP requests never enter this
                    // arm and receive an explicit error instead.
                    st.record_appended(&samples, spool.segment_bytes(), now);
                    fan_out(&samples, st, subscribers);
                    Response::Appended {
                        count: samples.len(),
                        durable: false,
                    }
                }
                Err(e) => Response::Error {
                    kind: ErrorKind::Io,
                    detail: format!("spool append failed: {e}"),
                },
            };
            let _ = reply.try_send(resp);
        }
        Cmd::Subscribe { id, selector, tx } => {
            st.add_subscriber(id, selector);
            subscribers.push((id, tx));
        }
        Cmd::Unsubscribe { id } => {
            st.remove_subscriber(id);
            subscribers.retain(|(sid, _)| *sid != id);
        }
        Cmd::Health { reply } => {
            let mut health = st.health(now);
            update_drain_health(&mut health, spool);
            if let Some(blocker) = rotation_blocker {
                health.degraded.push(blocker.to_owned());
            }
            health.heartbeat.drain_progressing =
                health.lag_bytes == 0 || health.lag_age_seconds <= 300;
            health.heartbeat_pulsing = health
                .heartbeat
                .should_pulse(state::HEARTBEAT_MAX_AGE.as_secs());
            if !health.heartbeat.drain_progressing {
                health
                    .degraded
                    .push("heartbeat blocked: spool drain offset is not advancing".to_owned());
            }
            let _ = reply.try_send(Response::Health {
                health: Box::new(health),
            });
        }
        Cmd::Query { reply } => {
            // An empty result set and "there is no store yet" are different
            // claims, and answering the first would be a lie.
            let _ = reply.try_send(Response::Error {
                kind: ErrorKind::Unavailable,
                detail: "query needs the PostgreSQL store, which is gated on G3 \
                         (a verified restore). Blocked by: T-C1, T-C2, T-C3."
                    .to_owned(),
            });
        }
    }
}

fn update_drain_health(health: &mut vitals_proto::Health, spool: &Spool) {
    let Some(dir) = spool.segment_path().parent() else {
        health
            .degraded
            .push("drain liveness unavailable: spool has no parent directory".to_owned());
        return;
    };
    let now = std::time::SystemTime::now();
    let mut oldest = 0_u64;
    let mut lag = 0_u64;
    let mut advanced = None;
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) => {
            health.degraded.push(format!(
                "drain liveness unavailable: cannot read {}: {e}",
                dir.display()
            ));
            return;
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|x| x.to_str()) != Some("spool")
            || path == spool.segment_path()
        {
            continue;
        }
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        let mut offset = 37_u64;
        let offset_path = path.with_extension("offset");
        if let Ok(text) = std::fs::read_to_string(&offset_path)
            && let Ok(value) = text.trim().parse::<u64>()
        {
            offset = value;
        }
        lag = lag.saturating_add(meta.len().saturating_sub(offset));
        if let Ok(modified) = meta.modified() {
            let age = now.duration_since(modified).map_or(0, |d| d.as_secs());
            oldest = oldest.max(age);
        }
        if let Ok(meta) = std::fs::metadata(offset_path)
            && let Ok(modified) = meta
                .modified()
                .and_then(|t| t.duration_since(UNIX_EPOCH).map_err(std::io::Error::other))
        {
            advanced = Some(advanced.map_or(modified.as_secs(), |prior: u64| {
                prior.max(modified.as_secs())
            }));
        }
    }
    health.lag_bytes = lag;
    health.lag_age_seconds = oldest;
    health.last_advanced_at = advanced;
    if lag > 0 && oldest > 300 {
        health.degraded.push(format!("spool drain stalled: lag_bytes={lag}, lag_age_seconds={oldest}, last_advanced_at={advanced:?}; check PostgreSQL reachability, advisory lock, and schema fingerprint"));
    }
}

/// Fan out to subscribers without ever blocking the writer.
fn fan_out(
    samples: &[WireSample],
    st: &mut State,
    subscribers: &mut [(u64, mpsc::Sender<Response>)],
) {
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
                st.record_drop(id, 1, SUBSCRIBER_QUEUE_DEPTH);
            }
        }
    }
}

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
                    if wr.write_all(&bytes).await.is_err() {
                        break;
                    }
                }
                Err(e) => eprintln!("vitalsd: encode failed: {e}"),
            }
        }
    });

    let mut subscribed = false;
    loop {
        let mut len_buf = [0u8; 4];
        if rd.read_exact(&mut len_buf).await.is_err() {
            break; // clean disconnect
        }
        let len = match check_frame_len(u32::from_le_bytes(len_buf)) {
            Ok(n) => n,
            Err(e) => {
                let _ = out_tx.try_send(Response::Error {
                    kind: ErrorKind::Malformed,
                    detail: e.to_string(),
                });
                break; // the stream is desynchronised; there is no resync point
            }
        };
        let mut payload = vec![0u8; len];
        if rd.read_exact(&mut payload).await.is_err() {
            break;
        }

        let req: Request = match decode_payload(&payload) {
            Ok(r) => r,
            Err(e) => {
                let _ = out_tx.try_send(Response::Error {
                    kind: ErrorKind::Malformed,
                    detail: e.to_string(),
                });
                continue; // framing is still intact, so the connection survives
            }
        };

        let cmd = match req {
            Request::Append { samples, sync } => Cmd::Append {
                samples,
                sync,
                reply: out_tx.clone(),
            },
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

    #[test]
    fn wire_time_survives_the_conversion_in_both_directions() {
        let w = WireSample {
            source: 1,
            r#type: 1,
            session_id: 1,
            seq: 1,
            timestamp_unix_micros: 1_785_783_900_123_456,
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
        assert_eq!(back, 1_785_783_900_123_456);
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
