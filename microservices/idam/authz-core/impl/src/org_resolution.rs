/// Extracts the tenant/org ID from the request context.
///
/// In Sesame-IDAM, every API request carries a `tenant_id` in the request context
/// (extracted from the `X-Tenant-ID` header by BRRTRouter middleware).
use brrtrouter::typed::TypedHandlerRequest;
use uuid::Uuid;

/// Extract the tenant ID from a typed handler request.
/// Returns the `tenant_id` field if present on the inner data type.
pub fn tenant_id<T: HasTenantId>(req: &TypedHandlerRequest<T>) -> Option<Uuid> {
    req.data.tenant_id().parse::<Uuid>().ok()
}

/// Extract the user ID from a typed handler request.
/// Returns the `user_id` field if present on the inner data type.
pub fn user_id<T: HasUserId>(req: &TypedHandlerRequest<T>) -> Option<Uuid> {
    req.data.user_id().parse::<Uuid>().ok()
}

/// Trait for accessing the tenant ID from handler request data.
pub trait HasTenantId {
    fn tenant_id(&self) -> String;
}

/// Trait for accessing the user ID from handler request data.
pub trait HasUserId {
    fn user_id(&self) -> String;
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_tenant_id_valid() {
        assert!("550e8400-e29b-41d4-a716-446655440000".parse::<Uuid>().is_ok());
    }
}
