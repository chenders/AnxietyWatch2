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
    let _ = write!(o, ",\"total_samples\":{}", h.total_samples);
    let _ = write!(o, ",\"spool_bytes\":{}", h.spool_bytes);
    let _ = write!(o, ",\"lag_bytes\":{}", h.lag_bytes);
    let _ = write!(o, ",\"lag_age_seconds\":{}", h.lag_age_seconds);
    let advanced = h
        .last_advanced_at
        .map_or_else(|| "null".to_owned(), |v| v.to_string());
    let _ = write!(o, ",\"last_advanced_at\":{advanced}");
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
