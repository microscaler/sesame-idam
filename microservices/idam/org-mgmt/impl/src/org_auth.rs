//! Authenticated principal helpers for typed org-mgmt handlers.

use brrtrouter::typed::HttpJson;
use serde_json::Value;
use uuid::Uuid;

pub fn caller_user_id(jwt_claims: &Option<Value>) -> Option<String> {
    let claims = jwt_claims.as_ref()?;
    claims
        .get("sub")
        .or_else(|| claims.get("user_id"))
        .and_then(|v| v.as_str())
        .map(str::to_string)
}

pub fn error_json(status: u16, error: &str, message: &str) -> HttpJson<serde_json::Value> {
    HttpJson::new(
        status,
        serde_json::json!({
            "error": error,
            "message": message,
        }),
    )
}

pub fn require_caller(
    jwt_claims: &Option<Value>,
    tenant_header: &str,
) -> Result<(String, String), HttpJson<serde_json::Value>> {
    if tenant_header.trim().is_empty() {
        return Err(error_json(
            400,
            "missing_tenant",
            "X-Tenant-ID header is required",
        ));
    }

    let Some(user_id) = caller_user_id(jwt_claims) else {
        return Err(error_json(401, "unauthorized", "Authentication required"));
    };

    if let Some(claims) = jwt_claims.as_ref() {
        if let Some(claim_tenant) = claims.get("tenant_id").and_then(|v| v.as_str()) {
            if claim_tenant != tenant_header {
                return Err(error_json(
                    403,
                    "tenant_mismatch",
                    "Token tenant does not match X-Tenant-ID",
                ));
            }
        }
    }

    if Uuid::parse_str(&user_id).is_err() {
        return Err(error_json(400, "validation_error", "Invalid user id"));
    }

    Ok((user_id, tenant_header.to_string()))
}
