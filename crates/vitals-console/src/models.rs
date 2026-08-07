use serde::Serialize;
use vitals_proto::Liveness;

#[derive(Serialize)]
pub struct SourceHealthView {
    pub id: i32,
    pub name: String,
    pub state: String,
    pub reason: String,
    pub degrade_deadline_epoch_s: Option<i64>,
    pub observation_age_s: Option<u64>,
    pub raw_liveness: Option<Liveness>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DaemonReachable {
    Reachable,
    Unreachable(String),
}

#[derive(Serialize)]
pub struct HealthView {
    pub vitalsd: DaemonReachable,
    pub uptime: Option<u64>,
    pub degraded: Vec<String>,
    pub heartbeat_pulsing: Option<bool>,
    pub sources: Vec<SourceHealthView>,
}

#[derive(Serialize)]
pub struct VersionView {
    pub version: &'static str,
    pub git: Option<&'static str>,
    pub note: &'static str,
}
