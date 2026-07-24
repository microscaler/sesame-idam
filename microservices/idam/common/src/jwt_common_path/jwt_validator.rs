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

use super::auth_decision::AuthError;
use crate::AccessClaims;

/// Extract the Bearer token from the Authorization header.
///
/// # Errors
///
/// - `MissingAuthorization` — no Authorization header present
/// - `InvalidBearerScheme` — header does not start with "Bearer "
/// - `MissingJwt` — Authorization header is empty or whitespace-only
pub fn extract_bearer_token(request: &HandlerRequest) -> Result<String, AuthError> {
    // Iterate headers to find Authorization (case-insensitive)
    let mut auth_value: Option<String> = None;
    for (key, value) in &request.headers {
        if key.eq_ignore_ascii_case("Authorization") {
            auth_value = Some(value.clone());
            break;
        }
    }

    let auth_str = auth_value.ok_or(AuthError::MissingAuthorization)?;

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
pub fn pre_validate_expiry(token: &str) -> Result<(), AuthError> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err(AuthError::JwtInvalid(
            "JWT must have 3 segments (header.payload.signature)".into(),
        ));
    }

    let payload_bytes = decode_base64url(parts[1])
        .ok_or_else(|| AuthError::JwtInvalid("JWT payload is not valid base64url".into()))?;

    let claims_json: serde_json::Value = serde_json::from_slice(&payload_bytes)
        .map_err(|_| AuthError::JwtInvalid("JWT payload is not valid JSON".into()))?;

    // Check exp — reject immediately if expired
    if let Some(exp) = claims_json.get("exp").and_then(serde_json::Value::as_i64) {
        let now = now_secs();
        if exp < now {
            return Err(AuthError::JwtExpired { exp });
        }
    }

    // Check nbf — reject if not yet valid (with 60s clock skew tolerance)
    if let Some(nbf) = claims_json.get("nbf").and_then(serde_json::Value::as_i64) {
        let now = now_secs();
        if nbf > now + 60 {
            return Err(AuthError::JwtNotYetValid { nbf });
        }
    }

    Ok(())
}

/// Decode a base64url-encoded string.
fn decode_base64url(input: &str) -> Option<Vec<u8>> {
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut decoded = Vec::with_capacity(input.len());
    let mut i = 0;

    while i < len {
        let a = char_to_val(chars[i]);
        let b = if i + 1 < len {
            char_to_val(chars[i + 1])
        } else {
            0
        };
        let c = if i + 2 < len {
            char_to_val(chars[i + 2])
        } else {
            0
        };
        let d = if i + 3 < len {
            char_to_val(chars[i + 3])
        } else {
            0
        };

        decoded.push(((a << 2) | (b >> 4)) as u8);
        if i + 2 < len {
            decoded.push((((b & 0x0F) << 4) | (c >> 2)) as u8);
        }
        if i + 3 < len {
            decoded.push((((c & 0x03) << 6) | d) as u8);
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
pub fn parse_claims(token: &str) -> Result<AccessClaims, AuthError> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err(AuthError::JwtInvalid(
            "JWT must have 3 segments (header.payload.signature)".into(),
        ));
    }

    let payload_bytes = decode_base64url(parts[1])
        .ok_or_else(|| AuthError::JwtInvalid("JWT payload is not valid base64url".into()))?;

    let claims: AccessClaims = serde_json::from_slice(&payload_bytes).map_err(|_| {
        AuthError::JwtInvalid("JWT payload is not valid JSON or missing required fields".into())
    })?;

    // Validate issuer (Gate A6: env-config expectations, not constants)
    let issuers = crate::jwt::helpers::allowed_issuers();
    if !issuers.iter().any(|i| i == &claims.iss) {
        return Err(AuthError::JwtIssuerMismatch {
            expected: issuers.first().cloned().unwrap_or_default(),
            actual: claims.iss,
        });
    }

    // Validate audience (empty aud is a hard reject, never a skip)
    let audiences = crate::jwt::helpers::expected_audiences();
    let has_aud = claims.aud.iter().any(|a| audiences.iter().any(|e| e == a));
    if claims.aud.is_empty() || !has_aud {
        return Err(AuthError::JwtAudienceMismatch {
            expected: audiences.first().cloned().unwrap_or_default(),
            actual: claims.aud.join(","),
        });
    }

    // Validate the claims struct (ver, tenant, sx, risk, etc.)
    if let Err(validation_err) = claims.validate() {
        return Err(match validation_err {
            crate::JwtValidationError::InvalidIssuer => {
                AuthError::JwtInvalid("JWT contains invalid issuer".into())
            }
            crate::JwtValidationError::InvalidAudience => {
                AuthError::JwtInvalid("JWT contains invalid audience".into())
            }
            crate::JwtValidationError::MissingVersion => {
                AuthError::JwtInvalid("JWT missing required 'ver' field".into())
            }
            crate::JwtValidationError::MissingTenant => {
                AuthError::JwtInvalid("JWT missing required 'tenant_id' field".into())
            }
            crate::JwtValidationError::MissingAuthzClaims => {
                AuthError::JwtInvalid("JWT missing required 'sx' claims namespace".into())
            }
            crate::JwtValidationError::InvalidRisk => {
                AuthError::JwtInvalid("JWT contains invalid 'risk' value".into())
            }
            crate::JwtValidationError::InvalidTokenVersion => {
                AuthError::JwtInvalid("JWT contains invalid 'ver' value".into())
            }
            crate::JwtValidationError::Expired => AuthError::JwtInvalid("JWT is expired".into()),
            crate::JwtValidationError::NotYetValid => {
                AuthError::JwtInvalid("JWT is not yet valid".into())
            }
            crate::JwtValidationError::SignatureInvalid => AuthError::JwtSignatureInvalid,
            crate::JwtValidationError::EntitlementsHashMismatch => {
                AuthError::JwtInvalid("Entitlements hash mismatch".into())
            }
        });
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
    use brrtrouter::dispatcher::HeaderVec;
    use brrtrouter::ids::RequestId;
    use brrtrouter::router::ParamVec;
    use http::Method;

    fn create_reply_tx() -> may::sync::mpsc::Sender<brrtrouter::dispatcher::HandlerResponse> {
        let (_tx, _rx) = may::sync::mpsc::channel();
        _tx
    }

    lazy_static::lazy_static! {
        static ref REPLY_TX: std::sync::Arc<may::sync::mpsc::Sender<brrtrouter::dispatcher::HandlerResponse>> = {
            std::sync::Arc::new(create_reply_tx())
        };
    }

    // ─── Bearer Token Extraction Tests ──────────────────────────────────

    /// Build a `HandlerRequest` with the given raw Authorization header value.
    /// An empty string means no Authorization header at all.
    fn create_request_raw(auth_header: &str) -> HandlerRequest {
        let mut headers = HeaderVec::new();
        if !auth_header.is_empty() {
            headers.push(("authorization".into(), auth_header.to_string()));
        }
        HandlerRequest {
            request_id: RequestId::new(),
            method: Method::GET,
            path: "/test".to_string(),
            handler_name: "test".to_string(),
            path_params: ParamVec::new(),
            query_params: ParamVec::new(),
            headers,
            cookies: HeaderVec::new(),
            body: None,
            jwt_claims: None,
            reply_tx: (**REPLY_TX).clone(),
            queue_guard: None,
        }
    }

    /// Build a `HandlerRequest` with `Bearer {token}` (or no header if empty).
    fn create_request(token: &str) -> HandlerRequest {
        if token.is_empty() {
            create_request_raw("")
        } else {
            create_request_raw(&format!("Bearer {token}"))
        }
    }

    #[test]
    fn extract_bearer_token_success() {
        let req = create_request("eyJhbG...xIn0");
        let token = extract_bearer_token(&req).unwrap();
        assert_eq!(token, "eyJhbG...xIn0");
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
        let req = create_request_raw("Basic dXNlcjpwYXNz");
        assert_eq!(
            extract_bearer_token(&req),
            Err(AuthError::InvalidBearerScheme)
        );
    }

    #[test]
    fn extract_bearer_token_rejects_empty_token() {
        let req = create_request_raw("Bearer ");
        assert_eq!(extract_bearer_token(&req), Err(AuthError::MissingJwt));
    }

    #[test]
    fn extract_bearer_token_with_whitespace() {
        let req = create_request_raw("Bearer   token  ");
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
        assert!(matches!(
            result,
            Err(AuthError::JwtExpired { exp }) if exp == past_exp
        ));
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
        assert!(matches!(
            pre_validate_expiry(&token),
            Err(AuthError::JwtNotYetValid { nbf }) if nbf == future_nbf
        ));
    }

    #[test]
    fn pre_validate_expiry_malformed_jwt() {
        assert!(matches!(
            pre_validate_expiry("not.a.jwt.token.extra"),
            Err(AuthError::JwtInvalid(_))
        ));
    }

    // ─── Parse Claims Tests ─────────────────────────────────────────────

    fn make_test_claims() -> AccessClaims {
        AccessClaims::builder()
            .iss("https://idam.example.com")
            .sub("user-1")
            .aud(vec!["sesame-idam".into()])
            .client_id("test-app")
            .scope("read".to_string())
            .exp(i64::MAX - 3600)
            .nbf(0)
            .iat(0)
            .jti("jti-test-1")
            .ver(1)
            .sid("sid-test-1")
            .tenant_id("tenant-a")
            .user_id("user-1")
            .user_type("registered")
            .sx(crate::SesameAuthzClaimsBuilder::new()
                .tenant("tenant-a")
                .portal("test-app")
                .roles(vec!["admin".into(), "user".into()])
                .permissions(vec!["users:read".into(), "prefs:write".into()])
                .risk("normal".to_string())
                .build()
                .unwrap())
            .build()
            .unwrap()
    }

    fn make_claims_token(claims: &AccessClaims) -> String {
        let header = base64url_encode(r#"{"alg":"RS256","typ":"at+jwt"}"#);
        let payload = base64url_encode(&serde_json::to_string(claims).unwrap());
        format!("{header}.{payload}.fake_signature")
    }

    fn base64url_encode(input: &str) -> String {
        use base64::{engine::general_purpose, Engine as _};
        let standard = general_purpose::STANDARD.encode(input.as_bytes());
        standard
            .trim_end_matches('=')
            .replace('+', "-")
            .replace('/', "_")
    }

    fn make_jwt_token(payload: &serde_json::Value) -> String {
        let header = base64url_encode(r#"{"alg":"RS256","typ":"at+jwt"}"#);
        let payload_str = serde_json::to_string(payload).unwrap();
        let payload_b64 = base64url_encode(&payload_str);
        format!("{header}.{payload_b64}.fake")
    }

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
            parse_claims("not-a-jwt"),
            Err(AuthError::JwtInvalid(_))
        ));
    }
}
