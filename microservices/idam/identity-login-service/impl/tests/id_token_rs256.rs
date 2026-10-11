//! ADR-015: ID tokens are RS256, signed with the configured ID-token key, and verify against its public JWK.
//!
//! Own test binary: `SESAME_ID_TOKEN_KEYS` is read once per process.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use rsa::pkcs8::{EncodePrivateKey, LineEnding};
use sesame_common::jwt::{ID_TOKEN_KEYS, ID_TOKEN_KEYS_ENV};
use sesame_idam_identity_login_service::services::token_issuer::issue_id_token;

#[test]
fn id_tokens_are_rs256_with_the_configured_key() {
    let dir = std::env::temp_dir().join(format!("sesame-id-token-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    // A throwaway key generated for this test; nothing private is committed.
    let key = rsa::RsaPrivateKey::new(&mut rand::thread_rng(), 2048).expect("keygen");
    let pem_path = dir.join("id-token.pem");
    std::fs::write(
        &pem_path,
        key.to_pkcs8_pem(LineEnding::LF).expect("pem").as_bytes(),
    )
    .expect("write key");
    let config_path = dir.join("id-token-keys.json");
    let config = serde_json::json!({"backend": "file", "keys": [pem_path]});
    std::fs::write(&config_path, config.to_string()).expect("write config");
    std::env::set_var(ID_TOKEN_KEYS_ENV, &config_path);

    let token = issue_id_token("user-1", "client-a", "nonce-1", 1_700_000_000).expect("issued");

    let header: serde_json::Value = serde_json::from_slice(
        &URL_SAFE_NO_PAD
            .decode(token.split('.').next().expect("header"))
            .expect("b64"),
    )
    .expect("json");
    assert_eq!(header["alg"], "RS256");
    assert_eq!(header["typ"], "JWT");

    let keys = ID_TOKEN_KEYS.get().expect("loads").expect("configured");
    let jwk = &keys.jwks()[0];
    assert_eq!(header["kid"], jwk["kid"]);
    let decoding = jsonwebtoken::DecodingKey::from_rsa_components(
        jwk["n"].as_str().expect("n"),
        jwk["e"].as_str().expect("e"),
    )
    .expect("key");
    let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::RS256);
    validation.set_audience(&["client-a"]);
    validation.validate_exp = true;
    let claims = jsonwebtoken::decode::<serde_json::Value>(&token, &decoding, &validation)
        .expect("verifies")
        .claims;
    assert_eq!(claims["sub"], "user-1");
    assert_eq!(claims["nonce"], "nonce-1");
    assert_eq!(claims["azp"], "client-a");

    // EdDSA is never accepted for it, and an access-token validator (EdDSA only) rejects it.
    let mut eddsa = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::EdDSA);
    eddsa.set_audience(&["client-a"]);
    assert!(jsonwebtoken::decode::<serde_json::Value>(&token, &decoding, &eddsa).is_err());

    std::fs::remove_dir_all(&dir).ok();
}
