//! `K_enroll` in Cloud KMS: an HMAC-SHA256 key at HSM protection level, used through `macSign`.
//!
//! The key never leaves KMS. Each epoch is a key version; the configuration names the current version and,
//! during rotation, the previous one. The workloads authenticate as themselves (Workload Identity) and need only
//! `cloudkms.cryptoKeyVersions.useToSign` on this one key (`roles/cloudkms.signer`). The client and its integrity
//! checks are [`crate::gcp_kms`].

use std::fmt;

use super::keyring::{EnrollKeyring, KeyringError, MAC_LEN};
use crate::gcp_kms::{
    is_key_name, KmsClient, KmsError, KmsSettings, DEFAULT_ENDPOINT, DEFAULT_TOKEN_URL,
};

fn default_endpoint() -> String {
    DEFAULT_ENDPOINT.to_string()
}
fn default_token_url() -> String {
    DEFAULT_TOKEN_URL.to_string()
}
const fn default_timeout_ms() -> u64 {
    3000
}
const fn default_true() -> bool {
    true
}

/// Cloud KMS keyring configuration.
#[derive(Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GcpKmsConfig {
    /// `projects/<p>/locations/<l>/keyRings/<r>/cryptoKeys/<k>` (purpose MAC, algorithm `HMAC_SHA256`).
    pub key: String,
    /// Key version new records are issued under (the current epoch).
    pub current_version: u32,
    /// Previous key version, accepted during rotation.
    #[serde(default)]
    pub previous_version: Option<u32>,
    /// Refuse answers from a key version that is not HSM-protected (default true).
    #[serde(default = "default_true")]
    pub require_hsm: bool,
    /// KMS API endpoint (tests point this at a local server).
    #[serde(default = "default_endpoint")]
    pub endpoint: String,
    /// Where the workload's access token comes from (the GKE metadata server).
    #[serde(default = "default_token_url")]
    pub token_url: String,
    /// Per-request timeout.
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
}

impl fmt::Debug for GcpKmsConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GcpKmsConfig")
            .field("key", &self.key)
            .field("current_version", &self.current_version)
            .field("previous_version", &self.previous_version)
            .field("require_hsm", &self.require_hsm)
            .field("endpoint", &self.endpoint)
            .finish_non_exhaustive()
    }
}

/// HMAC through Cloud KMS `macSign`.
#[derive(Debug)]
pub struct GcpKmsKeyring {
    key: String,
    current_version: u32,
    previous_version: Option<u32>,
    client: KmsClient,
}

fn keyring_error(e: KmsError) -> KeyringError {
    match e {
        KmsError::Malformed(why) => KeyringError::Malformed(why),
        KmsError::Unavailable(why) => KeyringError::Unavailable(why),
        KmsError::Integrity(why) => KeyringError::Integrity(why),
    }
}

impl GcpKmsKeyring {
    /// Validate the configuration and build the keyring. No network call is made until the first MAC.
    ///
    /// # Errors
    ///
    /// [`KeyringError::Malformed`] for a bad key name or URL, [`KeyringError::EpochClash`] for equal versions.
    pub fn new(config: GcpKmsConfig) -> Result<Self, KeyringError> {
        if !is_key_name(&config.key) {
            return Err(KeyringError::Malformed("KMS key name"));
        }
        if config.current_version == 0 || config.previous_version == Some(0) {
            return Err(KeyringError::Malformed("KMS key versions start at 1"));
        }
        if config.previous_version == Some(config.current_version) {
            return Err(KeyringError::EpochClash);
        }
        let client = KmsClient::new(KmsSettings {
            require_hsm: config.require_hsm,
            endpoint: config.endpoint,
            token_url: config.token_url,
            timeout_ms: config.timeout_ms,
        })
        .map_err(keyring_error)?;
        Ok(Self {
            key: config.key,
            current_version: config.current_version,
            previous_version: config.previous_version,
            client,
        })
    }
}

impl EnrollKeyring for GcpKmsKeyring {
    fn current_epoch(&self) -> u32 {
        self.current_version
    }

    fn accepted_epochs(&self) -> Vec<u32> {
        std::iter::once(self.current_version)
            .chain(self.previous_version)
            .collect()
    }

    fn mac(&self, epoch: u32, message: &[u8]) -> Result<[u8; MAC_LEN], KeyringError> {
        if !self.accepted_epochs().contains(&epoch) {
            return Err(KeyringError::UnknownEpoch(epoch));
        }
        let name = format!("{}/cryptoKeyVersions/{epoch}", self.key);
        let mac = self
            .client
            .mac_sign(&name, message)
            .map_err(keyring_error)?;
        mac.try_into()
            .map_err(|_| KeyringError::Integrity("mac is not 32 bytes"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gcp_kms::fake;
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    const KEY: &str =
        "projects/sesame-test/locations/europe-west1/keyRings/sesame/cryptoKeys/domain-verify";

    fn test_mac(version: &str, data: &[u8]) -> Vec<u8> {
        let mut mac = Hmac::<Sha256>::new_from_slice(format!("test-key-for-{version}").as_bytes())
            .expect("key");
        mac.update(data);
        mac.finalize().into_bytes().to_vec()
    }

    fn keyring(base: &str) -> GcpKmsKeyring {
        let s = fake::settings(base);
        GcpKmsKeyring::new(GcpKmsConfig {
            key: KEY.to_string(),
            current_version: 2,
            previous_version: Some(1),
            require_hsm: s.require_hsm,
            endpoint: s.endpoint,
            token_url: s.token_url,
            timeout_ms: s.timeout_ms,
        })
        .expect("config")
    }

    #[test]
    fn mac_per_version_through_kms() {
        let (base, calls) = fake::start(
            fake::Mode::Good,
            fake::Script {
                mac: Some(Box::new(test_mac)),
                ..fake::Script::default()
            },
        );
        let k = keyring(&base);
        assert_eq!(k.accepted_epochs(), vec![2, 1]);
        let v2 = test_mac(&format!("{KEY}/cryptoKeyVersions/2"), b"hello");
        let v1 = test_mac(&format!("{KEY}/cryptoKeyVersions/1"), b"hello");
        assert_eq!(k.mac(2, b"hello").map(|m| m.to_vec()), Ok(v2.clone()));
        assert_eq!(k.mac(1, b"hello").map(|m| m.to_vec()), Ok(v1.clone()));
        assert_ne!(v1, v2);
        assert_eq!(k.mac(3, b"hello"), Err(KeyringError::UnknownEpoch(3)));
        assert_eq!(*calls.lock().expect("calls"), 2);
    }

    #[test]
    fn kms_failures_map_to_keyring_errors() {
        let (base, _) = fake::start(
            fake::Mode::Software,
            fake::Script {
                mac: Some(Box::new(test_mac)),
                ..fake::Script::default()
            },
        );
        assert_eq!(
            keyring(&base).mac(2, b"m"),
            Err(KeyringError::Integrity("key version is not HSM-protected"))
        );
        let (base, _) = fake::start(fake::Mode::Denied, fake::Script::default());
        assert_eq!(
            keyring(&base).mac(2, b"m"),
            Err(KeyringError::Unavailable("KMS status 403".to_string()))
        );
    }

    #[test]
    fn configuration_is_validated() {
        let base = GcpKmsConfig {
            key: KEY.to_string(),
            current_version: 1,
            previous_version: None,
            require_hsm: true,
            endpoint: DEFAULT_ENDPOINT.to_string(),
            token_url: DEFAULT_TOKEN_URL.to_string(),
            timeout_ms: 3000,
        };
        assert!(GcpKmsKeyring::new(base.clone()).is_ok());
        let bad = |f: &dyn Fn(&mut GcpKmsConfig)| {
            let mut c = base.clone();
            f(&mut c);
            GcpKmsKeyring::new(c).is_err()
        };
        assert!(bad(&|c| c.key = "projects/x/keyRings/r".to_string()));
        assert!(bad(&|c| c.key = format!("{KEY}/cryptoKeyVersions/1")));
        assert!(bad(&|c| c.current_version = 0));
        assert!(bad(&|c| c.previous_version = Some(1)));
        assert!(bad(
            &|c| c.endpoint = "http://cloudkms.googleapis.com".to_string()
        ));
        assert!(bad(
            &|c| c.token_url = "http://evil.example.com/token".to_string()
        ));
    }
}
