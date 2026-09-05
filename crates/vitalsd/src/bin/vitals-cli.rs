//! `vitals-cli` — talk to `vitalsd` from a shell.
//!
//! Exists because a daemon with no client cannot be checked in production, and
//! because T-D4's acceptance is `vitals-cli health | jq -e '.uptime_secs > 0'`.
//! Output is JSON so `jq` can gate on it.
//!
//! ```sh
//! vitals-cli health
//! vitals-cli subscribe            # streams until interrupted
//! vitals-cli --socket /tmp/s health
//! ```

use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::ExitCode;
use vitals_proto::{Request, Response, check_frame_len, decode_payload, encode_frame};

const DEFAULT_SOCKET: &str = "/run/vitals/vitalsd.sock";

fn usage() -> ExitCode {
    eprintln!(
        "usage: vitals-cli [--socket PATH] <health|subscribe>\n\
         \n\
         health     print the daemon's self-report as JSON and exit\n\
         subscribe  stream matching samples as JSON lines until interrupted"
    );
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let mut socket = PathBuf::from(DEFAULT_SOCKET);
    let mut verb: Option<String> = None;

    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--socket" => match it.next() {
                Some(v) => socket = PathBuf::from(v),
                None => return usage(),
            },
            "-h" | "--help" => return usage(),
            other => verb = Some(other.to_owned()),
        }
    }

    let Some(verb) = verb else { return usage() };

    match run(&socket, &verb) {
        Ok(code) => code,
        Err(e) => {
            // Name the socket: "connection refused" without a path sends the
            // reader looking in the wrong place.
            eprintln!("vitals-cli: {}: {e}", socket.display());
            ExitCode::FAILURE
        }
    }
}

fn run(socket: &std::path::Path, verb: &str) -> std::io::Result<ExitCode> {
    let req = match verb {
        "health" => Request::Health,
        "subscribe" => Request::Subscribe {
            source: None,
            r#type: None,
        },
        _ => return Ok(usage()),
    };

    let mut s = UnixStream::connect(socket)?;
    let frame = encode_frame(&req).map_err(std::io::Error::other)?;
    s.write_all(&frame)?;

    let streaming = verb == "subscribe";
    loop {
        let mut len = [0u8; 4];
        if s.read_exact(&mut len).is_err() {
            return Ok(if streaming {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            });
        }
        let n = check_frame_len(u32::from_le_bytes(len)).map_err(std::io::Error::other)?;
        let mut payload = vec![0u8; n];
        s.read_exact(&mut payload)?;
        let resp: Response = decode_payload(&payload).map_err(std::io::Error::other)?;

        match &resp {
            Response::Health { health } => {
                println!("{}", health_json(health));
                // A daemon reporting degraded states exits nonzero, so a shell
                // check does not need to parse the JSON to notice trouble.
                return Ok(if health.degraded.is_empty() {
                    ExitCode::SUCCESS
                } else {
                    for d in &health.degraded {
                        eprintln!("degraded: {d}");
                    }
                    ExitCode::FAILURE
                });
            }
            Response::Sample { sample } => {
                println!(
                    "{{\"source\":{},\"type\":{},\"session_id\":{},\"seq\":{},\
                     \"timestamp_unix_micros\":{},\"value\":{}}}",
                    sample.source,
                    sample.r#type,
                    sample.session_id,
                    sample.seq,
                    sample.timestamp_unix_micros,
                    sample.value
                );
            }
            Response::Error { kind, detail } => {
                eprintln!("vitalsd: {kind:?}: {detail}");
                return Ok(ExitCode::FAILURE);
            }
            Response::Appended { .. } => {}
        }

        if !streaming {
            return Ok(ExitCode::SUCCESS);
        }
    }
}

/// Hand-rolled so the CLI needs no serialiser and the field names are visible
/// here rather than derived from a struct three crates away.
fn health_json(h: &vitals_proto::Health) -> String {
    use std::fmt::Write as _;
    let mut o = String::new();
    let _ = write!(o, "{{\"uptime_secs\":{}", h.uptime_secs);
    let _ = write!(o, ",\"received_samples\":{}", h.received_samples);
    let _ = write!(o, ",\"spooled_samples\":{}", h.spooled_samples);
    let _ = write!(o, ",\"fsynced_samples\":{}", h.fsynced_samples);
    let _ = write!(o, ",\"published_samples\":{}", h.published_samples);
    let _ = write!(
        o,
        ",\"displayed_samples\":{}",
        h.displayed_samples
            .map_or_else(|| "null".to_owned(), |count| count.to_string())
    );
    let _ = write!(o, ",\"total_samples\":{}", h.total_samples);
    let _ = write!(o, ",\"spool_bytes\":{}", h.spool_bytes);
    let _ = write!(o, ",\"heartbeat_pulsing\":{}", h.heartbeat_pulsing);
    o.push_str(",\"sources\":[");
    for (i, s) in h.sources.iter().enumerate() {
        if i > 0 {
            o.push(',');
        }
        let age = s
            .last_sample_age_secs
            .map_or_else(|| "null".to_owned(), |a| a.to_string());
        let _ = write!(
            o,
            "{{\"source\":{},\"liveness\":\"{:?}\",\"last_sample_age_secs\":{},\"samples\":{}}}",
            s.source, s.liveness, age, s.samples
        );
    }
    o.push_str("],\"subscribers\":[");
    for (i, s) in h.subscribers.iter().enumerate() {
        if i > 0 {
            o.push(',');
        }
        let _ = write!(
            o,
            "{{\"id\":{},\"dropped\":{},\"queued\":{}}}",
            s.id, s.dropped, s.queued
        );
    }
    o.push_str("],\"degraded\":[");
    for (i, d) in h.degraded.iter().enumerate() {
        if i > 0 {
            o.push(',');
        }
        let _ = write!(o, "\"{}\"", d.replace('\\', "\\\\").replace('"', "\\\""));
    }
    o.push_str("]}");
    o
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    reason = "CLI socket fixtures should fail loudly in tests"
)]
mod tests {
    use super::*;
    use std::os::unix::net::UnixListener;
    use std::path::Path;
    use std::thread;
    use vitals_proto::{DrainHealth, HeartbeatInputs, Liveness, SourceHealth, SubscriberHealth};

    fn health(degraded: Vec<String>) -> vitals_proto::Health {
        vitals_proto::Health {
            uptime_secs: 7,
            received_samples: 3,
            spooled_samples: 2,
            fsynced_samples: 1,
            published_samples: 1,
            displayed_samples: None,
            sources: vec![SourceHealth {
                source: 6,
                liveness: Liveness::Stale,
                last_sample_age_secs: Some(9),
                samples: 2,
            }],
            total_samples: 2,
            spool_bytes: 128,
            subscribers: vec![SubscriberHealth {
                id: 4,
                dropped: 1,
                queued: 2,
            }],
            drain: DrainHealth::default(),
            heartbeat: HeartbeatInputs {
                monitoring_window_active: true,
                primary_sample_age_secs: Some(1),
                tier_evaluation_age_secs: Some(1),
                drain_progressing: true,
            },
            heartbeat_pulsing: false,
            degraded,
        }
    }

    fn server(response: Response) -> (tempfile::TempDir, PathBuf, thread::JoinHandle<()>) {
        let dir = tempfile::tempdir().expect("socket tempdir");
        let path = dir.path().join("vitalsd.sock");
        let listener = UnixListener::bind(&path).expect("bind fixture socket");
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept fixture client");
            let mut prefix = [0_u8; 4];
            stream.read_exact(&mut prefix).expect("read request prefix");
            let n = check_frame_len(u32::from_le_bytes(prefix)).expect("request length");
            let mut body = vec![0_u8; n];
            stream.read_exact(&mut body).expect("read request body");
            stream
                .write_all(&encode_frame(&response).expect("encode fixture response"))
                .expect("write fixture response");
        });
        (dir, path, handle)
    }

    #[test]
    fn health_json_contains_stage_evidence_and_escapes_blockers() {
        let json = health_json(&health(vec![r#"bad "quote"\path"#.to_owned()]));
        assert!(json.contains("\"received_samples\":3"));
        assert!(json.contains("\"spooled_samples\":2"));
        assert!(json.contains("\"fsynced_samples\":1"));
        assert!(json.contains("\"published_samples\":1"));
        assert!(json.contains("\"displayed_samples\":null"));
        assert!(json.contains(r#"bad \"quote\"\\path"#));
        assert!(json.contains("\"liveness\":\"Stale\""));
    }

    #[test]
    fn run_reports_health_and_protocol_outcomes() {
        let (dir, path, thread) = server(Response::Health {
            health: Box::new(health(Vec::new())),
        });
        assert_eq!(
            run(&path, "health").expect("healthy CLI"),
            ExitCode::SUCCESS
        );
        thread.join().expect("fixture thread");
        drop(dir);

        let (dir, path, thread) = server(Response::Health {
            health: Box::new(health(vec!["blocked=1".to_owned()])),
        });
        assert_eq!(
            run(&path, "health").expect("degraded CLI"),
            ExitCode::FAILURE
        );
        thread.join().expect("fixture thread");
        drop(dir);

        let (dir, path, thread) = server(Response::Error {
            kind: vitals_proto::ErrorKind::Unavailable,
            detail: "store missing".to_owned(),
        });
        assert_eq!(
            run(&path, "health").expect("error response"),
            ExitCode::FAILURE
        );
        thread.join().expect("fixture thread");
        drop(dir);

        let (dir, path, thread) = server(Response::Appended {
            count: 1,
            durable: false,
        });
        assert_eq!(
            run(&path, "health").expect("append response"),
            ExitCode::SUCCESS
        );
        thread.join().expect("fixture thread");
        drop(dir);

        assert!(run(Path::new("/no/such/vitalsd.sock"), "health").is_err());
        assert_eq!(
            run(Path::new("/no/socket"), "unknown").expect("unknown verb"),
            usage()
        );
    }

    #[test]
    fn subscribe_streams_samples_and_eof_is_clean() {
        let sample = vitals_proto::WireSample {
            source: 6,
            r#type: 6,
            session_id: 4,
            seq: 2,
            timestamp_unix_micros: 1,
            value: 98.0,
            extra: None,
            batch_id: None,
        };
        let (dir, path, thread) = server(Response::Sample {
            sample: Box::new(sample),
        });
        assert_eq!(
            run(&path, "subscribe").expect("subscribe CLI"),
            ExitCode::SUCCESS
        );
        thread.join().expect("fixture thread");
        drop(dir);
    }

    #[test]
    fn malformed_response_length_is_an_error() {
        let dir = tempfile::tempdir().expect("socket tempdir");
        let path = dir.path().join("vitalsd.sock");
        let listener = UnixListener::bind(&path).expect("bind fixture socket");
        let thread = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept fixture client");
            let mut request = [0_u8; 4];
            stream
                .read_exact(&mut request)
                .expect("read request prefix");
            let n = check_frame_len(u32::from_le_bytes(request)).expect("request length");
            let mut body = vec![0_u8; n];
            stream.read_exact(&mut body).expect("read request body");
            stream
                .write_all(&u32::MAX.to_le_bytes())
                .expect("write bad length");
        });
        assert!(run(&path, "health").is_err());
        thread.join().expect("fixture thread");
    }
}
