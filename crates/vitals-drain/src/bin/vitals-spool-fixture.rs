//! Generates the isolated COPY-binary segment used by the Pi crash test.

use std::env;
use std::path::Path;
use std::process::ExitCode;
use std::time::{Duration, UNIX_EPOCH};
use vitals_core::{NodeId, Sample, Spool};

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args().collect();
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

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("vitals-spool-fixture: {e}");
            ExitCode::FAILURE
        }
    }
}
