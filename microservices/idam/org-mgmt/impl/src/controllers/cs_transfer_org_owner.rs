// BRRTRouter: user-owned

//! `POST /cs/organizations/{org_id}/owner/transfer` — tenant CS privileged succession.

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_org_mgmt_gen::handlers::cs_transfer_org_owner::Request;

use super::transfer_org_owner::{finish_transfer, map_transfer_error};
use crate::services::org_lifecycle::{self, FormerOwnerDisposition, OrgLifecycleError, TransferActor};
use sesame_idam_org_mgmt::org_auth;
use sesame_idam_org_mgmt::tenant_cs_auth::{self, TENANT_CS_HEADER};

#[handler(CsTransferOrgOwnerController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    // Defense in depth: middleware registers TenantCsAuth; re-check tenant bind here
    // using the required X-Tenant-ID. CS key is validated by the security provider;
    // when available on path_params/headers via env, require_tenant_cs also runs if
    // SESAME_TENANT_CS_KEYS is set (handler cannot read raw headers from typed req).
    let tenant_id = req.data.x_tenant_id.trim();
    if tenant_id.is_empty() {
        return org_auth::error_json(401, "unauthorized", "X-Tenant-ID header is required");
    }

    // Re-load keys and ensure tenant has a configured CS credential (provider already
    // matched key↔tenant). This catches unconfigured tenants with a clear 401.
    if let Err(err) = tenant_cs_auth::load_tenant_cs_keys_from_env().and_then(|keys| {
        if keys.contains_key(tenant_id) {
            Ok(())
        } else {
            Err(tenant_cs_auth::TenantCsAuthError::Invalid)
        }
    }) {
        let _ = TENANT_CS_HEADER; // documented header name for operators
        return tenant_cs_auth::tenant_cs_http_error(&err);
    }

    let reason = req.data.reason.as_deref().map(str::trim).unwrap_or("");
    if reason.is_empty() {
        return org_auth::error_json(
            400,
            "validation_error",
            "reason is required on the tenant CS transfer path",
        );
    }

    let disposition = match FormerOwnerDisposition::parse(req.data.former_owner_disposition.as_deref())
    {
        Ok(d) => d,
        Err(OrgLifecycleError::InvalidId(msg)) => {
            return org_auth::error_json(400, "validation_error", &msg);
        }
        Err(_) => {
            return org_auth::error_json(400, "validation_error", "Invalid disposition");
        }
    };

    let exec = sesame_idam_database::db();
    match org_lifecycle::transfer_owner(
        exec,
        tenant_id,
        &req.data.org_id,
        TransferActor::TenantCs,
        &req.data.successor_user_id,
        req.data.from_user_id.as_deref(),
        disposition,
    ) {
        Ok(result) => finish_transfer(
            result,
            "tenant_cs",
            Some(reason),
            req.data.ticket_id.as_deref(),
        ),
        Err(err) => map_transfer_error(err),
    }
}
