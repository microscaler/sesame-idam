// BRRTRouter: user-owned
//! `PATCH /platform/tenants/{slug}/oidc/clients/{client_id}` — the operator variant of
//! `PATCH /tenant/oidc/clients/{client_id}` (ADR-011 §2.4).
//!
//! The operator registers a platform-provisioned tenant's first relying parties
//! (`POST /platform/tenants/{slug}/oidc/clients`); this lets the same operator correct
//! their policy before the tenant has an administrator. The behaviour is the tenant
//! surface's ([`OidcClientAdmin::update`]); only the authority differs, and the audit
//! record says so.

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::platform_tenant_oidc_client_update::Request;

use crate::controllers::oidc_client_http::{
    admin_error, client_json, emit_platform_lifecycle_audit, platform_tenant_gate,
};
use crate::services::oidc_client_admin::{ClientAdminError, OidcClientAdmin, UpdateClientInput};

#[handler(PlatformTenantOidcClientUpdateController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let tenant = match platform_tenant_gate(&req.data.slug) {
        Ok(tenant) => tenant,
        Err(response) => return response,
    };
    let client_id = req.data.client_id.trim().to_string();
    let input = UpdateClientInput {
        status: req.data.status.clone(),
        redirect_uris: req.data.redirect_uris.clone(),
        post_logout_redirect_uris: req.data.post_logout_redirect_uris.clone(),
        grants: req.data.grants.clone(),
        response_types: req.data.response_types.clone(),
        scopes: req.data.scopes.clone(),
        audiences: req.data.audiences.clone(),
    };
    // The client tables are tenant-RLS'd: write inside the tenant's own context.
    let updated = sesame_idam_database::with_pre_auth_tenant(&tenant, |exec| {
        Ok(OidcClientAdmin::update(&tenant, &client_id, input, exec))
    })
    .map_err(|error| ClientAdminError::Db(error.to_string()))
    .and_then(|result| result);
    match updated {
        Ok(client) => {
            emit_platform_lifecycle_audit("oidc_client.updated", &tenant, &client_id, "success");
            HttpJson::ok(client_json(&client))
        }
        Err(error) => {
            emit_platform_lifecycle_audit("oidc_client.updated", &tenant, &client_id, "failure");
            admin_error(error)
        }
    }
}
