//! `K_enroll` in Cloud KMS: an HMAC-SHA256 key, normally at HSM protection level, used through `macSign`.
//!
//! The key never leaves KMS. Each epoch is a key version; the configuration names the current version and,
//! during rotation, the previous one. Access is by the workload's own identity (GKE Workload Identity: an
//! access token from the metadata server), never a service-account key; the identity needs only
//! `cloudkms.cryptoKeyVersions.useToSign` on this one key (`roles/cloudkms.signer`).
//!
//! Every response is checked: the request carries `dataCrc32c` and KMS must confirm it; the returned MAC must
//! match `macCrc32c`, be 32 bytes, come from the requested key version and, unless disabled, from an HSM.

use std::fmt;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use regex::Regex;

use super::keyring::{EnrollKeyring, KeyringError, MAC_LEN};
use crate::http::{fetch_get, fetch_post, HttpFetchOptions};

const DEFAULT_ENDPOINT: &str = "https://cloudkms.googleapis.com";
const DEFAULT_TOKEN_URL: &str =
    "http://metadata.google.internal/computeMetadata/v1/instance/service-accounts/default/token";
/// Refresh the access token this long before it expires.
const TOKEN_MARGIN: Duration = Duration::from_mins(1);
const MAX_RESPONSE: usize = 16 * 1024;

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
pub struct GcpKmsKeyring {
    config: GcpKmsConfig,
    token: Mutex<Option<(String, Instant)>>,
}

impl fmt::Debug for GcpKmsKeyring {
    /// The cached access token is never shown.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GcpKmsKeyring")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

fn is_local_host(url: &url::Url) -> bool {
    matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "[::1]"))
}

impl GcpKmsKeyring {
    /// Validate the configuration and build the keyring. No network call is made until the first MAC.
    ///
    /// # Errors
    ///
    /// [`KeyringError::Malformed`] for a bad key name or URL, [`KeyringError::EpochClash`] for equal versions.
    ///
    /// # Panics
    ///
    /// Never: the key-name pattern is a constant.
    pub fn new(config: GcpKmsConfig) -> Result<Self, KeyringError> {
        #[allow(clippy::expect_used)] // constant pattern
        let key_name = Regex::new(
            r"^projects/[a-z][a-z0-9-]{4,28}[a-z0-9]/locations/[a-z0-9-]+/keyRings/[A-Za-z0-9_-]{1,63}/cryptoKeys/[A-Za-z0-9_-]{1,63}$",
        )
        .expect("valid regex");
        if !key_name.is_match(&config.key) {
            return Err(KeyringError::Malformed("KMS key name"));
        }
        if config.current_version == 0 || config.previous_version == Some(0) {
            return Err(KeyringError::Malformed("KMS key versions start at 1"));
        }
        if config.previous_version == Some(config.current_version) {
            return Err(KeyringError::EpochClash);
        }
        let endpoint =
            url::Url::parse(&config.endpoint).map_err(|_| KeyringError::Malformed("endpoint"))?;
        if endpoint.scheme() != "https" && !is_local_host(&endpoint) {
            return Err(KeyringError::Malformed("endpoint must be https"));
        }
        let token_url =
            url::Url::parse(&config.token_url).map_err(|_| KeyringError::Malformed("token_url"))?;
        let metadata_host = matches!(
            token_url.host_str(),
            Some("metadata.google.internal" | "169.254.169.254")
        );
        if !metadata_host && !is_local_host(&token_url) {
            return Err(KeyringError::Malformed(
                "token_url must be the metadata server",
            ));
        }
        Ok(Self {
            config,
            token: Mutex::new(None),
        })
    }

    fn options(&self, headers: Vec<(String, String)>) -> HttpFetchOptions {
        HttpFetchOptions {
            timeout: Duration::from_millis(self.config.timeout_ms),
            max_body_bytes: MAX_RESPONSE,
            extra_headers: headers,
        }
    }

    fn access_token(&self) -> Result<String, KeyringError> {
        let mut cached = self
            .token
            .lock()
            .map_err(|_| KeyringError::Unavailable("token cache poisoned".to_string()))?;
        if let Some((token, until)) = cached.as_ref() {
            if Instant::now() < *until {
                return Ok(token.clone());
            }
        }
        let options = self.options(vec![("Metadata-Flavor".to_string(), "Google".to_string())]);
        let (status, body) = fetch_get(&self.config.token_url, &options)
            .map_err(|_| KeyringError::Unavailable("metadata server unreachable".to_string()))?;
        if status != 200 {
            return Err(KeyringError::Unavailable(format!(
                "metadata server status {status}"
            )));
        }
        let token: MetadataToken = serde_json::from_slice(&body).map_err(|_| {
            KeyringError::Unavailable("metadata server answer malformed".to_string())
        })?;
        let lifetime = Duration::from_secs(token.expires_in).saturating_sub(TOKEN_MARGIN);
        *cached = Some((token.access_token.clone(), Instant::now() + lifetime));
        Ok(token.access_token)
    }

    fn version_name(&self, version: u32) -> String {
        format!("{}/cryptoKeyVersions/{version}", self.config.key)
    }
}

#[derive(serde::Deserialize)]
struct MetadataToken {
    access_token: String,
    expires_in: u64,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct MacSignResponse {
    name: String,
    mac: String,
    #[serde(default)]
    verified_data_crc32c: bool,
    mac_crc32c: Option<String>,
    protection_level: Option<String>,
}

impl EnrollKeyring for GcpKmsKeyring {
    fn current_epoch(&self) -> u32 {
        self.config.current_version
    }

    fn accepted_epochs(&self) -> Vec<u32> {
        std::iter::once(self.config.current_version)
            .chain(self.config.previous_version)
            .collect()
    }

    fn mac(&self, epoch: u32, message: &[u8]) -> Result<[u8; MAC_LEN], KeyringError> {
        if !self.accepted_epochs().contains(&epoch) {
            return Err(KeyringError::UnknownEpoch(epoch));
        }
        let name = self.version_name(epoch);
        let url = format!(
            "{}/v1/{name}:macSign",
            self.config.endpoint.trim_end_matches('/')
        );
        let body = serde_json::json!({
            "data": STANDARD.encode(message),
            "dataCrc32c": crc32c(message).to_string(),
        });
        let body =
            serde_json::to_vec(&body).map_err(|_| KeyringError::Malformed("request encoding"))?;
        let token = self.access_token()?;
        let options = self.options(vec![
            ("Authorization".to_string(), format!("Bearer {token}")),
            ("Content-Type".to_string(), "application/json".to_string()),
        ]);
        let (status, resp) = fetch_post(&url, &body, &options)
            .map_err(|_| KeyringError::Unavailable("KMS unreachable".to_string()))?;
        if status != 200 {
            if status == 401 {
                // Drop a token KMS no longer accepts; the next call fetches a fresh one.
                if let Ok(mut cached) = self.token.lock() {
                    *cached = None;
                }
            }
            return Err(KeyringError::Unavailable(format!("KMS status {status}")));
        }
        let resp: MacSignResponse = serde_json::from_slice(&resp)
            .map_err(|_| KeyringError::Integrity("response not understood"))?;
        if resp.name != name {
            return Err(KeyringError::Integrity("answered by another key version"));
        }
        if !resp.verified_data_crc32c {
            return Err(KeyringError::Integrity("request checksum not verified"));
        }
        if self.config.require_hsm && resp.protection_level.as_deref() != Some("HSM") {
            return Err(KeyringError::Integrity("key version is not HSM-protected"));
        }
        let mac = STANDARD
            .decode(resp.mac.as_bytes())
            .map_err(|_| KeyringError::Integrity("mac not base64"))?;
        let expected_crc = resp
            .mac_crc32c
            .as_deref()
            .and_then(|c| c.parse::<u64>().ok())
            .ok_or(KeyringError::Integrity("mac checksum missing"))?;
        if u64::from(crc32c(&mac)) != expected_crc {
            return Err(KeyringError::Integrity("mac checksum mismatch"));
        }
        mac.try_into()
            .map_err(|_| KeyringError::Integrity("mac is not 32 bytes"))
    }
}

/// CRC-32C (Castagnoli, reflected polynomial 0x82F63B78), as KMS uses for integrity fields.
#[must_use]
pub(super) fn crc32c(data: &[u8]) -> u32 {
    const POLY: u32 = 0x82F6_3B78;
    let mut crc = !0u32;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (POLY & mask);
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::thread;

    const KEY: &str =
        "projects/sesame-test/locations/europe-west1/keyRings/sesame/cryptoKeys/domain-verify";

    #[test]
    fn crc32c_check_value() {
        assert_eq!(crc32c(b"123456789"), 0xE306_9283);
        assert_eq!(crc32c(b""), 0);
    }

    fn read_request(stream: &mut TcpStream) -> String {
        let mut buf = vec![0u8; 16 * 1024];
        let mut total = 0;
        loop {
            let n = stream.read(&mut buf[total..]).unwrap_or(0);
            if n == 0 {
                break;
            }
            total += n;
            let text = String::from_utf8_lossy(&buf[..total]).to_string();
            if let Some(head_end) = text.find("\r\n\r\n") {
                let len = text[..head_end]
                    .lines()
                    .find_map(|l| {
                        let l = l.to_ascii_lowercase();
                        l.strip_prefix("content-length:")
                            .map(|v| v.trim().parse::<usize>().unwrap_or(0))
                    })
                    .unwrap_or(0);
                if total >= head_end + 4 + len {
                    return text;
                }
            }
        }
        String::from_utf8_lossy(&buf[..total]).to_string()
    }

    fn respond(stream: &mut TcpStream, status: &str, body: &str) {
        let resp = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(resp.as_bytes()).ok();
    }

    #[derive(Clone, Copy)]
    enum Fake {
        Good,
        WrongVersion,
        Software,
        BadCrc,
        Unverified,
        Denied,
    }

    /// A fake metadata server and KMS on one port: answers tokens and computes real HMACs with a test key.
    fn fake_kms(mode: Fake, calls: Arc<AtomicUsize>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let base = format!("http://{}", listener.local_addr().expect("addr"));
        thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { break };
                let req = read_request(&mut stream);
                if req.starts_with("GET /token") {
                    assert!(req.to_ascii_lowercase().contains("metadata-flavor: google"));
                    respond(
                        &mut stream,
                        "200 OK",
                        r#"{"access_token":"tok-1","expires_in":3600,"token_type":"Bearer"}"#,
                    );
                    continue;
                }
                calls.fetch_add(1, Ordering::SeqCst);
                assert!(req
                    .to_ascii_lowercase()
                    .contains("authorization: bearer tok-1"));
                if matches!(mode, Fake::Denied) {
                    respond(&mut stream, "403 Forbidden", r#"{"error":{"code":403}}"#);
                    continue;
                }
                let path = req
                    .split_whitespace()
                    .nth(1)
                    .unwrap_or_default()
                    .to_string();
                let version = path
                    .trim_start_matches("/v1/")
                    .trim_end_matches(":macSign")
                    .to_string();
                let body = &req[req.find("\r\n\r\n").map_or(0, |i| i + 4)..];
                let json: serde_json::Value = serde_json::from_str(body).expect("json body");
                let data = STANDARD
                    .decode(json["data"].as_str().expect("data"))
                    .expect("b64");
                assert_eq!(
                    json["dataCrc32c"].as_str().expect("crc"),
                    crc32c(&data).to_string()
                );
                let key = format!("test-key-for-{version}");
                let mut mac = Hmac::<Sha256>::new_from_slice(key.as_bytes()).expect("key");
                mac.update(&data);
                let tag = mac.finalize().into_bytes().to_vec();
                let crc = match mode {
                    Fake::BadCrc => crc32c(&tag) ^ 1,
                    _ => crc32c(&tag),
                };
                let name = match mode {
                    Fake::WrongVersion => format!("{KEY}/cryptoKeyVersions/99"),
                    _ => version,
                };
                let level = if matches!(mode, Fake::Software) {
                    "SOFTWARE"
                } else {
                    "HSM"
                };
                let verified = !matches!(mode, Fake::Unverified);
                let out = serde_json::json!({
                    "name": name,
                    "mac": STANDARD.encode(&tag),
                    "verifiedDataCrc32c": verified,
                    "macCrc32c": crc.to_string(),
                    "protectionLevel": level,
                });
                respond(&mut stream, "200 OK", &out.to_string());
            }
        });
        base
    }

    fn keyring(base: &str) -> GcpKmsKeyring {
        GcpKmsKeyring::new(GcpKmsConfig {
            key: KEY.to_string(),
            current_version: 2,
            previous_version: Some(1),
            require_hsm: true,
            endpoint: base.to_string(),
            token_url: format!("{base}/token"),
            timeout_ms: 2000,
        })
        .expect("config")
    }

    fn expected(version: u32, message: &[u8]) -> [u8; 32] {
        let key = format!("{KEY}/cryptoKeyVersions/{version}");
        let mut mac =
            Hmac::<Sha256>::new_from_slice(format!("test-key-for-{key}").as_bytes()).expect("key");
        mac.update(message);
        mac.finalize().into_bytes().into()
    }

    #[test]
    fn mac_sign_round_trip_per_version() {
        let calls = Arc::new(AtomicUsize::new(0));
        let base = fake_kms(Fake::Good, calls.clone());
        let k = keyring(&base);
        assert_eq!(k.accepted_epochs(), vec![2, 1]);
        assert_eq!(k.mac(2, b"hello"), Ok(expected(2, b"hello")));
        assert_eq!(k.mac(1, b"hello"), Ok(expected(1, b"hello")));
        assert_ne!(expected(1, b"hello"), expected(2, b"hello"));
        assert_eq!(k.mac(3, b"hello"), Err(KeyringError::UnknownEpoch(3)));
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn integrity_failures_are_refused() {
        for (mode, why) in [
            (Fake::WrongVersion, "answered by another key version"),
            (Fake::Software, "key version is not HSM-protected"),
            (Fake::BadCrc, "mac checksum mismatch"),
            (Fake::Unverified, "request checksum not verified"),
        ] {
            let base = fake_kms(mode, Arc::new(AtomicUsize::new(0)));
            assert_eq!(
                keyring(&base).mac(2, b"m"),
                Err(KeyringError::Integrity(why))
            );
        }
    }

    #[test]
    fn refusals_and_outages_are_unavailable_without_secrets() {
        let base = fake_kms(Fake::Denied, Arc::new(AtomicUsize::new(0)));
        let err = keyring(&base).mac(2, b"m").expect_err("denied");
        assert_eq!(err, KeyringError::Unavailable("KMS status 403".to_string()));
        assert!(!err.to_string().contains("tok-1"));

        // Nothing listening.
        let dead = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = dead.local_addr().expect("addr");
        drop(dead);
        let k = keyring(&format!("http://{addr}"));
        assert!(matches!(k.mac(2, b"m"), Err(KeyringError::Unavailable(_))));
        assert!(!format!("{k:?}").contains("tok-1"));
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
