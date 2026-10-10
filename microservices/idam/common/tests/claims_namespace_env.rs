//! The claim namespace is deployment configuration (`SESAME_CLAIMS_NAMESPACE`): tokens carry the authz
//! claims under the configured key, and only that key is accepted. Its own test binary, because the
//! namespace is read once per process; the variable is set before any read.

use sesame_common::jwt::helpers::{claims_namespace, DEFAULT_CLAIMS_NAMESPACE};
use sesame_common::{AccessClaims, AccessClaimsBuilder, SesameAuthzClaimsBuilder};

const NAMESPACE: &str = "https://sesame-idam.example.org/claims";

fn configure() {
    std::env::set_var("SESAME_CLAIMS_NAMESPACE", NAMESPACE);
}

fn claims() -> AccessClaims {
    let now = chrono::Utc::now().timestamp();
    let sx = SesameAuthzClaimsBuilder::new()
        .tenant("tenant-a")
        .portal("frontend")
        .roles(vec!["user".to_string()])
        .permissions(vec![])
        .build()
        .expect("sx claims");
    AccessClaimsBuilder::new()
        .iss("https://sesame-idam.example.com")
        .sub("user-1")
        .aud(vec!["api".to_string()])
        .client_id("frontend")
        .scope("openid")
        .exp(now + 300)
        .nbf(now)
        .iat(now)
        .jti("jti-1")
        .ver(1)
        .sid("sid-1")
        .tenant_id("tenant-a")
        .user_id("user-1")
        .user_type("customer")
        .sx(sx)
        .build()
        .expect("claims build")
}

#[test]
fn authz_claims_travel_under_the_configured_namespace_only() {
    configure();
    assert_eq!(claims_namespace(), NAMESPACE);

    let claims = claims();
    let json = serde_json::to_value(&claims).expect("serializes");
    assert_eq!(json[NAMESPACE]["tenant"], "tenant-a");
    assert!(
        json.get(DEFAULT_CLAIMS_NAMESPACE).is_none(),
        "the default key is not written"
    );
    assert!(json.get("sx").is_none(), "sx is never a wire name");

    let back: AccessClaims = serde_json::from_value(json.clone()).expect("round trip");
    assert_eq!(back, claims);

    // A token from a deployment with another namespace carries no authz claims here: rejected.
    let mut foreign = json;
    let sx = foreign.as_object_mut().unwrap().remove(NAMESPACE).unwrap();
    foreign[DEFAULT_CLAIMS_NAMESPACE] = sx;
    assert!(serde_json::from_value::<AccessClaims>(foreign).is_err());
}
