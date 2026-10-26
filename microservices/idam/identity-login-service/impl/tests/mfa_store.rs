//! The second-factor store and policy lookup against a real Postgres, through the real driver (ADR-008).
//!
//! Written after v0.1.39 answered 503 on every sign-in in pp: the statements cast text parameters with `$n::uuid`,
//! which makes Postgres infer the parameter as `uuid`, and the driver refuses to bind a `String` to it. A psql smoke
//! test with literals could not see that; only the driver can. Runs when `MFA_STORE_DB=1` and `DB_HOST` point at a
//! database with Sesame's migrations applied, as a login that is NOT a superuser (so RLS applies); skips otherwise.

use sesame_idam_identity_login_service::services::mfa::{self, factors, policy, recovery};
use uuid::Uuid;

fn enabled() -> bool {
    std::env::var("MFA_STORE_DB").is_ok_and(|v| v == "1")
}

fn exec_sql(tenant: &str, sql: &str) {
    use lifeguard::LifeExecutor;
    sesame_idam_database::with_pre_auth_tenant(tenant, |exec| {
        (exec as &dyn LifeExecutor)
            .execute_values(sql, &sea_query::Values(vec![]))
            .map(|_| ())
    })
    .unwrap_or_else(|e| panic!("fixture {sql}: {e}"));
}

#[test]
fn factors_recovery_codes_and_policy_round_trip_through_the_driver() {
    if !enabled() {
        eprintln!("SKIP mfa_store: MFA_STORE_DB=1 not set");
        return;
    }
    let tenant = format!("mfa-store-{}", &Uuid::new_v4().to_string()[..8]);
    let other = format!("{tenant}-other");
    let user = Uuid::new_v4();
    let org = Uuid::new_v4();
    exec_sql(
        &tenant,
        &format!(
            "INSERT INTO sesame_idam.users (id, email, password_hash, tenant_id, status, created_at, updated_at) \
             VALUES ('{user}', 'u@example.com', 'x', '{tenant}', 'active', now(), now())"
        ),
    );

    // Nothing yet.
    let (held, required) = mfa::load_state(&tenant, user).expect("load_state");
    assert!(!held.any_factor());
    assert_eq!(required, policy::Requirement::None);

    // TOTP: pending, confirmed, then one use per step.
    let totp = Uuid::new_v4();
    factors::insert_pending_totp(&tenant, user, totp, "App", "v1.x.y").expect("insert pending");
    assert_eq!(factors::list_active(&tenant, user).expect("list").len(), 0);
    assert!(factors::confirm_totp(&tenant, user, totp, 100).expect("confirm"));
    assert!(!factors::advance_totp_step(&tenant, user, totp, 100).expect("replay"));
    assert!(factors::advance_totp_step(&tenant, user, totp, 101).expect("next step"));
    let second = Uuid::new_v4();
    factors::insert_pending_totp(&tenant, user, second, "App 2", "v1.a.b").expect("second pending");
    assert!(
        !factors::confirm_totp(&tenant, user, second, 200).expect("second confirm"),
        "one authenticator app per account"
    );

    // Passkey.
    let pk = factors::insert_passkey(
        &tenant,
        user,
        "Mac",
        "Y3JlZA",
        r#"{"cred":{"backup_eligible":true}}"#,
        true,
    )
    .expect("insert passkey");
    assert!(
        factors::insert_passkey(&tenant, user, "Dup", "Y3JlZA", "{}", false).is_err(),
        "credential ids are unique in the tenant"
    );
    let found = factors::find_passkey(&tenant, "Y3JlZA")
        .expect("find")
        .expect("present");
    assert_eq!(found.user_id, user);
    assert_eq!(found.passkey_synced, Some(true));
    factors::touch_passkey(&tenant, user, pk, None).expect("touch, no update");
    factors::touch_passkey(&tenant, user, pk, Some(r#"{"cred":{"counter":2}}"#))
        .expect("touch, update");
    assert!(factors::rename(&tenant, user, pk, "Laptop").expect("rename"));

    let active = factors::list_active(&tenant, user).expect("list active");
    assert_eq!(active.len(), 2);
    assert!(active
        .iter()
        .any(|f| f.kind == factors::Kind::Totp && f.totp_last_step == Some(101)));
    assert!(active
        .iter()
        .any(|f| f.label == "Laptop" && f.last_used_at.is_some()));

    // Another tenant sees and finds nothing (RLS and the queries' own tenant filter).
    assert!(factors::list(&other, user).expect("other list").is_empty());
    assert!(factors::find_passkey(&other, "Y3JlZA")
        .expect("other find")
        .is_none());

    // Recovery codes.
    let codes = mfa::regenerate_recovery_codes(&tenant, user).expect("codes");
    assert_eq!(
        factors::recovery_codes_left(&tenant, user).expect("left"),
        10
    );
    let hash = recovery::hash(&tenant, &user, &recovery::normalize(&codes[0]).unwrap());
    assert!(factors::consume_recovery_code(&tenant, user, &hash).expect("consume"));
    assert!(!factors::consume_recovery_code(&tenant, user, &hash).expect("reuse"));
    assert_eq!(
        factors::recovery_codes_left(&tenant, user).expect("left"),
        9
    );

    // Policy: strictest membership.
    exec_sql(
        &tenant,
        &format!(
            "INSERT INTO sesame_idam.organizations (id, name, tenant_id, status, created_at, updated_at) \
             VALUES ('{org}', 'Desk', '{tenant}', 'active', now(), now())"
        ),
    );
    exec_sql(
        &tenant,
        &format!(
            "INSERT INTO sesame_idam.org_memberships (org_id, user_id, role, status, created_at, updated_at) \
             VALUES ('{org}', '{user}', 'member', 'active', now(), now())"
        ),
    );
    exec_sql(
        &tenant,
        &format!(
            "INSERT INTO sesame_idam.org_auth_policies (org_id, tenant_id, require_method) VALUES ('{org}', '{tenant}', 'passkey')"
        ),
    );
    let (held, required) = mfa::load_state(&tenant, user).expect("load_state");
    assert!(held.totp && held.passkey && held.recovery_codes);
    assert_eq!(required, policy::Requirement::Passkey);

    // Removal.
    assert!(factors::remove(&tenant, user, totp).expect("remove"));
    assert!(!factors::remove(&tenant, user, totp).expect("remove again"));
    assert_eq!(factors::remove_all(&tenant, user).expect("remove all"), 2);
    assert_eq!(
        factors::recovery_codes_left(&tenant, user).expect("left"),
        0
    );
}
