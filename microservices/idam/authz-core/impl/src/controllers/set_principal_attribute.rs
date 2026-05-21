use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_authz_core_gen::handlers::set_principal_attribute::{Request, Response};
use sesame_token_versioning::BumpReason;

/// Handler for Set Principal Attribute
#[handler(SetPrincipalAttributeController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    // Story 5.4: Publish push invalidation event for principal attribute change
    if let Some(publisher) = &*crate::audit::PUBLISHER {
        publisher.publish_tenant(
            &req.data.tenant_id,
            0, // version is managed by VersionStore
            BumpReason::PrincipalAttributeModified,
        );
    }

    Response {
        error: String::new(),
        error_description: None,
    }
}
