//! Embeds factual build identity and the Sqitch plan size in the console binary.

use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    println!("cargo:rerun-if-changed=../../.git/refs");
    println!("cargo:rerun-if-changed=../../db/sqitch.plan");

    let commit = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "UNKNOWN".to_string());
    println!("cargo:rustc-env=VITALS_BUILD_COMMIT={commit}");

    let build_epoch_s = SystemTime::now().duration_since(UNIX_EPOCH).map_or_else(
        |_| "UNKNOWN".to_string(),
        |duration| duration.as_secs().to_string(),
    );
    println!("cargo:rustc-env=VITALS_BUILD_EPOCH_S={build_epoch_s}");

    let planned_changes = fs::read_to_string("../../db/sqitch.plan")
        .ok()
        .map_or(0, |plan| {
            plan.lines()
                .filter(|line| {
                    let trimmed = line.trim();
                    !trimmed.is_empty() && !trimmed.starts_with('%') && !trimmed.starts_with('#')
                })
                .count()
        });
    println!("cargo:rustc-env=VITALS_PLANNED_SCHEMA_CHANGES={planned_changes}");
}
