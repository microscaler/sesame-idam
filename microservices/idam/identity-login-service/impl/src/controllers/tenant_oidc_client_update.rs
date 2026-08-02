use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::tenant_oidc_client_update::Request;

use crate::controllers::oidc_client_http::{admin_error, client_json, emit_lifecycle_audit};
use crate::services::oidc_client_admin::{OidcClientAdmin, UpdateClientInput};
use crate::services::tenant_admin::tenant_admin_principal;

#[handler(TenantOidcClientUpdateController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let admin = match tenant_admin_principal(&req.jwt_claims) {
        Ok(admin) => admin,
        Err(response) => return response,
    };
    let client_id = req.data.client_id.trim();
    let input = UpdateClientInput {
        status: req.data.status.clone(),
        redirect_uris: req.data.redirect_uris.clone(),
        post_logout_redirect_uris: req.data.post_logout_redirect_uris.clone(),
        grants: req.data.grants.clone(),
        response_types: req.data.response_types.clone(),
        scopes: req.data.scopes.clone(),
        audiences: req.data.audiences.clone(),
    };
    match OidcClientAdmin::update(&admin.tenant, client_id, input, sesame_idam_database::db()) {
        Ok(client) => {
            emit_lifecycle_audit("oidc_client.updated", &admin, client_id, "success");
            HttpJson::ok(client_json(&client))
        }
        Err(error) => {
            emit_lifecycle_audit("oidc_client.updated", &admin, client_id, "failure");
            admin_error(error)
        }
    }
}
