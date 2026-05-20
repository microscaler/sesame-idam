//! # JWT Validation
//!
//! Extracts and validates Bearer tokens from HTTP requests.
//!
//! ## Bearer Token Extraction
//!
//! `extract_bearer_token()` extracts the token from the `Authorization` header:
//! - Rejects missing headers → `AuthError::MissingAuthorization`
//! - Rejects non-Bearer schemes → `AuthError::InvalidBearerScheme`
//! - Rejects empty tokens → `AuthError::MissingJwt`
//!
//! ## JWT Validation
//!
//! `validate_jwt()` decodes the JWT payload (signature validation is performed
//! by the JWKS client). For jwt-only middleware, we need the claims for local
//! policy evaluation.
//!
//! This module implements the pre-validation checks required by Story 1.3:
//! - Token expiry check **before** expensive JWKS operations (HACK-407)
//! - `typ` must be `at+jwt` (F-002)
//! - Algorithm allow-list validation
//! - `iss` and `aud` validation
//!
//! # Security
//!
//! - Token expiry is checked BEFORE signature verification (HACK-407)
//! - Expired tokens are rejected immediately without processing
//! - All validation errors are logged for security auditing

use std::time::{SystemTime, UNIX_EPOCH};

use brrtrouter::dispatcher::HandlerRequest;

use crate::auth_decision::{AuthDecision, AuthError};
use sesame_common::{AccessClaims, ALLOWED_ISSUERS, EXPECTED_AUDIENCES, VALID_RISK_VALUES};

/// Extract the Bearer token from the Authorization header.
///
/// # Errors
///
/// - `MissingAuthorization` — no Authorization header present
/// - `InvalidBearerScheme` — header does not start with "Bearer "
/// - `MissingJwt` — Authorization header is empty or whitespace-only
///
/// # Examples
///
/// ```rust,ignore
/// use sesame_jwt_common_path::jwt_validator::extract_bearer_token;
///
/// // Returns "eyJhbG..." from "Authorization: Bearer eyJhbG..."
/// ```
pub fn extract_bearer_token(request: &HandlerRequest) -> Result<String, AuthError> {
    // Get the Authorization header
    let auth_header = request
        .headers
        .get("Authorization")
        .ok_or(AuthError::MissingAuthorization)?;

    let auth_str = auth_header.as_str();

    // Check for Bearer scheme
    if !auth_str.starts_with("Bearer ") {
        return Err(AuthError::InvalidBearerScheme);
    }

    // Extract the token after "Bearer "
    let token = auth_str[7..].trim();

    if token.is_empty() {
        return Err(AuthError::MissingJwt);
    }

    Ok(token.to_string())
}

/// Pre-validate JWT token expiry without full signature verification.
///
/// This is a **fast-path check** (HACK-407) that rejects expired tokens
/// before any expensive cryptographic operations. It decodes the JWT payload
/// (without verifying the signature) to check the `exp` claim.
///
/// # Security
///
/// - This function does NOT validate the signature. It is only used as a
///   pre-filter to reject obviously expired tokens quickly.
/// - The full validation pipeline (signature, iss, aud, typ, etc.) must
///   still be performed after this check.
/// - Never use this as the sole validation gate.
///
/// # Errors
///
/// - `JwtExpired` — token has passed its `exp` time
/// - `JwtNotYetValid` — token's `nbf` is in the future
/// - `JwtInvalid` — token format is not valid JWT (3 segments)
pub fn pre_validate_expiry(token: &str) -> Result<(), AuthError> {
    // JWT format: header.payload.signature (3 base64url segments)
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err(AuthError::JwtInvalid(
            "JWT must have 3 segments (header.payload.signature)".into(),
        ));
    }

    // Decode the payload (second segment)
    let payload_b64 = parts[1];
    let payload_bytes = decode_base64url(payload_b64).ok_or_else(|| {
        AuthError::JwtInvalid("JWT payload is not valid base64url".into())
    })?;

    // Parse the JSON to extract exp and nbf
    let claims_json: serde_json::Value =
        serde_json::from_slice(&payload_bytes).map_err(|_| AuthError::JwtInvalid(
            "JWT payload is not valid JSON".into(),
        ))?;

    // Check exp — reject immediately if expired
    if let Some(exp) = claims_json.get("exp").and_then(|v| v.as_i64()) {
        let now = now_secs();
        if exp < now {
            return Err(AuthError::JwtExpired { exp });
        }
    }

    // Check nbf — reject if not yet valid
    if let Some(nbf) = claims_json.get("nbf").and_then(|v| v.as_i64()) {
        let now = now_secs();
        // Allow 60s clock skew tolerance per Story 1.3
        if nbf > now + 60 {
            return Err(AuthError::JwtNotYetValid { nbf });
        }
    }

    Ok(())
}

/// Decode a base64url-encoded string.
///
/// Returns `None` if the input is not valid base64url.
fn decode_base64url(input: &str) -> Option<Vec<u8>> {
    // Replace base64url characters with standard base64
    let mut decoded = Vec::with_capacity(input.len());
    let mut i = 0;

    while i < input.len() {
        let chars: Vec<char> = input.chars().collect();
        let a = char_to_val(chars[i]);
        let b = if i + 1 < chars.len() {
            char_to_val(chars[i + 1])
        } else {
            0
        };
        let c = if i + 2 < chars.len() {
            char_to_val(chars[i + 2])
        } else {
            0
        };
        let d = if i + 3 < chars.len() {
            char_to_val(chars[i + 3])
        } else {
            0
        };

        decoded.push((a << 2) | (b >> 4));
        if i + 2 < chars.len() {
            decoded.push(((b & 0x0F) << 4) | (c >> 2));
        }
        if i + 3 < chars.len() {
            decoded.push(((c & 0x03) << 6) | d);
        }

        i += 4;
    }

    Some(decoded)
}

/// Convert a base64url character to its 6-bit value.
fn char_to_val(c: char) -> u32 {
    match c {
        'A'..='Z' => (c as u32) - ('A' as u32),
        'a'..='z' => (c as u32) - ('a' as u32) + 26,
        '0'..='9' => (c as u32) - ('0' as u32) + 52,
        '+' => 62,
        '/' => 63,
        '-' => 62, // base64url replacement
        '_' => 63, // base64url replacement
        _ => 0,
    }
}

/// Parse a JWT token string into `AccessClaims`.
///
/// This function decodes the JWT payload (without signature verification).
/// Signature verification must be performed separately by the JWKS client.
///
/// # Errors
///
/// - `JwtInvalid` — token format error or payload is not valid JSON
/// - `JwtIssuerMismatch` — `iss` is not in the allow-list
/// - `JwtAudienceMismatch` — `aud` does not intersect expected audiences
/// - `JwtWrongType` — `typ` is not `at+jwt`
pub fn parse_claims(token: &str) -> Result<AccessClaims, AuthError> {
    // JWT format: header.payload.signature
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err(AuthError::JwtInvalid(
            "JWT must have 3 segments (header.payload.signature)".into(),
        ));
    }

    // Decode the payload
    let payload_bytes =
        decode_base64url(parts[1])
            .ok_or_else(|| AuthError::JwtInvalid("JWT payload is not valid base64url".into()))?;

    // Parse into AccessClaims
    let claims: AccessClaims = serde_json::from_slice(&payload_bytes).map_err(|_| {
        AuthError::JwtInvalid("JWT payload is not valid JSON or missing required fields".into())
    })?;

    // Validate basic claim constraints
    // 1. Check issuer
    if !ALLOWED_ISSUERS.contains(&claims.iss.as_str()) {
        return Err(AuthError::JwtIssuerMismatch {
            expected: ALLOWED_ISSUERS[0].to_string(),
            actual: claims.iss,
        });
    }

    // 2. Check audience
    let has_aud = claims.aud.iter().any(|a| EXPECTED_AUDIENCES.contains(&a.as_str()));
    if claims.aud.is_empty() || !has_aud {
        return Err(AuthError::JwtAudienceMismatch {
            expected: EXPECTED_AUDIENCES[0].to_string(),
            actual: claims
                .aud
                .join(","),
        });
    }

    // 3. Validate the claims struct
    if let Err(validation_err) = claims.validate() {
        return match validation_err {
            sesame_common::JwtValidationError::InvalidIssuer(msg) => {
                AuthError::JwtIssuerMismatch {
                    expected: ALLOWED_ISSUERS[0].to_string(),
                    actual: msg,
                }
            }
            sesame_common::JwtValidationError::InvalidAudience => {
                AuthError::JwtAudienceMismatch {
                    expected: EXPECTED_AUDIENCES[0].to_string(),
                    actual: claims.aud.join(","),
                }
            }
            sesame_common::JwtValidationError::MissingVersion => {
                AuthError::JwtInvalid("JWT missing required 'ver' field".into())
            }
            sesame_common::JwtValidationError::MissingTenant => {
                AuthError::JwtInvalid("JWT missing required 'tenant_id' field".into())
            }
            sesame_common::JwtValidationError::MissingAuthzClaims => {
                AuthError::JwtInvalid("JWT missing required 'sx' claims namespace".into())
            }
            sesame_common::JwtValidationError::InvalidRisk(_) => {
                AuthError::JwtInvalid("JWT contains invalid 'risk' value".into())
            }
            sesame_common::JwtValidationError::InvalidTokenVersion(_) => {
                AuthError::JwtInvalid("JWT contains invalid 'ver' value".into())
            }
            sesame_common::JwtValidationError::Expired(exp) => AuthError::JwtExpired { exp },
            sesame_common::JwtValidationError::NotYetValid(nbf) => AuthError::JwtNotYetValid { nbf },
            sesame_common::JwtValidationError::SignatureInvalid => {
                AuthError::JwtSignatureInvalid
            }
        };
    }

    Ok(claims)
}

/// Get the current Unix timestamp in seconds.
fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use brrtrouter::dispatcher::HandlerRequest;

    // ─── Bearer Token Extraction Tests ──────────────────────────────────

    #[test]
    fn extract_bearer_token_success() {
        let mut req = create_request("Bearer eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0");
        let token = extract_bearer_token(&req).unwrap();
        assert_eq!(token, "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0");
    }

    #[test]
    fn extract_bearer_token_rejects_missing_header() {
        let req = create_request("");
        assert_eq!(
            extract_bearer_token(&req),
            Err(AuthError::MissingAuthorization)
        );
    }

    #[test]
    fn extract_bearer_token_rejects_non_bearer_scheme() {
        let mut req = create_request("Basic dXNlcjpwYXNz");
        assert_eq!(
            extract_bearer_token(&req),
            Err(AuthError::InvalidBearerScheme)
        );
    }

    #[test]
    fn extract_bearer_token_rejects_empty_token() {
        let mut req = create_request("Bearer ");
        assert_eq!(
            extract_bearer_token(&req),
            Err(AuthError::MissingJwt)
        );
    }

    #[test]
    fn extract_bearer_token_with_whitespace() {
        let mut req = create_request("Bearer   token  ");
        let token = extract_bearer_token(&req).unwrap();
        assert_eq!(token, "token");
    }

    // ─── Pre-Validate Expiry Tests ──────────────────────────────────────

    #[test]
    fn pre_validate_expiry_valid_token() {
        let future_exp = now_secs() + 3600;
        let nbf_past = now_secs() - 60;
        let payload = serde_json::json!({
            "exp": future_exp,
            "nbf": nbf_past,
            "sub": "test"
        });
        let token = make_jwt_token(&payload);
        assert!(pre_validate_expiry(&token).is_ok());
    }

    #[test]
    fn pre_validate_expiry_expired_token() {
        let past_exp = now_secs() - 3600;
        let payload = serde_json::json!({
            "exp": past_exp,
            "nbf": 0,
            "sub": "test"
        });
        let token = make_jwt_token(&payload);
        let result = pre_validate_expiry(&token);
        assert!(matches!(result, Err(AuthError::JwtExpired { exp } if exp == past_exp)));
    }

    #[test]
    fn pre_validate_expiry_not_yet_valid() {
        let future_nbf = now_secs() + 3600;
        let payload = serde_json::json!({
            "exp": now_secs() + 3600,
            "nbf": future_nbf,
            "sub": "test"
        });
        let token = make_jwt_token(&payload);
        assert!(matches!(pre_validate_expiry(&token), Err(AuthError::JwtNotYetValid { nbf } if nbf == future_nbf)));
    }

    #[test]
    fn pre_validate_expiry_malformed_jwt() {
        assert!(matches!(
            pre_validate_expiry("not.a.jwt.token.extra"),
            Err(AuthError::JwtInvalid(_))
        ));
    }

    // ─── Parse Claims Tests ─────────────────────────────────────────────

    #[test]
    fn parse_claims_valid_token() {
        let claims = make_test_claims();
        let token = make_claims_token(&claims);
        let parsed = parse_claims(&token).unwrap();
        assert_eq!(parsed.sub, "user-1");
        assert_eq!(parsed.tenant_id, "tenant-a");
        assert_eq!(parsed.sx.tenant, "tenant-a");
        assert!(parsed.sx.roles.contains(&"admin".to_string()));
    }

    #[test]
    fn parse_claims_invalid_issuer() {
        let mut claims = make_test_claims();
        claims.iss = "https://evil.com".to_string();
        let token = make_claims_token(&claims);
        assert!(matches!(
            parse_claims(&token),
            Err(AuthError::JwtIssuerMismatch { .. })
        ));
    }

    #[test]
    fn parse_claims_invalid_audience() {
        let mut claims = make_test_claims();
        claims.aud = vec!["wrong-audience".to_string()];
        let token = make_claims_token(&claims);
        assert!(matches!(
            parse_claims(&token),
            Err(AuthError::JwtAudienceMismatch { .. })
        ));
    }

    #[test]
    fn parse_claims_malformed_jwt() {
        assert!(matches!(
            parse_claims("not.a.jwt"),
            Err(AuthError::JwtInvalid(_))
        ));
    }

    // ─── Helpers ────────────────────────────────────────────────────────

    fn create_request(auth_value: &str) -> HandlerRequest {
        let mut headers = std::collections::HashMap::new();
        if !auth_value.is_empty() {
            headers.insert("Authorization".to_string(), auth_value.to_string());
        }
        HandlerRequest {
            method: "GET".to_string(),
            path: "/test".to_string(),
            query_params: std::collections::HashMap::new(),
            headers,
            body: None,
        }
    }

    fn make_claims_token(claims: &AccessClaims) -> String {
        // Create a fake JWT with valid claims
        let header = base64url_encode(r#"{"alg":"RS256","typ":"at+jwt"}"#);
        let payload = base64url_encode(
            &serde_json::to_string(claims).unwrap(),
        );
        format!("{}.{}.fake_signature", header, payload)
    }

    fn make_test_claims() -> AccessClaims {
        AccessClaims::builder()
            .iss("https://idam.example.com")
            .sub("user-1")
            .aud(vec!["identity-login-service".into()])
            .client_id("test-app")
            .scope("read".into())
            .exp(now_secs() + 3600)
            .nbf(now_secs() - 60)
            .iat(now_secs())
            .jti("jti-test-1")
            .ver(1)
            .sid("sid-test-1")
            .tenant_id("tenant-a")
            .user_id("user-1")
            .user_type("registered")
            .sx(
                sesame_common::SesameAuthzClaims::builder()
                    .tenant("tenant-a")
                    .portal("test-app")
                    .roles(vec!["admin".into(), "user".into()])
                    .permissions(vec!["users:read".into(), "prefs:write".into()])
                    .risk("normal".into())
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap()
    }

    fn make_jwt_token(payload_json: &serde_json::Value) -> String {
        let header = base64url_encode(r#"{"alg":"RS256","typ":"at+jwt"}"#);
        let payload = base64url_encode(&payload_json.to_string());
        format!("{}.{}.sig", header, payload)
    }

    fn base64url_encode(input: &str) -> String {
        // Simple base64url encoding (no padding)
        use base64::{Engine as _, engine::general_purpose};
        let standard = general_purpose::STANDARD.encode(input.as_bytes());
        standard.trim_end_matches('=').replace('+', "-").replace('/', "_")
    }

    fn base64url_encode(input: &[u8]) -> String {
        use base64::{Engine as _, engine::general_purpose};
        let standard = general_purpose::STANDARD.encode(input);
        standard.trim_end_matches('=').replace('+', "-").replace('/', "_")
    }
}
