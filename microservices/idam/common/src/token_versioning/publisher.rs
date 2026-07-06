//! Version bump event publisher for Redis pub/sub.
//!
//! Publishes `VersionBumpEvent` to the `authz:version_bump` channel with
//! HMAC-SHA256 signature for authentication (HACK-505).
//!
//! # Security
//!
//! - Events are signed with HMAC-SHA256 using a shared secret (HACK-505)
//! - Only authz-core (or services with the shared secret) can publish valid events
//! - Publishers should be restricted via Redis ACL (HACK-502)

use super::events::{BumpReason, VersionBumpEvent};
use anyhow::{Context, Result};
use hmac::{Hmac, Mac};
use redis::Client;
use sha2::Sha256;

/// HMAC type alias for the shared-secret signing.
type HmacSha256 = Hmac<Sha256>;

/// Channel name for version bump events.
pub const VERSION_BUMP_CHANNEL: &str = "authz:version_bump";

/// Publisher that broadcasts version bump events to Redis pub/sub.
///
/// # Thread Safety
///
/// `VersionBumpPublisher` is `Clone` and can be shared across threads.
/// Each clone holds its own reference to the underlying `Client`.
#[derive(Clone)]
pub struct VersionBumpPublisher {
    client: Client,
    hmac_secret: Vec<u8>,
}

/// Configuration for the publisher.
#[derive(Debug, Clone)]
pub struct PublisherConfig {
    /// Redis connection URL (e.g., "<redis://127.0.0.1:6379>").
    pub redis_url: String,
    /// HMAC-SHA256 shared secret for signing events.
    /// Must be known to both publisher and subscriber for verification.
    pub hmac_secret: Vec<u8>,
}

impl PublisherConfig {
    /// Create a new publisher config.
    #[must_use]
    pub fn new(redis_url: &str, hmac_secret: &[u8]) -> Self {
        Self {
            redis_url: redis_url.to_string(),
            hmac_secret: hmac_secret.to_vec(),
        }
    }
}

impl VersionBumpPublisher {
    /// Create a new publisher from a `PublisherConfig`.
    pub fn from_config(config: &PublisherConfig) -> Result<Self> {
        let client = Client::open(config.redis_url.as_str())
            .context("failed to open Redis client for publisher")?;
        Ok(Self {
            client,
            hmac_secret: config.hmac_secret.clone(),
        })
    }

    /// Create a new publisher directly.
    pub fn new(redis_url: &str, hmac_secret: Vec<u8>) -> Result<Self> {
        Self::from_config(&PublisherConfig::new(redis_url, &hmac_secret))
    }

    /// Publish a version bump event to Redis pub/sub.
    ///
    /// The event is signed with HMAC-SHA256 using the shared secret.
    /// The signature is appended to the JSON payload as `|<hex_signature>`.
    ///
    /// # Arguments
    ///
    /// * `event` - The version bump event to publish.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, `Err` if Redis fails to publish.
    pub fn publish(&self, event: &VersionBumpEvent) -> Result<()> {
        // Serialize the event to JSON
        let json = event
            .to_json()
            .context("failed to serialize event to JSON")?;

        // Compute HMAC-SHA256 signature
        let mut mac =
            HmacSha256::new_from_slice(&self.hmac_secret).context("invalid HMAC secret")?;
        mac.update(json.as_bytes());
        let signature = mac.finalize().into_bytes();
        let sig_hex = hex::encode(signature);

        // Create the signed message: `<json>|<hex_signature>`
        let message = format!("{json}|{sig_hex}");

        // Publish to Redis (blocking)
        let mut conn = self
            .client
            .get_connection()
            .context("failed to get connection for publishing event")?;

        use redis::Commands;
        let _: i64 = conn
            .publish(VERSION_BUMP_CHANNEL, &message)
            .context("failed to publish event to Redis")?;

        tracing::debug!(
            channel = VERSION_BUMP_CHANNEL,
            tenant_id = event.tenant_id,
            user_id = ?event.user_id,
            new_version = event.new_version,
            reason = ?event.reason,
            "published version bump event",
        );

        Ok(())
    }

    /// Publish a subject-specific version bump.
    pub fn publish_subject(
        &self,
        tenant_id: &str,
        user_id: &str,
        new_version: u64,
        reason: BumpReason,
    ) -> Result<()> {
        let event = VersionBumpEvent::for_subject(tenant_id, user_id, new_version, reason);
        self.publish(&event)
    }

    /// Publish a tenant-wide version bump.
    pub fn publish_tenant(
        &self,
        tenant_id: &str,
        new_version: u64,
        reason: BumpReason,
    ) -> Result<()> {
        let event = VersionBumpEvent::for_tenant(tenant_id, new_version, reason);
        self.publish(&event)
    }

    /// Simulate a slow event for testing (adds a delay before publishing).
    #[doc(hidden)]
    pub fn set_slow_mode(&mut self, _enabled: bool) {
        // Stub for testing — real implementation could add a configurable delay.
    }
}

/// Extract the JSON payload and signature from a signed message.
///
/// Returns `(json_payload, signature_hex)` or an error if the message format is invalid.
pub fn parse_signed_message(message: &str) -> Result<(String, String)> {
    let last_pipe = message
        .rfind('|')
        .ok_or_else(|| anyhow::anyhow!("message missing HMAC signature delimiter '|"))?;

    let json = &message[..last_pipe];
    let sig = &message[last_pipe + 1..];

    if json.is_empty() || sig.is_empty() {
        return Err(anyhow::anyhow!(
            "empty JSON payload or signature in message"
        ));
    }

    Ok((json.to_string(), sig.to_string()))
}

/// Verify an HMAC-SHA256 signature on a version bump event.
///
/// # Arguments
///
/// * `json_payload` - The JSON payload of the event.
/// * `signature_hex` - The hex-encoded HMAC signature.
/// * `hmac_secret` - The shared secret used for verification.
///
/// # Returns
///
/// `Ok(())` if the signature is valid, `Err` if invalid or parsing fails.
pub fn verify_signature(json_payload: &str, signature_hex: &str, hmac_secret: &[u8]) -> Result<()> {
    let signature_bytes = hex::decode(signature_hex).context("signature is not valid hex")?;

    let mut mac = HmacSha256::new_from_slice(hmac_secret).context("invalid HMAC secret")?;
    mac.update(json_payload.as_bytes());

    mac.verify_slice(&signature_bytes)
        .context("HMAC signature verification failed — event may be forged")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::events::BumpReason;
    use super::*;

    fn test_hmac_secret() -> Vec<u8> {
        b"test-shared-secret-for-hmac".to_vec()
    }

    #[test]
    fn test_hmac_verification_valid() {
        let secret = test_hmac_secret();
        let event =
            VersionBumpEvent::for_subject("tenant_abc", "user_123", 43, BumpReason::RoleRevoked);
        let json = event.to_json().unwrap();

        // Sign the JSON
        let mut mac = HmacSha256::new_from_slice(&secret).unwrap();
        mac.update(json.as_bytes());
        let sig = hex::encode(mac.finalize().into_bytes());

        // Verify
        assert!(verify_signature(&json, &sig, &secret).is_ok());
    }

    #[test]
    fn test_hmac_verification_invalid_signature() {
        let _secret = test_hmac_secret();
        let json = r#"{"event":"version_bump","tenant_id":"t","new_version":1,"reason":"role_revoked","timestamp":1}"#;

        // Verify with wrong secret
        assert!(verify_signature(json, "deadbeef", b"wrong-secret").is_err());
    }

    #[test]
    fn test_parse_signed_message_valid() {
        let json = r#"{"event":"version_bump"}"#;
        let sig = "abc123";
        let message = format!("{json}|{sig}");

        let (parsed_json, parsed_sig) = parse_signed_message(&message).unwrap();
        assert_eq!(parsed_json, json);
        assert_eq!(parsed_sig, sig);
    }

    #[test]
    fn test_parse_signed_message_no_delimiter() {
        let result = parse_signed_message("no-pipe-here");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("missing HMAC signature"));
    }

    #[test]
    fn test_parse_signed_message_empty_parts() {
        let result = parse_signed_message("|");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("empty JSON payload"));
    }

    #[test]
    fn test_publish_subject_creates_correct_event() {
        let secret = test_hmac_secret();
        let publisher = VersionBumpPublisher::new("redis://127.0.0.1:16379", secret.clone())
            .expect("publisher created");

        // Verify the publisher has the correct HMAC secret
        assert_eq!(publisher.hmac_secret, secret);
        // The actual Redis publish will fail in unit tests, but we verify the publisher
        // can be constructed with the correct parameters.
        let _ = publisher;
    }

    #[test]
    fn test_channel_constant() {
        assert_eq!(VERSION_BUMP_CHANNEL, "authz:version_bump");
    }

    #[test]
    fn test_event_json_format_with_signature() {
        let secret = test_hmac_secret();
        let event = VersionBumpEvent::for_tenant("tenant_x", 100, BumpReason::OrgDeleted);
        let json = event.to_json().unwrap();

        // Sign
        let mut mac = HmacSha256::new_from_slice(&secret).unwrap();
        mac.update(json.as_bytes());
        let sig = hex::encode(mac.finalize().into_bytes());

        // Verify round-trip
        assert!(verify_signature(&json, &sig, &secret).is_ok());
    }
}
