use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::tenant_oidc_client_create::Request;

use crate::controllers::oidc_client_http::{admin_error, client_json, emit_lifecycle_audit};
use crate::services::oidc_client_admin::{CreateClientInput, OidcClientAdmin};
use crate::services::tenant_admin::tenant_admin_principal;

#[handler(TenantOidcClientCreateController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let admin = match tenant_admin_principal(&req.jwt_claims) {
        Ok(admin) => admin,
        Err(response) => return response,
    };
    let exec = sesame_idam_database::db();
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
        grants: req.data.grants.clone(),
        response_types: req.data.response_types.clone(),
        scopes: req.data.scopes.clone(),
        audiences: req.data.audiences.clone(),
    };

    match OidcClientAdmin::create(&admin.tenant, input, exec) {
        Ok(created) => {
            emit_lifecycle_audit(
                "oidc_client.created",
                &admin,
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
            emit_lifecycle_audit("oidc_client.created", &admin, "redacted", "failure");
            admin_error(error)
        }
    }
}
