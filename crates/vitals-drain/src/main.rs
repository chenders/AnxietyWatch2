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

fn process_segments<F>(
    spool: &Path,
    live: vitals_core::spool::SchemaFingerprint,
    verify_only: bool,
    health: &mut DrainHealth,
    mut drain: F,
) -> Result<(), DrainError>
where
    F: FnMut(
        &mut vitals_drain::Segment,
        vitals_core::spool::SchemaFingerprint,
    ) -> Result<vitals_drain::DrainOutcome, DrainError>,
{
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
            let outcome = drain(&mut segment, live)?;
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
        if let Err(error) =
            process_segments(&args.spool, live, args.verify, health, |segment, live| {
                segment.drain(&mut client, live)
            })
        {
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

fn main_with_args(raw: &[String]) -> ExitCode {
    let args = match parse_args(raw) {
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

fn main() -> ExitCode {
    let raw: Vec<_> = env::args().collect();
    main_with_args(&raw)
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    reason = "drainer tests should fail loudly when a fixture invariant breaks"
)]
mod tests {
    use super::*;

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(ToString::to_string).collect()
    }

    fn make_segment(dir: &Path) -> PathBuf {
        let mut spool = vitals_core::Spool::open(dir).expect("spool");
        let node = vitals_core::NodeId::from_bytes([0xA5; 16]);
        let sample = vitals_core::Sample {
            source: 1,
            r#type: 1,
            session_id: 2,
            seq: 3,
            timestamp: std::time::UNIX_EPOCH,
            value: 96.0,
            extra: None,
            batch_id: None,
        };
        spool.append(&[sample], &node, false).expect("append");
        spool.sync().expect("sync");
        let path = spool.segment_path().to_path_buf();
        drop(spool);
        path
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
        assert!(parse_args(&strings(&["drain", "--spool", "-not-a-path"])).is_err());
    }

    #[test]
    fn verify_also_selects_one_shot_mode() {
        let args = parse_args(&strings(&["drain", "--verify"])).expect("arguments");
        assert!(args.verify);
        assert!(args.once);
    }

    #[test]
    fn publish_failure_keeps_rejection_and_pending_evidence() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut health = DrainHealth::default();
        let error = DrainError::Rejected {
            rows: 3,
            source: Box::new(DrainError::LockHeld),
        };
        publish_failure(dir.path(), &mut health, &error);
        assert!(!health.available);
        assert_eq!(health.rows_rejected, 3);
        assert_eq!(health.pending_segments, 0);
        assert!(health.last_error.is_some());
        let health_path = dir.path().join(vitals_drain::DRAIN_HEALTH_FILENAME);
        let bytes = std::fs::read(health_path).expect("published health");
        let published: DrainHealth = serde_json::from_slice(&bytes).expect("health JSON");
        assert_eq!(published.rows_rejected, 3);
        assert!(!published.available);
    }

    #[test]
    fn publish_failure_names_pending_inspection_blocker() {
        let dir = tempfile::tempdir().expect("tempdir");
        let missing = dir.path().join("missing-spool");
        let mut health = DrainHealth::default();
        let error = DrainError::LockHeld;
        publish_failure(&missing, &mut health, &error);
        assert!(
            health
                .last_error
                .as_deref()
                .is_some_and(|detail| detail.contains("pending inspection failed"))
        );
    }

    #[test]
    fn process_segments_validates_and_drains_without_a_database() {
        let dir = tempfile::tempdir().expect("tempdir");
        let segment_path = make_segment(dir.path());
        let live = expected_fingerprint();
        let mut verify_health = DrainHealth::default();
        let mut called = false;
        process_segments(
            dir.path(),
            live,
            true,
            &mut verify_health,
            |_segment, _live| {
                called = true;
                Ok(vitals_drain::DrainOutcome {
                    rows_read: 0,
                    rows_inserted: 0,
                    rows_duplicate: 0,
                    rows_rejected: 0,
                    offset: 0,
                })
            },
        )
        .expect("verify segment");
        assert!(!called);
        assert!(segment_path.is_file());

        let mut health = DrainHealth::default();
        process_segments(dir.path(), live, false, &mut health, |segment, _live| {
            let offset = segment.length;
            segment.offset = offset;
            Ok(vitals_drain::DrainOutcome {
                rows_read: 2,
                rows_inserted: 1,
                rows_duplicate: 1,
                rows_rejected: 0,
                offset,
            })
        })
        .expect("drain segment");
        assert_eq!(health.rows_read, 2);
        assert_eq!(health.rows_inserted, 1);
        assert_eq!(health.rows_duplicate, 1);
        assert!(health.last_success_unix_secs.is_some());
        assert!(!segment_path.exists());
    }

    #[test]
    fn process_segments_preserves_locked_active_data_and_verify_fails_closed() {
        let dir = tempfile::tempdir().expect("tempdir");
        let _spool = vitals_core::Spool::open(dir.path()).expect("spool");
        let mut health = DrainHealth::default();
        process_segments(
            dir.path(),
            expected_fingerprint(),
            false,
            &mut health,
            |_segment, _live| unreachable!("active segment must not be drained"),
        )
        .expect("active segment remains pending");
        assert_eq!(health.rows_read, 0);
        let verify_error = process_segments(
            dir.path(),
            expected_fingerprint(),
            true,
            &mut health,
            |_segment, _live| unreachable!("locked segment must not be drained"),
        )
        .expect_err("verify must identify the locked segment");
        assert!(
            verify_error
                .to_string()
                .contains("verify could not inspect locked")
        );
    }

    #[test]
    fn process_segments_rejects_a_segment_with_the_wrong_fingerprint() {
        let dir = tempfile::tempdir().expect("tempdir");
        let _segment_path = make_segment(dir.path());
        let mut health = DrainHealth::default();
        let wrong = vitals_core::spool::SchemaFingerprint::from_value(
            expected_fingerprint().value().wrapping_add(1),
        );
        let error = process_segments(dir.path(), wrong, true, &mut health, |_segment, _live| {
            unreachable!("fingerprint mismatch must stop first")
        })
        .expect_err("wrong fingerprint");
        assert!(matches!(error, DrainError::FingerprintMismatch { .. }));
    }

    #[test]
    fn run_publishes_health_when_database_connection_is_blocked() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut health = DrainHealth::default();
        let args = Args {
            dsn: "host=/tmp/vitals-no-postgres port=5432 dbname=missing user=missing".to_owned(),
            spool: dir.path().to_path_buf(),
            verify: false,
            once: true,
        };
        let result = run(&args, &mut health);
        assert!(result.is_err());
        assert!(!health.available);
        assert!(
            dir.path()
                .join(vitals_drain::DRAIN_HEALTH_FILENAME)
                .is_file()
        );
    }

    #[test]
    fn main_reports_argument_and_one_shot_failures_without_retrying() {
        let argument_error = main_with_args(&strings(&["drain", "--unknown"]));
        assert_eq!(argument_error, ExitCode::FAILURE);

        let dir = tempfile::tempdir().expect("tempdir");
        let result = main_with_args(&strings(&[
            "drain",
            "--database",
            "host=/tmp/vitals-no-postgres port=5432 dbname=missing user=missing",
            "--spool",
            dir.path().to_str().expect("UTF-8 temp path"),
            "--once",
        ]));
        assert_eq!(result, ExitCode::FAILURE);
    }
}
