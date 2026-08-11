// Test helpers for identity-login-service BDD tests.

use lifeguard::LifeExecutor;
use sea_query::Values;
use uuid::Uuid;

use sesame_idam_identity_login_service::services::tenant_service::{TenantService, STATUS_ACTIVE};

/// Default demo tenant slug used by classic-login BDD and OSS seeds.
///
/// Override live/private lab deployments with `SESAME_LIVE_TEST_TENANT`.
pub const FIXTURE_TENANT: &str = "acme";
/// Public SPA client bound to [`FIXTURE_TENANT`] in the relying-party registry.
///
/// Override with `SESAME_LIVE_TEST_CLIENT_ID` for private lab clients.
pub const FIXTURE_WEB_CLIENT: &str = "acme-web";

/// Tenant for live API tests (env override for private labs).
pub fn live_test_tenant() -> String {
    std::env::var("SESAME_LIVE_TEST_TENANT").unwrap_or_else(|_| FIXTURE_TENANT.to_string())
}

/// Client id for live API tests (env override for private labs).
pub fn live_test_client_id() -> String {
    std::env::var("SESAME_LIVE_TEST_CLIENT_ID").unwrap_or_else(|_| FIXTURE_WEB_CLIENT.to_string())
}

/// Redirect URI for live authorize tests (env override for private labs).
pub fn live_test_redirect() -> String {
    std::env::var("SESAME_LIVE_TEST_REDIRECT")
        .unwrap_or_else(|_| "https://app.example.com/auth/callback".to_string())
}

/// Demo user email for live login tests (env override for private labs).
pub fn live_test_email() -> String {
    std::env::var("SESAME_LIVE_TEST_EMAIL").unwrap_or_else(|_| "owner@acme.example".to_string())
}

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

/// Ensure the fixture SPA client [`FIXTURE_WEB_CLIENT`] exists for [`FIXTURE_TENANT`].
///
/// Used by Pact provider verification when the LAN/demo DB is missing seed rows.
///
/// # Panics
///
/// Panics when the registry upsert fails.
pub fn ensure_fixture_web_client() {
    use sesame_idam_identity_login_service::services::client_registry::ClientRegistry;

    ensure_active_tenant(FIXTURE_TENANT);
    let exec = sesame_idam_database::db();
    if ClientRegistry::resolve_active(FIXTURE_WEB_CLIENT, exec).is_ok() {
        return;
    }

    // Fresh UUID — demo DBs may already use seed PKs for other client_ids.
    let _ = ensure_public_login_client_named(FIXTURE_TENANT, FIXTURE_WEB_CLIENT);
}

/// Idempotent public client insert for a fixed `client_id` (Pact provider state).
fn ensure_public_login_client_named(tenant: &str, client_id: &str) -> String {
    let client_pk = Uuid::new_v4();
    let redirect = "https://app.example.com/auth/callback";

    sesame_idam_database::with_pre_auth_tenant(tenant, |exec| {
        exec.execute_values(
            "INSERT INTO sesame_idam.relying_party_clients (
                id, client_id, tenant_slug, portal, application_id, client_type,
                token_endpoint_auth_method, pkce_s256_required, authority_class,
                status, created_at, updated_at
             ) VALUES (
                $1, $2, $3, 'frontend', 'frontend', 'public',
                'none', true, 'tenant', 'active', NOW(), NOW()
             )
             ON CONFLICT (client_id) DO UPDATE SET
                status = 'active',
                updated_at = NOW()",
            &Values(vec![
                client_pk.into(),
                client_id.to_string().into(),
                tenant.into(),
            ]),
        )?;

        // Best-effort redirect/capability rows; ignore duplicates.
        let _ = exec.execute_values(
            "INSERT INTO sesame_idam.relying_party_client_redirect_uris (
                id, relying_party_client_id, kind, uri, created_at
             )
             SELECT $1, id, 'login', $2, NOW()
             FROM sesame_idam.relying_party_clients WHERE client_id = $3
             ON CONFLICT DO NOTHING",
            &Values(vec![
                Uuid::new_v4().into(),
                redirect.into(),
                client_id.to_string().into(),
            ]),
        );

        for (kind, value) in [
            ("grant", "authorization_code"),
            ("grant", "refresh_token"),
            ("response_type", "code"),
            ("scope", "openid"),
            ("scope", "profile"),
            ("scope", "email"),
            ("audience", "sesame-idam"),
        ] {
            let _ = exec.execute_values(
                "INSERT INTO sesame_idam.relying_party_client_capabilities (
                    id, relying_party_client_id, kind, value, created_at
                 )
                 SELECT $1, id, $2, $3, NOW()
                 FROM sesame_idam.relying_party_clients WHERE client_id = $4
                 ON CONFLICT DO NOTHING",
                &Values(vec![
                    Uuid::new_v4().into(),
                    kind.into(),
                    value.into(),
                    client_id.to_string().into(),
                ]),
            );
        }
        Ok(client_id.to_string())
    })
    .unwrap_or_else(|e| panic!("ensure_public_login_client_named({tenant},{client_id}): {e}"))
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
