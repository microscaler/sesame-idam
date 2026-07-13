//! Org lifecycle BDD (D4): `GET /organizations/{id}` membership + tenant isolation.
//!
//! Run on ms02 with Postgres port-forwarded:
//!
//! ```bash
//! ssh ms02 'source ~/.cargo/env && cd ~/Workspace/microscaler/seasame-idam/microservices && \
//!   cargo test -p sesame_idam_org_mgmt --test main_bdd org_lifecycle -- --nocapture'
//! ```

use chrono::Utc;
use lifeguard::LifeExecutor;
use uuid::Uuid;

use sesame_idam_org_mgmt::services::org_lifecycle::{self, OrgLifecycleError};

use crate::common::{infra_available, unique_tenant};

fn seed_org(tenant_id: &str, org_id: Uuid, name: &str) {
    let exec = sesame_idam_database::db();
    let now = Utc::now();
    exec.execute_values(
        "INSERT INTO sesame_idam.organizations (id, name, tenant_id, status, created_at, updated_at)
         VALUES ($1, $2, $3, 'active', $4, $4)
         ON CONFLICT (id) DO UPDATE SET tenant_id = EXCLUDED.tenant_id, name = EXCLUDED.name",
        &sea_query::Values(vec![org_id.into(), name.into(), tenant_id.into(), now.into()]),
    )
    .expect("seed organization");
}

fn seed_user(tenant_id: &str, user_id: Uuid) {
    let exec = sesame_idam_database::db();
    let now = Utc::now();
    exec.execute_values(
        "INSERT INTO sesame_idam.users (id, email, password_hash, tenant_id, status, created_at, updated_at)
         VALUES ($1, $2, 'x', $3, 'active', $4, $4)
         ON CONFLICT (id) DO NOTHING",
        &sea_query::Values(vec![
            user_id.into(),
            format!("orgtest_{user_id}@example.com").into(),
            tenant_id.into(),
            now.into(),
        ]),
    )
    .expect("seed user");
}

fn seed_membership(org_id: Uuid, user_id: Uuid, role: &str) {
    let exec = sesame_idam_database::db();
    let now = Utc::now();
    exec.execute_values(
        "INSERT INTO sesame_idam.org_memberships (id, org_id, user_id, role, status, created_at, updated_at)
         VALUES ($1, $2, $3, $4, 'active', $5, $5)
         ON CONFLICT DO NOTHING",
        &sea_query::Values(vec![
            Uuid::new_v4().into(),
            org_id.into(),
            user_id.into(),
            role.into(),
            now.into(),
        ]),
    )
    .expect("seed membership");
}

/// Scenario: a member fetches their org and receives its metadata.
#[test]
fn get_organization_returns_org_for_member() {
    if !infra_available() {
        println!("SKIP: Postgres not available");
        return;
    }

    let tenant = unique_tenant("orgread");
    let org_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    seed_org(&tenant, org_id, "Acme Logistics");
    seed_user(&tenant, user_id);
    seed_membership(org_id, user_id, "owner");

    let exec = sesame_idam_database::db();
    let org = org_lifecycle::get_organization(
        exec,
        &tenant,
        &org_id.to_string(),
        &user_id.to_string(),
    )
    .expect("member should read org");

    assert_eq!(org.id, org_id);
    assert_eq!(org.name, "Acme Logistics");
    assert_eq!(org.tenant_id, tenant);
    assert_eq!(org.status, "active");
}

/// Scenario: a non-member is forbidden (does not leak org existence).
#[test]
fn get_organization_forbidden_for_non_member() {
    if !infra_available() {
        println!("SKIP: Postgres not available");
        return;
    }

    let tenant = unique_tenant("orgdeny");
    let org_id = Uuid::new_v4();
    let outsider = Uuid::new_v4();
    seed_org(&tenant, org_id, "Members Only Org");
    // No membership seeded for `outsider`.

    let exec = sesame_idam_database::db();
    let err = org_lifecycle::get_organization(
        exec,
        &tenant,
        &org_id.to_string(),
        &outsider.to_string(),
    )
    .expect_err("non-member must be forbidden");

    assert!(matches!(err, OrgLifecycleError::Forbidden), "got {err:?}");
}

/// Scenario: a member of the org in tenant A cannot read it under tenant B (zero bleed).
#[test]
fn get_organization_enforces_tenant_isolation() {
    if !infra_available() {
        println!("SKIP: Postgres not available");
        return;
    }

    let tenant_a = unique_tenant("orgiso-a");
    let tenant_b = unique_tenant("orgiso-b");
    let org_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    seed_org(&tenant_a, org_id, "Tenant A Org");
    seed_user(&tenant_a, user_id);
    seed_membership(org_id, user_id, "owner");

    let exec = sesame_idam_database::db();
    let err = org_lifecycle::get_organization(
        exec,
        &tenant_b,
        &org_id.to_string(),
        &user_id.to_string(),
    )
    .expect_err("cross-tenant read must be forbidden");

    assert!(matches!(err, OrgLifecycleError::Forbidden), "got {err:?}");
}
