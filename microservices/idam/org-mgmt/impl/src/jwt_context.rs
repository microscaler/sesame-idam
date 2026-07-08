//! JWT claim helpers for org-mgmt consumer handlers.

use brrtrouter::dispatcher::HandlerRequest;

pub fn claims_from_request(req: &HandlerRequest) -> Option<serde_json::Value> {
    if let Some(claims) = req.jwt_claims.as_ref() {
        return Some(claims.clone());
    }
    let token = bearer_token(req)?;
    decode_jwt_payload_unverified(token)
}

pub fn user_id_from_request(req: &HandlerRequest) -> Option<String> {
    let claims = claims_from_request(req)?;
    claims
        .get("sub")
        .or_else(|| claims.get("user_id"))
        .and_then(|v| v.as_str())
        .map(str::to_string)
}

pub fn tenant_from_request(req: &HandlerRequest) -> Option<String> {
    req.headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("x-tenant-id"))
        .map(|(_, v)| v.clone())
        .or_else(|| {
            claims_from_request(req)?
                .get("tenant_id")
                .and_then(|v| v.as_str())
                .map(str::to_string)
        })
}

fn bearer_token(req: &HandlerRequest) -> Option<&str> {
    req.headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("authorization"))
        .and_then(|(_, v)| v.strip_prefix("Bearer ").or_else(|| v.strip_prefix("bearer ")))
}

fn decode_jwt_payload_unverified(token: &str) -> Option<serde_json::Value> {
    use base64::Engine;
    let payload_b64 = token.split('.').nth(1)?;
    let mut padded = payload_b64.to_string();
    let rem = padded.len() % 4;
    if rem != 0 {
        padded.extend(std::iter::repeat_n('=', 4 - rem));
    }
    let bytes = base64::engine::general_purpose::URL_SAFE
        .decode(padded.as_bytes())
        .ok()?;
    serde_json::from_slice(&bytes).ok()
}
