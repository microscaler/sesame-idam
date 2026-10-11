// BRRTRouter: user-owned

//! `GET /platform/tenants/{slug}/oidc/clients` — the operator variant of
//! `GET /tenant/oidc/clients` (ADR-011 §2.4): the same redacted view.

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::platform_tenant_oidc_client_list::Request;

use crate::controllers::oidc_client_http::{admin_error, client_json, platform_tenant_gate};
use crate::services::oidc_client_admin::{ClientAdminError, OidcClientAdmin};

#[handler(PlatformTenantOidcClientListController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let tenant = match platform_tenant_gate(&req.data.slug) {
        Ok(tenant) => tenant,
        Err(response) => return response,
    };
    let listed = sesame_idam_database::with_pre_auth_tenant(&tenant, |exec| {
        Ok(OidcClientAdmin::list(&tenant, exec))
    })
    .map_err(|error| ClientAdminError::Db(error.to_string()))
    .and_then(|result| result);
    match listed {
        Ok(clients) => HttpJson::ok(serde_json::json!({
            "items": clients.iter().map(client_json).collect::<Vec<_>>(),
        })),
        Err(error) => admin_error(error),
    }
}
