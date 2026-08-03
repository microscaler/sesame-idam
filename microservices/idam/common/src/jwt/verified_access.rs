//! Shared access-token verification boundary (Epic 14.2).
//!
//! Credential-minting paths must call [`verify_access_token`] instead of
//! decoding JWT payloads without checking typ/alg/signature/claims.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;

use super::signer::Ed25519Signer;
use super::types::{AccessClaims, JwtError, JwtValidationError};

/// Verify a compact access token and return validated claims.
///
/// Steps: structure → header typ/alg → signature → deserialize →
/// [`AccessClaims::validate`] → optional tenant match.
pub fn verify_access_token(
    signer: &Ed25519Signer,
    token: &str,
    expected_tenant: Option<&str>,
) -> Result<AccessClaims, JwtError> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err(JwtError::Validation(JwtValidationError::SignatureInvalid));
    }

    let header = decode_json_segment(parts[0])?;
    let typ = header.get("typ").and_then(|v| v.as_str()).unwrap_or("");
    if typ != "at+jwt" {
        return Err(JwtError::Validation(JwtValidationError::InvalidTyp));
    }
    let alg = header.get("alg").and_then(|v| v.as_str()).unwrap_or("");
    if alg != "EdDSA" {
        return Err(JwtError::Validation(JwtValidationError::InvalidAlgorithm));
    }

    signer
        .verify(token)
        .map_err(|_| JwtError::Validation(JwtValidationError::SignatureInvalid))?;

    let claims: AccessClaims = serde_json::from_slice(
        &URL_SAFE_NO_PAD
            .decode(parts[1])
            .map_err(|_| JwtError::Validation(JwtValidationError::SignatureInvalid))?,
    )
    .map_err(|_| JwtError::MissingRequiredField("access_token.payload".into()))?;

    claims
        .validate()
        .map_err(JwtError::Validation)?;

    if let Some(tenant) = expected_tenant {
        claims.validate_tenant(tenant)?;
    }

    Ok(claims)
}

fn decode_json_segment(segment: &str) -> Result<serde_json::Value, JwtError> {
    let bytes = URL_SAFE_NO_PAD
        .decode(segment)
        .map_err(|_| JwtError::Validation(JwtValidationError::SignatureInvalid))?;
    serde_json::from_slice(&bytes)
        .map_err(|_| JwtError::MissingRequiredField("jwt.header".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jwt::builders::{AccessClaimsBuilder, SesameAuthzClaimsBuilder};

    fn signer() -> Ed25519Signer {
        Ed25519Signer::from_env_or_generate().expect("signer")
    }

    /// Use compile-time default issuer/audience so OnceLock env overrides are unnecessary.
    fn claims(iss: &str, aud: &str, tenant: &str, exp_offset: i64) -> AccessClaims {
        let now = chrono::Utc::now().timestamp();
        let sx = SesameAuthzClaimsBuilder::new()
            .tenant(tenant)
            .portal("frontend")
            .roles(vec!["user".into()])
            .permissions(vec![])
            .build()
            .expect("sx");
        AccessClaimsBuilder::new()
            .iss(iss)
            .sub("user-1")
            .aud(vec![aud.into()])
            .client_id("frontend")
            .scope("openid")
            .exp(now + exp_offset)
            .nbf(now - 10)
            .iat(now)
            .jti("jti-1")
            .ver(1)
            .sid("sid-1")
            .tenant_id(tenant)
            .user_id("user-1")
            .user_type("customer")
            .sx(sx)
            .build()
            .expect("claims")
    }

    #[test]
    fn accepts_valid_signed_access_token() {
        let signer = signer();
        let token = signer
            .sign_access_claims(&claims(
                "https://idam.example.com",
                "identity-login",
                "acme",
                300,
            ))
            .expect("sign");
        let got = verify_access_token(&signer, &token, Some("acme")).expect("verify");
        assert_eq!(got.tenant_id, "acme");
    }

    #[test]
    fn rejects_alg_none_header() {
        let signer = signer();
        let token = signer
            .sign_access_claims(&claims(
                "https://idam.example.com",
                "identity-login",
                "acme",
                300,
            ))
            .expect("sign");
        let parts: Vec<&str> = token.split('.').collect();
        let none_header = URL_SAFE_NO_PAD.encode(br#"{"alg":"none","typ":"at+jwt"}"#);
        let forged = format!("{none_header}.{}.{}", parts[1], parts[2]);
        let err = verify_access_token(&signer, &forged, None).unwrap_err();
        assert!(matches!(
            err,
            JwtError::Validation(JwtValidationError::InvalidAlgorithm)
        ));
    }

    #[test]
    fn rejects_wrong_issuer_after_signature() {
        let signer = signer();
        let token = signer
            .sign_access_claims(&claims(
                "https://attacker.example",
                "identity-login",
                "acme",
                300,
            ))
            .expect("sign");
        let err = verify_access_token(&signer, &token, None).unwrap_err();
        assert!(matches!(
            err,
            JwtError::Validation(JwtValidationError::InvalidIssuer)
        ));
    }

    #[test]
    fn rejects_expired_token() {
        let signer = signer();
        let token = signer
            .sign_access_claims(&claims(
                "https://idam.example.com",
                "identity-login",
                "acme",
                -60,
            ))
            .expect("sign");
        let err = verify_access_token(&signer, &token, None).unwrap_err();
        assert!(matches!(
            err,
            JwtError::Validation(JwtValidationError::Expired)
        ));
    }

    #[test]
    fn rejects_tenant_mismatch() {
        let signer = signer();
        let token = signer
            .sign_access_claims(&claims(
                "https://idam.example.com",
                "identity-login",
                "acme",
                300,
            ))
            .expect("sign");
        assert!(verify_access_token(&signer, &token, Some("other")).is_err());
    }

    #[test]
    fn rejects_wrong_audience() {
        let signer = signer();
        let token = signer
            .sign_access_claims(&claims(
                "https://idam.example.com",
                "not-a-sesame-audience",
                "acme",
                300,
            ))
            .expect("sign");
        let err = verify_access_token(&signer, &token, None).unwrap_err();
        assert!(matches!(
            err,
            JwtError::Validation(JwtValidationError::InvalidAudience)
        ));
    }
}
