//! Tenant-only RLS context for unauthenticated IDAM flows (login, register, signup validate).
//!
//! Uses Lifeguard's `with_session_transaction` so `rls_set_session` and application queries share
//! one pinned connection/transaction (per-job autocommit would clear transaction-local GUCs).

use lifeguard::pool::pooled::ExclusivePrimaryLifeExecutor;
use lifeguard::{LifeError, SessionContext};
use uuid::Uuid;

use crate::db;

fn pre_auth_session_context(tenant_id: &str) -> Result<SessionContext, LifeError> {
    let tenant_id = tenant_id.trim();
    if tenant_id.is_empty() {
        return Err(LifeError::Other(
            "pre-auth tenant id is required".to_string(),
        ));
    }

    Ok(SessionContext {
        tenant_id: tenant_id.to_string(),
        subject_id: Uuid::nil(),
        organization_id: Uuid::nil(),
        session_id: "pre-auth".to_string(),
        roles: vec![],
        permissions: vec![],
        user_type: None,
        org_type: None,
    })
}

/// Run `operation` inside a contextual transaction scoped to `tenant_id` only.
///
/// Subject/org UUIDs are nil placeholders — sufficient for the `users` tenant RLS policy until a
/// real JWT-backed session exists.
///
/// # Errors
///
/// Returns [`LifeError`] when context injection or the operation fails.
pub fn with_pre_auth_tenant<T>(
    tenant_id: &str,
    operation: impl FnOnce(&ExclusivePrimaryLifeExecutor<'_>) -> Result<T, LifeError>,
) -> Result<T, LifeError> {
    let context = pre_auth_session_context(tenant_id)?;
    db().pool().with_session_transaction(&context, operation)
}
