//! Long-running `PostgreSQL` COPY-binary spool drainer.

use std::env;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::thread;
use std::time::Duration;
use vitals_drain::{
    DrainError, connect_locked, discover, expected_fingerprint, live_fingerprint, open_segment,
};

fn arg_value(args: &[String], flag: &str, default: &str) -> String {
    args.windows(2)
        .find(|w| w.first().is_some_and(|v| v == flag))
        .and_then(|w| w.get(1))
        .cloned()
        .unwrap_or_else(|| default.to_owned())
}

fn run_once(dsn: &str, spool: &Path, verify_only: bool) -> Result<(), DrainError> {
    let mut client = connect_locked(dsn)?;
    let live = live_fingerprint(&mut client)?;
    let expected = expected_fingerprint();
    if live != expected {
        return Err(DrainError::FingerprintMismatch {
            expected: expected.value(),
            actual: live.value(),
        });
    }
    for path in discover(spool)? {
        let mut segment = match open_segment(&path) {
            Ok(segment) => segment,
            Err(DrainError::Io(e)) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
            Err(e) => return Err(e),
        };
        if segment.fingerprint != live {
            return Err(DrainError::FingerprintMismatch {
                expected: live.value(),
                actual: segment.fingerprint.value(),
            });
        }
        if !verify_only {
            let inserted = segment.drain(&mut client, live)?;
            eprintln!(
                "vitals-spool-drain: segment={} offset={} inserted={inserted}",
                segment.name, segment.length
            );
            segment.remove_if_drained()?;
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<_> = env::args().collect();
    let dsn = arg_value(
        &args,
        "--database",
        "host=/var/run/postgresql port=5432 dbname=vitals user=vitals",
    );
    let spool = PathBuf::from(arg_value(&args, "--spool", "/var/lib/vitals/spool"));
    let verify_only = args.iter().any(|a| a == "--verify");
    let once = verify_only || args.iter().any(|a| a == "--once");
    loop {
        match run_once(&dsn, &spool, verify_only) {
            Ok(()) if once => return ExitCode::SUCCESS,
            Ok(()) => thread::sleep(Duration::from_secs(1)),
            Err(DrainError::LockHeld) if verify_only => {
                eprintln!(
                    "vitals-spool-drain: verify blocked: advisory lock held by active drainer"
                );
                return ExitCode::FAILURE;
            }
            Err(DrainError::LockHeld) => {
                eprintln!("vitals-spool-drain: advisory lock held; another drainer is active");
                return ExitCode::SUCCESS;
            }
            Err(e) if once => {
                eprintln!("vitals-spool-drain: {e}");
                return ExitCode::FAILURE;
            }
            Err(e) => {
                eprintln!(
                    "vitals-spool-drain: {e}; retrying in 30 seconds without advancing offset"
                );
                thread::sleep(Duration::from_secs(30));
            }
        }
    }
}
