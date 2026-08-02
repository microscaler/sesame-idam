use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::tenant_oidc_client_secret_rotate::Request;

use crate::controllers::oidc_client_http::{admin_error, emit_lifecycle_audit};
use crate::services::oidc_client_admin::OidcClientAdmin;
use crate::services::tenant_admin::tenant_admin_principal;

#[handler(TenantOidcClientSecretRotateController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let admin = match tenant_admin_principal(&req.jwt_claims) {
        Ok(admin) => admin,
        Err(response) => return response,
    };
    let client_id = req.data.client_id.trim();
    match OidcClientAdmin::rotate_secret(
        &admin.tenant,
        client_id,
        i64::from(req.data.overlap_seconds.unwrap_or(300)),
        sesame_idam_database::db(),
    ) {
        Ok(rotated) => {
            emit_lifecycle_audit("oidc_client.secret_rotated", &admin, client_id, "success");
            HttpJson::new(
                201,
                serde_json::json!({
                    "client_id": rotated.client_id,
                    "secret_id": rotated.secret_id.to_string(),
                    "client_secret": rotated.client_secret.expose_once(),
                    "created_at": rotated.created_at.to_rfc3339(),
                    "previous_secrets_valid_until": rotated.previous_secrets_valid_until.to_rfc3339(),
                }),
            )
        }
        Err(error) => {
            emit_lifecycle_audit("oidc_client.secret_rotated", &admin, client_id, "failure");
            admin_error(error)
        }
    }
}
