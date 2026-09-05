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

fn validate_live_fingerprint(
    live: vitals_core::spool::SchemaFingerprint,
) -> Result<vitals_core::spool::SchemaFingerprint, DrainError> {
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

trait DrainSession {
    fn verify_schema(&mut self) -> Result<vitals_core::spool::SchemaFingerprint, DrainError>;

    fn drain_segments(
        &mut self,
        spool: &Path,
        live: vitals_core::spool::SchemaFingerprint,
        verify_only: bool,
        health: &mut DrainHealth,
    ) -> Result<(), DrainError>;
}

trait RunEnvironment {
    fn connect(&mut self, dsn: &str) -> Result<Box<dyn DrainSession>, DrainError>;
    fn refresh_health(&mut self, spool: &Path, health: &mut DrainHealth) -> Result<(), DrainError>;
    fn sleep(&mut self, duration: Duration);
}

struct PostgresSession {
    client: postgres::Client,
}

impl DrainSession for PostgresSession {
    fn verify_schema(&mut self) -> Result<vitals_core::spool::SchemaFingerprint, DrainError> {
        validate_live_fingerprint(live_fingerprint(&mut self.client)?)
    }

    fn drain_segments(
        &mut self,
        spool: &Path,
        live: vitals_core::spool::SchemaFingerprint,
        verify_only: bool,
        health: &mut DrainHealth,
    ) -> Result<(), DrainError> {
        process_segments(spool, live, verify_only, health, |segment, live| {
            segment.drain(&mut self.client, live)
        })
    }
}

struct PostgresEnvironment;

impl RunEnvironment for PostgresEnvironment {
    fn connect(&mut self, dsn: &str) -> Result<Box<dyn DrainSession>, DrainError> {
        Ok(Box::new(PostgresSession {
            client: connect_locked(dsn)?,
        }))
    }

    fn refresh_health(&mut self, spool: &Path, health: &mut DrainHealth) -> Result<(), DrainError> {
        refresh_health(spool, health)
    }

    fn sleep(&mut self, duration: Duration) {
        thread::sleep(duration);
    }
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

fn run_with(
    args: &Args,
    health: &mut DrainHealth,
    environment: &mut dyn RunEnvironment,
) -> Result<(), DrainError> {
    eprintln!(
        "event=drainer.start spool={} verify={} once={}",
        args.spool.display(),
        args.verify,
        args.once
    );
    // Hold the PostgreSQL advisory lock for the lifetime of a healthy daemon,
    // including idle periods. Reconnect only after a database/drain failure.
    let mut session = match environment.connect(&args.dsn) {
        Ok(session) => session,
        Err(error @ DrainError::LockHeld) => return Err(error),
        Err(error) => {
            publish_failure(&args.spool, health, &error);
            return Err(error);
        }
    };
    let live = match session.verify_schema() {
        Ok(live) => live,
        Err(error) => {
            publish_failure(&args.spool, health, &error);
            return Err(error);
        }
    };
    loop {
        if let Err(error) = session.drain_segments(&args.spool, live, args.verify, health) {
            publish_failure(&args.spool, health, &error);
            return Err(error);
        }
        if let Err(error) = environment.refresh_health(&args.spool, health) {
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
        environment.sleep(Duration::from_secs(1));
    }
}

fn run(args: &Args, health: &mut DrainHealth) -> Result<(), DrainError> {
    let mut environment = PostgresEnvironment;
    run_with(args, health, &mut environment)
}

fn main_with_runner<R, S>(raw: &[String], mut run_once: R, mut sleep: S) -> ExitCode
where
    R: FnMut(&Args, &mut DrainHealth) -> Result<(), DrainError>,
    S: FnMut(Duration),
{
    let args = match parse_args(raw) {
        Ok(args) => args,
        Err(e) => {
            eprintln!("event=drainer.failure stage=argument_parse detail={e:?}");
            return ExitCode::FAILURE;
        }
    };
    let mut health = DrainHealth::default();
    loop {
        match run_once(&args, &mut health) {
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
                sleep(Duration::from_secs(30));
            }
        }
    }
}

fn main_with_args(raw: &[String]) -> ExitCode {
    main_with_runner(raw, run, thread::sleep)
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
    use std::collections::VecDeque;
    use std::io;

    struct FakeSession {
        verify: Option<Result<vitals_core::spool::SchemaFingerprint, DrainError>>,
        drains: VecDeque<Result<(), DrainError>>,
    }

    impl DrainSession for FakeSession {
        fn verify_schema(&mut self) -> Result<vitals_core::spool::SchemaFingerprint, DrainError> {
            self.verify.take().unwrap_or_else(|| {
                Err(DrainError::Io(io::Error::other(
                    "fake session was asked to verify twice",
                )))
            })
        }

        fn drain_segments(
            &mut self,
            _spool: &Path,
            _live: vitals_core::spool::SchemaFingerprint,
            _verify_only: bool,
            _health: &mut DrainHealth,
        ) -> Result<(), DrainError> {
            self.drains.pop_front().unwrap_or_else(|| {
                Err(DrainError::Io(io::Error::other(
                    "fake session has no drain result",
                )))
            })
        }
    }

    struct FakeEnvironment {
        connect: Option<Result<FakeSession, DrainError>>,
        refresh: VecDeque<Result<(), DrainError>>,
        sleeps: Vec<Duration>,
    }

    impl RunEnvironment for FakeEnvironment {
        fn connect(&mut self, _dsn: &str) -> Result<Box<dyn DrainSession>, DrainError> {
            self.connect
                .take()
                .ok_or_else(|| DrainError::Io(io::Error::other("fake connect called twice")))?
                .map(|session| Box::new(session) as Box<dyn DrainSession>)
        }

        fn refresh_health(
            &mut self,
            _spool: &Path,
            _health: &mut DrainHealth,
        ) -> Result<(), DrainError> {
            self.refresh.pop_front().unwrap_or_else(|| {
                Err(DrainError::Io(io::Error::other(
                    "fake environment has no refresh result",
                )))
            })
        }

        fn sleep(&mut self, duration: Duration) {
            self.sleeps.push(duration);
        }
    }

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

    #[expect(
        clippy::unnecessary_wraps,
        reason = "the callback must match process_segments' fallible drain boundary"
    )]
    fn mark_drained(
        segment: &mut vitals_drain::Segment,
        _live: vitals_core::spool::SchemaFingerprint,
    ) -> Result<vitals_drain::DrainOutcome, DrainError> {
        let offset = segment.length;
        segment.offset = offset;
        Ok(vitals_drain::DrainOutcome {
            rows_read: 2,
            rows_inserted: 1,
            rows_duplicate: 1,
            rows_rejected: 0,
            offset,
        })
    }

    fn no_wait(_duration: Duration) {}

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
        process_segments(dir.path(), live, true, &mut verify_health, mark_drained)
            .expect("verify segment");
        assert_eq!(verify_health.rows_read, 0);
        assert!(segment_path.is_file());

        let mut health = DrainHealth::default();
        process_segments(dir.path(), live, false, &mut health, mark_drained)
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
            mark_drained,
        )
        .expect("active segment remains pending");
        assert_eq!(health.rows_read, 0);
        let verify_error = process_segments(
            dir.path(),
            expected_fingerprint(),
            true,
            &mut health,
            mark_drained,
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
        let error = process_segments(dir.path(), wrong, true, &mut health, mark_drained)
            .expect_err("wrong fingerprint");
        assert!(matches!(error, DrainError::FingerprintMismatch { .. }));
    }

    #[test]
    fn process_segments_propagates_non_lock_open_and_drain_errors() {
        let invalid_dir = tempfile::tempdir().expect("invalid segment tempdir");
        std::fs::write(invalid_dir.path().join("bad.spool"), b"not a segment")
            .expect("invalid segment");
        let mut health = DrainHealth::default();
        let error = process_segments(
            invalid_dir.path(),
            expected_fingerprint(),
            false,
            &mut health,
            mark_drained,
        )
        .expect_err("invalid framing");
        assert!(matches!(error, DrainError::Io(_)));

        let drain_dir = tempfile::tempdir().expect("drain error tempdir");
        let segment_path = make_segment(drain_dir.path());
        let mut drain_health = DrainHealth::default();
        let error = process_segments(
            drain_dir.path(),
            expected_fingerprint(),
            false,
            &mut drain_health,
            |_segment, _live| Err(DrainError::Io(io::Error::other("fake COPY failed"))),
        )
        .expect_err("drain callback failure");
        assert!(error.to_string().contains("fake COPY failed"));
        assert!(segment_path.exists());
    }

    #[test]
    fn production_environment_delegates_health_and_zero_duration_sleep() {
        let dir = tempfile::tempdir().expect("health tempdir");
        let mut environment = PostgresEnvironment;
        let mut health = DrainHealth::default();
        environment
            .refresh_health(dir.path(), &mut health)
            .expect("refresh health");
        assert!(health.available);
        environment.sleep(Duration::ZERO);
    }

    #[test]
    fn fake_boundaries_fail_loudly_when_a_result_is_missing() {
        let mut session = FakeSession {
            verify: None,
            drains: VecDeque::new(),
        };
        assert!(session.verify_schema().is_err());
        assert!(
            session
                .drain_segments(
                    Path::new("/tmp"),
                    expected_fingerprint(),
                    false,
                    &mut DrainHealth::default(),
                )
                .is_err()
        );

        let mut environment = FakeEnvironment {
            connect: None,
            refresh: VecDeque::new(),
            sleeps: Vec::new(),
        };
        assert!(environment.connect("unused").is_err());
        assert!(
            environment
                .refresh_health(Path::new("/tmp"), &mut DrainHealth::default())
                .is_err()
        );
    }

    fn fake_args(spool: &Path, once: bool) -> Args {
        Args {
            dsn: "fake-database".to_owned(),
            spool: spool.to_path_buf(),
            verify: false,
            once,
        }
    }

    fn fake_session(drains: impl IntoIterator<Item = Result<(), DrainError>>) -> FakeSession {
        FakeSession {
            verify: Some(Ok(expected_fingerprint())),
            drains: drains.into_iter().collect(),
        }
    }

    #[test]
    fn validates_live_schema_fingerprint_without_a_database() {
        assert_eq!(
            validate_live_fingerprint(expected_fingerprint()).expect("matching schema"),
            expected_fingerprint()
        );
        let wrong = vitals_core::spool::SchemaFingerprint::from_value(
            expected_fingerprint().value().wrapping_add(1),
        );
        let error = validate_live_fingerprint(wrong).expect_err("mismatched schema");
        assert!(error.to_string().contains("live samples table differs"));
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
    fn run_with_preserves_lock_held_as_a_caller_decision() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut environment = FakeEnvironment {
            connect: Some(Err(DrainError::LockHeld)),
            refresh: VecDeque::new(),
            sleeps: Vec::new(),
        };
        let mut health = DrainHealth::default();
        let error = run_with(&fake_args(dir.path(), true), &mut health, &mut environment)
            .expect_err("lock must stop this run");
        assert!(matches!(error, DrainError::LockHeld));
        assert!(
            !dir.path()
                .join(vitals_drain::DRAIN_HEALTH_FILENAME)
                .exists()
        );
    }

    #[test]
    fn run_with_publishes_connect_and_schema_failures() {
        let connect_dir = tempfile::tempdir().expect("connect tempdir");
        let mut connect_environment = FakeEnvironment {
            connect: Some(Err(DrainError::Io(io::Error::other("database offline")))),
            refresh: VecDeque::new(),
            sleeps: Vec::new(),
        };
        let mut health = DrainHealth::default();
        assert!(
            run_with(
                &fake_args(connect_dir.path(), true),
                &mut health,
                &mut connect_environment,
            )
            .is_err()
        );
        assert!(!health.available);
        assert!(
            connect_dir
                .path()
                .join(vitals_drain::DRAIN_HEALTH_FILENAME)
                .is_file()
        );

        let schema_dir = tempfile::tempdir().expect("schema tempdir");
        let mut schema_environment = FakeEnvironment {
            connect: Some(Ok(FakeSession {
                verify: Some(Err(DrainError::SchemaMismatch(
                    "value field changed".to_owned(),
                ))),
                drains: VecDeque::new(),
            })),
            refresh: VecDeque::new(),
            sleeps: Vec::new(),
        };
        let mut schema_health = DrainHealth::default();
        assert!(
            run_with(
                &fake_args(schema_dir.path(), true),
                &mut schema_health,
                &mut schema_environment,
            )
            .is_err()
        );
        assert!(
            schema_health
                .last_error
                .as_deref()
                .is_some_and(|detail| detail.contains("value field changed"))
        );
    }

    #[test]
    fn run_with_completes_a_one_shot_without_postgres() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut environment = FakeEnvironment {
            connect: Some(Ok(fake_session([Ok(())]))),
            refresh: VecDeque::from([Ok(())]),
            sleeps: Vec::new(),
        };
        let mut health = DrainHealth::default();
        run_with(&fake_args(dir.path(), true), &mut health, &mut environment)
            .expect("one-shot run");
        assert!(environment.sleeps.is_empty());
    }

    #[test]
    fn run_with_reports_drain_and_refresh_errors_and_never_waits_in_tests() {
        let drain_dir = tempfile::tempdir().expect("drain tempdir");
        let mut drain_environment = FakeEnvironment {
            connect: Some(Ok(fake_session([
                Ok(()),
                Err(DrainError::Io(io::Error::other("COPY failed"))),
            ]))),
            refresh: VecDeque::from([Ok(())]),
            sleeps: Vec::new(),
        };
        let mut drain_health = DrainHealth::default();
        assert!(
            run_with(
                &fake_args(drain_dir.path(), false),
                &mut drain_health,
                &mut drain_environment,
            )
            .is_err()
        );
        assert_eq!(drain_environment.sleeps, [Duration::from_secs(1)]);
        assert!(
            drain_health
                .last_error
                .as_deref()
                .is_some_and(|detail| detail.contains("COPY failed"))
        );

        let refresh_dir = tempfile::tempdir().expect("refresh tempdir");
        let mut refresh_environment = FakeEnvironment {
            connect: Some(Ok(fake_session([Ok(())]))),
            refresh: VecDeque::from([Err(DrainError::Io(io::Error::other(
                "health filesystem failed",
            )))]),
            sleeps: Vec::new(),
        };
        let mut refresh_health_state = DrainHealth::default();
        assert!(
            run_with(
                &fake_args(refresh_dir.path(), true),
                &mut refresh_health_state,
                &mut refresh_environment,
            )
            .is_err()
        );
        assert!(
            refresh_health_state
                .last_error
                .as_deref()
                .is_some_and(|detail| detail.contains("health filesystem failed"))
        );
    }

    #[test]
    fn main_runner_handles_lock_modes_and_one_shot_failure() {
        assert_eq!(
            main_with_runner(
                &strings(&["drain", "--verify"]),
                |_args, _health| Err(DrainError::LockHeld),
                no_wait,
            ),
            ExitCode::FAILURE
        );

        assert_eq!(
            main_with_runner(
                &strings(&["drain"]),
                |_args, _health| Err(DrainError::LockHeld),
                no_wait,
            ),
            ExitCode::SUCCESS
        );

        assert_eq!(
            main_with_runner(
                &strings(&["drain", "--once"]),
                |_args, _health| Err(DrainError::Io(io::Error::other("one-shot down"))),
                no_wait,
            ),
            ExitCode::FAILURE
        );
    }

    #[test]
    fn main_runner_retries_without_sleeping_for_thirty_seconds() {
        let mut attempts = 0;
        let mut waits = Vec::new();
        let result = main_with_runner(
            &strings(&["drain"]),
            |_args, _health| {
                attempts += 1;
                if attempts == 1 {
                    Err(DrainError::Io(io::Error::other("temporary outage")))
                } else {
                    Ok(())
                }
            },
            |duration| waits.push(duration),
        );
        assert_eq!(result, ExitCode::SUCCESS);
        assert_eq!(attempts, 2);
        assert_eq!(waits, [Duration::from_secs(30)]);
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
