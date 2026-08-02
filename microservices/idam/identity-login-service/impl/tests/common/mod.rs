// Test helpers for identity-login-service BDD tests.

use lifeguard::LifeExecutor;
use sea_query::Values;
use uuid::Uuid;

use sesame_idam_identity_login_service::services::tenant_service::{TenantService, STATUS_ACTIVE};

/// Seeded public SPA client used by hauliage (and most classic-login BDD).
pub const HAULIAGE_WEB_CLIENT: &str = "hauliage-web";
/// Tenant bound to [`HAULIAGE_WEB_CLIENT`] in the relying-party registry.
pub const HAULIAGE_TENANT: &str = "hauliage";

/// Ensure a tenant slug exists in the platform registry before auth operations.
///
/// BDD tests use synthetic tenant slugs; the registry rejects unknown slugs
/// (`tenant_unknown`). Idempotent — safe to call on every test.
///
/// # Panics
///
/// Panics when the test database cannot look up or create the requested tenant.
pub fn ensure_active_tenant(slug: &str) {
    let exec = sesame_idam_database::db();
    match TenantService::find_by_slug(slug, exec) {
        Ok(Some(t)) if t.status == STATUS_ACTIVE => {}
        Ok(Some(_)) => {
            // Suspended/provisioning — recreate is not supported; tests use fresh slugs.
        }
        Ok(None) => match TenantService::create_active_platform(slug, slug, exec) {
            Ok(_) => {}
            Err(e) => {
                let msg = e.to_string();
                // Parallel BDD tests may race on the same synthetic slug.
                if msg.contains("duplicate") || msg.contains("unique") {
                    match TenantService::find_by_slug(slug, exec) {
                        Ok(Some(t)) if t.status == STATUS_ACTIVE => {}
                        other => {
                            panic!("ensure_active_tenant({slug}) race recovery: {other:?}");
                        }
                    }
                } else {
                    panic!("ensure_active_tenant({slug}): {e}");
                }
            }
        },
        Err(e) => panic!("ensure_active_tenant({slug}) lookup: {e}"),
    }
}

/// Create a public OIDC client bound to `tenant` so classic `/auth/login`
/// (which resolves tenant from `client_id`) can succeed for synthetic tenants.
///
/// Uses raw SQL inside a pre-auth RLS transaction (`app.tenant_id` set) so the
/// insert satisfies `relying_party_clients_tenant_isolation`.
///
/// Returns the generated `client_id` (`ses_bdd_…`).
///
/// # Panics
///
/// Panics when the registry insert fails.
pub fn ensure_public_login_client(tenant: &str) -> String {
    let client_id = format!("ses_bdd_{}", Uuid::new_v4().simple());
    let client_pk = Uuid::new_v4();
    let redirect = "https://bdd.example.test/callback";

    sesame_idam_database::with_pre_auth_tenant(tenant, |exec| {
        exec.execute_values(
            "INSERT INTO sesame_idam.relying_party_clients (
                id, client_id, tenant_slug, portal, application_id, client_type,
                token_endpoint_auth_method, pkce_s256_required, authority_class,
                status, created_at, updated_at
             ) VALUES (
                $1, $2, $3, 'bdd-login', 'bdd-login', 'public',
                'none', true, 'tenant', 'active', NOW(), NOW()
             )",
            &Values(vec![client_pk.into(), client_id.clone().into(), tenant.into()]),
        )?;

        exec.execute_values(
            "INSERT INTO sesame_idam.relying_party_client_redirect_uris (
                id, relying_party_client_id, kind, uri, created_at
             ) VALUES ($1, $2, 'login', $3, NOW())",
            &Values(vec![
                Uuid::new_v4().into(),
                client_pk.into(),
                redirect.into(),
            ]),
        )?;

        for (kind, value) in [
            ("grant", "authorization_code"),
            ("grant", "refresh_token"),
            ("response_type", "code"),
            ("scope", "openid"),
            ("scope", "profile"),
            ("scope", "email"),
            ("audience", "sesame-idam"),
        ] {
            exec.execute_values(
                "INSERT INTO sesame_idam.relying_party_client_capabilities (
                    id, relying_party_client_id, kind, value, created_at
                 ) VALUES ($1, $2, $3, $4, NOW())",
                &Values(vec![
                    Uuid::new_v4().into(),
                    client_pk.into(),
                    kind.into(),
                    value.into(),
                ]),
            )?;
        }
        Ok(client_id.clone())
    })
    .unwrap_or_else(|e| panic!("ensure_public_login_client({tenant}): {e}"))
}
