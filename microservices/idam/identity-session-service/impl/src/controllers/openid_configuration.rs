/// Handler for `OpenID` Configuration — returns the `OpenID` Connect provider configuration.
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_common::audit::AuditEventType;
use sesame_idam_identity_session_service_gen::handlers::openid_configuration::{Request, Response};

use crate::services::discovery;

#[handler(OpenidConfigurationController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;

    let entry = sesame_common::audit::AuditLogEntry::new(
        AuditEventType::JwtValidated,
        "identity-session-service",
    )
    .decision_source("openid_configuration")
    .result("allowed")
    .build();

    if let Ok(entry) = entry {
        EMITTER.emit(entry);
    }

    let doc = discovery::openid_configuration();

    Response {
        authorization_endpoint: Some(doc.authorization_endpoint),
        code_challenge_methods_supported: Some(doc.code_challenge_methods_supported),
        grant_types_supported: Some(doc.grant_types_supported),
        id_token_signing_alg_values_supported: Some(doc.id_token_signing_alg_values_supported),
        issuer: Some(doc.issuer),
        jwks_uri: Some(doc.jwks_uri),
        registration_endpoint: None,
        response_modes_supported: Some(doc.response_modes_supported),
        response_types_supported: Some(doc.response_types_supported),
        scopes_supported: Some(doc.scopes_supported),
        subject_types_supported: Some(doc.subject_types_supported),
        token_endpoint: Some(doc.token_endpoint),
        userinfo_encryption_alg_values_supported: Some(doc.userinfo_encryption_alg_values_supported),
        userinfo_encryption_enc_values_supported: Some(doc.userinfo_encryption_enc_values_supported),
        userinfo_endpoint: Some(doc.userinfo_endpoint),
        userinfo_signing_alg_values_supported: Some(doc.userinfo_signing_alg_values_supported),
    }
}
