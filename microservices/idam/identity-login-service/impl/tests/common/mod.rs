// Test helpers for identity-login-service BDD tests.

use sesame_idam_identity_login_service::services::tenant_service::{
    TenantService, PROVISIONING_PLATFORM, STATUS_ACTIVE,
};

/// Ensure a tenant slug exists in the platform registry before auth operations.
///
/// BDD tests use synthetic tenant slugs; the registry rejects unknown slugs
/// (`tenant_unknown`). Idempotent — safe to call on every test.
pub fn ensure_active_tenant(slug: &str) {
    let exec = sesame_idam_database::db();
    match TenantService::find_by_slug(slug, exec) {
        Ok(Some(t)) if t.status == STATUS_ACTIVE => {}
        Ok(Some(_)) => {
            // Suspended/provisioning — recreate is not supported; tests use fresh slugs.
        }
        Ok(None) => {
            TenantService::create(slug, slug, PROVISIONING_PLATFORM, exec)
                .unwrap_or_else(|e| panic!("ensure_active_tenant({slug}): {e}"));
        }
        Err(e) => panic!("ensure_active_tenant({slug}) lookup: {e}"),
    }
}
