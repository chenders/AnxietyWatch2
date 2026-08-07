use serde::Serialize;
use vitals_proto::Liveness;

/// One source's health as rendered by `/api/health`.
#[derive(Serialize)]
pub struct SourceHealthView {
    /// Registry source id.
    pub id: i32,
    /// Source name, as the registry enum's debug name.
    pub name: String,
    /// Derived health state: `Live`, `Failing`, or `Unknown`.
    pub state: String,
    /// Why the state is what it is, naming the value and its threshold.
    pub reason: String,
    /// Unix epoch seconds by which the source must recover or be degraded.
    pub degrade_deadline_epoch_s: Option<i64>,
    /// Seconds since the last sample arrived, or `None` if never seen.
    pub observation_age_s: Option<u64>,
    /// The daemon's own liveness verdict, before local re-evaluation.
    pub raw_liveness: Option<Liveness>,
}

/// Whether the `vitalsd` daemon answered the health request.
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DaemonReachable {
    /// The daemon answered; per-source health follows.
    Reachable,
    /// The daemon did not answer; the string names why.
    Unreachable(String),
}

/// The `/api/health` response body.
#[derive(Serialize)]
pub struct HealthView {
    /// Whether `vitalsd` itself answered.
    pub vitalsd: DaemonReachable,
    /// Daemon uptime in seconds, when reachable.
    pub uptime: Option<u64>,
    /// Named degradations reported by the daemon.
    pub degraded: Vec<String>,
    /// Whether the hardware dead-man's-switch heartbeat is pulsing.
    pub heartbeat_pulsing: Option<bool>,
    /// One entry per registry source.
    pub sources: Vec<SourceHealthView>,
}

/// The `/api/version` response body.
#[derive(Serialize)]
pub struct VersionView {
    /// Crate version from `Cargo.toml`.
    pub version: &'static str,
    /// Value of `GIT_DESCRIBE` at build time, when set.
    pub git: Option<&'static str>,
    /// Milestone note on what this build ships.
    pub note: &'static str,
}
