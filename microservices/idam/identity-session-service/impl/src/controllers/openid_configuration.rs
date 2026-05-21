/// Handler for OpenID Configuration — returns the OpenID Connect provider configuration.
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_audit::AuditEventType;
use sesame_idam_identity_session_service_gen::handlers::openid_configuration::{Request, Response};

#[handler(OpenidConfigurationController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    use crate::audit::EMITTER;

    let entry =
        sesame_audit::AuditLogEntry::new(AuditEventType::JwtValidated, "identity-session-service")
            .decision_source("openid_configuration")
            .result("allowed")
            .build();

    if let Ok(entry) = entry {
        EMITTER.emit(entry);
    }

    Response {
        authorization_endpoint: None,
        code_challenge_methods_supported: None,
        grant_types_supported: None,
        id_token_signing_alg_values_supported: None,
        issuer: None,
        jwks_uri: None,
        registration_endpoint: None,
        response_modes_supported: None,
        response_types_supported: None,
        scopes_supported: None,
        subject_types_supported: None,
        token_endpoint: None,
        userinfo_encryption_alg_values_supported: None,
        userinfo_encryption_enc_values_supported: None,
        userinfo_endpoint: None,
        userinfo_signing_alg_values_supported: None,
    }
}
