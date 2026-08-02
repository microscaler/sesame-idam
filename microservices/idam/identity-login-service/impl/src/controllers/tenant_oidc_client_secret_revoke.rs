use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::tenant_oidc_client_secret_revoke::Request;
use uuid::Uuid;

use crate::controllers::oidc_client_http::{admin_error, emit_lifecycle_audit};
use crate::services::oidc_client_admin::{ClientAdminError, OidcClientAdmin};
use crate::services::tenant_admin::tenant_admin_principal;

#[handler(TenantOidcClientSecretRevokeController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let admin = match tenant_admin_principal(&req.jwt_claims) {
        Ok(admin) => admin,
        Err(response) => return response,
    };
    let client_id = req.data.client_id.trim();
    let secret_id = match Uuid::parse_str(&req.data.secret_id) {
        Ok(secret_id) => secret_id,
        Err(_) => {
            return admin_error(ClientAdminError::InvalidPolicy(
                "secret_id must be a UUID".to_string(),
            ))
        }
    };
    match OidcClientAdmin::revoke_secret(
        &admin.tenant,
        client_id,
        secret_id,
        sesame_idam_database::db(),
    ) {
        Ok(()) => {
            emit_lifecycle_audit("oidc_client.secret_revoked", &admin, client_id, "success");
            HttpJson::new(204, serde_json::Value::Null)
        }
        Err(error) => {
            emit_lifecycle_audit("oidc_client.secret_revoked", &admin, client_id, "failure");
            admin_error(error)
        }
    }
}
