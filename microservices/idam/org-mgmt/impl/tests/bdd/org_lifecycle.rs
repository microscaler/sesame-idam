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

/// Scenario: create org with opaque persona metadata → fetch echoes it verbatim.
#[test]
fn create_and_fetch_org_roundtrips_metadata() {
    if !infra_available() {
        println!("SKIP: Postgres not available");
        return;
    }

    let tenant = unique_tenant("orgmeta");
    let user_id = Uuid::new_v4();
    seed_user(&tenant, user_id);

    let exec = sesame_idam_database::db();
    let meta = serde_json::json!({ "hauliage_profile_type": "SHIPPER" });
    let summary = org_lifecycle::create_organization(
        exec,
        &tenant,
        &user_id.to_string(),
        "Shipper Co",
        Some(&meta),
    )
    .expect("create org with metadata");

    let detail = org_lifecycle::get_organization(
        exec,
        &tenant,
        &summary.id.to_string(),
        &user_id.to_string(),
    )
    .expect("creator reads own org");

    assert_eq!(detail.metadata, Some(meta));
    assert_eq!(detail.name, "Shipper Co");
}

/// Scenario: ORM invite + idempotent add-membership; the added user can then read the org.
#[test]
fn invite_and_add_membership_via_orm() {
    if !infra_available() {
        println!("SKIP: Postgres not available");
        return;
    }

    let tenant = unique_tenant("invite");
    let owner = Uuid::new_v4();
    seed_user(&tenant, owner);

    let exec = sesame_idam_database::db();
    let org = org_lifecycle::create_organization(
        exec,
        &tenant,
        &owner.to_string(),
        "Inviter Co",
        None,
    )
    .expect("create org");

    let invite_id = org_lifecycle::invite_by_email(
        exec,
        &tenant,
        &org.id.to_string(),
        "Newbie@Example.com",
        "member",
    )
    .expect("invite by email");
    assert!(!invite_id.is_nil());

    let newbie = Uuid::new_v4();
    seed_user(&tenant, newbie);
    let org_id = org.id.to_string();

    // First add + idempotent repeat must both succeed.
    org_lifecycle::add_user_membership(exec, &tenant, &org_id, &newbie.to_string(), "member")
        .expect("add membership");
    org_lifecycle::add_user_membership(exec, &tenant, &org_id, &newbie.to_string(), "member")
        .expect("add membership idempotent");

    // Added user is now a member and can read the org.
    let detail = org_lifecycle::get_organization(exec, &tenant, &org_id, &newbie.to_string())
        .expect("added member reads org");
    assert_eq!(detail.name, "Inviter Co");
}

/// Scenario: list_memberships returns the user's orgs in the caller's tenant only.
#[test]
fn list_memberships_is_tenant_scoped() {
    if !infra_available() {
        println!("SKIP: Postgres not available");
        return;
    }

    let tenant = unique_tenant("memlist");
    let other_tenant = unique_tenant("memlist-other");
    let user_id = Uuid::new_v4();
    seed_user(&tenant, user_id);

    let org_a = Uuid::new_v4();
    let org_b = Uuid::new_v4();
    let org_other = Uuid::new_v4();
    seed_org(&tenant, org_a, "Org A");
    seed_org(&tenant, org_b, "Org B");
    seed_org(&other_tenant, org_other, "Foreign Org");
    seed_membership(org_a, user_id, "owner");
    seed_membership(org_b, user_id, "member");
    seed_membership(org_other, user_id, "member");

    let exec = sesame_idam_database::db();
    let memberships = org_lifecycle::list_memberships(exec, &tenant, &user_id.to_string())
        .expect("list memberships");

    // Only the two orgs in `tenant` — the other-tenant membership is excluded.
    assert_eq!(memberships.len(), 2, "got {memberships:?}");
    let names: Vec<&str> = memberships.iter().map(|m| m.org_name.as_str()).collect();
    assert!(names.contains(&"Org A") && names.contains(&"Org B"));
    assert!(!names.contains(&"Foreign Org"));
}

/// Scenario: accept a valid invite → membership created + invite marked accepted.
#[test]
fn accept_invitation_adds_membership_and_marks_accepted() {
    use lifeguard::{ColumnTrait, LifeModelTrait};
    use sesame_idam_org_mgmt::models::org_invite;

    if !infra_available() {
        println!("SKIP: Postgres not available");
        return;
    }

    let tenant = unique_tenant("accept");
    let owner = Uuid::new_v4();
    seed_user(&tenant, owner);

    let exec = sesame_idam_database::db();
    let org = org_lifecycle::create_organization(exec, &tenant, &owner.to_string(), "Accept Co", None)
        .expect("create org");

    let email = "Invitee@Example.com";
    let invite_id =
        org_lifecycle::invite_by_email(exec, &tenant, &org.id.to_string(), email, "member")
            .expect("invite");

    // Recover the generated token via the ORM.
    let token = org_invite::Entity::find()
        .filter(org_invite::Column::Id.eq(invite_id))
        .find_one(exec)
        .unwrap()
        .expect("invite row")
        .token;

    let invitee = Uuid::new_v4();
    seed_user(&tenant, invitee);
    let summary =
        org_lifecycle::accept_invitation(exec, &tenant, &invitee.to_string(), email, &token)
            .expect("accept invitation");
    assert_eq!(summary.name, "Accept Co");

    // Invite marked accepted.
    let accepted = org_invite::Entity::find()
        .filter(org_invite::Column::Id.eq(invite_id))
        .find_one(exec)
        .unwrap()
        .expect("invite row");
    assert!(accepted.accepted_at.is_some(), "invite must be marked accepted");

    // Invitee is now a member and can read the org.
    let detail =
        org_lifecycle::get_organization(exec, &tenant, &org.id.to_string(), &invitee.to_string())
            .expect("member read");
    assert_eq!(detail.name, "Accept Co");
}

/// Scenario: accepting with an email that does not match the invite is rejected.
#[test]
fn accept_invitation_rejects_email_mismatch() {
    use lifeguard::{ColumnTrait, LifeModelTrait};
    use sesame_idam_org_mgmt::models::org_invite;
    use sesame_idam_org_mgmt::services::org_lifecycle::OrgLifecycleError;

    if !infra_available() {
        println!("SKIP: Postgres not available");
        return;
    }

    let tenant = unique_tenant("accept-mismatch");
    let owner = Uuid::new_v4();
    seed_user(&tenant, owner);

    let exec = sesame_idam_database::db();
    let org = org_lifecycle::create_organization(exec, &tenant, &owner.to_string(), "Mismatch Co", None)
        .expect("create org");
    let invite_id =
        org_lifecycle::invite_by_email(exec, &tenant, &org.id.to_string(), "right@example.com", "member")
            .expect("invite");
    let token = org_invite::Entity::find()
        .filter(org_invite::Column::Id.eq(invite_id))
        .find_one(exec)
        .unwrap()
        .expect("invite row")
        .token;

    let wrong_user = Uuid::new_v4();
    seed_user(&tenant, wrong_user);
    let err = org_lifecycle::accept_invitation(
        exec,
        &tenant,
        &wrong_user.to_string(),
        "wrong@example.com",
        &token,
    )
    .expect_err("email mismatch must be rejected");
    assert!(matches!(err, OrgLifecycleError::EmailMismatch), "got {err:?}");
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
