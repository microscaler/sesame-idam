//! Role administration against a real Postgres with forced row-level security (scripts/test-db.sh sets one up the
//! way production is: the services' login is a member of the table-owner group, so only FORCE protects `users`).
//!
//! The rule under test is the one that matters most: a role change is never ALLOWED without a tenant
//! administrator's token for that same tenant, a role the deployment lets them grant, and a target that is a user of
//! that tenant; a tenant never loses its last administrator; and a change makes the user's tokens stale.
//!
//! Every test uses its own tenant, so they run in parallel without seeing each other. Skips without Postgres unless
//! SESAME_TEST_DB_REQUIRED is set (scripts/test-db.sh), where a skip is a failure.

use std::net::TcpStream;
use std::sync::Once;
use std::time::Duration;

use brrtrouter::typed::TypedHandlerRequest;
use chrono::Utc;
use http::Method;
use lifeguard::LifeExecutor;
use serde_json::{json, Value};
use uuid::Uuid;

use sesame_idam_authz_core::controllers::{
    assign_principal_role, list_principal_roles, revoke_principal_role,
};
use sesame_idam_authz_core_gen::handlers::assign_principal_role::Request as AssignRequest;
use sesame_idam_authz_core_gen::handlers::list_principal_roles::Request as ListRequest;
use sesame_idam_authz_core_gen::handlers::revoke_principal_role::Request as RevokeRequest;

static INIT: Once = Once::new();

fn db_available() -> bool {
    let host = std::env::var("TEST_DB_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("TEST_DB_PORT").unwrap_or_else(|_| "5432".to_string());
    let reachable = format!("{host}:{port}")
        .parse()
        .ok()
        .and_then(|sa| TcpStream::connect_timeout(&sa, Duration::from_millis(500)).ok())
        .is_some();
    if !reachable {
        assert!(
            std::env::var_os("SESAME_TEST_DB_REQUIRED").is_none(),
            "SESAME_TEST_DB_REQUIRED is set but Postgres is not reachable"
        );
        return false;
    }
    INIT.call_once(|| {
        std::env::set_var("DB_POOL_MAX", "2");
        std::env::set_var("DB_HOST", &host);
        std::env::set_var("DB_PORT", &port);
        for (k, t, d) in [
            ("DB_USER", "TEST_DB_USER", "sesame_idam"),
            ("DB_PASS", "TEST_DB_PASS", "dev_password_change_in_prod"),
            ("DB_NAME", "TEST_DB_NAME", "sesame_idam"),
        ] {
            std::env::set_var(k, std::env::var(t).unwrap_or_else(|_| d.to_string()));
        }
        // PriceWhisperer's deployment: platform_admin administers the tenant (read once, before any handler runs)
        std::env::set_var("SESAME_TENANT_ADMIN_ROLES", "platform_admin");
    });
    true
}

fn tenant() -> String {
    format!("bdd-roleadmin-{}", Uuid::new_v4().simple())
}

fn insert_user(tenant: &str, status: &str) -> Uuid {
    let id = Uuid::new_v4();
    let now = Utc::now();
    sesame_idam_database::with_pre_auth_tenant(tenant, |exec| {
        exec.execute_values(
            "INSERT INTO sesame_idam.users \
             (id, email, password_hash, tenant_id, status, email_verified, phone, phone_verified, created_at, updated_at) \
             VALUES ($1, $2, 'x', $3, $4, true, NULL, false, $5, $5)",
            &sea_query::Values(vec![
                id.into(),
                format!("role-admin-{id}@example.com").into(),
                tenant.into(),
                status.into(),
                now.into(),
            ]),
        )
    })
    .expect("insert user");
    id
}

fn insert_role(tenant: &str, user: Uuid, role: &str, org: Option<Uuid>) {
    let now = Utc::now();
    sesame_idam_database::db()
        .execute_values(
            "INSERT INTO sesame_idam.role_assignments \
             (id, principal_id, role_name, resource_type, resource_id, tenant_id, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $7)",
            &sea_query::Values(vec![
                Uuid::new_v4().into(),
                user.into(),
                role.into(),
                if org.is_some() { "organization" } else { "tenant" }.into(),
                org.into(),
                tenant.into(),
                now.into(),
            ]),
        )
        .expect("insert role");
}

fn insert_org(tenant: &str) -> Uuid {
    let id = Uuid::new_v4();
    let now = Utc::now();
    sesame_idam_database::db()
        .execute_values(
            "INSERT INTO sesame_idam.organizations (id, name, tenant_id, status, created_at, updated_at) \
             VALUES ($1, $2, $3, 'active', $4, $4)",
            &sea_query::Values(vec![id.into(), format!("org {id}").into(), tenant.into(), now.into()]),
        )
        .expect("insert org");
    id
}

/// Role names `user` holds in `tenant`, lowercased and sorted (straight from the table).
fn held(tenant: &str, user: Uuid) -> Vec<String> {
    let mut out = Vec::new();
    for role in [
        "crm.editor",
        "crm.viewer",
        "platform_admin",
        "tenant_admin",
        "owner",
        "pw_service",
        "service",
    ] {
        let row = sesame_idam_database::db()
            .query_one_values(
                "SELECT count(*)::bigint FROM sesame_idam.role_assignments \
                 WHERE tenant_id = $1 AND principal_id = $2 AND lower(role_name) = $3",
                &sea_query::Values(vec![tenant.into(), user.into(), role.into()]),
            )
            .expect("count");
        if row.try_get::<usize, i64>(0).expect("count") > 0 {
            out.push(role.to_string());
        }
    }
    out
}

/// A verified token's claims: `roles` for `tenant`, or none at all.
fn claims(tenant: &str, sub: Uuid, roles: &[&str]) -> Option<Value> {
    Some(json!({ "sub": sub.to_string(), "tenant_id": tenant, "roles": roles }))
}

fn assign(
    caller: Option<Value>,
    header_tenant: &str,
    user: Uuid,
    role: &str,
    org: Option<Uuid>,
) -> u16 {
    assign_principal_role::handle(TypedHandlerRequest {
        method: Method::POST,
        path: "/authz/principals/roles".into(),
        handler_name: "assign_principal_role".into(),
        path_params: Default::default(),
        query_params: Default::default(),
        data: AssignRequest {
            app_id: None,
            expires_at: None,
            org_id: org.map(|o| json!(o.to_string())),
            role: role.into(),
            tenant_id: None,
            user_id: user.to_string(),
            x_tenant_id: header_tenant.into(),
        },
        jwt_claims: caller,
    })
    .status
}

fn revoke(caller: Option<Value>, header_tenant: &str, user: Uuid, role: &str) -> u16 {
    revoke_principal_role::handle(TypedHandlerRequest {
        method: Method::DELETE,
        path: "/authz/principals/roles".into(),
        handler_name: "revoke_principal_role".into(),
        path_params: Default::default(),
        query_params: Default::default(),
        data: RevokeRequest {
            app_id: None,
            role: role.into(),
            user_id: user.to_string(),
            x_tenant_id: header_tenant.into(),
        },
        jwt_claims: caller,
    })
    .status
}

fn version(user: Uuid) -> Option<u64> {
    let store = sesame_common::VersionStore::from_env().ok()?;
    store.current_subject_version(&user.to_string()).ok()
}

macro_rules! need_db {
    () => {
        if !db_available() {
            println!("SKIP: Postgres not available");
            return;
        }
    };
}

#[test]
fn an_administrator_grants_and_revokes_in_their_own_tenant() {
    need_db!();
    let t = tenant();
    let admin = insert_user(&t, "active");
    insert_role(&t, admin, "platform_admin", None);
    let user = insert_user(&t, "active");
    let me = claims(&t, admin, &["platform_admin"]);

    let before = version(user);
    assert_eq!(assign(me.clone(), &t, user, "crm.editor", None), 204);
    assert_eq!(held(&t, user), vec!["crm.editor"]);
    assert_eq!(
        assign(me.clone(), &t, user, "CRM.Editor", None),
        204,
        "idempotent"
    );
    assert_eq!(held(&t, user), vec!["crm.editor"], "no duplicate");
    if std::env::var_os("REDIS_URL").is_some() {
        assert!(
            version(user) > before,
            "a grant makes the user's tokens stale"
        );
    }

    let listed = list_principal_roles::handle(TypedHandlerRequest {
        method: Method::GET,
        path: "/authz/principals/roles".into(),
        handler_name: "list_principal_roles".into(),
        path_params: Default::default(),
        query_params: Default::default(),
        data: ListRequest {
            user_id: Some(user.to_string()),
        },
        jwt_claims: me.clone(),
    });
    assert_eq!(listed.status, 200);
    assert!(listed.body.to_string().contains("crm.editor"));

    let before = version(user);
    assert_eq!(revoke(me.clone(), &t, user, "crm.editor"), 204);
    assert!(held(&t, user).is_empty());
    if std::env::var_os("REDIS_URL").is_some() {
        assert!(
            version(user) > before,
            "a revoke makes the user's tokens stale at once"
        );
    }
}

#[test]
fn nobody_without_an_administrator_token_for_this_tenant_changes_anything() {
    need_db!();
    let t = tenant();
    let other = tenant();
    let admin = insert_user(&t, "active");
    insert_role(&t, admin, "platform_admin", None);
    let user = insert_user(&t, "active");
    let stranger = insert_user(&other, "active");

    // no token, a member's token, an observer's token
    assert_eq!(assign(None, &t, user, "crm.editor", None), 401);
    assert_eq!(
        assign(claims(&t, user, &["member"]), &t, user, "crm.editor", None),
        403
    );
    assert_eq!(
        assign(
            claims(&t, user, &["platform_observer"]),
            &t,
            user,
            "platform_admin",
            None
        ),
        403
    );
    assert_eq!(
        revoke(claims(&t, user, &["member"]), &t, admin, "platform_admin"),
        403
    );
    // an administrator of ANOTHER tenant, naming this one in the header
    assert_eq!(
        assign(
            claims(&other, stranger, &["platform_admin"]),
            &t,
            user,
            "crm.editor",
            None
        ),
        403
    );
    // this tenant's administrator naming another tenant, or a user who is not in this tenant
    assert_eq!(
        assign(
            claims(&t, admin, &["platform_admin"]),
            &other,
            stranger,
            "crm.editor",
            None
        ),
        403
    );
    assert_eq!(
        assign(
            claims(&t, admin, &["platform_admin"]),
            &t,
            stranger,
            "crm.editor",
            None
        ),
        404
    );
    // a token whose two tenant claims disagree
    let forged = Some(
        json!({ "sub": admin.to_string(), "tenant_id": t, "sx": { "tenant": other, "roles": ["platform_admin"] } }),
    );
    assert_eq!(assign(forged, &t, user, "crm.editor", None), 401);

    assert!(held(&t, user).is_empty(), "nothing was granted");
    assert!(
        held(&other, stranger).is_empty(),
        "nothing was granted across tenants"
    );
    assert_eq!(
        held(&t, admin),
        vec!["platform_admin"],
        "nothing was revoked"
    );
}

#[test]
fn reserved_roles_and_roles_above_the_caller_are_refused() {
    need_db!();
    let t = tenant();
    let admin = insert_user(&t, "active");
    insert_role(&t, admin, "platform_admin", None);
    let ta = insert_user(&t, "active");
    insert_role(&t, ta, "tenant_admin", None);
    let user = insert_user(&t, "active");
    let platform = claims(&t, admin, &["platform_admin"]);
    let tenant_admin = claims(&t, ta, &["tenant_admin"]);
    let owner = claims(&t, admin, &["owner"]);

    for r in ["pw_service", "PW_Service", "service", "root"] {
        assert_eq!(
            assign(owner.clone(), &t, user, r, None),
            403,
            "{r} is reserved even for an owner"
        );
    }
    assert_eq!(assign(platform.clone(), &t, user, "owner", None), 403);
    assert_eq!(
        assign(platform.clone(), &t, user, "tenant_admin", None),
        403
    );
    assert_eq!(
        assign(tenant_admin.clone(), &t, user, "platform_admin", None),
        403
    );
    assert_eq!(
        revoke(tenant_admin.clone(), &t, admin, "platform_admin"),
        403,
        "nor take it away"
    );
    assert!(held(&t, user).is_empty());
    assert_eq!(held(&t, admin), vec!["platform_admin"]);

    assert_eq!(
        assign(platform, &t, user, "platform_admin", None),
        204,
        "the same role is fine"
    );
    assert_eq!(
        assign(owner, &t, user, "tenant_admin", None),
        204,
        "an owner grants any admin role"
    );
    assert_eq!(held(&t, user), vec!["platform_admin", "tenant_admin"]);
}

#[test]
fn an_organisation_scope_must_be_this_tenants_and_never_for_an_admin_role() {
    need_db!();
    let t = tenant();
    let admin = insert_user(&t, "active");
    insert_role(&t, admin, "platform_admin", None);
    let user = insert_user(&t, "active");
    let mine = insert_org(&t);
    let theirs = insert_org(&tenant());
    let me = claims(&t, admin, &["platform_admin", "owner"]);

    assert_eq!(assign(me.clone(), &t, user, "owner", Some(mine)), 400);
    assert_eq!(
        assign(me.clone(), &t, user, "platform_admin", Some(mine)),
        400
    );
    assert_eq!(
        assign(me.clone(), &t, user, "crm.editor", Some(theirs)),
        404
    );
    assert!(held(&t, user).is_empty());
    assert_eq!(assign(me, &t, user, "crm.editor", Some(mine)), 204);
    assert_eq!(held(&t, user), vec!["crm.editor"]);
}

#[test]
fn a_tenant_never_loses_its_last_active_administrator() {
    need_db!();
    let t = tenant();
    let a = insert_user(&t, "active");
    let b = insert_user(&t, "active");
    let gone = insert_user(&t, "disabled");
    insert_role(&t, a, "platform_admin", None);
    insert_role(&t, b, "platform_admin", None);
    insert_role(&t, gone, "platform_admin", None);
    let org = insert_org(&t);
    let scoped = insert_user(&t, "active");
    insert_role(&t, scoped, "platform_admin", Some(org)); // legacy org-scoped row: not a tenant administrator

    // B demotes A: B is still there
    assert_eq!(
        revoke(claims(&t, b, &["platform_admin"]), &t, a, "platform_admin"),
        204
    );
    // B removing their own last role would leave only a disabled user and an org-scoped row: refused
    assert_eq!(
        revoke(claims(&t, b, &["platform_admin"]), &t, b, "platform_admin"),
        403
    );
    assert_eq!(held(&t, b), vec!["platform_admin"]);
    // A (with a stale token still saying platform_admin) cannot take B's either
    assert_eq!(
        revoke(claims(&t, a, &["platform_admin"]), &t, b, "platform_admin"),
        403
    );
    assert_eq!(held(&t, b), vec!["platform_admin"]);
}

#[test]
fn the_service_login_sees_no_user_without_the_tenant_session() {
    need_db!();
    // The premise of every test above: under FORCE, a query outside the tenant's session reads nothing, even as a
    // member of the owner group (production's arrangement). If this fails, RLS is not protecting `users` here.
    let t = tenant();
    let user = insert_user(&t, "active");
    let unscoped = sesame_idam_database::db()
        .query_one_values(
            "SELECT count(*)::bigint FROM sesame_idam.users WHERE id = $1",
            &sea_query::Values(vec![user.into()]),
        )
        .expect("count")
        .try_get::<usize, i64>(0)
        .expect("count");
    assert_eq!(unscoped, 0);
}
