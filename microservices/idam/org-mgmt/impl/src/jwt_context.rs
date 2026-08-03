//! JWT claim helpers for org-mgmt consumer handlers.

use brrtrouter::dispatcher::HandlerRequest;

pub fn claims_from_request(req: &HandlerRequest) -> Option<serde_json::Value> {
    req.jwt_claims.clone()
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
    tenant_from_validated_claims(&req.jwt_claims)
}

fn tenant_from_validated_claims(claims: &Option<serde_json::Value>) -> Option<String> {
    claims
        .as_ref()?
        .get("tenant_id")
        .and_then(|value| value.as_str())
        .filter(|tenant| !tenant.trim().is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tenant_comes_only_from_validated_claims() {
        let claims = Some(serde_json::json!({"tenant_id": "acme"}));
        assert_eq!(
            tenant_from_validated_claims(&claims),
            Some("acme".to_string())
        );
    }

    #[test]
    fn missing_validated_claims_have_no_tenant_context() {
        assert_eq!(tenant_from_validated_claims(&None), None);
    }
}
