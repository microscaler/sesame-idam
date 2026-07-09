// BRRTRouter: user-owned

//! POST /sessions/active-organization — re-issue JWT with `org_id` after org create/accept.

use base64::Engine;
use brrtrouter::dispatcher::{HandlerRequest, HandlerResponse};
use sesame_idam_database::db;

const DEFAULT_PORTAL: &str = "frontend";

fn bearer_token(req: &HandlerRequest) -> Option<&str> {
    req.headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("authorization"))
        .and_then(|(_, v)| v.strip_prefix("Bearer ").or_else(|| v.strip_prefix("bearer ")))
}

fn claims_from_request(req: &HandlerRequest) -> Option<serde_json::Value> {
    if let Some(claims) = req.jwt_claims.as_ref() {
        return Some(claims.clone());
    }
    let token = bearer_token(req)?;
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

fn tenant_from_request(req: &HandlerRequest) -> Option<String> {
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

pub fn handle(req: HandlerRequest) -> HandlerResponse {
    let Some(tenant_id) = tenant_from_request(&req) else {
        return HandlerResponse::json(
            400,
            serde_json::json!({
                "error": "missing_tenant",
                "message": "X-Tenant-ID header is required"
            }),
        );
    };

    let Some(claims) = claims_from_request(&req) else {
        return HandlerResponse::json(
            401,
            serde_json::json!({
                "error": "unauthorized",
                "message": "Bearer token required"
            }),
        );
    };

    let user_id = claims
        .get("sub")
        .or_else(|| claims.get("user_id"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if user_id.is_empty() {
        return HandlerResponse::json(
            401,
            serde_json::json!({
                "error": "unauthorized",
                "message": "Invalid token subject"
            }),
        );
    }

    let body = req.body.clone().unwrap_or(serde_json::json!({}));
    let org_id = body
        .get("organization_id")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .unwrap_or("");

    if org_id.is_empty() {
        return HandlerResponse::json(
            400,
            serde_json::json!({
                "error": "validation_error",
                "message": "organization_id is required"
            }),
        );
    }

    let exec = db();
    let active = crate::services::org_context::resolve_active_org_id(
        exec,
        user_id,
        &tenant_id,
        Some(org_id),
    );

    if active.is_none() {
        return HandlerResponse::json(
            403,
            serde_json::json!({
                "error": "forbidden",
                "message": "User is not an active member of this organization"
            }),
        );
    }

    let roles = crate::services::authz_client::fetch_effective_roles(
        user_id,
        &tenant_id,
        DEFAULT_PORTAL,
    )
    .unwrap_or_default();

    let tokens = match crate::services::token_issuer::issue_tokens(
        user_id,
        &tenant_id,
        DEFAULT_PORTAL,
        roles,
        "customer",
        Some(org_id),
    ) {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(error = %e, "set_active_organization: token issuance failed");
            return HandlerResponse::error(500, "Token issuance failed");
        }
    };

    HandlerResponse::json(
        200,
        serde_json::json!({
            "access_token": tokens.access_token,
            "refresh_token": tokens.refresh_token,
            "expires_in": tokens.expires_in,
            "token_type": "Bearer",
            "user_id": user_id,
            "organization_id": org_id,
        }),
    )
}
