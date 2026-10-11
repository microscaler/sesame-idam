//! The enrolment key `K_enroll`, wherever it lives.
//!
//! Tokens are HMAC-SHA256 tags computed by an [`EnrollKeyring`]. Production keeps the key in an HSM that
//! never releases it ([`super::GcpKmsKeyring`]: Cloud KMS, HSM protection level, `macSign`); development and
//! tests use [`LocalKeyring`], a key file. Epochs are key versions: records issued under the previous epoch keep
//! verifying until it is dropped from the configuration.

use std::fmt;
use std::path::{Path, PathBuf};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use hmac::{Hmac, Mac};
use sha2::Sha256;

use super::gcp_kms::{GcpKmsConfig, GcpKmsKeyring};

/// HMAC-SHA256 tag length.
pub const MAC_LEN: usize = 32;
/// Minimum local key length (256 bits).
const MIN_KEY_BYTES: usize = 32;
/// Largest key or configuration file accepted.
const MAX_FILE: u64 = 16 * 1024;

/// Computes HMAC-SHA256 tags under `K_enroll`.
pub trait EnrollKeyring: Send + Sync {
    /// Epoch new records are issued under.
    fn current_epoch(&self) -> u32;
    /// Epochs whose records still verify, current first.
    fn accepted_epochs(&self) -> Vec<u32>;
    /// HMAC-SHA256 of `message` under the key of `epoch`.
    ///
    /// # Errors
    ///
    /// [`KeyringError::UnknownEpoch`] for an epoch not configured; [`KeyringError::Unavailable`] or
    /// [`KeyringError::Integrity`] when a remote key service fails.
    fn mac(&self, epoch: u32, message: &[u8]) -> Result<[u8; MAC_LEN], KeyringError>;
}

/// Why the keyring could not be loaded or used. Never contains key material, tokens or file contents.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum KeyringError {
    /// A file could not be read (the I/O error kind only).
    #[error("keyring file unreadable: {0}")]
    Unreadable(String),
    /// A file or configuration is not the expected shape.
    #[error("keyring configuration malformed: {0}")]
    Malformed(&'static str),
    /// A local key shorter than 256 bits.
    #[error("enrolment key shorter than 32 bytes")]
    TooShort,
    /// Current and previous must be different epochs (and, locally, different keys).
    #[error("enrolment key epochs must differ")]
    EpochClash,
    /// A MAC was requested for an epoch that is not configured.
    #[error("epoch {0} is not configured")]
    UnknownEpoch(u32),
    /// The key service could not be reached or refused (status and reason only).
    #[error("key service unavailable: {0}")]
    Unavailable(String),
    /// The key service answered, but the answer failed integrity checks (CRC32C, key version, length).
    #[error("key service response failed integrity checks: {0}")]
    Integrity(&'static str),
}

/// Which keyring a deployment uses. Read from the file named by `SESAME_DOMAIN_VERIFY_KEYRING` (JSON).
///
/// ```json
/// {"backend": "gcp-kms", "key": "projects/p/locations/europe-west1/keyRings/sesame-pp/cryptoKeys/domain-verify",
///  "current_version": 2, "previous_version": 1}
/// {"backend": "file", "path": "/var/run/sesame/domain-verify-key.json"}
/// ```
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(tag = "backend", rename_all = "kebab-case", deny_unknown_fields)]
pub enum KeyringConfig {
    /// A local key file ([`LocalKeyring::from_file`]); development and tests.
    File {
        /// Path of the key file.
        path: PathBuf,
    },
    /// Cloud KMS HMAC key, normally at HSM protection level.
    GcpKms(GcpKmsConfig),
}

impl KeyringConfig {
    /// Parse the configuration JSON.
    ///
    /// # Errors
    ///
    /// [`KeyringError::Malformed`].
    pub fn from_json(bytes: &[u8]) -> Result<Self, KeyringError> {
        serde_json::from_slice(bytes).map_err(|_| KeyringError::Malformed("keyring configuration"))
    }

    /// Build the keyring this configuration describes.
    ///
    /// # Errors
    ///
    /// See [`KeyringError`].
    pub fn build(&self) -> Result<Box<dyn EnrollKeyring>, KeyringError> {
        match self {
            Self::File { path } => Ok(Box::new(LocalKeyring::from_file(path)?)),
            Self::GcpKms(config) => Ok(Box::new(GcpKmsKeyring::new(config.clone())?)),
        }
    }
}

/// Read a small file, refusing anything larger than 16 KiB; errors carry the I/O kind only.
pub(super) fn read_small_file(path: &Path) -> Result<Vec<u8>, KeyringError> {
    let meta =
        std::fs::metadata(path).map_err(|e| KeyringError::Unreadable(e.kind().to_string()))?;
    if meta.len() > MAX_FILE {
        return Err(KeyringError::Malformed("file too large"));
    }
    std::fs::read(path).map_err(|e| KeyringError::Unreadable(e.kind().to_string()))
}

struct EpochKey {
    epoch: u32,
    key: Vec<u8>,
}

/// `K_enroll` held in process memory, loaded from a file (a mounted secret). For development and tests, and
/// for deployments without an HSM-backed key service.
///
/// ```json
/// {"current": {"epoch": 2, "key": "<base64url, at least 32 bytes>"},
///  "previous": {"epoch": 1, "key": "<base64url>"}}
/// ```
///
/// `previous` is optional. `Debug` never shows key material, and load errors never quote the file.
pub struct LocalKeyring {
    current: EpochKey,
    previous: Option<EpochKey>,
}

impl fmt::Debug for LocalKeyring {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LocalKeyring")
            .field("current_epoch", &self.current.epoch)
            .field("previous_epoch", &self.previous.as_ref().map(|k| k.epoch))
            .field("keys", &"<redacted>")
            .finish()
    }
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct EpochKeyFile {
    epoch: u32,
    key: String,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct LocalKeyFile {
    current: EpochKeyFile,
    previous: Option<EpochKeyFile>,
}

fn decode_key(file: &EpochKeyFile) -> Result<EpochKey, KeyringError> {
    let key = URL_SAFE_NO_PAD
        .decode(file.key.trim_end_matches('='))
        .map_err(|_| KeyringError::Malformed("key is not base64url"))?;
    if key.len() < MIN_KEY_BYTES {
        return Err(KeyringError::TooShort);
    }
    Ok(EpochKey {
        epoch: file.epoch,
        key,
    })
}

impl LocalKeyring {
    /// Build from raw keys (tests and tools).
    ///
    /// # Errors
    ///
    /// [`KeyringError::TooShort`] or [`KeyringError::EpochClash`].
    pub fn new(
        current: (u32, Vec<u8>),
        previous: Option<(u32, Vec<u8>)>,
    ) -> Result<Self, KeyringError> {
        let make = |(epoch, key): (u32, Vec<u8>)| {
            if key.len() < MIN_KEY_BYTES {
                Err(KeyringError::TooShort)
            } else {
                Ok(EpochKey { epoch, key })
            }
        };
        let current = make(current)?;
        let previous = previous.map(make).transpose()?;
        Self::checked(current, previous)
    }

    fn checked(current: EpochKey, previous: Option<EpochKey>) -> Result<Self, KeyringError> {
        if let Some(prev) = &previous {
            if prev.epoch == current.epoch || prev.key == current.key {
                return Err(KeyringError::EpochClash);
            }
        }
        Ok(Self { current, previous })
    }

    /// Parse the JSON key file contents.
    ///
    /// # Errors
    ///
    /// See [`KeyringError`]; messages never quote the input.
    pub fn from_json(bytes: &[u8]) -> Result<Self, KeyringError> {
        let file: LocalKeyFile = serde_json::from_slice(bytes)
            .map_err(|_| KeyringError::Malformed("expected {current:{epoch,key}, previous?}"))?;
        let current = decode_key(&file.current)?;
        let previous = file.previous.as_ref().map(decode_key).transpose()?;
        Self::checked(current, previous)
    }

    /// Read and parse the key file.
    ///
    /// # Errors
    ///
    /// See [`KeyringError`].
    pub fn from_file(path: &Path) -> Result<Self, KeyringError> {
        Self::from_json(&read_small_file(path)?)
    }
}

impl EnrollKeyring for LocalKeyring {
    fn current_epoch(&self) -> u32 {
        self.current.epoch
    }

    fn accepted_epochs(&self) -> Vec<u32> {
        std::iter::once(self.current.epoch)
            .chain(self.previous.as_ref().map(|k| k.epoch))
            .collect()
    }

    fn mac(&self, epoch: u32, message: &[u8]) -> Result<[u8; MAC_LEN], KeyringError> {
        let key = std::iter::once(&self.current)
            .chain(self.previous.as_ref())
            .find(|k| k.epoch == epoch)
            .ok_or(KeyringError::UnknownEpoch(epoch))?;
        // HMAC accepts keys of any length; `new_from_slice` cannot fail for it.
        #[allow(clippy::expect_used)]
        let mut mac = Hmac::<Sha256>::new_from_slice(&key.key).expect("HMAC takes any key length");
        mac.update(message);
        Ok(mac.finalize().into_bytes().into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_key_file_parsing_and_redaction() {
        let k1 = URL_SAFE_NO_PAD.encode([0x11u8; 32]);
        let k2 = URL_SAFE_NO_PAD.encode([0x22u8; 32]);
        let json = format!(
            r#"{{"current":{{"epoch":2,"key":"{k2}"}},"previous":{{"epoch":1,"key":"{k1}"}}}}"#
        );
        let k = LocalKeyring::from_json(json.as_bytes()).expect("parses");
        assert_eq!(k.current_epoch(), 2);
        assert_eq!(k.accepted_epochs(), vec![2, 1]);
        assert_eq!(k.mac(3, b"x"), Err(KeyringError::UnknownEpoch(3)));
        let debug = format!("{k:?}");
        assert!(!debug.contains(&k1) && !debug.contains(&k2), "{debug}");
        assert!(debug.contains("redacted"));

        let short = URL_SAFE_NO_PAD.encode([0x33u8; 16]);
        let json = format!(r#"{{"current":{{"epoch":1,"key":"{short}"}}}}"#);
        assert_eq!(
            LocalKeyring::from_json(json.as_bytes()).err(),
            Some(KeyringError::TooShort)
        );

        let clash = format!(
            r#"{{"current":{{"epoch":1,"key":"{k1}"}},"previous":{{"epoch":1,"key":"{k2}"}}}}"#
        );
        assert_eq!(
            LocalKeyring::from_json(clash.as_bytes()).err(),
            Some(KeyringError::EpochClash)
        );

        // Errors never quote the input, even when it holds key material.
        let leaky = format!(r#"{{"current":{{"epoch":"x","key":"{k1}"}}}}"#);
        let err = LocalKeyring::from_json(leaky.as_bytes()).expect_err("malformed");
        assert!(!err.to_string().contains(&k1));
        let unknown = format!(r#"{{"current":{{"epoch":1,"key":"{k1}","note":"{k2}"}}}}"#);
        let err = LocalKeyring::from_json(unknown.as_bytes()).expect_err("unknown field");
        assert!(!err.to_string().contains(&k2));
    }

    #[test]
    fn configuration_selects_the_backend() {
        let file = KeyringConfig::from_json(br#"{"backend":"file","path":"/k.json"}"#).expect("ok");
        assert_eq!(
            file,
            KeyringConfig::File {
                path: PathBuf::from("/k.json")
            }
        );
        let kms = KeyringConfig::from_json(
            br#"{"backend":"gcp-kms","key":"projects/p/locations/europe-west1/keyRings/r/cryptoKeys/k","current_version":2,"previous_version":1}"#,
        )
        .expect("ok");
        assert!(matches!(kms, KeyringConfig::GcpKms(_)));
        assert!(KeyringConfig::from_json(br#"{"backend":"vault"}"#).is_err());
        assert!(KeyringConfig::from_json(br#"{"backend":"file","path":"/k","x":1}"#).is_err());
    }
}
