//! Account-first onboarding BDD (D2): register → org membership → set active org → JWT `org_id`.
//!
//! Run on ms02 with Postgres + Redis port-forwarded:
//!
//! ```bash
//! ssh ms02 'source ~/.cargo/env && cd ~/Workspace/microscaler/seasame-idam/microservices && \
//!   cargo test -p sesame_idam_identity_login_service --test main_bdd account_first -- --nocapture'
//! ```

use brrtrouter::typed::TypedHandlerRequest;
use chrono::Utc;
use http::Method;
use lifeguard::LifeExecutor;
use sesame_common::jwt::Ed25519Signer;
use uuid::Uuid;

use sesame_idam_identity_login_service::controllers::{auth_register, set_active_organization};
use sesame_idam_identity_login_service_gen::handlers::auth_register::Request as RegisterRequest;
use sesame_idam_identity_login_service_gen::handlers::set_active_organization::Request as SetActiveOrgRequest;

use super::token_lifecycle::{assert_token_response_shape, infra_available, unique_email};

use crate::common::ensure_active_tenant;

const TEST_TENANT: &str = "bdd-account-first-tenant";

fn register_request(email: &str, password: &str) -> TypedHandlerRequest<RegisterRequest> {
    TypedHandlerRequest {
        method: Method::POST,
        path: "/auth/register".to_string(),
        handler_name: "auth_register".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: RegisterRequest {
            email: email.to_string(),
            first_name: Some("Account".to_string()),
            last_name: Some("First".to_string()),
            password: password.to_string(),
            phone: None,
            username: None,
            x_tenant_id: TEST_TENANT.to_string(),
        },
        jwt_claims: None,
    }
}

fn seed_org_and_membership(user_id: &str, org_id: &str) {
    let exec = sesame_idam_database::db();
    let user_uuid = Uuid::parse_str(user_id).expect("user uuid");
    let org_uuid = Uuid::parse_str(org_id).expect("org uuid");
    let membership_id = Uuid::new_v4();
    let now = Utc::now();

    exec.execute_values(
        "INSERT INTO sesame_idam.organizations (id, name, tenant_id, status, created_at, updated_at)
         VALUES ($1, $2, $3, 'active', $4, $4)
         ON CONFLICT (id) DO UPDATE SET tenant_id = EXCLUDED.tenant_id, status = EXCLUDED.status",
        &sea_query::Values(vec![
            org_uuid.into(),
            "BDD Test Org".into(),
            TEST_TENANT.into(),
            now.into(),
        ]),
    )
    .expect("seed organization");

    exec.execute_values(
        "INSERT INTO sesame_idam.org_memberships (id, org_id, user_id, role, status, created_at, updated_at)
         VALUES ($1, $2, $3, 'owner', 'active', $4, $4)",
        &sea_query::Values(vec![
            membership_id.into(),
            org_uuid.into(),
            user_uuid.into(),
            now.into(),
        ]),
    )
    .expect("seed membership");
}

fn seed_org_only(org_id: &str) {
    let exec = sesame_idam_database::db();
    let org_uuid = Uuid::parse_str(org_id).expect("org uuid");
    let now = Utc::now();

    exec.execute_values(
        "INSERT INTO sesame_idam.organizations (id, name, tenant_id, status, created_at, updated_at)
         VALUES ($1, $2, $3, 'active', $4, $4)
         ON CONFLICT (id) DO UPDATE SET tenant_id = EXCLUDED.tenant_id, status = EXCLUDED.status",
        &sea_query::Values(vec![
            org_uuid.into(),
            "BDD Foreign Org".into(),
            TEST_TENANT.into(),
            now.into(),
        ]),
    )
    .expect("seed organization only");
}

fn claims_from_access_token(token: &str) -> serde_json::Value {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;
    let payload_b64 = token.split('.').nth(1).expect("jwt payload");
    let bytes = URL_SAFE_NO_PAD.decode(payload_b64).expect("payload base64");
    let payload: serde_json::Value = serde_json::from_slice(&bytes).expect("payload json");
    serde_json::json!({
        "sub": payload["sub"],
        "tenant_id": payload["tenant_id"],
        "iss": payload["iss"],
        "aud": payload["aud"],
    })
}

fn set_active_org_request(
    org_id: &str,
    access_token: &str,
) -> TypedHandlerRequest<SetActiveOrgRequest> {
    TypedHandlerRequest {
        method: Method::POST,
        path: "/sessions/active-organization".to_string(),
        handler_name: "set_active_organization".to_string(),
        path_params: std::collections::HashMap::new(),
        query_params: std::collections::HashMap::new(),
        data: SetActiveOrgRequest {
            organization_id: org_id.to_string(),
            x_tenant_id: TEST_TENANT.to_string(),
        },
        jwt_claims: Some(claims_from_access_token(access_token)),
    }
}

fn decode_payload(token: &str) -> serde_json::Value {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;
    let bytes = URL_SAFE_NO_PAD
        .decode(token.split('.').nth(1).unwrap())
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

/// Scenario: User with membership can set active org and receive JWT with `org_id`.
#[test]
fn account_first_set_active_organization_embeds_org_id() {
    if !infra_available() {
        println!("SKIP: Postgres and/or Redis not available");
        return;
    }
    ensure_active_tenant(TEST_TENANT);

    let email = unique_email("account-first");
    let password = "SecureP@ss123!";
    let org_id = Uuid::new_v4().to_string();

    let reg = auth_register::handle(register_request(&email, password));
    assert_eq!(reg.status, 201, "register: {:?}", reg.body);
    let user_id = reg.body["user_id"].as_str().unwrap().to_string();
    let access_token = reg.body["access_token"].as_str().unwrap();

    seed_org_and_membership(&user_id, &org_id);

    let resp = set_active_organization::handle(set_active_org_request(&org_id, access_token));
    assert_eq!(resp.status, 200, "set_active_organization: {:?}", resp.body);
    assert_token_response_shape(&resp.body, Some(&user_id));
    assert_eq!(resp.body["organization_id"], org_id);

    let payload = decode_payload(resp.body["access_token"].as_str().unwrap());
    assert_eq!(
        payload["org_id"].as_str().unwrap_or(""),
        org_id,
        "access token must carry org_id after active-org"
    );

    let signer = Ed25519Signer::from_env()
        .expect("signer")
        .expect("signing key");
    signer
        .verify(resp.body["access_token"].as_str().unwrap())
        .expect("active-org access token signature");
}

/// Scenario: User cannot set active org without membership.
#[test]
fn account_first_rejects_non_member_org() {
    if !infra_available() {
        println!("SKIP: Postgres and/or Redis not available");
        return;
    }
    ensure_active_tenant(TEST_TENANT);

    let email = unique_email("account-first-deny");
    let password = "SecureP@ss123!";
    let foreign_org = Uuid::new_v4().to_string();

    let reg = auth_register::handle(register_request(&email, password));
    assert_eq!(reg.status, 201);
    let access_token = reg.body["access_token"].as_str().unwrap();

    seed_org_only(&foreign_org);

    let resp = set_active_organization::handle(set_active_org_request(&foreign_org, access_token));
    assert_eq!(resp.status, 403, "expected forbidden: {:?}", resp.body);
    assert_eq!(resp.body["error"], "forbidden");
}
