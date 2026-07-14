//! Zero-bleed proof: forced RLS on `sesame_idam.users` filters by tenant context.
//!
//! Skips when Postgres is unreachable or the RLS contract is not installed.

use std::net::TcpStream;
use std::sync::Once;
use std::time::Duration;

use lifeguard::{execute_statement, query_value, SessionContext};
use sesame_idam_database::db;
use uuid::Uuid;

static INIT: Once = Once::new();

fn db_available() -> bool {
    let host = std::env::var("TEST_DB_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("TEST_DB_PORT").unwrap_or_else(|_| "5432".to_string());
    let addr = format!("{host}:{port}");
    let reachable = addr
        .parse()
        .ok()
        .and_then(|sa| TcpStream::connect_timeout(&sa, Duration::from_millis(500)).ok())
        .is_some();
    if !reachable {
        return false;
    }

    INIT.call_once(|| {
        std::env::set_var("DB_POOL_MAX", "2");
        std::env::set_var("DB_HOST", &host);
        std::env::set_var("DB_PORT", &port);
        std::env::set_var(
            "DB_USER",
            std::env::var("TEST_DB_USER").unwrap_or_else(|_| "sesame_idam".to_string()),
        );
        std::env::set_var(
            "DB_PASS",
            std::env::var("TEST_DB_PASS")
                .unwrap_or_else(|_| "dev_password_change_in_prod".to_string()),
        );
        std::env::set_var(
            "DB_NAME",
            std::env::var("TEST_DB_NAME").unwrap_or_else(|_| "sesame_idam".to_string()),
        );
    });
    true
}

fn rls_contract_installed() -> bool {
    let exec = db();
    query_value::<i32, _>(
        exec,
        "SELECT public.sesame_rls_contract_version()",
        &[],
    )
    .is_ok()
}

fn session_context(tenant: &str, subject: Uuid, org: Uuid) -> SessionContext {
    SessionContext {
        tenant_id: tenant.to_string(),
        subject_id: subject,
        organization_id: org,
        session_id: format!("rls-test-{subject}"),
        roles: vec!["member".to_string()],
        permissions: vec!["user:read".to_string()],
        user_type: Some("customer".to_string()),
        org_type: None,
    }
}

fn insert_user(
    exec: &impl lifeguard::LifeExecutor,
    id: Uuid,
    tenant: &str,
    email: &str,
) -> Result<(), lifeguard::LifeError> {
    execute_statement(
        exec,
        "INSERT INTO sesame_idam.users \
         (id, email, password_hash, tenant_id, status, email_verified, phone_verified, created_at, updated_at) \
         VALUES ($1, $2, 'rls-test-hash', $3, 'active', true, false, NOW(), NOW())",
        &[&id, &email, &tenant],
    )
    .map(|_| ())
}

fn delete_user(exec: &impl lifeguard::LifeExecutor, id: Uuid) -> Result<(), lifeguard::LifeError> {
    execute_statement(
        exec,
        "DELETE FROM sesame_idam.users WHERE id = $1",
        &[&id],
    )
    .map(|_| ())
}

#[test]
fn users_rls_unqualified_select_is_tenant_scoped() {
    if !db_available() {
        println!("SKIP: Postgres not available");
        return;
    }
    if !rls_contract_installed() {
        println!("SKIP: RLS contract not installed (run migrations/rls/*.sql)");
        return;
    }

    let run_id = Uuid::new_v4().simple();
    let tenant_alpha = format!("rls-alpha-{run_id}");
    let tenant_beta = format!("rls-beta-{run_id}");
    let alpha_id = Uuid::new_v4();
    let beta_id = Uuid::new_v4();
    let alpha_org = Uuid::new_v4();
    let beta_org = Uuid::new_v4();
    let alpha_email = format!("rls_alpha_{run_id}@example.com");
    let beta_email = format!("rls_beta_{run_id}@example.com");

    let alpha_ctx = session_context(&tenant_alpha, alpha_id, alpha_org);
    let beta_ctx = session_context(&tenant_beta, beta_id, beta_org);
    let pool = db().pool();

    pool.with_session_transaction(&alpha_ctx, |exec| insert_user(exec, alpha_id, &tenant_alpha, &alpha_email))
        .expect("insert alpha user");
    pool.with_session_transaction(&beta_ctx, |exec| insert_user(exec, beta_id, &tenant_beta, &beta_email))
        .expect("insert beta user");

    let alpha_count = pool
        .with_session_transaction(&alpha_ctx, |exec| {
            query_value::<i64, _>(exec, "SELECT COUNT(*)::bigint FROM sesame_idam.users", &[])
        })
        .expect("alpha count");
    let beta_count = pool
        .with_session_transaction(&beta_ctx, |exec| {
            query_value::<i64, _>(exec, "SELECT COUNT(*)::bigint FROM sesame_idam.users", &[])
        })
        .expect("beta count");

    assert!(
        alpha_count >= 1,
        "alpha context should see at least its own row, got {alpha_count}"
    );
    assert!(
        beta_count >= 1,
        "beta context should see at least its own row, got {beta_count}"
    );

    // Proof: each tenant's unqualified COUNT must not include the other tenant's row.
    pool.with_session_transaction(&alpha_ctx, |exec| {
        let visible_beta: i64 = query_value(
            exec,
            "SELECT COUNT(*)::bigint FROM sesame_idam.users WHERE tenant_id = $1",
            &[&tenant_beta],
        )?;
        assert_eq!(
            visible_beta, 0,
            "alpha context must not observe beta tenant rows via RLS"
        );
        Ok(())
    })
    .expect("alpha isolation check");

    pool.with_session_transaction(&beta_ctx, |exec| {
        let visible_alpha: i64 = query_value(
            exec,
            "SELECT COUNT(*)::bigint FROM sesame_idam.users WHERE tenant_id = $1",
            &[&tenant_alpha],
        )?;
        assert_eq!(
            visible_alpha, 0,
            "beta context must not observe alpha tenant rows via RLS"
        );
        Ok(())
    })
    .expect("beta isolation check");

    pool.with_session_transaction(&alpha_ctx, |exec| delete_user(exec, alpha_id))
        .expect("cleanup alpha");
    pool.with_session_transaction(&beta_ctx, |exec| delete_user(exec, beta_id))
        .expect("cleanup beta");
}
