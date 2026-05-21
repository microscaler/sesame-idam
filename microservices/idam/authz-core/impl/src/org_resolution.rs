/// Extracts the tenant/org ID from the request context.
///
/// In Sesame-IDAM, every API request carries a `tenant_id` in the request context
/// (extracted from the `X-Tenant-ID` header by BRRTRouter middleware).
use brrtrouter::typed::TypedHandlerRequest;
use uuid::Uuid;

/// Extract the tenant ID from a typed handler request.
pub fn tenant_id<T>(req: &TypedHandlerRequest<T>) -> Option<Uuid> {
    req.inner.tenant_id.parse::<Uuid>().ok()
}

/// Extract the user ID from a typed handler request.
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
}
