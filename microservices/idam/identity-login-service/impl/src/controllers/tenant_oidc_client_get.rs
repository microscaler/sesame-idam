use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::tenant_oidc_client_get::Request;

use crate::controllers::oidc_client_http::{admin_error, client_json};
use crate::services::oidc_client_admin::OidcClientAdmin;
use crate::services::tenant_admin::tenant_admin_principal;

#[handler(TenantOidcClientGetController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let admin = match tenant_admin_principal(&req.jwt_claims) {
        Ok(admin) => admin,
        Err(response) => return response,
    };
    match OidcClientAdmin::get(
        &admin.tenant,
        req.data.client_id.trim(),
        sesame_idam_database::db(),
    ) {
        Ok(client) => HttpJson::ok(client_json(&client)),
        Err(error) => admin_error(error),
    }
}
