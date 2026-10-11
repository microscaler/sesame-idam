//! Organisation domain claims against Postgres (story 17.3): tenant RLS, the one-holder rule, the append-only history
//! and the consumer queries. Skips when Postgres is unreachable (`crate::common::infra_available`).

use chrono::{Duration, Utc};
use lifeguard::LifeExecutor;
use sesame_common::domain_verify::lifecycle::{ClaimState, LifecycleEvent, Withdrawer};
use sesame_common::domain_verify::{normalize_domain, Domain, Scope, Verdict};
use uuid::Uuid;

use sesame_idam_org_mgmt::services::domain_claims::{
    apply_event, create_claim, discoverable_domain, load_claim, record_verdict, verified_domain_for,
};

use crate::common::{infra_available, unique_tenant};

const MATCH: Verdict = Verdict::Match {
    epoch: 1,
    dnssec: false,
};

fn d(s: &str) -> Domain {
    normalize_domain(s).expect("domain")
}

fn seed_org(tenant_id: &str) -> Uuid {
    let id = Uuid::new_v4();
    sesame_idam_database::db()
        .execute_values(
            "INSERT INTO sesame_idam.organizations (id, name, tenant_id, status, created_at, updated_at)
             VALUES ($1, 'org', $2, 'active', now(), now())",
            &sea_query::Values(vec![id.into(), tenant_id.into()]),
        )
        .expect("seed organization");
    id
}

fn claim(tenant: &str, org: Uuid, domain: &str, scope: Scope) -> Uuid {
    sesame_idam_database::with_pre_auth_tenant(tenant, |exec| {
        Ok(
            create_claim(exec, tenant, org, d(domain), scope, None, Utc::now())
                .expect("claim")
                .id,
        )
    })
    .expect("tenant tx")
}

fn events(tenant: &str, claim_id: Uuid) -> i64 {
    sesame_idam_database::with_pre_auth_tenant(tenant, |exec| {
        let row = exec.query_one_values(
            "SELECT count(*) FROM sesame_idam.org_domain_events WHERE org_domain_id = $1",
            &sea_query::Values(vec![claim_id.into()]),
        )?;
        Ok(row.get::<usize, i64>(0))
    })
    .expect("count")
}

fn state(tenant: &str, claim_id: Uuid) -> ClaimState {
    sesame_idam_database::with_pre_auth_tenant(tenant, |exec| {
        Ok(load_claim(exec, claim_id).expect("load").claim.state)
    })
    .expect("tenant tx")
}

#[test]
fn one_holder_per_tenant_and_tenants_are_isolated() {
    if !infra_available() {
        return;
    }
    let acme = unique_tenant("dom-acme");
    let globex = unique_tenant("dom-globex");
    let domain = format!("{}.com", Uuid::new_v4().simple());

    let (a1, a2, g1) = (seed_org(&acme), seed_org(&acme), seed_org(&globex));
    let c1 = claim(&acme, a1, &domain, Scope::Exact);
    let c2 = claim(&acme, a2, &domain, Scope::Exact);
    let c3 = claim(&globex, g1, &domain, Scope::Exact);
    assert_eq!(events(&acme, c1), 1, "created writes one event");

    assert_eq!(
        record_verdict(&acme, c1, MATCH, Utc::now())
            .expect("verify")
            .claim
            .state,
        ClaimState::Verified
    );
    // the second organisation of the tenant loses: CONFLICT, not an error
    assert_eq!(
        record_verdict(&acme, c2, MATCH, Utc::now())
            .expect("conflict")
            .claim
            .state,
        ClaimState::Conflict
    );
    // another tenant may hold the same domain
    assert_eq!(
        record_verdict(&globex, c3, MATCH, Utc::now())
            .expect("verify")
            .claim
            .state,
        ClaimState::Verified
    );
    assert_eq!(events(&acme, c1), 2);
    assert_eq!(events(&acme, c2), 2);

    // RLS: a tenant cannot see another tenant's claim
    let seen =
        sesame_idam_database::with_pre_auth_tenant(
            &globex,
            |exec| Ok(load_claim(exec, c1).is_ok()),
        )
        .expect("tx");
    assert!(!seen, "globex must not see acme's claim");
}

#[test]
fn scope_grace_and_revocation_drive_the_queries() {
    if !infra_available() {
        return;
    }
    let tenant = unique_tenant("dom-scope");
    let org = seed_org(&tenant);
    let base = format!("{}.com", Uuid::new_v4().simple());
    let sub = format!("eu.{base}");
    let c = claim(&tenant, org, &base, Scope::Exact);
    record_verdict(&tenant, c, MATCH, Utc::now()).expect("verify");

    let q = |name: &str| {
        sesame_idam_database::with_pre_auth_tenant(&tenant, |exec| {
            Ok((
                verified_domain_for(exec, org, &d(name))
                    .expect("q")
                    .is_some(),
                discoverable_domain(exec, &d(name)).expect("q"),
            ))
        })
        .expect("tx")
    };
    assert_eq!(q(&base), (true, Some(org)));
    assert_eq!(
        q(&sub),
        (false, None),
        "exact scope never covers a subdomain"
    );

    // GRACE: discoverable, never vouches
    record_verdict(&tenant, c, Verdict::NoMatch, Utc::now()).expect("grace");
    assert_eq!(state(&tenant, c), ClaimState::Grace);
    assert_eq!(q(&base), (false, Some(org)));

    // grace exhausted: REVOKED satisfies neither
    sesame_idam_database::with_pre_auth_tenant(&tenant, |exec| {
        Ok(apply_event(
            exec,
            c,
            &LifecycleEvent::Tick,
            Utc::now() + Duration::hours(73),
        )
        .expect("tick"))
    })
    .expect("tx");
    assert_eq!(state(&tenant, c), ClaimState::Revoked);
    assert_eq!(q(&base), (false, None));

    // a subdomains claim covers descendants, not siblings
    let c2 = claim(&tenant, org, &base, Scope::Subdomains);
    record_verdict(&tenant, c2, MATCH, Utc::now()).expect("verify");
    assert_eq!(q(&sub), (true, Some(org)));
    assert_eq!(q(&format!("not{base}")), (false, None));

    // final states refuse events
    let refused = sesame_idam_database::with_pre_auth_tenant(&tenant, |exec| {
        Ok(apply_event(
            exec,
            c,
            &LifecycleEvent::Withdrawn {
                by: Withdrawer::OrgAdmin,
                reason: "deleted",
            },
            Utc::now(),
        )
        .is_err())
    })
    .expect("tx");
    assert!(refused);
}

#[test]
fn history_is_append_only() {
    if !infra_available() {
        return;
    }
    let tenant = unique_tenant("dom-history");
    let org = seed_org(&tenant);
    let c = claim(
        &tenant,
        org,
        &format!("{}.com", Uuid::new_v4().simple()),
        Scope::Exact,
    );
    let before = events(&tenant, c);
    let changed = sesame_idam_database::with_pre_auth_tenant(&tenant, |exec| {
        let u = exec
            .execute_values(
                "UPDATE sesame_idam.org_domain_events SET event = 'tampered' WHERE org_domain_id = $1",
                &sea_query::Values(vec![c.into()]),
            )
            .unwrap_or(0);
        let d = exec
            .execute_values(
                "DELETE FROM sesame_idam.org_domain_events WHERE org_domain_id = $1",
                &sea_query::Values(vec![c.into()]),
            )
            .unwrap_or(0);
        Ok(u + d)
    })
    .expect("tx");
    assert_eq!(
        changed, 0,
        "the service login can neither update nor delete history"
    );
    assert_eq!(events(&tenant, c), before);
}
