use brrtrouter_macros::handler;
use brrtrouter::typed::TypedHandlerRequest;
use sesame_idam_identity_login_service_gen::handlers::signup_validate::{Request, Response};

/// Handler for Signup Validate.
#[handler(SignupValidateController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    // TODO: Check if email is already registered in the tenant
    // TODO: Check if username is already taken
    // TODO: Validate email format (RFC 5322)
    // TODO: Check domain against org-level restrictions (e.g., only @company.com emails)
    // TODO: Check password strength requirements

    // Placeholder: all validations pass
    Response {
        allowed: true,
        reasons: vec![],
        requires_mfa: false,
    }
}
