//! JWT principal extraction for typed handlers (BR-2 / SI-3).

use brrtrouter::typed::HttpJson;
use serde_json::Value;

/// Extract the authenticated principal (`sub`) and tenant from validated JWT
/// claims, cross-checked against the `X-Tenant-ID` header (HACK-401: both
/// locations must agree).
pub fn authenticated_principal(
    jwt_claims: &Option<Value>,
    x_tenant_id: &str,
) -> Result<(uuid::Uuid, String), HttpJson<Value>> {
    let unauthorized = |desc: &str| {
        HttpJson::new(
            401,
            serde_json::json!({
                "error": "invalid_request",
                "error_description": desc,
            }),
        )
    };

    let Some(claims) = jwt_claims else {
        return Err(unauthorized("Unauthorized (invalid or missing token)"));
    };

    let Some(sub) = claims.get("sub").and_then(|v| v.as_str()) else {
        return Err(unauthorized("Token missing sub claim"));
    };
    let Ok(user_id) = sub.parse::<uuid::Uuid>() else {
        return Err(unauthorized("Token sub is not a valid user id"));
    };

    let Some(tenant_id) = claims.get("tenant_id").and_then(|v| v.as_str()) else {
        return Err(unauthorized("Token missing tenant_id claim"));
    };

    if x_tenant_id != tenant_id {
        return Err(unauthorized("X-Tenant-ID does not match token tenant"));
    }

    Ok((user_id, tenant_id.to_string()))
}
