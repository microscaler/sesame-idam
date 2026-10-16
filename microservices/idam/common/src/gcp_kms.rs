//! A small Cloud KMS client for keys that never leave an HSM (ADR-015): `macSign` (`K_enroll`, ADR-007),
//! `asymmetricSign` and `getPublicKey` (RS256 ID tokens, the SAML SP key).
//!
//! Authentication is the workload's own identity: an access token from the GKE metadata server (Workload
//! Identity), cached until shortly before it expires; never a service-account key. Every answer is checked:
//! request and response CRC32C, the key version that answered, and (unless disabled) protection level `HSM`.
//! Errors carry an HTTP status or a fixed reason, never a token, key material or a response body.

use std::fmt;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use regex::Regex;

use crate::http::{fetch_get, fetch_post, HttpFetchOptions};

/// Public Cloud KMS endpoint.
pub const DEFAULT_ENDPOINT: &str = "https://cloudkms.googleapis.com";
/// GKE metadata server token endpoint (Workload Identity).
pub const DEFAULT_TOKEN_URL: &str =
    "http://metadata.google.internal/computeMetadata/v1/instance/service-accounts/default/token";
/// Refresh the access token this long before it expires.
// `from_secs`, not `from_mins`: the Octopilot Rust buildpack builds with Rust 1.89 (see clippy.toml `msrv`).
#[allow(clippy::duration_suboptimal_units)]
const TOKEN_MARGIN: Duration = Duration::from_secs(60);
const MAX_RESPONSE: usize = 16 * 1024;

static KEY_NAME: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(
        r"^projects/[a-z][a-z0-9-]{4,28}[a-z0-9]/locations/[a-z0-9-]+/keyRings/[A-Za-z0-9_-]{1,63}/cryptoKeys/[A-Za-z0-9_-]{1,63}$",
    )
    .ok()
});
static VERSION_NAME: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(
        r"^projects/[a-z][a-z0-9-]{4,28}[a-z0-9]/locations/[a-z0-9-]+/keyRings/[A-Za-z0-9_-]{1,63}/cryptoKeys/[A-Za-z0-9_-]{1,63}/cryptoKeyVersions/[1-9][0-9]{0,9}$",
    )
    .ok()
});

/// Whether `name` is a crypto key name (`projects/…/cryptoKeys/<k>`).
#[must_use]
pub fn is_key_name(name: &str) -> bool {
    KEY_NAME.as_ref().is_some_and(|re| re.is_match(name))
}

/// Whether `name` is a crypto key version name (`projects/…/cryptoKeys/<k>/cryptoKeyVersions/<n>`).
#[must_use]
pub fn is_key_version_name(name: &str) -> bool {
    VERSION_NAME.as_ref().is_some_and(|re| re.is_match(name))
}

/// Why a KMS call failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum KmsError {
    /// Configuration not usable.
    #[error("KMS configuration malformed: {0}")]
    Malformed(&'static str),
    /// KMS or the metadata server unreachable, or refused (status and reason only).
    #[error("KMS unavailable: {0}")]
    Unavailable(String),
    /// KMS answered, but the answer failed its checks.
    #[error("KMS response failed integrity checks: {0}")]
    Integrity(&'static str),
}

/// Connection settings shared by every KMS-backed key.
#[derive(Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KmsSettings {
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

impl Default for KmsSettings {
    fn default() -> Self {
        Self {
            require_hsm: true,
            endpoint: default_endpoint(),
            token_url: default_token_url(),
            timeout_ms: default_timeout_ms(),
        }
    }
}

impl fmt::Debug for KmsSettings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KmsSettings")
            .field("require_hsm", &self.require_hsm)
            .field("endpoint", &self.endpoint)
            .field("token_url", &self.token_url)
            .field("timeout_ms", &self.timeout_ms)
            .finish()
    }
}

fn is_local_host(url: &url::Url) -> bool {
    matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "[::1]"))
}

/// Cloud KMS client with a cached workload access token.
pub struct KmsClient {
    settings: KmsSettings,
    token: Mutex<Option<(String, Instant)>>,
}

impl fmt::Debug for KmsClient {
    /// The cached access token is never shown.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KmsClient")
            .field("settings", &self.settings)
            .finish_non_exhaustive()
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

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct AsymmetricSignResponse {
    name: String,
    signature: String,
    #[serde(default)]
    verified_digest_crc32c: bool,
    signature_crc32c: Option<String>,
    protection_level: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PublicKeyResponse {
    name: String,
    pem: String,
    algorithm: String,
    pem_crc32c: Option<String>,
    protection_level: Option<String>,
}

/// A public key as KMS returns it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KmsPublicKey {
    /// PEM-encoded `SubjectPublicKeyInfo`.
    pub pem: String,
    /// KMS algorithm name, e.g. `RSA_SIGN_PKCS1_2048_SHA256`.
    pub algorithm: String,
}

/// The KMS operation a request URL names, as a metric label.
fn kms_op(url: &str) -> &'static str {
    if url.ends_with(":asymmetricSign") {
        "asymmetric_sign"
    } else if url.ends_with(":macSign") {
        "mac_sign"
    } else if url.ends_with("/publicKey") {
        "public_key"
    } else {
        "other"
    }
}

fn crc_matches(data: &[u8], expected: Option<&str>) -> bool {
    expected
        .and_then(|c| c.parse::<u64>().ok())
        .is_some_and(|c| c == u64::from(crc32c(data)))
}

impl KmsClient {
    /// Validate the settings. No network call is made until the first operation.
    ///
    /// # Errors
    ///
    /// [`KmsError::Malformed`] for a non-HTTPS endpoint (localhost excepted, for tests) or a token URL that is not
    /// the metadata server.
    pub fn new(settings: KmsSettings) -> Result<Self, KmsError> {
        let endpoint =
            url::Url::parse(&settings.endpoint).map_err(|_| KmsError::Malformed("endpoint"))?;
        if endpoint.scheme() != "https" && !is_local_host(&endpoint) {
            return Err(KmsError::Malformed("endpoint must be https"));
        }
        let token_url =
            url::Url::parse(&settings.token_url).map_err(|_| KmsError::Malformed("token_url"))?;
        let metadata_host = matches!(
            token_url.host_str(),
            Some("metadata.google.internal" | "169.254.169.254")
        );
        if !metadata_host && !is_local_host(&token_url) {
            return Err(KmsError::Malformed("token_url must be the metadata server"));
        }
        Ok(Self {
            settings,
            token: Mutex::new(None),
        })
    }

    fn options(&self, headers: Vec<(String, String)>) -> HttpFetchOptions {
        HttpFetchOptions {
            timeout: Duration::from_millis(self.settings.timeout_ms),
            max_body_bytes: MAX_RESPONSE,
            extra_headers: headers,
        }
    }

    fn access_token(&self) -> Result<String, KmsError> {
        let mut cached = self
            .token
            .lock()
            .map_err(|_| KmsError::Unavailable("token cache poisoned".to_string()))?;
        if let Some((token, until)) = cached.as_ref() {
            if Instant::now() < *until {
                return Ok(token.clone());
            }
        }
        let options = self.options(vec![("Metadata-Flavor".to_string(), "Google".to_string())]);
        let (status, body) = fetch_get(&self.settings.token_url, &options)
            .map_err(|_| KmsError::Unavailable("metadata server unreachable".to_string()))?;
        if status != 200 {
            return Err(KmsError::Unavailable(format!(
                "metadata server status {status}"
            )));
        }
        let token: MetadataToken = serde_json::from_slice(&body)
            .map_err(|_| KmsError::Unavailable("metadata server answer malformed".to_string()))?;
        let lifetime = Duration::from_secs(token.expires_in).saturating_sub(TOKEN_MARGIN);
        *cached = Some((token.access_token.clone(), Instant::now() + lifetime));
        Ok(token.access_token)
    }

    fn call(&self, url: &str, body: Option<&serde_json::Value>) -> Result<Vec<u8>, KmsError> {
        let op = kms_op(url);
        let started = Instant::now();
        let result = self.call_inner(url, body);
        crate::observability::metrics::KMS_DURATION
            .with_label_values(&[op])
            .observe(started.elapsed().as_secs_f64());
        let outcome = match &result {
            Ok(_) => "ok",
            Err(KmsError::Unavailable(_)) => "unavailable",
            Err(KmsError::Integrity(_)) => "integrity",
            Err(_) => "error",
        };
        crate::observability::metrics::KMS_REQUESTS
            .with_label_values(&[op, outcome])
            .inc();
        if let Err(error) = &result {
            crate::observability::metrics::DEPENDENCY_ERRORS
                .with_label_values(&["kms", op])
                .inc();
            tracing::warn!(op, outcome, error = %error, "sesame_kms_error");
        }
        result
    }

    fn call_inner(&self, url: &str, body: Option<&serde_json::Value>) -> Result<Vec<u8>, KmsError> {
        let token = self.access_token()?;
        let mut headers = vec![("Authorization".to_string(), format!("Bearer {token}"))];
        let result = if let Some(body) = body {
            headers.push(("Content-Type".to_string(), "application/json".to_string()));
            let bytes =
                serde_json::to_vec(body).map_err(|_| KmsError::Malformed("request encoding"))?;
            fetch_post(url, &bytes, &self.options(headers))
        } else {
            fetch_get(url, &self.options(headers))
        };
        let (status, resp) =
            result.map_err(|_| KmsError::Unavailable("KMS unreachable".to_string()))?;
        if status != 200 {
            if status == 401 {
                // Drop a token KMS no longer accepts; the next call fetches a fresh one.
                if let Ok(mut cached) = self.token.lock() {
                    *cached = None;
                }
            }
            return Err(KmsError::Unavailable(format!("KMS status {status}")));
        }
        Ok(resp)
    }

    fn url(&self, version_name: &str, verb: &str) -> String {
        format!(
            "{}/v1/{version_name}{verb}",
            self.settings.endpoint.trim_end_matches('/')
        )
    }

    fn check_answer(
        &self,
        requested: &str,
        name: &str,
        protection_level: Option<&str>,
    ) -> Result<(), KmsError> {
        if name != requested {
            return Err(KmsError::Integrity("answered by another key version"));
        }
        if self.settings.require_hsm && protection_level != Some("HSM") {
            return Err(KmsError::Integrity("key version is not HSM-protected"));
        }
        Ok(())
    }

    /// HMAC of `data` under an HMAC key version (`macSign`).
    ///
    /// # Errors
    ///
    /// See [`KmsError`].
    pub fn mac_sign(&self, version_name: &str, data: &[u8]) -> Result<Vec<u8>, KmsError> {
        if !is_key_version_name(version_name) {
            return Err(KmsError::Malformed("key version name"));
        }
        let body = serde_json::json!({
            "data": STANDARD.encode(data),
            "dataCrc32c": crc32c(data).to_string(),
        });
        let resp = self.call(&self.url(version_name, ":macSign"), Some(&body))?;
        let resp: MacSignResponse = serde_json::from_slice(&resp)
            .map_err(|_| KmsError::Integrity("response not understood"))?;
        self.check_answer(version_name, &resp.name, resp.protection_level.as_deref())?;
        if !resp.verified_data_crc32c {
            return Err(KmsError::Integrity("request checksum not verified"));
        }
        let mac = STANDARD
            .decode(resp.mac.as_bytes())
            .map_err(|_| KmsError::Integrity("mac not base64"))?;
        if !crc_matches(&mac, resp.mac_crc32c.as_deref()) {
            return Err(KmsError::Integrity("mac checksum mismatch"));
        }
        Ok(mac)
    }

    /// Signature over a SHA-256 digest (`asymmetricSign`), for RSA PKCS#1 v1.5 and ECDSA keys.
    ///
    /// # Errors
    ///
    /// See [`KmsError`].
    pub fn asymmetric_sign_sha256(
        &self,
        version_name: &str,
        digest: &[u8; 32],
    ) -> Result<Vec<u8>, KmsError> {
        if !is_key_version_name(version_name) {
            return Err(KmsError::Malformed("key version name"));
        }
        let body = serde_json::json!({
            "digest": {"sha256": STANDARD.encode(digest)},
            "digestCrc32c": crc32c(digest).to_string(),
        });
        let resp = self.call(&self.url(version_name, ":asymmetricSign"), Some(&body))?;
        let resp: AsymmetricSignResponse = serde_json::from_slice(&resp)
            .map_err(|_| KmsError::Integrity("response not understood"))?;
        self.check_answer(version_name, &resp.name, resp.protection_level.as_deref())?;
        if !resp.verified_digest_crc32c {
            return Err(KmsError::Integrity("request checksum not verified"));
        }
        let signature = STANDARD
            .decode(resp.signature.as_bytes())
            .map_err(|_| KmsError::Integrity("signature not base64"))?;
        if !crc_matches(&signature, resp.signature_crc32c.as_deref()) {
            return Err(KmsError::Integrity("signature checksum mismatch"));
        }
        Ok(signature)
    }

    /// The public key of an asymmetric key version (`getPublicKey`).
    ///
    /// # Errors
    ///
    /// See [`KmsError`].
    pub fn public_key(&self, version_name: &str) -> Result<KmsPublicKey, KmsError> {
        if !is_key_version_name(version_name) {
            return Err(KmsError::Malformed("key version name"));
        }
        let resp = self.call(&self.url(version_name, "/publicKey"), None)?;
        let resp: PublicKeyResponse = serde_json::from_slice(&resp)
            .map_err(|_| KmsError::Integrity("response not understood"))?;
        self.check_answer(version_name, &resp.name, resp.protection_level.as_deref())?;
        if !crc_matches(resp.pem.as_bytes(), resp.pem_crc32c.as_deref()) {
            return Err(KmsError::Integrity("public key checksum mismatch"));
        }
        Ok(KmsPublicKey {
            pem: resp.pem,
            algorithm: resp.algorithm,
        })
    }
}

/// CRC-32C (Castagnoli, reflected polynomial 0x82F63B78), as KMS uses for integrity fields.
#[must_use]
pub fn crc32c(data: &[u8]) -> u32 {
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

/// A fake metadata server and Cloud KMS for tests: real HTTP on localhost, scripted answers.
#[cfg(test)]
pub(crate) mod fake {
    use super::{crc32c, STANDARD};
    use base64::Engine as _;
    use std::collections::HashMap;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::{Arc, Mutex};
    use std::thread;

    /// How the fake misbehaves, if at all.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Mode {
        Good,
        WrongVersion,
        Software,
        BadCrc,
        Unverified,
        Denied,
    }

    /// Scripted behaviour: `mac` computes a MAC for (version, data); `signatures` maps a digest to a signature;
    /// `public_keys` maps a version name to (PEM, algorithm).
    /// Computes a MAC for (key version, data).
    pub type MacFn = Box<dyn Fn(&str, &[u8]) -> Vec<u8> + Send>;

    #[derive(Default)]
    pub struct Script {
        pub mac: Option<MacFn>,
        pub signatures: HashMap<Vec<u8>, Vec<u8>>,
        pub public_keys: HashMap<String, (String, String)>,
    }

    fn read_request(stream: &mut TcpStream) -> String {
        let mut buf = vec![0u8; 64 * 1024];
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

    /// Start the fake; returns its base URL (`http://127.0.0.1:<port>`) and a counter of KMS calls.
    pub fn start(mode: Mode, script: Script) -> (String, Arc<Mutex<usize>>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let base = format!("http://{}", listener.local_addr().expect("addr"));
        let calls = Arc::new(Mutex::new(0usize));
        let counter = calls.clone();
        thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { break };
                let req = read_request(&mut stream);
                let lower = req.to_ascii_lowercase();
                if req.starts_with("GET /token") {
                    assert!(lower.contains("metadata-flavor: google"));
                    respond(
                        &mut stream,
                        "200 OK",
                        r#"{"access_token":"tok-1","expires_in":3600,"token_type":"Bearer"}"#,
                    );
                    continue;
                }
                *counter.lock().expect("counter") += 1;
                assert!(lower.contains("authorization: bearer tok-1"));
                if mode == Mode::Denied {
                    respond(&mut stream, "403 Forbidden", r#"{"error":{"code":403}}"#);
                    continue;
                }
                let path = req
                    .split_whitespace()
                    .nth(1)
                    .unwrap_or_default()
                    .to_string();
                let path = path.trim_start_matches("/v1/").to_string();
                let body = &req[req.find("\r\n\r\n").map_or(0, |i| i + 4)..];
                let level = if mode == Mode::Software {
                    "SOFTWARE"
                } else {
                    "HSM"
                };
                let answer_name = |version: &str| {
                    if mode == Mode::WrongVersion {
                        format!(
                            "{}/cryptoKeyVersions/99",
                            version.split("/cryptoKeyVersions/").next().unwrap_or("")
                        )
                    } else {
                        version.to_string()
                    }
                };
                let crc = |data: &[u8]| {
                    let c = crc32c(data);
                    if mode == Mode::BadCrc {
                        c ^ 1
                    } else {
                        c
                    }
                };
                let verified = mode != Mode::Unverified;
                if let Some(version) = path.strip_suffix(":macSign") {
                    let json: serde_json::Value = serde_json::from_str(body).expect("json");
                    let data = STANDARD
                        .decode(json["data"].as_str().expect("data"))
                        .expect("b64");
                    assert_eq!(
                        json["dataCrc32c"].as_str().expect("crc"),
                        crc32c(&data).to_string()
                    );
                    let mac = (script.mac.as_ref().expect("mac script"))(version, &data);
                    let out = serde_json::json!({
                        "name": answer_name(version), "mac": STANDARD.encode(&mac),
                        "verifiedDataCrc32c": verified, "macCrc32c": crc(&mac).to_string(),
                        "protectionLevel": level,
                    });
                    respond(&mut stream, "200 OK", &out.to_string());
                } else if let Some(version) = path.strip_suffix(":asymmetricSign") {
                    let json: serde_json::Value = serde_json::from_str(body).expect("json");
                    let digest = STANDARD
                        .decode(json["digest"]["sha256"].as_str().expect("digest"))
                        .expect("b64");
                    assert_eq!(
                        json["digestCrc32c"].as_str().expect("crc"),
                        crc32c(&digest).to_string()
                    );
                    let sig = script.signatures.get(&digest).cloned().unwrap_or_default();
                    let out = serde_json::json!({
                        "name": answer_name(version), "signature": STANDARD.encode(&sig),
                        "verifiedDigestCrc32c": verified, "signatureCrc32c": crc(&sig).to_string(),
                        "protectionLevel": level,
                    });
                    respond(&mut stream, "200 OK", &out.to_string());
                } else if let Some(version) = path.strip_suffix("/publicKey") {
                    let (pem, alg) = script.public_keys.get(version).cloned().unwrap_or_default();
                    let out = serde_json::json!({
                        "name": answer_name(version), "pem": pem, "algorithm": alg,
                        "pemCrc32c": crc(pem.as_bytes()).to_string(), "protectionLevel": level,
                    });
                    respond(&mut stream, "200 OK", &out.to_string());
                } else {
                    respond(&mut stream, "404 Not Found", "{}");
                }
            }
        });
        (base, calls)
    }

    /// Settings pointing at a fake.
    pub fn settings(base: &str) -> super::KmsSettings {
        super::KmsSettings {
            require_hsm: true,
            endpoint: base.to_string(),
            token_url: format!("{base}/token"),
            timeout_ms: 2000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VERSION: &str =
        "projects/sesame-test/locations/europe-west1/keyRings/sesame/cryptoKeys/k/cryptoKeyVersions/3";

    #[test]
    fn crc32c_check_value() {
        assert_eq!(crc32c(b"123456789"), 0xE306_9283);
        assert_eq!(crc32c(b""), 0);
    }

    #[test]
    fn names_are_validated() {
        assert!(is_key_version_name(VERSION));
        assert!(!is_key_name(VERSION));
        assert!(is_key_name(
            VERSION
                .split("/cryptoKeyVersions/")
                .next()
                .unwrap_or_default()
        ));
        assert!(!is_key_version_name(&format!("{VERSION}/x")));
        assert!(!is_key_version_name(
            "projects/sesame-test/locations/l/keyRings/r/cryptoKeys/k/cryptoKeyVersions/0"
        ));
    }

    #[test]
    fn settings_are_validated() {
        assert!(KmsClient::new(KmsSettings::default()).is_ok());
        assert!(KmsClient::new(KmsSettings {
            endpoint: "http://cloudkms.googleapis.com".to_string(),
            ..KmsSettings::default()
        })
        .is_err());
        assert!(KmsClient::new(KmsSettings {
            token_url: "http://evil.example.com/token".to_string(),
            ..KmsSettings::default()
        })
        .is_err());
    }

    #[test]
    fn mac_sign_and_integrity_refusals() {
        let script = || fake::Script {
            mac: Some(Box::new(|version: &str, data: &[u8]| {
                let mut v = version.as_bytes().to_vec();
                v.extend_from_slice(data);
                ring::digest::digest(&ring::digest::SHA256, &v)
                    .as_ref()
                    .to_vec()
            })),
            ..fake::Script::default()
        };
        let (base, calls) = fake::start(fake::Mode::Good, script());
        let client = KmsClient::new(fake::settings(&base)).expect("client");
        assert_eq!(client.mac_sign(VERSION, b"m").expect("mac").len(), 32);
        assert_eq!(*calls.lock().expect("calls"), 1);
        assert!(!format!("{client:?}").contains("tok-1"));

        for (mode, why) in [
            (fake::Mode::WrongVersion, "answered by another key version"),
            (fake::Mode::Software, "key version is not HSM-protected"),
            (fake::Mode::BadCrc, "mac checksum mismatch"),
            (fake::Mode::Unverified, "request checksum not verified"),
        ] {
            let (base, _) = fake::start(mode, script());
            let client = KmsClient::new(fake::settings(&base)).expect("client");
            assert_eq!(
                client.mac_sign(VERSION, b"m"),
                Err(KmsError::Integrity(why)),
                "{mode:?}"
            );
        }
        let (base, _) = fake::start(fake::Mode::Denied, script());
        let client = KmsClient::new(fake::settings(&base)).expect("client");
        let err = client.mac_sign(VERSION, b"m").expect_err("denied");
        assert_eq!(err, KmsError::Unavailable("KMS status 403".to_string()));
        assert!(!err.to_string().contains("tok-1"));
    }

    #[test]
    fn unreachable_kms_is_unavailable() {
        let dead = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = dead.local_addr().expect("addr");
        drop(dead);
        let client = KmsClient::new(fake::settings(&format!("http://{addr}"))).expect("client");
        assert!(matches!(
            client.mac_sign(VERSION, b"m"),
            Err(KmsError::Unavailable(_))
        ));
        assert_eq!(
            client.mac_sign("not-a-version", b"m"),
            Err(KmsError::Malformed("key version name"))
        );
    }
}
