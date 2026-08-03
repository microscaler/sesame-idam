//! JWT principal extraction for typed handlers (BR-2 / SI-3 login-service).

use brrtrouter::typed::HttpJson;
use serde_json::Value;

/// Extract the authenticated principal (`sub`) and tenant from validated JWT
/// claims, cross-checked against the `X-Tenant-ID` header.
pub fn authenticated_principal(
    jwt_claims: &Option<Value>,
    x_tenant_id: Option<&str>,
) -> Result<(uuid::Uuid, String), HttpJson<Value>> {
    let unauthorized = |desc: &str| {
        HttpJson::new(
            401,
            serde_json::json!({
                "error": "unauthorized",
                "message": desc,
            }),
        )
    };

    let Some(claims) = jwt_claims else {
        return Err(unauthorized("Bearer token required"));
    };

    let Some(sub) = claims.get("sub").and_then(|v| v.as_str()) else {
        return Err(unauthorized("Token missing sub claim"));
    };
    let Ok(user_id) = sub.parse::<uuid::Uuid>() else {
        return Err(unauthorized("Invalid token subject"));
    };

    let Some(tenant_id) = claims.get("tenant_id").and_then(|v| v.as_str()) else {
        return Err(unauthorized("Token missing tenant_id claim"));
    };

    if let Some(x_tenant_id) = x_tenant_id.filter(|value| !value.trim().is_empty()) {
        if x_tenant_id != tenant_id {
            return Err(unauthorized("X-Tenant-ID does not match token tenant"));
        }
    }

    Ok((user_id, tenant_id.to_string()))
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
        let (user_id, tenant_id) =
            authenticated_principal(&claims(Some("acme")), None).expect("valid principal");
        assert_eq!(user_id.to_string(), USER_ID);
        assert_eq!(tenant_id, "acme");
    }

    #[test]
    fn rejects_legacy_header_that_conflicts_with_validated_claims() {
        assert!(
            authenticated_principal(&claims(Some("acme")), Some("other")).is_err(),
            "tenant mismatch must fail"
        );
    }

    #[test]
    fn rejects_token_without_tenant_claim() {
        assert!(
            authenticated_principal(&claims(None), None).is_err(),
            "tenant-less token must fail"
        );
    }
}
