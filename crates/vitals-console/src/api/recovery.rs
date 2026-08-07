use crate::api::ops::SysOps;
use crate::handlers::AppState;
use axum::{Json, extract::State, http::StatusCode};
use serde::Serialize;
use serde_json::Value;
use std::time::SystemTime;
use tokio_postgres::NoTls;

/// One `pgbackrest` backup as reported by `pgbackrest info --output=json`.
#[derive(Serialize)]
pub struct BackupInfo {
    /// Backup start time, Unix epoch seconds.
    pub timestamp_start: i64,
    /// Backup stop time, Unix epoch seconds.
    pub timestamp_stop: i64,
    /// Backup size in bytes.
    pub size: i64,
    /// Backup type: `full`, `incr`, or `diff`.
    pub backup_type: String,
}

/// WAL archive span for one stanza.
#[derive(Serialize)]
pub struct ArchiveInfo {
    /// Oldest archived WAL segment name, when known.
    pub wal_min: Option<String>,
    /// Newest archived WAL segment name, when known.
    pub wal_max: Option<String>,
}

/// One `pgbackrest` stanza's backup and archive state.
#[derive(Serialize)]
pub struct StanzaInfo {
    /// Stanza name.
    pub name: String,
    /// Stanza status message from `pgbackrest`.
    pub status: String,
    /// Backups listed for the stanza, oldest to newest.
    pub backups: Vec<BackupInfo>,
    /// WAL archive span.
    pub archive: ArchiveInfo,
}

/// Backup, restore-verify, and at-risk data state for `/api/recovery`.
#[derive(Serialize)]
pub struct RecoveryInfo {
    /// Per-stanza backup and archive state.
    pub pgbackrest: Vec<StanzaInfo>,
    /// Parsed contents of the restore-verify JSON, when present.
    pub restore_verify: Option<Value>,
    /// Seconds since the restore-verify file was written, when present.
    pub restore_verify_age_s: Option<u64>,
    /// Seconds between the newest backup stop and the newest sample — data
    /// newer than the last backup is at risk.
    pub data_at_risk_s: Option<i64>,
}

/// Parse `pgbackrest info --output=json` output into stanzas.
///
/// Returns the stanzas and the newest backup stop time across all of them.
/// Unparseable input yields empty stanzas rather than failing.
fn parse_pgbackrest(out: &str) -> (Vec<StanzaInfo>, Option<i64>) {
    let mut pgbackrest = Vec::new();
    let mut newest_backup_stop = None;

    let Ok(parsed) = serde_json::from_str::<Vec<Value>>(out) else {
        return (pgbackrest, newest_backup_stop);
    };

    for stanza in parsed {
        let name = stanza
            .get("name")
            .unwrap_or(&serde_json::Value::Null)
            .as_str()
            .unwrap_or("unknown")
            .to_string();
        let status = stanza
            .get("status")
            .and_then(|s| s.get("message"))
            .unwrap_or(&serde_json::Value::Null)
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        let mut backups = Vec::new();
        if let Some(bkps) = stanza
            .get("backup")
            .unwrap_or(&serde_json::Value::Null)
            .as_array()
        {
            for bkp in bkps {
                let ts_start = bkp
                    .get("timestamp")
                    .and_then(|s| s.get("start"))
                    .unwrap_or(&serde_json::Value::Null)
                    .as_i64()
                    .unwrap_or(0);
                let ts_stop = bkp
                    .get("timestamp")
                    .and_then(|s| s.get("stop"))
                    .unwrap_or(&serde_json::Value::Null)
                    .as_i64()
                    .unwrap_or(0);
                let size = bkp
                    .get("info")
                    .and_then(|s| s.get("size"))
                    .unwrap_or(&serde_json::Value::Null)
                    .as_i64()
                    .unwrap_or(0);
                let btype = bkp["type"].as_str().unwrap_or("unknown").to_string();

                backups.push(BackupInfo {
                    timestamp_start: ts_start,
                    timestamp_stop: ts_stop,
                    size,
                    backup_type: btype,
                });

                if newest_backup_stop.is_none() || Some(ts_stop) > newest_backup_stop {
                    newest_backup_stop = Some(ts_stop);
                }
            }
        }

        let wal_min = stanza
            .get("archive")
            .unwrap_or(&serde_json::Value::Null)
            .as_array()
            .and_then(|a| a.first())
            .and_then(|d| d["min"].as_str())
            .map(std::string::ToString::to_string);
        let wal_max = stanza
            .get("archive")
            .unwrap_or(&serde_json::Value::Null)
            .as_array()
            .and_then(|a| a.first())
            .and_then(|d| d["max"].as_str())
            .map(std::string::ToString::to_string);

        pgbackrest.push(StanzaInfo {
            name,
            status,
            backups,
            archive: ArchiveInfo { wal_min, wal_max },
        });
    }

    (pgbackrest, newest_backup_stop)
}

/// Read and parse the restore-verify file, plus its age in seconds.
///
/// A missing or unreadable file yields `(None, None)` rather than failing.
async fn read_restore_verify(ops: &SysOps) -> (Option<Value>, Option<u64>) {
    let verify_path = "/var/lib/vitals/restore-verify.json";
    if !(ops.path_exists)(verify_path) {
        return (None, None);
    }
    let Ok(content) = (ops.read_string)(verify_path).await else {
        return (None, None);
    };
    let parsed = serde_json::from_str(&content).ok();
    let age = match (ops.file_mtime)(verify_path) {
        Ok(mtime) => SystemTime::now()
            .duration_since(mtime)
            .ok()
            .map(|d| d.as_secs()),
        Err(_) => None,
    };
    (parsed, age)
}

/// Seconds between the newest backup stop and the newest sample.
///
/// Any database failure yields `None` (unknown), never a fabricated number.
async fn compute_data_at_risk(db_url: &str, stop_ts: i64) -> Option<i64> {
    let (client, connection) = tokio_postgres::connect(db_url, NoTls).await.ok()?;
    tokio::spawn(async move {
        let _ = connection.await;
    });
    let row = client
        .query_one(
            "SELECT EXTRACT(epoch FROM MAX(timestamp))::bigint FROM samples",
            &[],
        )
        .await
        .ok()?;
    let newest_sample_s: Option<i64> = row.get(0);
    newest_sample_s.map(|ts| ts.saturating_sub(stop_ts))
}

/// Collect backup and recovery state from `pgbackrest` and the database.
///
/// # Errors
/// Never returns an error: `pgbackrest`, restore-verify, and database
/// failures all degrade to empty or absent fields rather than failing.
pub async fn collect_recovery_info(ops: &SysOps, db_url: &str) -> Result<RecoveryInfo, String> {
    let out = (ops.cmd)("pgbackrest", &["info", "--output=json"])
        .await
        .unwrap_or_else(|_| "[]".to_string());

    let (pgbackrest, newest_backup_stop) = parse_pgbackrest(&out);

    let (restore_verify, restore_verify_age_s) = read_restore_verify(ops).await;

    let mut data_at_risk_s = None;
    if let Some(stop_ts) = newest_backup_stop {
        data_at_risk_s = compute_data_at_risk(db_url, stop_ts).await;
    }

    Ok(RecoveryInfo {
        pgbackrest,
        restore_verify,
        restore_verify_age_s,
        data_at_risk_s,
    })
}

/// GET /api/recovery
///
/// Serves backup, restore-verify, and at-risk data state.
///
/// # Errors
/// 500 if collection fails; unreachable today because `collect_recovery_info`
/// degrades instead of failing.
pub async fn recovery_handler(
    State(state): State<AppState>,
) -> Result<Json<RecoveryInfo>, (StatusCode, String)> {
    collect_recovery_info(&state.sys_ops, &state.db_url)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))
}
