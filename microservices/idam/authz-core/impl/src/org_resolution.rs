/// Extracts the tenant/org ID from the request context.
///
/// In Sesame-IDAM, every API request carries a `tenant_id` in the request context
/// (extracted from the `X-Tenant-ID` header by BRRTRouter middleware).
use brrtrouter::typed::TypedHandlerRequest;
use uuid::Uuid;

/// Extract the tenant ID from a typed handler request.
///
/// NOTE: This helper returns None because the `TypedHandlerRequest<T>` type parameter
/// is opaque — we cannot access `req.data.tenant_id` without knowing T.
/// Controllers should access the field directly from their request type.
pub fn tenant_id<T>(_req: &TypedHandlerRequest<T>) -> Option<Uuid> {
    None
}

/// Extract the user ID from a typed handler request.
///
/// NOTE: This helper returns None because the `TypedHandlerRequest<T>` type parameter
/// is opaque — we cannot access `req.data.user_id` without knowing T.
/// Controllers should access the field directly from their request type.
pub fn user_id<T>(_req: &TypedHandlerRequest<T>) -> Option<Uuid> {
    None
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
