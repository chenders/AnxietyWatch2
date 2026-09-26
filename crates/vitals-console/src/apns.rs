//! APNs provider safety contracts.
//!
//! This module deliberately keeps provider credentials out of browser state, logs, and serialized
//! responses. Provider acceptance remains a separate fact from delivery, display, and
//! acknowledgement; configuration without a provider key is `NOT_CONFIGURED`, never a send.

#![allow(missing_docs)]

use std::fmt;
use std::path::Path;
use std::sync::{Arc, Mutex};

use base64::Engine;
use p256::ecdsa::SigningKey;
use p256::ecdsa::signature::Signer;
use p256::pkcs8::DecodePrivateKey;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ApnsEnvironment {
    Sandbox,
    Production,
}

impl ApnsEnvironment {
    /// Parse the deployment environment name.
    ///
    /// # Errors
    ///
    /// Returns an error naming the unsupported value.
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "sandbox" => Ok(Self::Sandbox),
            "production" => Ok(Self::Production),
            other => Err(format!("APNs environment unsupported: value={other}")),
        }
    }

    #[must_use]
    pub fn host(self) -> &'static str {
        match self {
            Self::Sandbox => "https://api.sandbox.push.apple.com",
            Self::Production => "https://api.push.apple.com",
        }
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct ApnsProviderConfig {
    pub environment: ApnsEnvironment,
    pub team_id: String,
    pub key_id: String,
    pub bundle_id: String,
    pub private_key_path: String,
}

impl fmt::Debug for ApnsProviderConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ApnsProviderConfig")
            .field("environment", &self.environment)
            .field("team_id", &"[configured]")
            .field("key_id", &"[configured]")
            .field("bundle_id", &self.bundle_id)
            .field("private_key_path", &"[configured]")
            .finish()
    }
}

/// Encrypts APNs device tokens with the protected server-side token key.
#[derive(Clone)]
pub struct ApnsTokenProtector {
    key: [u8; 32],
}

impl ApnsTokenProtector {
    /// Decode a base64-encoded 256-bit token-encryption key.
    ///
    /// # Errors
    ///
    /// Returns an error naming an absent or incorrectly sized key.
    pub fn from_base64(value: &str) -> Result<Self, String> {
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(value)
            .map_err(|error| format!("APNs token key is not valid base64: {error}"))?;
        let key = <[u8; 32]>::try_from(decoded.as_slice())
            .map_err(|_| "APNs token key must decode to exactly 32 bytes".to_string())?;
        Ok(Self { key })
    }

    /// Encrypt a validated hexadecimal APNs token and return nonce plus ciphertext.
    ///
    /// # Errors
    ///
    /// Returns an error when the token is malformed or encryption fails.
    pub fn seal(&self, token: &str) -> Result<(Vec<u8>, String), String> {
        let token = token.trim();
        if token.is_empty()
            || !token.len().is_multiple_of(2)
            || !token.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(
                "APNs token rejected: token must be non-empty even-length hexadecimal".to_string(),
            );
        }
        let mut nonce = [0_u8; 24];
        getrandom::getrandom(&mut nonce)
            .map_err(|error| format!("APNs token encryption nonce failed: {error}"))?;
        let ciphertext = crate::crypto::encrypt(token.as_bytes(), &self.key, &nonce)?;
        let mut stored = nonce.to_vec();
        stored.extend(ciphertext);
        let fingerprint = hex::encode(Sha256::digest(token.as_bytes()));
        Ok((stored, fingerprint))
    }

    /// Decrypt a token previously returned by [`Self::seal`].
    ///
    /// # Errors
    ///
    /// Returns an error when the stored value is truncated, tampered with, or not UTF-8.
    pub fn open(&self, stored: &[u8]) -> Result<String, String> {
        let (nonce, ciphertext) = stored.split_at_checked(24).ok_or_else(|| {
            "APNs token decryption blocked: ciphertext is shorter than the nonce".to_string()
        })?;
        let nonce: [u8; 24] = nonce
            .try_into()
            .map_err(|_| "APNs token decryption blocked: nonce length is invalid".to_string())?;
        let plaintext = crate::crypto::decrypt(ciphertext, &self.key, &nonce)?;
        String::from_utf8(plaintext)
            .map_err(|error| format!("APNs token decryption blocked: token is not UTF-8: {error}"))
    }
}

impl ApnsProviderConfig {
    /// Construct a provider configuration without exposing secret material.
    ///
    /// # Errors
    ///
    /// Returns an error naming the first missing or unsupported field.
    pub fn from_values(
        environment: &str,
        team_id: &str,
        key_id: &str,
        bundle_id: &str,
        private_key_path: &str,
    ) -> Result<Self, String> {
        let environment = ApnsEnvironment::parse(environment)?;
        for (name, value) in [
            ("team_id", team_id),
            ("key_id", key_id),
            ("bundle_id", bundle_id),
            ("private_key_path", private_key_path),
        ] {
            if value.trim().is_empty() {
                return Err(format!("APNs provider configuration missing: field={name}"));
            }
        }
        Ok(Self {
            environment,
            team_id: team_id.to_string(),
            key_id: key_id.to_string(),
            bundle_id: bundle_id.to_string(),
            private_key_path: private_key_path.to_string(),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApnsProviderResult {
    pub accepted: bool,
    pub status_code: u16,
    pub reason: Option<String>,
    pub retryable: bool,
}

#[derive(Clone)]
pub struct ApnsProvider {
    config: ApnsProviderConfig,
    signing_key: SigningKey,
    client: reqwest::Client,
    cached_jwt: Arc<Mutex<Option<(i64, String)>>>,
}

impl ApnsProvider {
    /// Load a configured APNs provider without logging private key material.
    ///
    /// # Errors
    ///
    /// Returns an error when the key file cannot be read or parsed, or when the HTTP client cannot
    /// be created.
    pub fn from_config(config: ApnsProviderConfig) -> Result<Self, String> {
        let pem = std::fs::read_to_string(Path::new(&config.private_key_path))
            .map_err(|error| format!("APNs provider key read failed: {error}"))?;
        let signing_key = SigningKey::from_pkcs8_pem(&pem)
            .map_err(|error| format!("APNs provider key parse failed: {error}"))?;
        let client = reqwest::Client::builder()
            .http2_adaptive_window(true)
            .build()
            .map_err(|error| format!("APNs HTTP client setup failed: {error}"))?;
        Ok(Self {
            config,
            signing_key,
            client,
            cached_jwt: Arc::new(Mutex::new(None)),
        })
    }

    /// Send one minimal operator-directed message to one already-resolved token.
    ///
    /// # Errors
    ///
    /// Returns a transport or request-construction error. APNs HTTP rejection is returned as a
    /// non-panicking [`ApnsProviderResult`] with its sanitized reason.
    pub async fn send(
        &self,
        token: &str,
        payload: &ApnsPayload,
        now_epoch_s: i64,
    ) -> Result<ApnsProviderResult, String> {
        self.send_for(
            token,
            payload,
            self.config.environment,
            &self.config.bundle_id,
            now_epoch_s,
        )
        .await
    }

    /// Send using the environment and topic recorded for the target registration.
    ///
    /// # Errors
    ///
    /// Returns a transport, token, topic, or request-construction error.
    pub async fn send_for(
        &self,
        token: &str,
        payload: &ApnsPayload,
        environment: ApnsEnvironment,
        topic: &str,
        now_epoch_s: i64,
    ) -> Result<ApnsProviderResult, String> {
        if topic.trim().is_empty() {
            return Err("APNs send blocked: target topic is empty".to_string());
        }
        let token = token.trim();
        if token.is_empty() || !token.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err("APNs send blocked: token is not hexadecimal".to_string());
        }
        let jwt = self.jwt(now_epoch_s);
        let body = serde_json::json!({
            "aps": {
                "alert": {
                    "title": "AnxietyWatch",
                    "body": "Operator message"
                },
                "category": payload.category
            },
            "message_id": payload.message_id,
            "payload_class": payload.payload_class,
            "category": payload.category,
            "deep_link": payload.deep_link,
            "is_test": payload.is_test
        });
        let url = format!("{}/3/device/{token}", environment.host());
        let response = self
            .client
            .post(url)
            .bearer_auth(jwt)
            .header("apns-topic", topic)
            .header("apns-push-type", "alert")
            .header("apns-priority", "10")
            .json(&body)
            .send()
            .await
            .map_err(|error| format!("APNs provider request failed: {error}"))?;
        let status = response.status();
        let response_body = response
            .text()
            .await
            .map_err(|error| format!("APNs provider response read failed: {error}"))?;
        if status.is_success() {
            return Ok(ApnsProviderResult {
                accepted: true,
                status_code: status.as_u16(),
                reason: None,
                retryable: false,
            });
        }
        let reason = serde_json::from_str::<serde_json::Value>(&response_body)
            .ok()
            .and_then(|value| {
                value
                    .get("reason")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned)
            })
            .unwrap_or_else(|| "provider_rejected".to_string());
        Ok(ApnsProviderResult {
            accepted: false,
            status_code: status.as_u16(),
            reason: Some(reason),
            retryable: status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error(),
        })
    }

    fn jwt(&self, now_epoch_s: i64) -> String {
        if let Ok(cache) = self.cached_jwt.lock()
            && let Some((issued_at, token)) = cache.as_ref()
            && now_epoch_s.saturating_sub(*issued_at) < 900
        {
            return token.clone();
        }
        let header = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(format!(
            r#"{{"alg":"ES256","kid":"{}"}}"#,
            self.config.key_id
        ));
        let claims = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(format!(
            r#"{{"iss":"{}","iat":{now_epoch_s}}}"#,
            self.config.team_id
        ));
        let signing_input = format!("{header}.{claims}");
        let signature: p256::ecdsa::Signature = self.signing_key.sign(signing_input.as_bytes());
        let encoded_signature =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(signature.to_bytes());
        let token = format!("{signing_input}.{encoded_signature}");
        if let Ok(mut cache) = self.cached_jwt.lock() {
            *cache = Some((now_epoch_s, token.clone()));
        }
        token
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ApnsState {
    NotConfigured,
    Unknown,
    Ready,
    Attempted,
    Accepted,
    Delivered,
    Displayed,
    Acknowledged,
    Rejected,
    Revoked,
    Expired,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ApnsPayload {
    pub message_id: String,
    pub payload_class: String,
    pub category: Option<String>,
    pub deep_link: Option<String>,
    pub is_test: bool,
}

impl ApnsPayload {
    /// Construct a payload after enforcing the minimal-field limits.
    ///
    /// # Errors
    ///
    /// Returns an error naming the field that violates the payload policy.
    pub fn new(
        message_id: String,
        payload_class: String,
        category: Option<String>,
        deep_link: Option<String>,
        is_test: bool,
    ) -> Result<Self, String> {
        if message_id.trim().is_empty() {
            return Err("APNs payload blocked: message_id=missing".to_string());
        }
        if payload_class.trim().is_empty() {
            return Err("APNs payload blocked: payload_class=missing".to_string());
        }
        if category.as_deref().is_some_and(|value| value.len() > 128) {
            return Err("APNs payload blocked: category exceeds 128 bytes".to_string());
        }
        if deep_link.as_deref().is_some_and(|value| value.len() > 512) {
            return Err("APNs payload blocked: deep_link exceeds 512 bytes".to_string());
        }
        Ok(Self {
            message_id,
            payload_class,
            category,
            deep_link,
            is_test,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::expect_used)]
    fn provider_debug_redacts_secret_path_and_identifiers() {
        let config = ApnsProviderConfig::from_values(
            "production",
            "team-secret",
            "key-secret",
            "com.example.app",
            "/private/key.p8",
        )
        .expect("valid provider config");
        let debug = format!("{config:?}");
        assert!(!debug.contains("team-secret"));
        assert!(!debug.contains("key-secret"));
        assert!(!debug.contains("/private/key.p8"));
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn missing_provider_fields_fail_closed() {
        let error = ApnsProviderConfig::from_values("production", "", "key", "bundle", "/key")
            .expect_err("empty team id must block provider setup");
        assert!(error.contains("field=team_id"));
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn token_protector_returns_ciphertext_and_fingerprint_without_plaintext() {
        let protector =
            ApnsTokenProtector::from_base64("MDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDA=")
                .expect("valid token key");
        let (ciphertext, fingerprint) = protector.seal("a1b2c3d4").expect("token encryption");
        assert_ne!(ciphertext, b"a1b2c3d4");
        assert_eq!(fingerprint.len(), 64);
        assert!(!ciphertext.windows(8).any(|window| window == b"a1b2c3d4"));
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn payload_has_only_non_sensitive_fields() {
        let payload = ApnsPayload::new(
            "message-1".to_string(),
            "operator_message".to_string(),
            Some("protected".to_string()),
            Some("anxietywatch://message/1".to_string()),
            true,
        )
        .expect("minimal payload");
        let value = serde_json::to_value(payload).expect("payload JSON");
        assert!(value.get("journal").is_none());
        assert!(value.get("healthkit").is_none());
        assert!(value.get("medication").is_none());
    }
}
