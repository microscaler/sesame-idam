// BRRTRouter: user-owned

//! Shared plumbing of the organisation domain controllers (story 17.4): the process-wide [`DomainContext`] and the
//! mapping of [`DomainApiError`] to responses. The operations themselves are in `services::domain_api`.

use std::sync::OnceLock;

use brrtrouter::typed::HttpJson;
use serde_json::Value;
use sesame_idam_org_mgmt::org_auth;
use sesame_idam_org_mgmt::services::domain_api::{DomainApiError, DomainContext};
use uuid::Uuid;

static CONTEXT: OnceLock<DomainContext> = OnceLock::new();

/// Keyring, `dns-probe` and eligibility rules, read from the environment on first use.
pub fn context() -> &'static DomainContext {
    CONTEXT.get_or_init(DomainContext::from_env)
}

/// The caller and tenant from the validated token (never a header), or the 401/400 response.
pub fn caller(jwt_claims: &Option<Value>) -> Result<(Uuid, String), HttpJson<Value>> {
    let (user, tenant) = org_auth::require_caller(jwt_claims, None)?;
    let user = Uuid::parse_str(&user)
        .map_err(|_| org_auth::error_json(400, "validation_error", "Invalid user id"))?;
    Ok((user, tenant))
}

/// A path id; anything but a UUID is simply not found.
pub fn path_id(raw: &str) -> Result<Uuid, HttpJson<Value>> {
    Uuid::parse_str(raw).map_err(|_| error(&DomainApiError::NotFound))
}

/// The response for an error; internal details are logged, never sent.
pub fn error(e: &DomainApiError) -> HttpJson<Value> {
    if let DomainApiError::Internal(detail) = e {
        tracing::error!(error = %detail, "organisation domain operation failed");
    }
    HttpJson::new(e.status(), e.body())
}

/// `status` with the value, or the error response.
pub fn respond(status: u16, result: Result<Value, DomainApiError>) -> HttpJson<Value> {
    match result {
        Ok(body) => HttpJson::new(status, body),
        Err(e) => error(&e),
    }
}
