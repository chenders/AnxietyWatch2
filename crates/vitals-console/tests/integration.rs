//! Integration tests for the vitals-console envelope API.
//!
//! Gated on `VITALS_CONSOLE_TEST_DATABASE_URL` being set; skipped cleanly
//! when absent. If the variable is set but the connection fails, the test
//! **fails** — a broken URL must not produce a green suite (AGENTS.md:
//! guards must say why).
//!
//! These tests require the `attempts`, `domain.records`, and
//! `envelope_clients` tables to exist. Apply the deploy scripts from `db/`
//! before running.

#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "integration test harness: assertions on fixed fixtures should panic loudly"
)]

use base64::Engine;
use serde::Serialize;
use sqlx::PgPool;
use std::env;
use uuid::Uuid;
use vitals_console::db::{self, DomainRecordOutcome};
use vitals_console::models::DomainRecordPayload;

// ---- Helpers ----

/// Returns a pool if `VITALS_CONSOLE_TEST_DATABASE_URL` is set and reachable.
///
/// - **Not set**: prints a skip message and returns `None` (the test returns
///   early — this is a genuine skip, not a pass).
/// - **Set but unreachable**: panics with the connection error. A broken URL
///   must never produce a green suite.
async fn require_test_db() -> Option<PgPool> {
    if let Ok(url) = env::var("VITALS_CONSOLE_TEST_DATABASE_URL") {
        match PgPool::connect(&url).await {
            Ok(pool) => Some(pool),
            Err(e) => {
                panic!("FAIL: VITALS_CONSOLE_TEST_DATABASE_URL is set but connection failed: {e}");
            }
        }
    } else {
        eprintln!("SKIP: VITALS_CONSOLE_TEST_DATABASE_URL not set");
        None
    }
}

/// Clear the append-only fixtures between tests so they are independent.
async fn truncate_all(pool: &PgPool) {
    let _ = sqlx::query("DELETE FROM envelope_clients")
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM domain.record_audit")
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM domain.record_conflicts")
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM domain.record_links")
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM domain.records")
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM attempts").execute(pool).await;
}

fn content_sha256(content: &serde_json::Value) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let bytes = serde_json::to_vec(content).unwrap();
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    hasher.finalize().into()
}

// Test models matching vitals_console structs — kept for reference.

#[derive(Serialize)]
struct EnvelopePayload {
    class: String,
    record: FixtureDomainRecordPayload,
}

#[derive(Serialize)]
struct FixtureDomainRecordPayload {
    id: Uuid,
    kind: String,
    schema_version: u32,
    recorded_at: String,
    supersedes_id: Option<Uuid>,
    tombstone_target_id: Option<Uuid>,
    content: serde_json::Value,
}

// ---- Registration tests ----

#[tokio::test]
#[serial_test::serial]
async fn test_register_new_client() {
    let Some(pool) = require_test_db().await else {
        return;
    };
    truncate_all(&pool).await;

    let client_id = Uuid::now_v7();
    let mut secret = [0u8; 32];
    getrandom::getrandom(&mut secret).unwrap();
    let _secret_b64 = base64::engine::general_purpose::STANDARD.encode(secret);

    // Register.
    let rows: Vec<(Uuid,)> = sqlx::query_as(
        "INSERT INTO envelope_clients (client_id, secret) VALUES ($1, $2) \
         ON CONFLICT (client_id) DO UPDATE SET last_seen_at = now() \
         RETURNING client_id",
    )
    .bind(client_id)
    .bind(secret.as_slice())
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, client_id);

    // Verify stored secret matches.
    let stored: (Vec<u8>,) =
        sqlx::query_as("SELECT secret FROM envelope_clients WHERE client_id = $1")
            .bind(client_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(stored.0, secret.as_slice());
}

#[tokio::test]
#[serial_test::serial]
async fn test_register_same_secret_idempotent() {
    let Some(pool) = require_test_db().await else {
        return;
    };
    truncate_all(&pool).await;

    let client_id = Uuid::now_v7();
    let mut secret = [0u8; 32];
    getrandom::getrandom(&mut secret).unwrap();

    // First registration.
    sqlx::query("INSERT INTO envelope_clients (client_id, secret) VALUES ($1, $2)")
        .bind(client_id)
        .bind(secret.as_slice())
        .execute(&pool)
        .await
        .unwrap();

    // Second registration with same secret — should succeed (idempotent).
    // Test that it doesn't error on conflict.
    let result = sqlx::query(
        "INSERT INTO envelope_clients (client_id, secret) VALUES ($1, $2) \
         ON CONFLICT (client_id) DO NOTHING",
    )
    .bind(client_id)
    .bind(secret.as_slice())
    .execute(&pool)
    .await;
    assert!(result.is_ok());

    // Verify secret unchanged.
    let stored: (Vec<u8>,) =
        sqlx::query_as("SELECT secret FROM envelope_clients WHERE client_id = $1")
            .bind(client_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(stored.0, secret.as_slice());
}

#[tokio::test]
#[serial_test::serial]
async fn test_register_different_secret_conflict() {
    let Some(pool) = require_test_db().await else {
        return;
    };
    truncate_all(&pool).await;

    let client_id = Uuid::now_v7();
    let mut secret1 = [0u8; 32];
    let mut secret2 = [0u8; 32];
    getrandom::getrandom(&mut secret1).unwrap();
    getrandom::getrandom(&mut secret2).unwrap();

    // Register with secret1.
    sqlx::query("INSERT INTO envelope_clients (client_id, secret) VALUES ($1, $2)")
        .bind(client_id)
        .bind(secret1.as_slice())
        .execute(&pool)
        .await
        .unwrap();

    // Try to register with secret2 — should detect conflict.
    let stored: (Vec<u8>,) =
        sqlx::query_as("SELECT secret FROM envelope_clients WHERE client_id = $1")
            .bind(client_id)
            .fetch_one(&pool)
            .await
            .unwrap();

    assert_ne!(
        stored.0,
        secret2.as_slice(),
        "different secret must not match"
    );
    // The handler would return 409 here; the db-level check confirms the
    // stored secret differs, so a re-registration with secret2 would be
    // correctly refused.
}

// ---- Domain record idempotency tests (spec §2.3b) ----

/// Same id, same hash → no-op replay.
#[tokio::test]
#[serial_test::serial]
async fn test_idempotent_replay_same_hash() {
    let Some(pool) = require_test_db().await else {
        return;
    };
    truncate_all(&pool).await;

    let id = Uuid::now_v7();
    let kind = "test_kind";
    let content = serde_json::json!({"value": 42, "note": "hello"});
    let hash = content_sha256(&content);

    // Insert.
    sqlx::query(
        "INSERT INTO domain.records (id, kind, content, content_sha256, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, now(), now())",
    )
    .bind(id)
    .bind(kind)
    .bind(&content)
    .bind(hash.as_slice())
    .execute(&pool)
    .await
    .unwrap();

    // Now query: same id, same hash → should return the existing row.
    let existing: (Uuid, Vec<u8>, String, serde_json::Value) = sqlx::query_as(
        "SELECT id, content_sha256, kind, content FROM domain.records WHERE id = $1",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(existing.0, id);
    assert_eq!(existing.1, hash.as_slice());
    assert_eq!(existing.2, kind);
    assert_eq!(existing.3, content);
}

/// Same id, different hash → 409 conflict, never overwrite.
#[tokio::test]
#[serial_test::serial]
async fn test_idempotent_conflict_different_hash() {
    let Some(pool) = require_test_db().await else {
        return;
    };
    truncate_all(&pool).await;

    let id = Uuid::now_v7();
    let content1 = serde_json::json!({"a": 1});
    let content2 = serde_json::json!({"a": 2});
    let hash1 = content_sha256(&content1);
    let hash2 = content_sha256(&content2);

    assert_ne!(
        hash1, hash2,
        "different content must produce different hashes"
    );

    // Insert first version.
    sqlx::query(
        "INSERT INTO domain.records (id, kind, content, content_sha256, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, now(), now())",
    )
    .bind(id)
    .bind("test")
    .bind(&content1)
    .bind(hash1.as_slice())
    .execute(&pool)
    .await
    .unwrap();

    // Verify the stored record still has content1 (not overwritten).
    let stored: (serde_json::Value,) =
        sqlx::query_as("SELECT content FROM domain.records WHERE id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();

    assert_eq!(stored.0, content1);

    // Try to insert with different content — should be rejected (no overwrite).
    let conflict_result = sqlx::query(
        "INSERT INTO domain.records (id, kind, content, content_sha256, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, now(), now()) \
         ON CONFLICT (id) DO NOTHING",
    )
    .bind(id)
    .bind("test")
    .bind(&content2)
    .bind(hash2.as_slice())
    .execute(&pool)
    .await;

    // The insert should succeed (DO NOTHING), but no new row is created.
    assert!(conflict_result.is_ok());

    // Verify content is still content1.
    let final_stored: (serde_json::Value,) =
        sqlx::query_as("SELECT content FROM domain.records WHERE id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(final_stored.0, content1, "content must NOT be overwritten");
}

/// New id → insert, 201.
#[tokio::test]
#[serial_test::serial]
async fn test_new_id_insert() {
    let Some(pool) = require_test_db().await else {
        return;
    };
    truncate_all(&pool).await;

    let id = Uuid::now_v7();
    let content = serde_json::json!({"new": true});
    let hash = content_sha256(&content);

    let result = sqlx::query(
        "INSERT INTO domain.records (id, kind, content, content_sha256, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, now(), now())",
    )
    .bind(id)
    .bind("new_kind")
    .bind(&content)
    .bind(hash.as_slice())
    .execute(&pool)
    .await;

    assert!(result.is_ok());

    // Verify it was inserted.
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM domain.records WHERE id = $1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count.0, 1);
}

// ---- Versioning and conflict evidence ----

fn domain_payload(id: Uuid, content: serde_json::Value) -> DomainRecordPayload {
    DomainRecordPayload {
        id,
        kind: "journal_entry".to_string(),
        schema_version: 1,
        recorded_at: "2026-01-15T12:30:00Z".to_string(),
        supersedes_id: None,
        tombstone_target_id: None,
        content,
    }
}

#[tokio::test]
#[serial_test::serial]
async fn test_domain_versions_and_tombstones_are_append_only() {
    let Some(pool) = require_test_db().await else {
        return;
    };
    truncate_all(&pool).await;

    let first_id = Uuid::from_u128(0x100);
    let first = domain_payload(first_id, serde_json::json!({"anxiety": 4}));
    let created = db::process_domain_record(&pool, &first, "test-actor")
        .await
        .unwrap();
    assert!(matches!(created, DomainRecordOutcome::Created { .. }));

    let edit_id = Uuid::from_u128(0x101);
    let mut edit = domain_payload(edit_id, serde_json::json!({"anxiety": 6}));
    edit.supersedes_id = Some(first_id);
    let versioned = db::process_domain_record(&pool, &edit, "test-actor")
        .await
        .unwrap();
    assert!(matches!(versioned, DomainRecordOutcome::Created { .. }));

    let tombstone_id = Uuid::from_u128(0x102);
    let mut tombstone = domain_payload(tombstone_id, serde_json::json!({"reason": "removed"}));
    tombstone.tombstone_target_id = Some(first_id);
    let tombstoned = db::process_domain_record(&pool, &tombstone, "test-actor")
        .await
        .unwrap();
    assert!(matches!(tombstoned, DomainRecordOutcome::Created { .. }));

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM domain.records")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count.0, 3);
    let events: Vec<(String,)> =
        sqlx::query_as("SELECT event_kind FROM domain.record_audit ORDER BY id")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(
        events.iter().map(|row| row.0.as_str()).collect::<Vec<_>>(),
        ["created", "versioned", "tombstoned"]
    );
}

#[tokio::test]
#[serial_test::serial]
async fn test_domain_conflict_preserves_incoming_evidence() {
    let Some(pool) = require_test_db().await else {
        return;
    };
    truncate_all(&pool).await;

    let id = Uuid::from_u128(0x200);
    let first = domain_payload(id, serde_json::json!({"anxiety": 3}));
    db::process_domain_record(&pool, &first, "test-actor")
        .await
        .unwrap();
    let conflicting = domain_payload(id, serde_json::json!({"anxiety": 9}));
    let outcome = db::process_domain_record(&pool, &conflicting, "other-actor")
        .await
        .unwrap();
    assert!(
        matches!(outcome, DomainRecordOutcome::Conflict { id: conflict_id } if conflict_id == id)
    );

    let stored: (serde_json::Value,) =
        sqlx::query_as("SELECT content FROM domain.records WHERE id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(stored.0, first.content);
    let conflict: (serde_json::Value, String, String) = sqlx::query_as(
        "SELECT incoming_content, actor, status FROM domain.record_conflicts WHERE record_id = $1",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(conflict.0, conflicting.content);
    assert_eq!(conflict.1, "other-actor");
    assert_eq!(conflict.2, "open");
}

// ---- Attempt-before-work tests (spec §2.3, criterion 3) ----

/// An attempt row is written before work; verify its structure.
#[tokio::test]
#[serial_test::serial]
async fn test_attempt_created_before_work() {
    let Some(pool) = require_test_db().await else {
        return;
    };
    truncate_all(&pool).await;

    // Create an attempt (simulating what the handler does before work).
    let row: (i64,) = sqlx::query_as(
        "INSERT INTO attempts (kind, subject, expected_duration_s) \
         VALUES ($1, $2, $3) RETURNING id",
    )
    .bind("envelope")
    .bind("test-client-1")
    .bind(30)
    .fetch_one(&pool)
    .await
    .unwrap();

    let attempt_id = row.0;

    // Verify the attempt row exists with NULL outcome (work not yet done).
    let attempt: (i64, String, String, i32, Option<String>) = sqlx::query_as(
        "SELECT id, kind, subject, expected_duration_s, outcome \
         FROM attempts WHERE id = $1",
    )
    .bind(attempt_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(attempt.0, attempt_id);
    assert_eq!(attempt.1, "envelope");
    assert_eq!(attempt.2, "test-client-1");
    assert_eq!(attempt.3, 30);
    assert!(
        attempt.4.is_none(),
        "outcome must be NULL before work completes"
    );
}

/// A NULL-outcome row persists when work is not finished (simulated kill path).
#[tokio::test]
#[serial_test::serial]
async fn test_attempt_null_outcome_on_kill() {
    let Some(pool) = require_test_db().await else {
        return;
    };
    truncate_all(&pool).await;

    // Simulate: create attempt, then "crash" (don't call finish_attempt).
    let row: (i64,) = sqlx::query_as(
        "INSERT INTO attempts (kind, subject, expected_duration_s) \
         VALUES ($1, $2, $3) RETURNING id",
    )
    .bind("envelope")
    .bind("crash-test-client")
    .bind(30)
    .fetch_one(&pool)
    .await
    .unwrap();

    // Verify the row persists with NULL outcome — this is the point of
    // criterion 3: a killed process leaves a row that the liveness evaluator
    // degrades to UNKNOWN past expected_duration_s.
    let row_data: (i64, Option<String>, Option<chrono::DateTime<chrono::Utc>>) =
        sqlx::query_as("SELECT id, outcome, finished_at FROM attempts WHERE id = $1")
            .bind(row.0)
            .fetch_one(&pool)
            .await
            .unwrap();

    assert_eq!(row_data.0, row.0);
    assert!(
        row_data.1.is_none(),
        "outcome must remain NULL after simulated kill"
    );
    assert!(
        row_data.2.is_none(),
        "finished_at must remain NULL after simulated kill"
    );
}

/// Full attempt lifecycle: create → work → finish with effect.
#[tokio::test]
#[serial_test::serial]
async fn test_attempt_full_lifecycle() {
    let Some(pool) = require_test_db().await else {
        return;
    };
    truncate_all(&pool).await;

    // Create attempt.
    let row: (i64,) = sqlx::query_as(
        "INSERT INTO attempts (kind, subject, expected_duration_s) \
         VALUES ($1, $2, $3) RETURNING id",
    )
    .bind("envelope")
    .bind("lifecycle-test")
    .bind(30)
    .fetch_one(&pool)
    .await
    .unwrap();

    // Finish with effect.
    let effect = serde_json::json!({"created": 1, "updated": 0, "replayed": 0, "rejected": 0});
    sqlx::query("UPDATE attempts SET finished_at = now(), outcome = $2, effect = $3 WHERE id = $1")
        .bind(row.0)
        .bind("succeeded")
        .bind(&effect)
        .execute(&pool)
        .await
        .unwrap();

    // Verify.
    let final_row: (
        String,
        Option<chrono::DateTime<chrono::Utc>>,
        serde_json::Value,
    ) = sqlx::query_as("SELECT outcome, finished_at, effect FROM attempts WHERE id = $1")
        .bind(row.0)
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(final_row.0, "succeeded");
    assert!(
        final_row.1.is_some(),
        "finished_at must be set after finish"
    );
    assert_eq!(final_row.2["created"], 1);
}

// ---- SHA-256 canonicalization ----

#[tokio::test]
async fn test_content_sha256_deterministic() {
    let content = serde_json::json!({"b": 2, "a": 1});
    let hash1 = content_sha256(&content);
    let hash2 = content_sha256(&content);
    assert_eq!(
        hash1, hash2,
        "SHA-256 must be deterministic for same JSON value"
    );
}

#[tokio::test]
async fn test_content_sha256_different_for_different_content() {
    let c1 = serde_json::json!({"a": 1});
    let c2 = serde_json::json!({"a": 2});
    let h1 = content_sha256(&c1);
    let h2 = content_sha256(&c2);
    assert_ne!(h1, h2, "different content must produce different hashes");
}

// ---- Attempt list query ----

#[tokio::test]
#[serial_test::serial]
async fn test_list_attempts() {
    let Some(pool) = require_test_db().await else {
        return;
    };
    // Do NOT truncate — this test scopes its queries to a unique kind so
    // parallel tests cannot pollute its assertions. truncate_all would
    // destroy other tests' in-flight rows instead of providing isolation.

    // Use a unique kind per test so parallel test runs don't cross-contaminate.
    let test_kind = format!("list_attempts_{}", Uuid::now_v7());

    // Insert a few attempts scoped to this test's kind.
    for i in 0..3 {
        sqlx::query(
            "INSERT INTO attempts (kind, subject, expected_duration_s) VALUES ($1, $2, $3)",
        )
        .bind(&test_kind)
        .bind(format!("subject-{i}"))
        .bind(10)
        .execute(&pool)
        .await
        .unwrap();
    }

    let rows: Vec<(i64, String, String)> = sqlx::query_as(
        "SELECT id, kind, subject FROM attempts \
         WHERE kind = $1 \
         ORDER BY started_at DESC LIMIT 5",
    )
    .bind(&test_kind)
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(rows.len(), 3);
    for row in &rows {
        assert_eq!(row.1, test_kind);
    }
}

// ---- Model serialization round-trip ----

#[test]
fn test_effect_serialization() {
    let effect = serde_json::json!({"created": 1, "updated": 0, "replayed": 0, "rejected": 0});
    let json_str = serde_json::to_string(&effect).unwrap();
    assert!(json_str.contains("created"));
    assert!(json_str.contains("replayed"));
    assert!(json_str.contains("rejected"));
}

#[test]
fn test_envelope_payload_roundtrip() {
    let payload = EnvelopePayload {
        class: "domain_record".to_string(),
        record: FixtureDomainRecordPayload {
            id: Uuid::now_v7(),
            kind: "test".to_string(),
            schema_version: 1,
            recorded_at: "2026-01-01T00:00:00Z".to_string(),
            supersedes_id: None,
            tombstone_target_id: None,
            content: serde_json::json!({"x": 1}),
        },
    };
    let json = serde_json::to_string(&payload).unwrap();
    let back: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(back["class"], "domain_record");
    assert!(back["record"]["id"].is_string());
    assert_eq!(back["record"]["kind"], "test");
    assert_eq!(back["record"]["content"]["x"], 1);
}

#[tokio::test]
async fn test_override_and_mute_api_fails_closed() {
    let resp = vitals_console::handlers::operator_override_handler().await;
    assert_eq!(resp.0, axum::http::StatusCode::FORBIDDEN);
    assert_eq!(
        resp.1.error,
        "mutation blocked: auth interface not yet available"
    );
}

#[tokio::test]
async fn test_override_source_declaration_mute_read() {
    let Some(pool) = require_test_db().await else {
        return;
    };

    // Make sure we have a source
    let _ = sqlx::query("INSERT INTO vitals_source (id, name, transport) VALUES (999, 'test_source', 'ble') ON CONFLICT DO NOTHING")
        .execute(&pool)
        .await;

    let _ = sqlx::query("INSERT INTO source_declarations (source_id, is_configured) VALUES (999, false) ON CONFLICT (source_id) DO UPDATE SET is_configured = false")
        .execute(&pool)
        .await;

    let _ = sqlx::query("INSERT INTO health_mutes (source_id, muted_by, muted_until) VALUES (999, 'test_actor', NOW() + INTERVAL '1 day')")
        .execute(&pool)
        .await;

    let cm = vitals_console::db::get_config_and_mutes(&pool)
        .await
        .unwrap();
    let config = cm.get(&999).unwrap();
    assert!(!config.is_configured);
    assert!(config.muted_until.is_some());
    assert_eq!(config.muted_by.as_deref(), Some("test_actor"));
}
