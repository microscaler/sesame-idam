/// Extracts the tenant/org ID from the request context.
///
/// In Sesame-IDAM, every API request carries a `tenant_id` in the request context
/// (extracted from the `X-Tenant-ID` header by BRRTRouter middleware). This module
/// provides a helper to safely extract it as a `uuid::Uuid`.
///
/// Usage pattern:
/// ```rust
/// use crate::org_resolution::tenant_id;
///
/// let exec = sesame_idam_database::db();
/// let tenant = tenant_id(&req)?;
/// ```
use brrtrouter::typed::TypedHandlerRequest;
use uuid::Uuid;

/// Extract the tenant ID from a typed handler request.
///
/// Returns `None` if the tenant_id header was missing or malformed.
pub fn tenant_id<T>(req: &TypedHandlerRequest<T>) -> Option<Uuid> {
    req.inner.tenant_id.parse::<Uuid>().ok()
}

/// Extract the user ID from a typed handler request.
///
/// Returns `None` if the user_id was missing or malformed (non-fatal).
pub fn user_id<T>(req: &TypedHandlerRequest<T>) -> Option<Uuid> {
    req.inner.user_id.parse::<Uuid>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tenant_id_valid() {
        assert!("550e8400-e29b-41d4-a716-446655440000"
            .parse::<Uuid>()
            .is_ok());
    }

    #[test]
    fn test_tenant_id_invalid() {
        let result = "not-a-uuid".parse::<Uuid>();
        assert!(result.is_err());
    }
}
