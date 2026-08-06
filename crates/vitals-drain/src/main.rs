//! Long-running `PostgreSQL` COPY-binary spool drainer.

use std::env;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::thread;
use std::time::Duration;
use vitals_drain::{
    DrainError, connect_locked, discover, expected_fingerprint, live_fingerprint, open_segment,
};

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
            let inserted = segment.drain(client, live)?;
            eprintln!(
                "vitals-spool-drain: segment={} offset={} inserted={inserted}",
                segment.name, segment.length
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

fn run(args: &Args) -> Result<(), DrainError> {
    // Hold the PostgreSQL advisory lock for the lifetime of a healthy daemon,
    // including idle periods. Reconnect only after a database/drain failure.
    let mut client = connect_locked(&args.dsn)?;
    let live = verify_live(&mut client)?;
    loop {
        process_segments(&mut client, &args.spool, live, args.verify)?;
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
            eprintln!("vitals-spool-drain: {e}");
            return ExitCode::FAILURE;
        }
    };
    loop {
        match run(&args) {
            Ok(()) => return ExitCode::SUCCESS,
            Err(DrainError::LockHeld) if args.verify => {
                eprintln!(
                    "vitals-spool-drain: verify blocked: advisory lock held by active drainer"
                );
                return ExitCode::FAILURE;
            }
            Err(DrainError::LockHeld) => {
                eprintln!("vitals-spool-drain: advisory lock held; another drainer is active");
                return ExitCode::SUCCESS;
            }
            Err(e) if args.once => {
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
