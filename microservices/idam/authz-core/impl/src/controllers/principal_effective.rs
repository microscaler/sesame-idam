use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;
use sesame_idam_authz_core_gen::handlers::principal_effective::{Request, Response};

/// Principal effective permissions controller.
#[handler(PrincipalEffectiveController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    Response {
        attributes: Some(serde_json::json!({})),
        permissions: vec![],
        roles: vec![],
        user_id: req.data.user_id,
    }
}
