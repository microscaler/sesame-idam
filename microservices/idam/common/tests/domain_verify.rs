//! Story 17.1: domain claim primitives against published vectors and properties.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use proptest::prelude::*;
use sesame_common::domain_verify::{
    challenge_name, check_eligible, derive_token, expected_record, normalize_domain, verify_record,
    ClaimSubject, Domain, EligibilityPolicy, LocalKeyring, Scope,
};
use uuid::Uuid;

const VECTORS: &str = include_str!("fixtures/domain_verify_vectors.json");

fn vectors() -> serde_json::Value {
    serde_json::from_str(VECTORS).expect("vectors parse")
}

const CLAIM: Uuid = Uuid::from_u128(0x0193_a4b2_7c1e_7d3a_9f00_0000_0000_0001);

fn subject(v: &serde_json::Value) -> ClaimSubject {
    let tenant = v["tenant_id"].as_str().expect("tenant");
    let claim = Uuid::parse_str(v["claim_id"].as_str().expect("claim")).expect("uuid");
    match v["kind"].as_str().expect("kind") {
        "organisation" => ClaimSubject::organisation(
            tenant,
            Uuid::parse_str(v["org_id"].as_str().expect("org")).expect("uuid"),
            claim,
            Scope::parse(v["scope"].as_str().expect("scope")).expect("scope"),
        )
        .expect("subject"),
        "tenant_issuer" => ClaimSubject::tenant_issuer(tenant, claim).expect("subject"),
        other => panic!("unknown kind {other}"),
    }
}

#[test]
fn published_vectors_reproduce_byte_for_byte() {
    let all = vectors();
    let keys = &all["keys"];
    let list = all["vectors"].as_array().expect("array");
    assert!(list.len() >= 10);
    for v in list {
        let key = URL_SAFE_NO_PAD
            .decode(
                keys[v["key_id"].as_str().expect("key id")]
                    .as_str()
                    .expect("key"),
            )
            .expect("key b64");
        let domain = normalize_domain(v["input"].as_str().expect("input")).expect("normalises");
        assert_eq!(domain.as_str(), v["domain"].as_str().expect("domain"));
        let s = subject(v);
        assert_eq!(
            derive_token(&key, &s, &domain),
            v["token"].as_str().expect("token"),
            "{v}"
        );
        assert_eq!(
            challenge_name(&domain),
            v["record_name"].as_str().expect("name")
        );

        let ring = LocalKeyring::new((4, key), None).expect("key");
        let record = expected_record(&ring, &s, &domain).expect("mac");
        assert_eq!(record.value, v["record_value"].as_str().expect("value"));
        assert_eq!(
            verify_record(&ring, &s, &domain, [&record.value]),
            Ok(Some(4))
        );
    }
}

#[test]
fn every_vector_token_is_distinct() {
    let all = vectors();
    let mut tokens: Vec<&str> = all["vectors"]
        .as_array()
        .expect("array")
        .iter()
        .map(|v| v["token"].as_str().expect("token"))
        .collect();
    let n = tokens.len();
    tokens.sort_unstable();
    tokens.dedup();
    assert_eq!(tokens.len(), n);
}

#[test]
fn story_acceptance_examples() {
    let policy = EligibilityPolicy::default();
    assert_eq!(
        normalize_domain("BüCher.Example.").expect("valid").as_str(),
        "xn--bcher-kva.example"
    );
    let code = |d: &str| {
        check_eligible(&normalize_domain(d).expect("valid"), &policy).map_err(|e| e.code())
    };
    for d in ["com", "co.uk", "github.io"] {
        assert_eq!(code(d), Err("public_suffix"), "{d}");
    }
    for d in ["gmail.com", "outlook.com"] {
        assert_eq!(code(d), Err("consumer_provider"), "{d}");
    }
    for d in ["bigbank.com", "eu.bigbank.co.uk"] {
        assert_eq!(code(d), Ok(()), "{d}");
    }
}

fn label() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9]{0,10}"
}

fn domain_strategy() -> impl Strategy<Value = Domain> {
    (label(), label()).prop_map(|(a, b)| normalize_domain(&format!("{a}.{b}.com")).expect("valid"))
}

fn scope_strategy() -> impl Strategy<Value = Scope> {
    prop_oneof![Just(Scope::Exact), Just(Scope::Subdomains)]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Changing any one of tenant, organisation, claim, scope, domain, purpose or epoch key changes the token.
    #[test]
    fn token_binds_every_field(
        tenant in "[a-z][a-z0-9-]{0,20}",
        tenant2 in "[a-z][a-z0-9-]{0,20}",
        org in any::<u128>(),
        org2 in any::<u128>(),
        claim in any::<u128>(),
        claim2 in any::<u128>(),
        scope in scope_strategy(),
        d1 in domain_strategy(),
        d2 in domain_strategy(),
        k1 in proptest::array::uniform32(any::<u8>()),
        k2 in proptest::array::uniform32(any::<u8>()),
    ) {
        let c = Uuid::from_u128(claim);
        let base = ClaimSubject::organisation(&tenant, Uuid::from_u128(org), c, scope).expect("subject");
        let t = derive_token(&k1, &base, &d1);
        prop_assert_eq!(t.len(), 43);

        if tenant2 != tenant {
            let s = ClaimSubject::organisation(&tenant2, Uuid::from_u128(org), c, scope).expect("subject");
            prop_assert_ne!(&derive_token(&k1, &s, &d1), &t);
        }
        if org2 != org {
            let s = ClaimSubject::organisation(&tenant, Uuid::from_u128(org2), c, scope).expect("subject");
            prop_assert_ne!(&derive_token(&k1, &s, &d1), &t);
        }
        if claim2 != claim {
            let s = ClaimSubject::organisation(&tenant, Uuid::from_u128(org), Uuid::from_u128(claim2), scope)
                .expect("subject");
            prop_assert_ne!(&derive_token(&k1, &s, &d1), &t);
        }
        let other_scope = if scope == Scope::Exact { Scope::Subdomains } else { Scope::Exact };
        let s = ClaimSubject::organisation(&tenant, Uuid::from_u128(org), c, other_scope).expect("subject");
        prop_assert_ne!(&derive_token(&k1, &s, &d1), &t);
        if d2 != d1 {
            prop_assert_ne!(&derive_token(&k1, &base, &d2), &t);
        }
        let issuer = ClaimSubject::tenant_issuer(&tenant, c).expect("subject");
        prop_assert_ne!(&derive_token(&k1, &issuer, &d1), &t);
        if k2 != k1 {
            prop_assert_ne!(&derive_token(&k2, &base, &d1), &t);
        }
    }

    /// A record verifies only for the subject and domain it was issued for.
    #[test]
    fn record_verifies_only_for_its_claim(
        org in any::<u128>(),
        d1 in domain_strategy(),
        d2 in domain_strategy(),
        scope in scope_strategy(),
    ) {
        let key = LocalKeyring::new((1, vec![7u8; 32]), None).expect("key");
        let s = ClaimSubject::organisation("acme", Uuid::from_u128(org), CLAIM, scope).expect("subject");
        let record = expected_record(&key, &s, &d1).expect("mac");
        prop_assert_eq!(verify_record(&key, &s, &d1, [&record.value]), Ok(Some(1)));
        if d2 != d1 {
            prop_assert_eq!(verify_record(&key, &s, &d2, [&record.value]), Ok(None));
        }
        let issuer = ClaimSubject::tenant_issuer("acme", CLAIM).expect("subject");
        prop_assert_eq!(verify_record(&key, &issuer, &d1, [&record.value]), Ok(None));
    }

    /// Normalisation is idempotent and never panics on arbitrary input.
    #[test]
    fn normalisation_is_idempotent(input in "\\PC{0,80}") {
        if let Ok(d) = normalize_domain(&input) {
            let again = normalize_domain(d.as_str()).expect("canonical form normalises");
            prop_assert_eq!(again, d);
        }
    }

    /// Arbitrary TXT strings never verify.
    #[test]
    fn arbitrary_txt_never_verifies(values in proptest::collection::vec("\\PC{0,120}", 0..6)) {
        let key = LocalKeyring::new((1, vec![3u8; 32]), None).expect("key");
        let s = ClaimSubject::tenant_issuer("acme", CLAIM).expect("subject");
        let d = normalize_domain("acme.com").expect("valid");
        prop_assert_eq!(verify_record(&key, &s, &d, &values), Ok(None));
    }
}
