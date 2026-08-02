use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::tenant_oidc_client_delete::Request;

use crate::controllers::oidc_client_http::{admin_error, emit_lifecycle_audit};
use crate::services::oidc_client_admin::OidcClientAdmin;
use crate::services::tenant_admin::tenant_admin_principal;

#[handler(TenantOidcClientDeleteController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let admin = match tenant_admin_principal(&req.jwt_claims) {
        Ok(admin) => admin,
        Err(response) => return response,
    };
    let client_id = req.data.client_id.trim();
    match OidcClientAdmin::delete(&admin.tenant, client_id, sesame_idam_database::db()) {
        Ok(()) => {
            emit_lifecycle_audit("oidc_client.deleted", &admin, client_id, "success");
            HttpJson::new(204, serde_json::Value::Null)
        }
        Err(error) => {
            emit_lifecycle_audit("oidc_client.deleted", &admin, client_id, "failure");
            admin_error(error)
        }
    }
}
