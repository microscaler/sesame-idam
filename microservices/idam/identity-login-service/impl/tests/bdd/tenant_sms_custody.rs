//! Tenant SMS custody BDD (ADR-009 Phase 2).
//!
//! These scenarios cover the properties that make the feature safe rather than
//! merely functional:
//!
//! 1. The credential is **write-only** — no response body can carry it back.
//! 2. Envelope custody is **denied by default**; only an explicit allow-list
//!    lets a tenant hand us a raw token.
//! 3. A newly stored credential is **not trusted** — it lands
//!    `pending_validation` and does not resolve for sending.
//! 4. A tenant-billed purpose with no usable sender **refuses** rather than
//!    falling back to the platform's account (no silent cross-subsidy).
//! 5. Revocation **clears** the sealed material, not just the status flag.
//! 6. Switching custody `envelope → connect` leaves **no sealed secret**
//!    behind.
//!
//! Run on ms02 with Postgres reachable:
//!
//! ```bash
//! ssh ms02 'source ~/.cargo/env && cd ~/Workspace/microscaler/sesame-idam/microservices && \
//!   cargo test -p sesame_idam_identity_login_service --test main_bdd tenant_sms -- --nocapture'
//! ```

use std::collections::HashMap;

use brrtrouter::typed::TypedHandlerRequest;
use http::Method;

use sesame_idam_identity_login_service::controllers::{
    platform_tenant_create, platform_tenant_sms_get, platform_tenant_sms_revoke,
    platform_tenant_sms_upsert,
};
use sesame_idam_identity_login_service::models::tenant_sms_config::STATUS_ACTIVE;
use sesame_idam_identity_login_service::services::envelope;
use sesame_idam_identity_login_service::services::sms::SmsPurpose;
use sesame_idam_identity_login_service::services::sms_sender::{
    resolve_sms_sender, BillingOwner, Credential, Unresolved,
};
use sesame_idam_identity_login_service::services::tenant_sms_service::TenantSmsService;
use sesame_idam_identity_login_service_gen::handlers::platform_tenant_create::Request as CreateRequest;
use sesame_idam_identity_login_service_gen::handlers::platform_tenant_sms_get::Request as SmsGetRequest;
use sesame_idam_identity_login_service_gen::handlers::platform_tenant_sms_revoke::Request as SmsRevokeRequest;
use sesame_idam_identity_login_service_gen::handlers::platform_tenant_sms_upsert::Request as SmsUpsertRequest;

use super::platform_tenant_admin::db_available;

const ENV: &str = "dev";
const TOKEN: &str = "super-secret-twilio-token-do-not-leak";

/// Env is process-global; these scenarios each install their own custody
/// allow-list and cost policy, so they must not interleave.
static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn unique_slug(prefix: &str) -> String {
    format!("{prefix}-{}", uuid::Uuid::new_v4().simple())
}

fn mint_tenant(slug: &str) {
    let req = TypedHandlerRequest {
        method: Method::POST,
        path: "/platform/tenants".to_string(),
        handler_name: "platform_tenant_create".to_string(),
        path_params: HashMap::new(),
        query_params: HashMap::new(),
        data: CreateRequest {
            slug: slug.to_string(),
            display_name: "SMS Custody BDD".to_string(),
            activate: Some(true),
            provisioning_mode: None,
        },
        jwt_claims: None,
    };
    let res = platform_tenant_create::handle(req);
    assert_eq!(res.status, 201, "tenant mint failed: {:?}", res.body);
}

fn upsert(slug: &str, data: SmsUpsertRequest) -> (u16, serde_json::Value) {
    let req = TypedHandlerRequest {
        method: Method::PUT,
        path: format!("/platform/tenants/{slug}/sms/{ENV}"),
        handler_name: "platform_tenant_sms_upsert".to_string(),
        path_params: HashMap::from([
            ("slug".to_string(), slug.to_string()),
            ("environment".to_string(), ENV.to_string()),
        ]),
        query_params: HashMap::new(),
        data,
        jwt_claims: None,
    };
    let res = platform_tenant_sms_upsert::handle(req);
    (res.status, res.body)
}

fn envelope_body(slug: &str, token: Option<&str>) -> SmsUpsertRequest {
    SmsUpsertRequest {
        slug: slug.to_string(),
        environment: ENV.to_string(),
        custody_mode: "envelope".to_string(),
        connected_account_sid: None,
        account_sid: Some("ACtenant0000000000000000000000000".to_string()),
        auth_token: token.map(ToString::to_string),
        messaging_service_sid: None,
        from_number: Some("+15551230000".to_string()),
        campaign_ref: None,
        daily_spend_ceiling_cents: Some(250),
    }
}

fn connect_body(slug: &str) -> SmsUpsertRequest {
    SmsUpsertRequest {
        slug: slug.to_string(),
        environment: ENV.to_string(),
        custody_mode: "connect".to_string(),
        connected_account_sid: Some("ACconnected000000000000000000000".to_string()),
        account_sid: None,
        auth_token: None,
        messaging_service_sid: None,
        from_number: None,
        campaign_ref: Some("CAMP-123".to_string()),
        daily_spend_ceiling_cents: None,
    }
}

fn get(slug: &str) -> (u16, serde_json::Value) {
    let req = TypedHandlerRequest {
        method: Method::GET,
        path: format!("/platform/tenants/{slug}/sms/{ENV}"),
        handler_name: "platform_tenant_sms_get".to_string(),
        path_params: HashMap::from([
            ("slug".to_string(), slug.to_string()),
            ("environment".to_string(), ENV.to_string()),
        ]),
        query_params: HashMap::new(),
        data: SmsGetRequest {
            slug: slug.to_string(),
            environment: ENV.to_string(),
        },
        jwt_claims: None,
    };
    let res = platform_tenant_sms_get::handle(req);
    (res.status, res.body)
}

fn revoke(slug: &str) -> (u16, serde_json::Value) {
    let req = TypedHandlerRequest {
        method: Method::DELETE,
        path: format!("/platform/tenants/{slug}/sms/{ENV}"),
        handler_name: "platform_tenant_sms_revoke".to_string(),
        path_params: HashMap::from([
            ("slug".to_string(), slug.to_string()),
            ("environment".to_string(), ENV.to_string()),
        ]),
        query_params: HashMap::new(),
        data: SmsRevokeRequest {
            slug: slug.to_string(),
            environment: ENV.to_string(),
        },
        jwt_claims: None,
    };
    let res = platform_tenant_sms_revoke::handle(req);
    (res.status, res.body)
}

/// Assert no response body anywhere in the flow contains the token. Cheap,
/// but it is the check that would catch a future "just add the field for
/// debugging" edit.
fn assert_no_secret(body: &serde_json::Value) {
    let rendered = body.to_string();
    assert!(
        !rendered.contains(TOKEN),
        "response leaked the auth token: {rendered}"
    );
    assert!(
        !rendered.contains("auth_token"),
        "response exposed an auth_token field: {rendered}"
    );
}

/// Scenario: envelope custody is refused unless the tenant is on the
/// allow-list — the default answer to "may we hold your credentials?" is no.
#[test]
fn envelope_custody_refused_when_not_allow_listed() {
    if !db_available() {
        println!("SKIP: Postgres not available");
        return;
    }
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    std::env::remove_var("SMS_ENVELOPE_CUSTODY_TENANTS");

    let slug = unique_slug("bdd-sms-deny");
    mint_tenant(&slug);

    let (status, body) = upsert(&slug, envelope_body(&slug, Some(TOKEN)));
    assert_eq!(status, 403, "expected refusal, got {body:?}");
    assert_eq!(body["error"], "envelope_custody_forbidden");
    assert_no_secret(&body);

    // Nothing was written on the way to refusing.
    let (get_status, _) = get(&slug);
    assert_eq!(get_status, 404, "a refused upsert must not persist anything");
}

/// Scenario: an allow-listed tenant may store a credential, but it is sealed,
/// never echoed back, and not trusted until validated.
#[test]
fn stored_credential_is_sealed_write_only_and_untrusted() {
    if !db_available() {
        println!("SKIP: Postgres not available");
        return;
    }
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    let slug = unique_slug("bdd-sms-store");
    mint_tenant(&slug);
    std::env::set_var("SMS_ENVELOPE_CUSTODY_TENANTS", &slug);
    std::env::set_var("SMS_CREDENTIAL_KEK", envelope::generate_kek().unwrap());

    let (status, body) = upsert(&slug, envelope_body(&slug, Some(TOKEN)));
    assert_eq!(status, 200, "upsert failed: {body:?}");
    assert_no_secret(&body);
    assert_eq!(body["credential_configured"], true);
    assert_eq!(
        body["status"], "pending_validation",
        "a fresh credential must not be trusted on sight"
    );
    assert!(body["last_validated_at"].is_null());

    // The stored ciphertext is not the plaintext.
    let exec = sesame_idam_database::db();
    let config = TenantSmsService::find(&slug, ENV, exec)
        .expect("lookup")
        .expect("config exists");
    let ciphertext = config
        .auth_token_ciphertext
        .as_deref()
        .expect("credential stored");
    assert!(!ciphertext.contains(TOKEN), "token stored in the clear");

    // Not active yet → does not resolve, so nothing can be sent with it.
    assert!(
        TenantSmsService::resolve_credential(&config).is_none(),
        "an unvalidated credential must not resolve for sending"
    );

    // Reading it back still never surfaces the secret.
    let (get_status, get_body) = get(&slug);
    assert_eq!(get_status, 200);
    assert_no_secret(&get_body);

    // Once validated it resolves — and round-trips to exactly what was sent.
    TenantSmsService::mark_validated(&slug, ENV, exec).expect("validate");
    let active = TenantSmsService::find(&slug, ENV, exec)
        .expect("lookup")
        .expect("config exists");
    assert_eq!(active.status, STATUS_ACTIVE);
    assert_eq!(
        TenantSmsService::resolve_credential(&active).as_deref(),
        Some(TOKEN)
    );

    std::env::remove_var("SMS_ENVELOPE_CUSTODY_TENANTS");
}

/// Scenario: a tenant-billed purpose resolves to the TENANT's credential once
/// active — and to nothing at all when the tenant has no sender. It must never
/// resolve to the platform's account (ADR-009 §2.5).
#[test]
fn tenant_purpose_never_falls_back_to_platform() {
    if !db_available() {
        println!("SKIP: Postgres not available");
        return;
    }
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    std::env::set_var("SMS_ALLOWED_PURPOSES", "registration,password_reset");

    let unconfigured = unique_slug("bdd-sms-none");
    mint_tenant(&unconfigured);
    match resolve_sms_sender(&unconfigured, ENV, SmsPurpose::Registration) {
        Err(Unresolved::NoTenantSender { tenant }) => assert_eq!(tenant, unconfigured),
        other => panic!("expected NoTenantSender, got {other:?}"),
    }

    let slug = unique_slug("bdd-sms-resolve");
    mint_tenant(&slug);
    std::env::set_var("SMS_ENVELOPE_CUSTODY_TENANTS", &slug);
    std::env::set_var("SMS_CREDENTIAL_KEK", envelope::generate_kek().unwrap());
    assert_eq!(upsert(&slug, envelope_body(&slug, Some(TOKEN))).0, 200);

    // Still pending → still refuses, so a typo'd credential fails closed.
    assert!(matches!(
        resolve_sms_sender(&slug, ENV, SmsPurpose::Registration),
        Err(Unresolved::NoTenantSender { .. })
    ));

    let exec = sesame_idam_database::db();
    TenantSmsService::mark_validated(&slug, ENV, exec).expect("validate");

    let sender = resolve_sms_sender(&slug, ENV, SmsPurpose::Registration).expect("resolves");
    assert_eq!(sender.owner, BillingOwner::Tenant(slug.clone()));
    assert_eq!(
        sender.spend_scope,
        format!("tenant:{slug}"),
        "tenant spend must be accounted against the tenant, not the platform"
    );
    assert_eq!(sender.daily_ceiling_cents, 250);
    match sender.credential {
        Credential::TenantEnvelope { auth_token, .. } => assert_eq!(auth_token, TOKEN),
        other => panic!("expected envelope credential, got {other:?}"),
    }

    std::env::remove_var("SMS_ENVELOPE_CUSTODY_TENANTS");
}

/// Scenario: revocation stops sending AND destroys the sealed material.
#[test]
fn revocation_clears_sealed_material() {
    if !db_available() {
        println!("SKIP: Postgres not available");
        return;
    }
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    let slug = unique_slug("bdd-sms-revoke");
    mint_tenant(&slug);
    std::env::set_var("SMS_ENVELOPE_CUSTODY_TENANTS", &slug);
    std::env::set_var("SMS_CREDENTIAL_KEK", envelope::generate_kek().unwrap());
    std::env::set_var("SMS_ALLOWED_PURPOSES", "registration");
    assert_eq!(upsert(&slug, envelope_body(&slug, Some(TOKEN))).0, 200);

    let exec = sesame_idam_database::db();
    TenantSmsService::mark_validated(&slug, ENV, exec).expect("validate");
    assert!(resolve_sms_sender(&slug, ENV, SmsPurpose::Registration).is_ok());

    let (status, body) = revoke(&slug);
    assert_eq!(status, 200, "revoke failed: {body:?}");
    assert_eq!(body["status"], "revoked");
    assert_eq!(body["credential_configured"], false);

    let config = TenantSmsService::find(&slug, ENV, exec)
        .expect("lookup")
        .expect("row survives for audit");
    assert!(
        config.auth_token_ciphertext.is_none() && config.dek_wrapped.is_none(),
        "revocation must destroy the sealed material, not just flip a flag"
    );
    assert!(matches!(
        resolve_sms_sender(&slug, ENV, SmsPurpose::Registration),
        Err(Unresolved::NoTenantSender { .. })
    ));

    std::env::remove_var("SMS_ENVELOPE_CUSTODY_TENANTS");
}

/// Scenario: moving to Connect erases the previously stored secret — the
/// tenant asked us to stop holding it, so we must actually stop holding it.
#[test]
fn switching_to_connect_erases_stored_secret() {
    if !db_available() {
        println!("SKIP: Postgres not available");
        return;
    }
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    let slug = unique_slug("bdd-sms-switch");
    mint_tenant(&slug);
    std::env::set_var("SMS_ENVELOPE_CUSTODY_TENANTS", &slug);
    std::env::set_var("SMS_CREDENTIAL_KEK", envelope::generate_kek().unwrap());
    assert_eq!(upsert(&slug, envelope_body(&slug, Some(TOKEN))).0, 200);

    let (status, body) = upsert(&slug, connect_body(&slug));
    assert_eq!(status, 200, "connect upsert failed: {body:?}");
    assert_eq!(body["custody_mode"], "connect");
    assert_no_secret(&body);

    let exec = sesame_idam_database::db();
    let config = TenantSmsService::find(&slug, ENV, exec)
        .expect("lookup")
        .expect("config exists");
    assert!(
        config.auth_token_ciphertext.is_none()
            && config.auth_token_nonce.is_none()
            && config.dek_wrapped.is_none(),
        "switching to Connect must leave no sealed secret behind"
    );
    // Non-secret settings carry over rather than being silently dropped.
    assert_eq!(config.from_number.as_deref(), Some("+15551230000"));
    assert_eq!(config.campaign_ref.as_deref(), Some("CAMP-123"));

    std::env::remove_var("SMS_ENVELOPE_CUSTODY_TENANTS");
}

/// Scenario: editing a non-secret field without re-sending the token keeps the
/// stored credential and its earned trust — an admin should not have to fetch
/// the secret out of a password manager to change a phone number.
#[test]
fn omitting_the_token_keeps_the_stored_credential() {
    if !db_available() {
        println!("SKIP: Postgres not available");
        return;
    }
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    let slug = unique_slug("bdd-sms-keep");
    mint_tenant(&slug);
    std::env::set_var("SMS_ENVELOPE_CUSTODY_TENANTS", &slug);
    std::env::set_var("SMS_CREDENTIAL_KEK", envelope::generate_kek().unwrap());
    assert_eq!(upsert(&slug, envelope_body(&slug, Some(TOKEN))).0, 200);

    let exec = sesame_idam_database::db();
    TenantSmsService::mark_validated(&slug, ENV, exec).expect("validate");

    let mut edit = envelope_body(&slug, None);
    edit.from_number = Some("+15559999999".to_string());
    let (status, body) = upsert(&slug, edit);
    assert_eq!(status, 200, "edit failed: {body:?}");
    assert_eq!(body["from_number"], "+15559999999");
    assert_eq!(
        body["status"], "active",
        "an untouched credential keeps the trust it already earned"
    );

    let config = TenantSmsService::find(&slug, ENV, exec)
        .expect("lookup")
        .expect("config exists");
    assert_eq!(
        TenantSmsService::resolve_credential(&config).as_deref(),
        Some(TOKEN),
        "the stored credential must survive an edit that omits it"
    );

    std::env::remove_var("SMS_ENVELOPE_CUSTODY_TENANTS");
}
