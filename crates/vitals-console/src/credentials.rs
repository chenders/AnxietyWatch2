//! Operator-authorized mobile enrollment and device credential lifecycle.
//!
//! Enrollment artifacts contain a random plaintext token only in the returned
//! one-time reveal. The store retains its SHA-256 hash; device records retain
//! public keys and fingerprints, never bearer tokens or private keys.

use base64::Engine;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use subtle::ConstantTimeEq;
use uuid::Uuid;

/// Maximum lifetime of an enrollment artifact.
pub const ENROLLMENT_LIFETIME_S: i64 = 10 * 60;
/// Device credential lifetime.
pub const CREDENTIAL_LIFETIME_S: i64 = 180 * 24 * 60 * 60;
/// Rotation overlap for the old device key.
pub const ROTATION_OVERLAP_S: i64 = 24 * 60 * 60;
/// Recent operator reauthentication required for sensitive operations.
pub const RECENT_AUTH_S: i64 = 5 * 60;
/// Response headers for a newly minted plaintext enrollment artifact.
pub const PLAINTEXT_RESPONSE_HEADERS: [(&str, &str); 2] = [
    ("cache-control", "no-store, max-age=0"),
    ("pragma", "no-cache"),
];

/// Least-privilege mobile scopes understood by the server.
pub const ALLOWED_SCOPES: [&str; 7] = [
    "submit_platform_health",
    "submit_domain_records",
    "read_projections",
    "read_registry",
    "read_credential_status",
    "rotate_own_key",
    "submit_notifications",
];

/// Recent operator reauthentication evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecentAuth {
    /// UTC epoch second of the successful password reauthentication.
    pub authenticated_at_epoch_s: i64,
}

impl RecentAuth {
    /// Require reauthentication within the bounded window.
    ///
    /// # Errors
    /// Returns a named blocker containing the evidence age.
    pub fn require(self, now_epoch_s: i64) -> Result<(), String> {
        let age = now_epoch_s.saturating_sub(self.authenticated_at_epoch_s);
        if (0..=RECENT_AUTH_S).contains(&age) {
            Ok(())
        } else {
            Err(format!(
                "sensitive credential action blocked: recent_auth_age={age}s, limit={RECENT_AUTH_S}s"
            ))
        }
    }
}

/// Public enrollment artifact metadata plus a token available for one reveal.
pub struct EnrollmentArtifactReveal {
    /// Single-use enrollment UUID.
    pub enrollment_id: Uuid,
    /// Server identity included in the phone confirmation ceremony.
    pub server_instance_id: Uuid,
    /// Expiry in UTC epoch seconds.
    pub expires_at_epoch_s: i64,
    /// Requested least-privilege scopes.
    pub scopes: Vec<String>,
    token: Option<String>,
}

impl EnrollmentArtifactReveal {
    /// Consume and return plaintext exactly once.
    pub fn reveal_once(&mut self) -> Option<String> {
        self.token.take()
    }
}

/// Public device credential metadata; the public key itself is shown only as a
/// fingerprint and is never treated as a bearer secret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceCredential {
    /// Stable device identifier.
    pub client_id: Uuid,
    /// Key identifier for rotation and revocation.
    pub key_id: Uuid,
    /// Operator label.
    pub label: String,
    /// SHA-256 fingerprint of the P-256 public key.
    pub public_key_fingerprint: String,
    /// Authorized route scopes.
    pub scopes: Vec<String>,
    /// Issue time.
    pub issued_at_epoch_s: i64,
    /// Fixed expiry time.
    pub expires_at_epoch_s: i64,
    /// Last authenticated request time.
    pub last_used_at_epoch_s: Option<i64>,
    /// Revocation time, if revoked.
    pub revoked_at_epoch_s: Option<i64>,
    /// Old-key overlap end during rotation.
    pub overlap_until_epoch_s: Option<i64>,
}

#[derive(Clone)]
struct StoredArtifact {
    server_instance_id: Uuid,
    token_hash: [u8; 32],
    expires_at_epoch_s: i64,
    scopes: Vec<String>,
    consumed_at_epoch_s: Option<i64>,
}

#[derive(Clone)]
struct StoredDevice {
    credential: DeviceCredential,
}

/// In-memory lifecycle model used by handlers and deterministic tests. The
/// production adapter persists the same fields in the credential tables.
#[derive(Default)]
pub struct CredentialStore {
    artifacts: HashMap<Uuid, StoredArtifact>,
    devices: HashMap<(Uuid, Uuid), StoredDevice>,
}

impl CredentialStore {
    /// Create a ten-minute operator-approved artifact.
    ///
    /// # Errors
    /// Rejects empty labels, invalid scopes, and random-token failure.
    pub fn create_artifact(
        &mut self,
        server_instance_id: Uuid,
        label: &str,
        scopes: &[&str],
        now_epoch_s: i64,
    ) -> Result<EnrollmentArtifactReveal, String> {
        validate_mobile_label(label)?;
        let scopes = validate_mobile_scopes(scopes)?;
        let mut token_bytes = [0_u8; 32];
        getrandom::getrandom(&mut token_bytes)
            .map_err(|error| format!("enrollment token generation failed: {error}"))?;
        let token = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(token_bytes);
        let enrollment_id = Uuid::new_v4();
        let expires_at_epoch_s = now_epoch_s.saturating_add(ENROLLMENT_LIFETIME_S);
        self.artifacts.insert(
            enrollment_id,
            StoredArtifact {
                server_instance_id,
                token_hash: hash_token(&token),
                expires_at_epoch_s,
                scopes: scopes.clone(),
                consumed_at_epoch_s: None,
            },
        );
        Ok(EnrollmentArtifactReveal {
            enrollment_id,
            server_instance_id,
            expires_at_epoch_s,
            scopes,
            token: Some(token),
        })
    }

    /// Atomically consume an artifact and issue a public-key credential.
    ///
    /// # Errors
    /// Rejects unknown, expired, consumed, or mismatched artifacts and invalid
    /// public-key/label input.
    pub fn claim(
        &mut self,
        enrollment_id: Uuid,
        token: &str,
        server_instance_id: Uuid,
        public_key: &[u8],
        label: &str,
        now_epoch_s: i64,
    ) -> Result<DeviceCredential, String> {
        validate_mobile_label(label)?;
        validate_mobile_public_key(public_key)?;
        let artifact = self.artifacts.get_mut(&enrollment_id).ok_or_else(|| {
            format!("enrollment blocked: enrollment_id={enrollment_id} is unknown")
        })?;
        if artifact.server_instance_id != server_instance_id {
            return Err(format!(
                "enrollment blocked: server_instance_id={server_instance_id} does not match artifact"
            ));
        }
        if artifact.expires_at_epoch_s <= now_epoch_s {
            return Err(format!(
                "enrollment blocked: enrollment_id={enrollment_id} expired_at={}",
                artifact.expires_at_epoch_s
            ));
        }
        if artifact.consumed_at_epoch_s.is_some() {
            return Err(format!(
                "enrollment blocked: enrollment_id={enrollment_id} was already consumed"
            ));
        }
        if !bool::from(hash_token(token).ct_eq(&artifact.token_hash)) {
            return Err(format!(
                "enrollment blocked: enrollment_id={enrollment_id} token did not match"
            ));
        }
        artifact.consumed_at_epoch_s = Some(now_epoch_s);
        let client_id = Uuid::new_v4();
        let key_id = Uuid::new_v4();
        let credential = DeviceCredential {
            client_id,
            key_id,
            label: label.to_string(),
            public_key_fingerprint: fingerprint(public_key),
            scopes: artifact.scopes.clone(),
            issued_at_epoch_s: now_epoch_s,
            expires_at_epoch_s: now_epoch_s.saturating_add(CREDENTIAL_LIFETIME_S),
            last_used_at_epoch_s: None,
            revoked_at_epoch_s: None,
            overlap_until_epoch_s: None,
        };
        self.devices.insert(
            (client_id, key_id),
            StoredDevice {
                credential: credential.clone(),
            },
        );
        Ok(credential)
    }

    /// Record authenticated use, preserving last-used evidence.
    ///
    /// # Errors
    /// Names an unknown, revoked, or expired key.
    pub fn record_use(
        &mut self,
        client_id: Uuid,
        key_id: Uuid,
        now_epoch_s: i64,
    ) -> Result<DeviceCredential, String> {
        let stored = self.devices.get_mut(&(client_id, key_id)).ok_or_else(|| {
            format!("credential use blocked: client_id={client_id}, key_id={key_id} unknown")
        })?;
        if stored.credential.revoked_at_epoch_s.is_some() {
            return Err(format!(
                "credential use blocked: key_id={key_id} is revoked"
            ));
        }
        if stored.credential.expires_at_epoch_s <= now_epoch_s {
            return Err(format!(
                "credential use blocked: key_id={key_id} expired_at={}",
                stored.credential.expires_at_epoch_s
            ));
        }
        stored.credential.last_used_at_epoch_s = Some(now_epoch_s);
        Ok(stored.credential.clone())
    }

    /// Revoke one key after recent operator reauthentication.
    ///
    /// # Errors
    /// Names stale auth, unknown key, or already-revoked state.
    pub fn revoke(
        &mut self,
        client_id: Uuid,
        key_id: Uuid,
        auth: RecentAuth,
        now_epoch_s: i64,
        reason: &str,
    ) -> Result<DeviceCredential, String> {
        auth.require(now_epoch_s)?;
        if reason.trim().is_empty() {
            return Err("credential revocation blocked: reason is empty".to_string());
        }
        let stored = self.devices.get_mut(&(client_id, key_id)).ok_or_else(|| {
            format!("credential revocation blocked: client_id={client_id}, key_id={key_id} unknown")
        })?;
        if stored.credential.revoked_at_epoch_s.is_some() {
            return Err(format!(
                "credential revocation blocked: key_id={key_id} already revoked"
            ));
        }
        stored.credential.revoked_at_epoch_s = Some(now_epoch_s);
        Ok(stored.credential.clone())
    }

    /// Issue a replacement key and keep the old key valid for at most 24 hours.
    ///
    /// # Errors
    /// Names stale auth, unknown/invalid current keys, or malformed replacement.
    pub fn rotate(
        &mut self,
        client_id: Uuid,
        old_key_id: Uuid,
        new_public_key: &[u8],
        auth: RecentAuth,
        now_epoch_s: i64,
    ) -> Result<DeviceCredential, String> {
        auth.require(now_epoch_s)?;
        validate_mobile_public_key(new_public_key)
            .map_err(|reason| format!("credential rotation blocked: {reason}"))?;
        let old = self
            .devices
            .get_mut(&(client_id, old_key_id))
            .ok_or_else(|| {
                format!("credential rotation blocked: old key_id={old_key_id} unknown")
            })?;
        if old.credential.revoked_at_epoch_s.is_some() {
            return Err(format!(
                "credential rotation blocked: old key_id={old_key_id} is revoked"
            ));
        }
        old.credential.overlap_until_epoch_s = Some(now_epoch_s.saturating_add(ROTATION_OVERLAP_S));
        let mut replacement = old.credential.clone();
        replacement.key_id = Uuid::new_v4();
        replacement.issued_at_epoch_s = now_epoch_s;
        replacement.expires_at_epoch_s = now_epoch_s.saturating_add(CREDENTIAL_LIFETIME_S);
        replacement.public_key_fingerprint = fingerprint(new_public_key);
        replacement.last_used_at_epoch_s = None;
        replacement.revoked_at_epoch_s = None;
        replacement.overlap_until_epoch_s = None;
        self.devices.insert(
            (client_id, replacement.key_id),
            StoredDevice {
                credential: replacement.clone(),
            },
        );
        Ok(replacement)
    }
}

/// Validate an operator-visible device label before persistence.
///
/// # Errors
///
/// Returns the rejected character count when the label is empty or exceeds 80 characters.
pub fn validate_mobile_label(label: &str) -> Result<(), String> {
    let length = label.chars().count();
    if (1..=80).contains(&length) {
        Ok(())
    } else {
        Err(format!(
            "credential label rejected: length={length}, allowed=1..=80"
        ))
    }
}

/// Validate and copy the least-privilege scopes granted to a device.
///
/// # Errors
///
/// Returns the first scope that is absent from the server allowlist, or reports an empty request.
pub fn validate_mobile_scopes(scopes: &[&str]) -> Result<Vec<String>, String> {
    if scopes.is_empty() {
        return Err("enrollment blocked: no initial scopes were requested".to_string());
    }
    scopes
        .iter()
        .map(|scope| {
            if ALLOWED_SCOPES.contains(scope) {
                Ok((*scope).to_string())
            } else {
                Err(format!(
                    "enrollment blocked: scope={scope} is not allowlisted"
                ))
            }
        })
        .collect()
}

/// Hash an enrollment token for constant-time comparison and database storage.
#[must_use]
pub fn hash_enrollment_token(token: &str) -> [u8; 32] {
    Sha256::digest(token.as_bytes()).into()
}

/// Validate an uncompressed P-256 public key and name the blocking value.
///
/// # Errors
///
/// Returns the supplied byte length when the key is not a valid 65-byte SEC1 point.
pub fn validate_mobile_public_key(public_key: &[u8]) -> Result<(), String> {
    if public_key.len() == 65
        && public_key.first() == Some(&0x04)
        && p256::PublicKey::from_sec1_bytes(public_key).is_ok()
    {
        Ok(())
    } else {
        Err(format!(
            "public key is not a valid uncompressed P-256 key: length={} bytes",
            public_key.len()
        ))
    }
}

/// Return the stable public-key fingerprint stored beside a credential.
#[must_use]
pub fn fingerprint_public_key(public_key: &[u8]) -> String {
    hex::encode(Sha256::digest(public_key))
}

fn hash_token(token: &str) -> [u8; 32] {
    hash_enrollment_token(token)
}

fn fingerprint(public_key: &[u8]) -> String {
    fingerprint_public_key(public_key)
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "credential tests use fixed fixtures")]
    use super::*;

    const SERVER: Uuid = Uuid::from_u128(1);
    const NOW: i64 = 100_000;
    const SCOPES: [&str; 2] = ["submit_domain_records", "read_credential_status"];

    fn valid_key() -> [u8; 65] {
        use p256::elliptic_curve::sec1::ToSec1Point;
        let private_key = p256::SecretKey::from_slice(&[1_u8; 32]).unwrap();
        private_key
            .public_key()
            .to_sec1_point(false)
            .as_bytes()
            .try_into()
            .unwrap()
    }

    #[test]
    fn credential_enrollment_reveals_plaintext_once_and_claims_once() {
        let mut store = CredentialStore::default();
        let mut reveal = store
            .create_artifact(SERVER, "bedside iPhone", &SCOPES, NOW)
            .unwrap();
        let token = reveal.reveal_once().unwrap();
        assert!(reveal.reveal_once().is_none());
        let key = valid_key();
        let credential = store
            .claim(
                reveal.enrollment_id,
                &token,
                SERVER,
                &key,
                "bedside iPhone",
                NOW + 1,
            )
            .unwrap();
        assert_eq!(credential.public_key_fingerprint.len(), 64);
        assert!(
            store
                .claim(
                    reveal.enrollment_id,
                    &token,
                    SERVER,
                    &key,
                    "bedside iPhone",
                    NOW + 2
                )
                .is_err()
        );
    }

    #[test]
    fn credential_expiry_rejects_claim_and_recent_auth_is_bounded() {
        let mut store = CredentialStore::default();
        let mut reveal = store
            .create_artifact(SERVER, "phone", &SCOPES, NOW)
            .unwrap();
        let token = reveal.reveal_once().unwrap();
        let key = valid_key();
        assert!(
            store
                .claim(
                    reveal.enrollment_id,
                    &token,
                    SERVER,
                    &key,
                    "phone",
                    NOW + ENROLLMENT_LIFETIME_S
                )
                .is_err()
        );
        assert!(
            RecentAuth {
                authenticated_at_epoch_s: NOW - RECENT_AUTH_S
            }
            .require(NOW)
            .is_ok()
        );
        assert!(
            RecentAuth {
                authenticated_at_epoch_s: NOW - RECENT_AUTH_S - 1
            }
            .require(NOW)
            .is_err()
        );
    }

    #[test]
    fn credential_use_rotation_and_revoke_preserve_public_metadata_only() {
        let mut store = CredentialStore::default();
        let mut reveal = store
            .create_artifact(SERVER, "phone", &SCOPES, NOW)
            .unwrap();
        let token = reveal.reveal_once().unwrap();
        let first_key = valid_key();
        let first = store
            .claim(
                reveal.enrollment_id,
                &token,
                SERVER,
                &first_key,
                "phone",
                NOW,
            )
            .unwrap();
        let used = store
            .record_use(first.client_id, first.key_id, NOW + 1)
            .unwrap();
        assert_eq!(used.last_used_at_epoch_s, Some(NOW + 1));
        let replacement = store
            .rotate(
                first.client_id,
                first.key_id,
                &valid_key(),
                RecentAuth {
                    authenticated_at_epoch_s: NOW,
                },
                NOW + 2,
            )
            .unwrap();
        assert_ne!(replacement.key_id, first.key_id);
        assert!(
            store
                .revoke(
                    first.client_id,
                    replacement.key_id,
                    RecentAuth {
                        authenticated_at_epoch_s: NOW
                    },
                    NOW + 3,
                    "lost phone"
                )
                .is_ok()
        );
        assert!(
            store
                .record_use(first.client_id, replacement.key_id, NOW + 4)
                .is_err()
        );
    }

    #[test]
    fn credential_validation_names_blocking_values() {
        let mut store = CredentialStore::default();
        assert!(
            store
                .create_artifact(SERVER, "", &SCOPES, NOW)
                .err()
                .unwrap()
                .contains("length=0")
        );
        assert!(
            store
                .create_artifact(SERVER, "phone", &["admin"], NOW)
                .err()
                .unwrap()
                .contains("scope=admin")
        );
    }
}
