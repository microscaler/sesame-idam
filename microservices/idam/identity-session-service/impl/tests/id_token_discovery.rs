//! ADR-015: with ID-token keys configured, discovery advertises RS256 and the keys are publishable as RSA JWKs.
//!
//! Own test binary: `SESAME_ID_TOKEN_KEYS` is read once per process.

use rsa::pkcs8::{EncodePrivateKey, LineEnding};
use sesame_common::jwt::{ID_TOKEN_KEYS, ID_TOKEN_KEYS_ENV};
use sesame_idam_identity_session_service::services::discovery::openid_configuration;

#[test]
fn discovery_and_jwks_follow_the_configured_id_token_keys() {
    let dir = std::env::temp_dir().join(format!("sesame-id-token-disc-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let key = rsa::RsaPrivateKey::new(&mut rand::thread_rng(), 2048).expect("keygen");
    let pem_path = dir.join("id-token.pem");
    std::fs::write(
        &pem_path,
        key.to_pkcs8_pem(LineEnding::LF).expect("pem").as_bytes(),
    )
    .expect("write key");
    let config_path = dir.join("id-token-keys.json");
    std::fs::write(
        &config_path,
        serde_json::json!({"backend": "file", "keys": [pem_path]}).to_string(),
    )
    .expect("write config");
    std::env::set_var(ID_TOKEN_KEYS_ENV, &config_path);

    assert_eq!(
        openid_configuration().id_token_signing_alg_values_supported,
        vec!["RS256".to_string()]
    );
    let jwks = ID_TOKEN_KEYS
        .get()
        .expect("loads")
        .expect("configured")
        .jwks();
    assert_eq!(jwks.len(), 1);
    assert_eq!(jwks[0]["kty"], "RSA");
    assert_eq!(jwks[0]["alg"], "RS256");
    assert_eq!(jwks[0]["use"], "sig");
    assert!(jwks[0].get("d").is_none(), "no private member is published");
    std::fs::remove_dir_all(&dir).ok();
}
