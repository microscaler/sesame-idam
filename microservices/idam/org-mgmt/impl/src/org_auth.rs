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
    tenant_header: Option<&str>,
) -> Result<(String, String), HttpJson<serde_json::Value>> {
    let Some(user_id) = caller_user_id(jwt_claims) else {
        return Err(error_json(401, "unauthorized", "Authentication required"));
    };

    let Some(claim_tenant) = jwt_claims
        .as_ref()
        .and_then(|claims| claims.get("tenant_id"))
        .and_then(|value| value.as_str())
        .filter(|tenant| !tenant.trim().is_empty())
    else {
        return Err(error_json(
            401,
            "unauthorized",
            "Token missing tenant_id claim",
        ));
    };

    if let Some(tenant_header) = tenant_header.filter(|value| !value.trim().is_empty()) {
        if claim_tenant != tenant_header {
            return Err(error_json(
                403,
                "tenant_mismatch",
                "Token tenant does not match X-Tenant-ID",
            ));
        }
    }

    if Uuid::parse_str(&user_id).is_err() {
        return Err(error_json(400, "validation_error", "Invalid user id"));
    }

    Ok((user_id, claim_tenant.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const USER_ID: &str = "1189c444-8a2d-4c41-8b4b-ae43ce79a492";

    fn claims(tenant_id: Option<&str>) -> Option<Value> {
        Some(serde_json::json!({
            "sub": USER_ID,
            "tenant_id": tenant_id,
        }))
    }

    #[test]
    fn derives_tenant_from_validated_claims_without_public_header() {
        let principal = require_caller(&claims(Some("hauliage")), None).expect("valid principal");
        assert_eq!(principal, (USER_ID.to_string(), "hauliage".to_string()));
    }

    #[test]
    fn rejects_legacy_header_that_conflicts_with_validated_claims() {
        let response = require_caller(&claims(Some("hauliage")), Some("other"))
            .expect_err("tenant mismatch must fail");
        assert_eq!(response.status, 403);
    }

    #[test]
    fn rejects_token_without_tenant_claim() {
        let response =
            require_caller(&claims(None), None).expect_err("tenant-less token must fail");
        assert_eq!(response.status, 401);
    }
}
