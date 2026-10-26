// BRRTRouter: user-owned

//! `POST /platform/tenants/{slug}/oidc/clients` — the operator variant of
//! `POST /tenant/oidc/clients` (ADR-011 §2.4).
//!
//! A platform-provisioned tenant has no administrator yet, so the operator who
//! minted it registers its first relying parties. The behaviour is the tenant
//! surface's ([`OidcClientAdmin::create`]); only the authority differs, and the
//! audit record says so.

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::platform_tenant_oidc_client_create::Request;

use crate::controllers::oidc_client_http::{
    admin_error, client_json, emit_platform_lifecycle_audit, platform_tenant_gate,
};
use crate::services::oidc_client_admin::{ClientAdminError, CreateClientInput, OidcClientAdmin};

#[handler(PlatformTenantOidcClientCreateController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let tenant = match platform_tenant_gate(&req.data.slug) {
        Ok(tenant) => tenant,
        Err(response) => return response,
    };
    let input = CreateClientInput {
        application_id: req.data.application_id.clone(),
        client_type: req.data.client_type.clone(),
        token_endpoint_auth_method: req.data.token_endpoint_auth_method.clone(),
        redirect_uris: req.data.redirect_uris.clone(),
        post_logout_redirect_uris: req
            .data
            .post_logout_redirect_uris
            .clone()
            .unwrap_or_default(),
        initiate_login_uri: req.data.initiate_login_uri.clone(),
        grants: req.data.grants.clone(),
        response_types: req.data.response_types.clone(),
        scopes: req.data.scopes.clone(),
        audiences: req.data.audiences.clone(),
    };

    // The client tables are tenant-RLS'd: write inside the tenant's own context.
    let created = sesame_idam_database::with_pre_auth_tenant(&tenant, |exec| {
        Ok(OidcClientAdmin::create(&tenant, input, exec))
    })
    .map_err(|error| ClientAdminError::Db(error.to_string()))
    .and_then(|result| result);

    match created {
        Ok(created) => {
            emit_platform_lifecycle_audit(
                "oidc_client.created",
                &tenant,
                &created.client.client_id,
                "success",
            );
            HttpJson::new(
                201,
                serde_json::json!({
                    "client": client_json(&created.client),
                    "client_secret": created.client_secret.as_ref().map(|secret| secret.expose_once()),
                    "secret_id": created.secret_id.map(|id| id.to_string()),
                }),
            )
        }
        Err(error) => {
            emit_platform_lifecycle_audit("oidc_client.created", &tenant, "redacted", "failure");
            admin_error(error)
        }
    }
}
