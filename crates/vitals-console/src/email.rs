//! Gmail STARTTLS notification channel.
//!
//! The channel is deliberately independent from the alarm path. Credentials
//! are encrypted before persistence, and the public status contains only
//! presence and last-four metadata.

use crate::crypto;
use crate::silence::NotificationEvent;
use async_trait::async_trait;
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use serde::Serialize;
use sqlx::PgPool;
use std::collections::BTreeMap;
use std::fmt;

/// Gmail's fixed endpoint; it is not operator-editable.
pub const SMTP_HOST: &str = "smtp.gmail.com";
/// Gmail submission port for STARTTLS.
pub const SMTP_PORT: u16 = 587;
const APP_PASSWORD_LENGTH: usize = 16;
/// Minimum interval between outbound digest deliveries.
pub const EMAIL_MIN_INTERVAL_S: i64 = 900;

/// Encrypted app password plus non-secret display metadata.
pub struct EncryptedAppPassword {
    nonce: [u8; 24],
    ciphertext: Vec<u8>,
    last_four: String,
}

impl fmt::Debug for EncryptedAppPassword {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EncryptedAppPassword")
            .field("configured", &true)
            .field("last_four", &self.last_four)
            .finish_non_exhaustive()
    }
}

impl EncryptedAppPassword {
    /// Encrypt a Gmail app password using the appliance credential key.
    ///
    /// Whitespace in the commonly displayed `xxxx xxxx xxxx xxxx` form is
    /// removed before validation and encryption.
    ///
    /// # Errors
    /// Returns an error for a malformed app password or random/encryption
    /// failure.
    pub fn seal(password: &str, key: &[u8; 32]) -> Result<Self, String> {
        let normalized = password
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect::<String>();
        if normalized.len() != APP_PASSWORD_LENGTH
            || !normalized
                .chars()
                .all(|character| character.is_ascii_alphanumeric())
        {
            return Err(
                "Gmail app password must contain exactly 16 ASCII letters or digits".to_string(),
            );
        }
        let mut nonce = [0_u8; 24];
        getrandom::getrandom(&mut nonce)
            .map_err(|error| format!("credential nonce generation failed: {error}"))?;
        let ciphertext = crypto::encrypt(normalized.as_bytes(), key, &nonce)?;
        let last_four = normalized.chars().skip(APP_PASSWORD_LENGTH - 4).collect();
        Ok(Self {
            nonce,
            ciphertext,
            last_four,
        })
    }

    fn open(&self, key: &[u8; 32]) -> Result<String, String> {
        let plaintext = crypto::decrypt(&self.ciphertext, key, &self.nonce)?;
        String::from_utf8(plaintext)
            .map_err(|error| format!("stored app password is not UTF-8: {error}"))
    }

    /// Whether a credential is configured.
    #[must_use]
    pub const fn configured(&self) -> bool {
        true
    }

    /// Non-secret last-four display value.
    #[must_use]
    pub fn last_four(&self) -> &str {
        &self.last_four
    }

    /// Ciphertext for a database row; never plaintext.
    #[must_use]
    pub fn ciphertext(&self) -> &[u8] {
        &self.ciphertext
    }

    /// Non-secret nonce for a database row.
    #[must_use]
    pub const fn nonce(&self) -> &[u8; 24] {
        &self.nonce
    }

    /// Rebuild a credential loaded from its encrypted database columns.
    ///
    /// # Errors
    /// Rejects malformed nonce, empty ciphertext, or invalid display metadata.
    pub fn from_storage(
        nonce: &[u8],
        ciphertext: Vec<u8>,
        last_four: String,
    ) -> Result<Self, String> {
        let nonce: [u8; 24] = nonce
            .try_into()
            .map_err(|_| "stored app-password nonce is not 24 bytes".to_string())?;
        if ciphertext.is_empty() || last_four.len() != 4 {
            return Err("stored app-password metadata is incomplete".to_string());
        }
        Ok(Self {
            nonce,
            ciphertext,
            last_four,
        })
    }
}

/// Persisted operator configuration. The secret itself is intentionally not
/// serializable or publicly readable.
pub struct EmailConfiguration {
    /// Whether outbound email is enabled; it defaults to false.
    pub enabled: bool,
    /// Gmail account used as the sender.
    pub from_address: String,
    /// Destination for operational notifications.
    pub destination_address: String,
    /// Encrypted app password, if configured.
    pub app_password: Option<EncryptedAppPassword>,
}

/// Last test/send result shown by the console.
#[derive(Debug, Clone, Serialize)]
pub struct EmailTestResult {
    /// Whether the SMTP operation succeeded.
    pub succeeded: bool,
    /// UTC epoch seconds at which the attempt completed.
    pub completed_at_epoch_s: i64,
    /// Sanitized operator-safe detail.
    pub detail: String,
}

/// Channel state. Configured-but-never-tested is explicitly `Unknown`.
#[derive(Debug, Clone, Serialize)]
pub struct EmailStatus {
    /// `Live`, `Failing`, or `Unknown`.
    pub state: String,
    /// Whether the channel is enabled.
    pub enabled: bool,
    /// Whether an app password exists.
    pub credential_configured: bool,
    /// App-password last four, never the secret.
    pub credential_last_four: Option<String>,
    /// Fixed SMTP host.
    pub smtp_host: &'static str,
    /// Fixed SMTP port.
    pub smtp_port: u16,
    /// Last test result.
    pub last_test: Option<EmailTestResult>,
    /// Why the channel is not live.
    pub reason: String,
}

/// Coalesces detector events by transition identity and rate-limits delivery.
#[derive(Debug, Default)]
pub struct EmailCoalescer {
    pending: BTreeMap<String, NotificationEvent>,
    last_sent_at_epoch_s: Option<i64>,
}

impl EmailCoalescer {
    /// Queue one unsuppressed event; a later reason supersedes an older repeat.
    pub fn enqueue(&mut self, event: NotificationEvent) {
        if !event.suppressed_by_mute {
            self.pending.insert(event.transition_id.clone(), event);
        }
    }

    /// Take one bounded digest when the rate limit permits.
    pub fn take_due(&mut self, now_epoch_s: i64) -> Option<Vec<NotificationEvent>> {
        let ready = self
            .last_sent_at_epoch_s
            .is_none_or(|last| now_epoch_s.saturating_sub(last) >= EMAIL_MIN_INTERVAL_S);
        if !ready || self.pending.is_empty() {
            return None;
        }
        self.last_sent_at_epoch_s = Some(now_epoch_s);
        Some(self.pending.split_off("").into_values().collect())
    }
}

/// A protocol-level SMTP client seam used by the sender and tests.
#[async_trait]
pub trait SmtpClient: Send + Sync {
    /// Send one message using the fixed STARTTLS transport.
    async fn send(
        &self,
        from: &str,
        destination: &str,
        app_password: &str,
        subject: &str,
        body: &str,
    ) -> Result<(), String>;
}

/// Production Gmail STARTTLS client.
#[derive(Debug, Default, Clone, Copy)]
pub struct GmailSmtpClient;

#[async_trait]
impl SmtpClient for GmailSmtpClient {
    async fn send(
        &self,
        from: &str,
        destination: &str,
        app_password: &str,
        subject: &str,
        body: &str,
    ) -> Result<(), String> {
        let from: Mailbox = from
            .parse()
            .map_err(|error| format!("invalid sender address: {error}"))?;
        let destination: Mailbox = destination
            .parse()
            .map_err(|error| format!("invalid destination address: {error}"))?;
        let username = from.email.to_string();
        let message = Message::builder()
            .from(from)
            .to(destination)
            .subject(subject)
            .body(body.to_string())
            .map_err(|error| format!("email message construction failed: {error}"))?;
        let transport = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(SMTP_HOST)
            .map_err(|error| format!("Gmail STARTTLS transport setup failed: {error}"))?
            .port(SMTP_PORT)
            .credentials(Credentials::new(username, app_password.to_string()))
            .build();
        transport
            .send(message)
            .await
            .map(|_| ())
            .map_err(|error| format!("Gmail STARTTLS send failed: {error}"))
    }
}

/// Injectable sender for test email and future notification delivery.
pub struct EmailSender<C> {
    client: C,
    key: [u8; 32],
}

impl<C: SmtpClient> EmailSender<C> {
    /// Construct a sender with the appliance's encryption key.
    #[must_use]
    pub const fn new(client: C, key: [u8; 32]) -> Self {
        Self { client, key }
    }

    /// Send a protocol-level test message without exposing the password.
    ///
    /// Configuration, decryption, address validation, and SMTP failures are
    /// returned as a failed result with sanitized detail.
    pub async fn send_test(
        &self,
        configuration: &EmailConfiguration,
        completed_at_epoch_s: i64,
    ) -> EmailTestResult {
        let result = async {
            if !configuration.enabled {
                return Err("outbound email is disabled".to_string());
            }
            if configuration.from_address.trim().is_empty()
                || configuration.destination_address.trim().is_empty()
            {
                return Err("sender and destination addresses are required".to_string());
            }
            let credential = configuration
                .app_password
                .as_ref()
                .ok_or_else(|| "Gmail app password is not configured".to_string())?;
            let password = credential.open(&self.key)?;
            self.client
                .send(
                    &configuration.from_address,
                    &configuration.destination_address,
                    &password,
                    "AnxietyWatch console email test",
                    "This is an AnxietyWatch operational email test.",
                )
                .await
        }
        .await;
        match result {
            Ok(()) => EmailTestResult {
                succeeded: true,
                completed_at_epoch_s,
                detail: "SMTP test succeeded via smtp.gmail.com:587 STARTTLS".to_string(),
            },
            Err(error) => EmailTestResult {
                succeeded: false,
                completed_at_epoch_s,
                detail: crate::api::ops::sanitize_detail(&error, None),
            },
        }
    }
}

/// Send a test after creating its attempt row, then finish that row.
///
/// The insert is deliberately before any SMTP work. A process killed during
/// delivery therefore remains an in-flight attempt and can degrade to
/// `UNKNOWN` through the shared liveness rules.
///
/// # Errors
/// Returns a database error when the attempt cannot be created or finished.
pub async fn send_test_recorded<C: SmtpClient>(
    pool: &PgPool,
    sender: &EmailSender<C>,
    configuration: &EmailConfiguration,
    completed_at_epoch_s: i64,
) -> Result<EmailTestResult, String> {
    let (attempt_id,): (i64,) = sqlx::query_as(
        "INSERT INTO attempts (kind, subject, expected_duration_s) \
         VALUES ('email_test', 'outbound_email', 60) RETURNING id",
    )
    .fetch_one(pool)
    .await
    .map_err(|error| format!("email test attempt could not be recorded: {error}"))?;
    let result = sender.send_test(configuration, completed_at_epoch_s).await;
    let outcome = if result.succeeded {
        "succeeded"
    } else {
        "failed"
    };
    sqlx::query(
        "UPDATE attempts SET finished_at = to_timestamp($2), outcome = $3, \
         effect = '{}'::jsonb, detail = $4 WHERE id = $1",
    )
    .bind(attempt_id)
    .bind(completed_at_epoch_s)
    .bind(outcome)
    .bind(&result.detail)
    .execute(pool)
    .await
    .map_err(|error| format!("email test attempt could not be finished: {error}"))?;
    Ok(result)
}

#[derive(Debug, sqlx::FromRow)]
struct StoredEmailConfiguration {
    enabled: bool,
    from_address: Option<String>,
    destination_address: Option<String>,
    password_nonce: Option<Vec<u8>>,
    password_ciphertext: Option<Vec<u8>>,
    password_last_four: Option<String>,
}

/// Load encrypted email configuration without ever selecting plaintext.
///
/// # Errors
/// Returns an error when the database row is malformed or unavailable.
pub async fn load_configuration(pool: &PgPool) -> Result<Option<EmailConfiguration>, String> {
    let row = sqlx::query_as::<_, StoredEmailConfiguration>(
        "SELECT enabled, from_address, destination_address, password_nonce, \
         password_ciphertext, password_last_four FROM email_channel_config \
         WHERE singleton = TRUE",
    )
    .fetch_optional(pool)
    .await
    .map_err(|error| format!("email configuration load failed: {error}"))?;
    row.map(|row| {
        let app_password = match (
            row.password_nonce,
            row.password_ciphertext,
            row.password_last_four,
        ) {
            (None, None, None) => Ok(None),
            (Some(nonce), Some(ciphertext), Some(last_four)) => {
                EncryptedAppPassword::from_storage(&nonce, ciphertext, last_four).map(Some)
            }
            _ => Err("email configuration has incomplete encrypted credential columns".to_string()),
        }?;
        Ok(EmailConfiguration {
            enabled: row.enabled,
            from_address: row.from_address.unwrap_or_default(),
            destination_address: row.destination_address.unwrap_or_default(),
            app_password,
        })
    })
    .transpose()
}

/// Store addresses and encrypted credential metadata; never store plaintext.
///
/// # Errors
/// Returns an error when the configuration row cannot be written.
pub async fn save_configuration(
    pool: &PgPool,
    configuration: &EmailConfiguration,
) -> Result<(), String> {
    let (password_nonce, password_ciphertext, password_last_four) = configuration
        .app_password
        .as_ref()
        .map_or((None, None, None), |credential| {
            (
                Some(credential.nonce().to_vec()),
                Some(credential.ciphertext().to_vec()),
                Some(credential.last_four().to_string()),
            )
        });
    sqlx::query(
        "INSERT INTO email_channel_config \
         (singleton, enabled, from_address, destination_address, password_nonce, \
          password_ciphertext, password_last_four) VALUES (TRUE, $1, $2, $3, $4, $5, $6) \
         ON CONFLICT (singleton) DO UPDATE SET enabled = EXCLUDED.enabled, \
          from_address = EXCLUDED.from_address, destination_address = EXCLUDED.destination_address, \
          password_nonce = EXCLUDED.password_nonce, password_ciphertext = EXCLUDED.password_ciphertext, \
          password_last_four = EXCLUDED.password_last_four",
    )
    .bind(configuration.enabled)
    .bind(&configuration.from_address)
    .bind(&configuration.destination_address)
    .bind(password_nonce)
    .bind(password_ciphertext)
    .bind(password_last_four)
    .execute(pool)
    .await
    .map(|_| ())
    .map_err(|error| format!("email configuration save failed: {error}"))
}

/// Resolve operator-facing channel state without revealing credentials.
#[must_use]
pub fn status(
    configuration: &EmailConfiguration,
    last_test: Option<EmailTestResult>,
) -> EmailStatus {
    let (state, reason) = if !configuration.enabled {
        ("Unknown", "outbound email is disabled")
    } else if configuration.app_password.is_none() {
        ("Unknown", "Gmail app password is not configured")
    } else {
        match last_test.as_ref() {
            None => ("Unknown", "configured but never tested"),
            Some(test) if test.succeeded => ("Live", "last SMTP test succeeded"),
            Some(_) => ("Failing", "last SMTP test failed"),
        }
    };
    EmailStatus {
        state: state.to_string(),
        enabled: configuration.enabled,
        credential_configured: configuration.app_password.is_some(),
        credential_last_four: configuration
            .app_password
            .as_ref()
            .map(|credential| credential.last_four().to_string()),
        smtp_host: SMTP_HOST,
        smtp_port: SMTP_PORT,
        last_test,
        reason: reason.to_string(),
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "email tests use fixed fixtures")]
    use super::*;
    use std::sync::{Arc, Mutex};
    use vitals_core::liveness::HealthState;
    use vitals_registry::VitalsSource;

    #[derive(Clone, Default)]
    struct FixtureClient {
        calls: Arc<Mutex<Vec<(String, String, String)>>>,
        failure: bool,
    }

    #[async_trait]
    impl SmtpClient for FixtureClient {
        async fn send(
            &self,
            from: &str,
            destination: &str,
            password: &str,
            _subject: &str,
            _body: &str,
        ) -> Result<(), String> {
            self.calls
                .lock()
                .map_err(|_| "fixture lock failed".to_string())?
                .push((
                    from.to_string(),
                    destination.to_string(),
                    password.to_string(),
                ));
            if self.failure {
                Err("fixture SMTP refused connection".to_string())
            } else {
                Ok(())
            }
        }
    }

    fn configuration(key: &[u8; 32]) -> EmailConfiguration {
        EmailConfiguration {
            enabled: true,
            from_address: "operator@example.test".to_string(),
            destination_address: "destination@example.test".to_string(),
            app_password: Some(EncryptedAppPassword::seal("abcd efgh ijkl mnop", key).unwrap()),
        }
    }

    #[test]
    fn email_password_is_encrypted_and_only_last_four_are_displayable() {
        let key = [7_u8; 32];
        let password = EncryptedAppPassword::seal("abcd efgh ijkl mnop", &key).unwrap();
        assert_ne!(password.ciphertext(), b"abcdefghijklmnop");
        assert_eq!(password.last_four(), "mnop");
        assert!(format!("{password:?}").contains("mnop"));
        assert!(!format!("{password:?}").contains("abcdefghijklmnop"));
        assert!(EncryptedAppPassword::seal("short", &key).is_err());
    }

    #[tokio::test]
    async fn email_test_uses_injected_starttls_fixture_and_never_returns_password() {
        let key = [9_u8; 32];
        let fixture = FixtureClient::default();
        let calls = fixture.calls.clone();
        let result = EmailSender::new(fixture, key)
            .send_test(&configuration(&key), 1_234)
            .await;
        assert!(result.succeeded);
        assert_eq!(result.completed_at_epoch_s, 1_234);
        let calls = calls.lock().unwrap();
        assert_eq!(
            calls.first().map(|call| call.2.as_str()),
            Some("abcdefghijklmnop")
        );
        assert!(!result.detail.contains("abcdefghijklmnop"));
    }

    #[test]
    fn email_configured_never_tested_is_unknown_and_failed_test_degrades() {
        let key = [3_u8; 32];
        let config = configuration(&key);
        let unknown = status(&config, None);
        assert_eq!(unknown.state, "Unknown");
        assert_eq!(unknown.reason, "configured but never tested");
        let failed = status(
            &config,
            Some(EmailTestResult {
                succeeded: false,
                completed_at_epoch_s: 55,
                detail: "refused".to_string(),
            }),
        );
        assert_eq!(failed.state, "Failing");
        assert_eq!(failed.credential_last_four.as_deref(), Some("mnop"));
    }

    #[test]
    fn email_coalesces_by_transition_and_rate_limits_digest_delivery() {
        let mut coalescer = EmailCoalescer::default();
        let event = NotificationEvent {
            source: VitalsSource::Emay,
            transition_id: "transition-1".to_string(),
            state: HealthState::Unknown,
            reason: "silent".to_string(),
            emitted_at_epoch_s: 10,
            suppressed_by_mute: false,
        };
        coalescer.enqueue(event.clone());
        coalescer.enqueue(NotificationEvent {
            reason: "still silent".to_string(),
            ..event
        });
        assert!(coalescer.take_due(100).is_some());
        assert!(coalescer.take_due(200).is_none());
        assert!(coalescer.take_due(1_000 + EMAIL_MIN_INTERVAL_S).is_none());
        coalescer.enqueue(NotificationEvent {
            transition_id: "transition-2".to_string(),
            ..NotificationEvent {
                source: VitalsSource::Emay,
                transition_id: "unused".to_string(),
                state: HealthState::Failing,
                reason: "failed".to_string(),
                emitted_at_epoch_s: 1,
                suppressed_by_mute: false,
            }
        });
        assert!(
            coalescer
                .take_due(1_000 + EMAIL_MIN_INTERVAL_S + 1)
                .is_some()
        );
    }

    #[tokio::test]
    async fn email_disabled_does_not_call_smtp() {
        let key = [1_u8; 32];
        let fixture = FixtureClient::default();
        let calls = fixture.calls.clone();
        let mut config = configuration(&key);
        config.enabled = false;
        let result = EmailSender::new(fixture, key).send_test(&config, 1).await;
        assert!(!result.succeeded);
        assert!(calls.lock().unwrap().is_empty());
    }
}
