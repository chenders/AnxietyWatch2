//! Generates the isolated COPY-binary segment used by the Pi crash test.

use std::env;
use std::path::Path;
use std::process::ExitCode;
use std::time::{Duration, UNIX_EPOCH};
use vitals_core::{NodeId, Sample, Spool};

fn run_with_args(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let dir = args
        .get(1)
        .ok_or("usage: vitals-spool-fixture DIR [ROWS]")?;
    let count = args.get(2).map_or(Ok(10_000_u64), |s| s.parse::<u64>())?;
    let mut spool = Spool::open(Path::new(dir))?;
    let node = NodeId::from_bytes([0xD5; 16]);
    for seq in 0..count {
        let sample = Sample {
            source: 901_005,
            r#type: 902_005,
            session_id: 9_005_000_000,
            seq: i64::try_from(seq)?,
            timestamp: UNIX_EPOCH + Duration::from_secs(1_800_000_000 + seq),
            value: 95.0 + f64::from(u32::try_from(seq % 5)?),
            extra: Some(seq.to_be_bytes().to_vec()),
            batch_id: None,
        };
        spool.append(&[sample], &node, false)?;
    }
    spool.sync()?;
    println!("{}", spool.segment_path().display());
    Ok(())
}

fn main_with_args(args: &[String]) -> ExitCode {
    match run_with_args(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("vitals-spool-fixture: {e}");
            ExitCode::FAILURE
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
    reason = "fixture tests should fail loudly when a spool invariant breaks"
)]
mod tests {
    use super::*;
    use std::fs;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn rejects_missing_and_invalid_arguments() {
        assert!(run_with_args(&args(&["fixture"])).is_err());
        assert!(run_with_args(&args(&["fixture", "/tmp", "not-a-number"])).is_err());
        let dir = tempfile::tempdir().expect("missing spool parent");
        let file = dir.path().join("not-a-directory");
        fs::write(&file, b"not a spool directory").expect("spool marker");
        assert!(run_with_args(&args(&["fixture", file.to_str().expect("utf8")])).is_err());
    }

    #[test]
    fn writes_the_requested_fixture_and_reports_a_segment() {
        let dir = tempfile::tempdir().expect("fixture tempdir");
        let fixture_args = args(&["fixture", dir.path().to_str().expect("utf8"), "2"]);
        assert_eq!(main_with_args(&fixture_args), ExitCode::SUCCESS);
        let spool = fs::read_dir(dir.path())
            .expect("list spool")
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .find(|path| path.extension().is_some_and(|ext| ext == "spool"))
            .expect("segment exists");
        assert!(fs::metadata(spool).expect("segment stat").len() > 37);
    }

    #[test]
    fn main_reports_an_argument_failure() {
        assert_eq!(main_with_args(&args(&["fixture"])), ExitCode::FAILURE);
        // Calling the real entry point is safe: it returns an ExitCode rather
        // than terminating the test process, and exercises env::args wiring.
        assert_eq!(main(), ExitCode::FAILURE);
    }
}
