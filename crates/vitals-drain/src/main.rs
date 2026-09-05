//! Long-running `PostgreSQL` COPY-binary spool drainer.

use std::env;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::thread;
use std::time::Duration;
use vitals_drain::{
    DrainError, connect_locked, discover, epoch_seconds, expected_fingerprint, live_fingerprint,
    open_segment, pending_spool, publish_health, refresh_health,
};
use vitals_proto::DrainHealth;

#[derive(Debug, PartialEq, Eq)]
struct Args {
    dsn: String,
    spool: PathBuf,
    verify: bool,
    once: bool,
}

fn parse_args(raw: &[String]) -> Result<Args, String> {
    let mut args = Args {
        dsn: "host=/var/run/postgresql port=5432 dbname=vitals user=vitals".to_owned(),
        spool: PathBuf::from("/var/lib/vitals/spool"),
        verify: false,
        once: false,
    };
    let mut index = 1;
    while let Some(argument) = raw.get(index) {
        match argument.as_str() {
            "--database" | "--spool" => {
                let flag = argument;
                let value = raw
                    .get(index + 1)
                    .ok_or_else(|| format!("{flag} requires a value"))?;
                if value.starts_with('-') {
                    return Err(format!("{flag} requires a value, found {value}"));
                }
                if flag == "--database" {
                    args.dsn.clone_from(value);
                } else {
                    args.spool = PathBuf::from(value);
                }
                index += 2;
            }
            "--verify" => {
                args.verify = true;
                index += 1;
            }
            "--once" => {
                args.once = true;
                index += 1;
            }
            unknown => {
                return Err(format!(
                    "unknown argument {unknown:?}; use --database DSN --spool DIR [--verify|--once] (flag=value is not supported)"
                ));
            }
        }
    }
    args.once |= args.verify;
    Ok(args)
}

fn process_segments(
    client: &mut postgres::Client,
    spool: &Path,
    live: vitals_core::spool::SchemaFingerprint,
    verify_only: bool,
    health: &mut DrainHealth,
) -> Result<(), DrainError> {
    for path in discover(spool)? {
        let mut segment = match open_segment(&path) {
            Ok(segment) => segment,
            Err(DrainError::Io(e)) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if verify_only {
                    return Err(DrainError::Io(std::io::Error::new(
                        std::io::ErrorKind::WouldBlock,
                        format!(
                            "verify could not inspect locked segment {}; stop the writer or rotate it, then retry",
                            path.display()
                        ),
                    )));
                }
                eprintln!(
                    "event=drainer.segment stage=pending reason=locked_active path={}",
                    path.display()
                );
                continue;
            }
            Err(e) => return Err(e),
        };
        if segment.fingerprint != live {
            return Err(DrainError::FingerprintMismatch {
                expected: live.value(),
                actual: segment.fingerprint.value(),
                context: format!(
                    "segment {} predates or differs from the live schema",
                    segment.name
                ),
            });
        }
        if !verify_only {
            let outcome = segment.drain(client, live)?;
            health.rows_read = health.rows_read.saturating_add(outcome.rows_read);
            health.rows_inserted = health.rows_inserted.saturating_add(outcome.rows_inserted);
            health.rows_duplicate = health.rows_duplicate.saturating_add(outcome.rows_duplicate);
            health.rows_rejected = health.rows_rejected.saturating_add(outcome.rows_rejected);
            health.last_success_unix_secs = Some(epoch_seconds());
            eprintln!(
                "event=drainer.segment stage=drained segment={} offset={} rows_read={}",
                segment.name, outcome.offset, outcome.rows_read
            );
            eprintln!(
                "event=drainer.segment stage=committed segment={} offset={} rows_read={} rows_inserted={} rows_duplicate={} rows_rejected={}",
                segment.name,
                outcome.offset,
                outcome.rows_read,
                outcome.rows_inserted,
                outcome.rows_duplicate,
                outcome.rows_rejected
            );
            segment.remove_if_drained()?;
        }
    }
    Ok(())
}

fn verify_live(
    client: &mut postgres::Client,
) -> Result<vitals_core::spool::SchemaFingerprint, DrainError> {
    let live = live_fingerprint(client)?;
    let expected = expected_fingerprint();
    if live != expected {
        return Err(DrainError::FingerprintMismatch {
            expected: expected.value(),
            actual: live.value(),
            context: "live samples table differs from the canonical COPY layout".to_owned(),
        });
    }
    Ok(live)
}

fn publish_failure(spool: &Path, health: &mut DrainHealth, error: &DrainError) {
    let mut detail = error.to_string();
    health.rows_rejected = health.rows_rejected.saturating_add(error.rejected_rows());
    health.schema = vitals_drain::DRAIN_HEALTH_SCHEMA;
    health.available = false;
    health.generated_at_unix_secs = epoch_seconds();
    health.last_error = Some(detail.clone());
    match pending_spool(spool) {
        Ok(pending) => {
            health.pending_segments = pending.segments;
            health.pending_bytes = pending.bytes;
            health.oldest_pending_age_secs = pending.oldest_age_secs;
        }
        Err(pending_error) => {
            detail = format!("{detail}; pending inspection failed: {pending_error}");
            health.last_error = Some(detail.clone());
            eprintln!(
                "event=drainer.failure stage=pending_inspection detail={:?}",
                pending_error.to_string()
            );
        }
    }
    if let Err(publish_error) = publish_health(spool, health) {
        eprintln!(
            "event=drainer.failure stage=health_publish detail={:?}",
            publish_error.to_string()
        );
    }
    eprintln!(
        "event=drainer.failure stage=run detail={:?} pending_segments={} pending_bytes={} rows_rejected={}",
        detail, health.pending_segments, health.pending_bytes, health.rows_rejected
    );
}

fn run(args: &Args, health: &mut DrainHealth) -> Result<(), DrainError> {
    eprintln!(
        "event=drainer.start spool={} verify={} once={}",
        args.spool.display(),
        args.verify,
        args.once
    );
    // Hold the PostgreSQL advisory lock for the lifetime of a healthy daemon,
    // including idle periods. Reconnect only after a database/drain failure.
    let mut client = match connect_locked(&args.dsn) {
        Ok(client) => client,
        Err(error @ DrainError::LockHeld) => return Err(error),
        Err(error) => {
            publish_failure(&args.spool, health, &error);
            return Err(error);
        }
    };
    let live = match verify_live(&mut client) {
        Ok(live) => live,
        Err(error) => {
            publish_failure(&args.spool, health, &error);
            return Err(error);
        }
    };
    loop {
        if let Err(error) = process_segments(&mut client, &args.spool, live, args.verify, health) {
            publish_failure(&args.spool, health, &error);
            return Err(error);
        }
        if let Err(error) = refresh_health(&args.spool, health) {
            publish_failure(&args.spool, health, &error);
            return Err(error);
        }
        eprintln!(
            "event=drainer.health stage=published available=true pending_segments={} pending_bytes={} oldest_age_secs={:?} rows_read={} rows_inserted={} rows_duplicate={} rows_rejected={}",
            health.pending_segments,
            health.pending_bytes,
            health.oldest_pending_age_secs,
            health.rows_read,
            health.rows_inserted,
            health.rows_duplicate,
            health.rows_rejected
        );
        if args.once {
            return Ok(());
        }
        thread::sleep(Duration::from_secs(1));
    }
}

fn main() -> ExitCode {
    let raw: Vec<_> = env::args().collect();
    let args = match parse_args(&raw) {
        Ok(args) => args,
        Err(e) => {
            eprintln!("event=drainer.failure stage=argument_parse detail={e:?}");
            return ExitCode::FAILURE;
        }
    };
    let mut health = DrainHealth::default();
    loop {
        match run(&args, &mut health) {
            Ok(()) => return ExitCode::SUCCESS,
            Err(DrainError::LockHeld) if args.verify => {
                eprintln!(
                    "event=drainer.failure stage=lock detail={:?}",
                    "verify blocked: advisory lock held by active drainer"
                );
                return ExitCode::FAILURE;
            }
            Err(DrainError::LockHeld) => {
                eprintln!(
                    "event=drainer.exit stage=lock detail={:?}",
                    "another drainer is active"
                );
                return ExitCode::SUCCESS;
            }
            Err(e) if args.once => {
                eprintln!("event=drainer.exit stage=once detail={:?}", e.to_string());
                return ExitCode::FAILURE;
            }
            Err(e) => {
                eprintln!(
                    "event=drainer.retry detail={:?} retry_after_secs=30",
                    e.to_string()
                );
                thread::sleep(Duration::from_secs(30));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn parses_only_documented_arguments() {
        let args = parse_args(&strings(&[
            "drain",
            "--database",
            "dbname=x",
            "--spool",
            "/tmp/s",
            "--once",
        ]));
        assert!(args.is_ok());
        let args = args.unwrap_or_else(|_| unreachable!("checked above"));
        assert_eq!(args.dsn, "dbname=x");
        assert_eq!(args.spool, PathBuf::from("/tmp/s"));
        assert!(args.once);
    }

    #[test]
    fn rejects_unknown_equals_and_missing_values() {
        assert!(parse_args(&strings(&["drain", "--bogus"])).is_err());
        assert!(parse_args(&strings(&["drain", "--database=dbname=x"])).is_err());
        assert!(parse_args(&strings(&["drain", "--spool"])).is_err());
    }
}
