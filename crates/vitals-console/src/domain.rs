//! Versioned user-domain record rules shared by the write path and fixtures.
//!
//! Domain records are deliberately open by kind: the registry owns
//! physiological identifiers, while this contract carries journal and
//! administrative records that may gain new kinds additively. The server
//! still validates the immutable envelope metadata before persistence.

use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// The first domain-envelope schema version.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// Return a deterministic JSON value with object keys ordered lexically.
fn canonical_json(value: &Value) -> Value {
    match value {
        Value::Object(object) => {
            let mut keys: Vec<&String> = object.keys().collect();
            keys.sort_unstable();
            let mut ordered = Map::new();
            for key in keys {
                if let Some(child) = object.get(key) {
                    ordered.insert(key.clone(), canonical_json(child));
                }
            }
            Value::Object(ordered)
        }
        Value::Array(array) => Value::Array(array.iter().map(canonical_json).collect()),
        scalar => scalar.clone(),
    }
}

/// Hash content after canonicalizing object-key order.
#[must_use]
pub fn content_sha256(content: &Value) -> [u8; 32] {
    let canonical = canonical_json(content);
    let bytes = serde_json::to_vec(&canonical).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize().into()
}

/// Validate the part of a domain record that must be safe before a database
/// write. Kind payload schemas remain open for additive server/client rollout.
///
/// # Errors
/// Returns the named metadata invariant that the record violates.
pub fn validate_record_metadata(
    id: Uuid,
    kind: &str,
    schema_version: u32,
    content: &Value,
    supersedes_id: Option<Uuid>,
    tombstone_target_id: Option<Uuid>,
) -> Result<(), String> {
    if id.is_nil() {
        return Err("record id must not be nil".to_string());
    }
    if kind.trim().is_empty() {
        return Err("record kind must not be empty".to_string());
    }
    if schema_version == 0 {
        return Err("schema_version=0 is unsupported".to_string());
    }
    if schema_version > i32::MAX as u32 {
        return Err(format!(
            "schema_version={schema_version} exceeds PostgreSQL integer range"
        ));
    }
    if !content.is_object() {
        return Err("record content must be a JSON object".to_string());
    }
    if supersedes_id == Some(id) {
        return Err(format!("record {id} cannot supersede itself"));
    }
    if tombstone_target_id == Some(id) {
        return Err(format!("record {id} cannot tombstone itself"));
    }
    if supersedes_id.is_some() && tombstone_target_id.is_some() {
        return Err("a record cannot both supersede and tombstone another record".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record_id() -> Uuid {
        Uuid::from_u128(1)
    }

    #[test]
    fn domain_hash_ignores_object_key_order() {
        let first = serde_json::json!({"z": 1, "a": {"y": 2, "b": 3}});
        let second = serde_json::json!({"a": {"b": 3, "y": 2}, "z": 1});
        assert_eq!(content_sha256(&first), content_sha256(&second));
    }

    #[test]
    fn domain_metadata_rejects_invalid_version_and_shape() {
        let id = record_id();
        let scalar = serde_json::json!(42);
        assert!(
            validate_record_metadata(id, "journal_entry", 0, &serde_json::json!({}), None, None)
                .is_err()
        );
        assert!(validate_record_metadata(id, "journal_entry", 1, &scalar, None, None).is_err());
        assert!(validate_record_metadata(id, "", 1, &serde_json::json!({}), None, None).is_err());
    }

    #[test]
    fn domain_metadata_rejects_self_and_mixed_mutations() {
        let id = record_id();
        let other = Uuid::from_u128(2);
        let content = serde_json::json!({});
        assert!(
            validate_record_metadata(id, "journal_entry", 1, &content, Some(id), None).is_err()
        );
        assert!(
            validate_record_metadata(id, "journal_entry", 1, &content, None, Some(id)).is_err()
        );
        assert!(
            validate_record_metadata(id, "journal_entry", 1, &content, Some(other), Some(other))
                .is_err()
        );
    }

    #[test]
    fn domain_metadata_accepts_version_and_tombstone() {
        let id = record_id();
        let target = Uuid::from_u128(2);
        assert!(
            validate_record_metadata(
                id,
                "journal_entry",
                CURRENT_SCHEMA_VERSION,
                &serde_json::json!({"deleted": true}),
                None,
                Some(target),
            )
            .is_ok()
        );
    }
}
