//! Organisation domain API against Postgres (story 17.4): authorisation and tenant isolation, the claim → record →
//! verify path through a mock `dns-probe`, the one answer for ineligible and held domains, conflict, rate limit, and
//! failure of the key or the probe leaving claims unchanged. Skips when Postgres is unreachable.

use std::sync::Mutex;

use chrono::{Duration, Utc};
use lifeguard::LifeExecutor;
use serde_json::Value;
use sesame_common::domain_verify::dns::{rcode, DecodedAnswer};
use sesame_common::domain_verify::{
    EligibilityPolicy, EnrollKeyring, KeyringError, LocalKeyring, ProviderAnswer, ProviderOutcome,
    MAC_LEN,
};
use uuid::Uuid;

use sesame_idam_org_mgmt::services::domain_api::{self, DnsProbe, DomainApiError, DomainContext};

use crate::common::{infra_available, unique_tenant};

/// Two providers answering the same TXT records, or `dns-probe` down.
#[derive(Default)]
struct FakeProbe {
    records: Mutex<Vec<String>>,
    down: Mutex<bool>,
}

impl FakeProbe {
    fn publish(&self, value: &str) {
        self.records.lock().unwrap().push(value.to_string());
    }
}

impl DnsProbe for FakeProbe {
    fn lookup(&self, _name: &str) -> Result<Vec<ProviderAnswer>, String> {
        if *self.down.lock().unwrap() {
            return Err("connection refused".to_string());
        }
        let records = self.records.lock().unwrap().clone();
        Ok(["cloudflare", "quad9"]
            .into_iter()
            .map(|provider| ProviderAnswer {
                provider: provider.to_string(),
                outcome: ProviderOutcome::Answered(DecodedAnswer {
                    rcode: rcode::NOERROR,
                    records: records.clone(),
                    ..DecodedAnswer::default()
                }),
            })
            .collect())
    }
}

/// A key service that is down.
struct DownKeyring;

impl EnrollKeyring for DownKeyring {
    fn current_epoch(&self) -> u32 {
        1
    }
    fn accepted_epochs(&self) -> Vec<u32> {
        vec![1]
    }
    fn mac(&self, _epoch: u32, _message: &[u8]) -> Result<[u8; MAC_LEN], KeyringError> {
        Err(KeyringError::Unavailable("503".to_string()))
    }
}

fn keyring() -> Box<dyn EnrollKeyring> {
    Box::new(LocalKeyring::new((1, vec![7u8; 32]), None).expect("keyring"))
}

/// A context whose probe the test controls: the probe is shared through a leaked reference (tests only).
fn context(keyring: Option<Box<dyn EnrollKeyring>>) -> (DomainContext, &'static FakeProbe) {
    let probe: &'static FakeProbe = Box::leak(Box::default());
    struct Shared(&'static FakeProbe);
    impl DnsProbe for Shared {
        fn lookup(&self, name: &str) -> Result<Vec<ProviderAnswer>, String> {
            self.0.lookup(name)
        }
    }
    let ctx = DomainContext {
        keyring,
        probe: Box::new(Shared(probe)),
        policy: EligibilityPolicy::default()
            .with_reserved(["sesame.example"])
            .expect("policy"),
    };
    (ctx, probe)
}

fn seed_org(tenant: &str) -> Uuid {
    let id = Uuid::new_v4();
    sesame_idam_database::with_pre_auth_tenant(tenant, |exec| {
        exec.execute_values(
            "INSERT INTO sesame_idam.organizations (id, name, tenant_id, status, created_at, updated_at)
             VALUES ($1, 'org', $2, 'active', now(), now())",
            &sea_query::Values(vec![id.into(), tenant.into()]),
        )
    })
    .expect("seed organization");
    id
}

/// A user of `tenant` with `role` in `org`.
fn member(tenant: &str, org: Uuid, role: &str) -> Uuid {
    let user = Uuid::new_v4();
    sesame_idam_database::with_pre_auth_tenant(tenant, |exec| {
        exec.execute_values(
            "INSERT INTO sesame_idam.users (id, email, password_hash, tenant_id, status, created_at, updated_at)
             VALUES ($1, $2, 'x', $3, 'active', now(), now())",
            &sea_query::Values(vec![
                user.into(),
                format!("dom_{user}@example.com").into(),
                tenant.into(),
            ]),
        )?;
        exec.execute_values(
            "INSERT INTO sesame_idam.org_memberships (id, org_id, user_id, role, status, created_at, updated_at)
             VALUES ($1, $2, $3, $4, 'active', now(), now())",
            &sea_query::Values(vec![
                Uuid::new_v4().into(),
                org.into(),
                user.into(),
                role.into(),
            ]),
        )
    })
    .expect("seed member");
    user
}

fn domain() -> String {
    format!("d{}.com", Uuid::new_v4().simple())
}

fn id_of(v: &Value) -> Uuid {
    Uuid::parse_str(v["id"].as_str().expect("id")).expect("uuid")
}

fn state(v: &Value) -> &str {
    v["state"].as_str().expect("state")
}

#[test]
fn claim_publish_verify_and_auto_join() {
    if !infra_available() {
        return;
    }
    let (ctx, probe) = context(Some(keyring()));
    let tenant = unique_tenant("dapi");
    let org = seed_org(&tenant);
    let admin = member(&tenant, org, "admin");
    let reader = member(&tenant, org, "member");
    let name = domain();
    let now = Utc::now();

    let created = domain_api::create(&ctx, &tenant, org, admin, &name.to_uppercase(), None, now)
        .expect("create");
    assert_eq!(state(&created), "PENDING");
    assert_eq!(created["domain"], name.as_str(), "normalised");
    assert_eq!(
        created["record"]["name"],
        format!("_sesame-challenge.{name}")
    );
    let value = created["record"]["value"]
        .as_str()
        .expect("value")
        .to_string();
    assert!(value.starts_with("sesame-domain-verify=v1;t="));
    let claim = id_of(&created);

    assert_eq!(
        domain_api::create(&ctx, &tenant, org, admin, &name, Some("subdomains"), now),
        Err(DomainApiError::ClaimExists(claim))
    );
    assert_eq!(
        domain_api::set_auto_join(&ctx, &tenant, org, claim, admin, true, now),
        Err(DomainApiError::NotVerified)
    );

    // nothing published yet
    let checked = domain_api::verify(&ctx, &tenant, org, claim, admin, now).expect("verify");
    assert_eq!(checked["verdict"], "no_match");
    assert_eq!(state(&checked["claim"]), "PENDING");
    assert!(matches!(
        domain_api::verify(
            &ctx,
            &tenant,
            org,
            claim,
            admin,
            now + Duration::seconds(10)
        ),
        Err(DomainApiError::RateLimited { retry_after: 50 })
    ));

    probe.publish("v=spf1 -all");
    probe.publish(&value);
    let later = now + Duration::seconds(61);
    let checked = domain_api::verify(&ctx, &tenant, org, claim, admin, later).expect("verify");
    assert_eq!(checked["verdict"], "match");
    assert_eq!(state(&checked["claim"]), "VERIFIED");

    let joined = domain_api::set_auto_join(&ctx, &tenant, org, claim, admin, true, later)
        .expect("auto-join");
    assert_eq!(joined["auto_join"], true);

    let listed = domain_api::list(&ctx, &tenant, org, reader).expect("member lists");
    let items = listed["items"].as_array().expect("items");
    assert_eq!(items.len(), 1);
    assert_eq!(state(&items[0]), "VERIFIED");
    assert_eq!(
        items[0]["record"]["value"],
        value.as_str(),
        "the record to keep published"
    );

    domain_api::delete(&tenant, org, claim, admin, later).expect("delete");
    domain_api::delete(&tenant, org, claim, admin, later).expect("delete is idempotent");
    let listed = domain_api::list(&ctx, &tenant, org, reader).expect("list");
    assert_eq!(state(&listed["items"][0]), "REVOKED");
    assert!(
        listed["items"][0]["record"].is_null(),
        "no record for a final claim"
    );
    assert_eq!(listed["items"][0]["auto_join"], false);
}

#[test]
fn only_admins_change_and_outsiders_see_nothing() {
    if !infra_available() {
        return;
    }
    let (ctx, _probe) = context(Some(keyring()));
    let tenant = unique_tenant("dapi-auth");
    let other_tenant = unique_tenant("dapi-auth-x");
    let org = seed_org(&tenant);
    let owner = member(&tenant, org, "owner");
    let plain = member(&tenant, org, "member");
    let elsewhere = member(&tenant, seed_org(&tenant), "owner");
    let foreign = member(&other_tenant, seed_org(&other_tenant), "owner");
    let now = Utc::now();

    let claim = id_of(
        &domain_api::create(&ctx, &tenant, org, owner, &domain(), None, now)
            .expect("owner creates"),
    );

    assert_eq!(
        domain_api::create(&ctx, &tenant, org, plain, &domain(), None, now),
        Err(DomainApiError::Forbidden)
    );
    assert_eq!(
        domain_api::verify(&ctx, &tenant, org, claim, plain, now),
        Err(DomainApiError::Forbidden)
    );
    assert_eq!(
        domain_api::set_auto_join(&ctx, &tenant, org, claim, plain, true, now),
        Err(DomainApiError::Forbidden)
    );
    assert_eq!(
        domain_api::delete(&tenant, org, claim, plain, now),
        Err(DomainApiError::Forbidden)
    );
    assert!(domain_api::list(&ctx, &tenant, org, plain).is_ok());

    // a non-member of the tenant, and a user of another tenant (whose token names their own tenant)
    for (t, user) in [(&tenant, elsewhere), (&other_tenant, foreign)] {
        assert_eq!(
            domain_api::list(&ctx, t, org, user),
            Err(DomainApiError::NotFound)
        );
        assert_eq!(
            domain_api::create(&ctx, t, org, user, &domain(), None, now),
            Err(DomainApiError::NotFound)
        );
        assert_eq!(
            domain_api::delete(t, org, claim, user, now),
            Err(DomainApiError::NotFound)
        );
    }

    // a claim id of another organisation through this organisation's path
    let theirs = seed_org(&tenant);
    let their_owner = member(&tenant, theirs, "owner");
    let their_claim = id_of(
        &domain_api::create(&ctx, &tenant, theirs, their_owner, &domain(), None, now)
            .expect("create"),
    );
    assert_eq!(
        domain_api::verify(&ctx, &tenant, org, their_claim, owner, now),
        Err(DomainApiError::NotFound)
    );

    // tenant admins revoke within their own tenant only
    assert_eq!(
        domain_api::revoke(&ctx, &other_tenant, claim, "policy", now),
        Err(DomainApiError::NotFound)
    );
    let revoked = domain_api::revoke(&ctx, &tenant, claim, "security", now).expect("revoke");
    assert_eq!(state(&revoked), "REVOKED");
    assert_eq!(
        domain_api::revoke(&ctx, &tenant, claim, "security", now),
        Err(DomainApiError::ClaimFinal)
    );
}

#[test]
fn ineligible_and_held_domains_get_one_answer() {
    if !infra_available() {
        return;
    }
    let (ctx, probe) = context(Some(keyring()));
    let tenant = unique_tenant("dapi-elig");
    let (a, b) = (seed_org(&tenant), seed_org(&tenant));
    let (admin_a, admin_b) = (member(&tenant, a, "owner"), member(&tenant, b, "owner"));
    let now = Utc::now();

    let mut answers = Vec::new();
    for name in ["gmail.com", "co.uk", "com", "login.sesame.example"] {
        let err = domain_api::create(&ctx, &tenant, a, admin_a, name, None, now)
            .expect_err("not claimable");
        answers.push(err.body());
        assert_eq!(err, DomainApiError::DomainUnavailable, "{name}");
    }
    assert_eq!(
        domain_api::create(&ctx, &tenant, a, admin_a, "not a domain", None, now),
        Err(DomainApiError::InvalidDomain)
    );

    // a held by another organisation of the tenant: the same body as ineligible
    let name = domain();
    let held = domain_api::create(&ctx, &tenant, a, admin_a, &name, None, now).expect("create");
    probe.publish(held["record"]["value"].as_str().expect("value"));
    domain_api::verify(&ctx, &tenant, a, id_of(&held), admin_a, now).expect("verify");
    let err = domain_api::create(&ctx, &tenant, b, admin_b, &name, None, now)
        .expect_err("held elsewhere");
    assert_eq!(err.body(), answers[0]);

    // another tenant may hold the same domain
    let other = unique_tenant("dapi-elig-x");
    let oo = seed_org(&other);
    assert!(domain_api::create(
        &ctx,
        &other,
        oo,
        member(&other, oo, "owner"),
        &name,
        None,
        now
    )
    .is_ok());
}

#[test]
fn a_record_for_another_organisation_does_not_verify_and_the_race_loser_conflicts() {
    if !infra_available() {
        return;
    }
    let (ctx, probe) = context(Some(keyring()));
    let tenant = unique_tenant("dapi-race");
    let (a, b) = (seed_org(&tenant), seed_org(&tenant));
    let (admin_a, admin_b) = (member(&tenant, a, "admin"), member(&tenant, b, "admin"));
    let name = domain();
    let now = Utc::now();

    let ca = domain_api::create(&ctx, &tenant, a, admin_a, &name, None, now).expect("a claims");
    let cb = domain_api::create(&ctx, &tenant, b, admin_b, &name, None, now).expect("b claims");
    assert_ne!(ca["record"]["value"], cb["record"]["value"]);

    // only a's record is published: b does not verify with it
    probe.publish(ca["record"]["value"].as_str().expect("value"));
    let checked = domain_api::verify(&ctx, &tenant, b, id_of(&cb), admin_b, now).expect("check");
    assert_eq!(checked["verdict"], "no_match");
    assert_eq!(state(&checked["claim"]), "PENDING");

    domain_api::verify(&ctx, &tenant, a, id_of(&ca), admin_a, now).expect("a verifies");
    // b publishes its own record too, but a holds the domain: b loses with domain_unavailable and is CONFLICT
    probe.publish(cb["record"]["value"].as_str().expect("value"));
    assert_eq!(
        domain_api::verify(
            &ctx,
            &tenant,
            b,
            id_of(&cb),
            admin_b,
            now + Duration::seconds(61)
        ),
        Err(DomainApiError::DomainUnavailable)
    );
    let listed = domain_api::list(&ctx, &tenant, b, admin_b).expect("list");
    assert_eq!(state(&listed["items"][0]), "CONFLICT");
}

#[test]
fn key_or_dns_failure_changes_nothing() {
    if !infra_available() {
        return;
    }
    let tenant = unique_tenant("dapi-fail");
    let org = seed_org(&tenant);
    let admin = member(&tenant, org, "owner");
    let now = Utc::now();

    // no keyring at all: no claim is created
    let (none, _) = context(None);
    assert_eq!(
        domain_api::create(&none, &tenant, org, admin, &domain(), None, now),
        Err(DomainApiError::KeyringUnavailable)
    );
    // a key service that fails: no claim is created either
    let (down, _) = context(Some(Box::new(DownKeyring)));
    assert_eq!(
        domain_api::create(&down, &tenant, org, admin, &domain(), None, now),
        Err(DomainApiError::KeyringUnavailable)
    );
    let (ctx, probe) = context(Some(keyring()));
    assert!(
        domain_api::list(&ctx, &tenant, org, admin).expect("list")["items"]
            .as_array()
            .expect("items")
            .is_empty()
    );

    let claim = id_of(
        &domain_api::create(&ctx, &tenant, org, admin, &domain(), None, now).expect("create"),
    );
    assert_eq!(
        domain_api::verify(&down, &tenant, org, claim, admin, now),
        Err(DomainApiError::KeyringUnavailable)
    );
    *probe.down.lock().unwrap() = true;
    assert_eq!(
        domain_api::verify(&ctx, &tenant, org, claim, admin, now),
        Err(DomainApiError::DnsUnavailable)
    );
    let listed = domain_api::list(&ctx, &tenant, org, admin).expect("list");
    assert_eq!(state(&listed["items"][0]), "PENDING");
    assert!(
        listed["items"][0]["last_checked_at"].is_null(),
        "no check was recorded"
    );
}

/// No request body of the domain API can carry a claim's state: only the verifier moves it.
#[test]
fn no_request_schema_sets_state() {
    let spec: serde_yaml::Value = serde_yaml::from_str(include_str!(
        "../../../../../../openapi/idam/org-mgmt/openapi.yaml"
    ))
    .expect("spec");
    let schemas = &spec["components"]["schemas"];
    for name in [
        "CreateOrgDomainRequest",
        "UpdateOrgDomainRequest",
        "RevokeOrgDomainRequest",
    ] {
        let schema = &schemas[name];
        assert_eq!(
            schema["additionalProperties"],
            serde_yaml::Value::Bool(false),
            "{name}"
        );
        let props = schema["properties"].as_mapping().expect("properties");
        for forbidden in ["state", "verified", "verified_at", "dnssec", "epoch"] {
            assert!(
                !props.contains_key(forbidden),
                "{name} must not carry {forbidden}"
            );
        }
    }
    assert!(
        spec["paths"]["/organizations/{org_id}/domains"]["put"].is_null(),
        "update_org_domains is gone"
    );
}
