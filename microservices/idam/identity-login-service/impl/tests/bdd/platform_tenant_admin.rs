//! Platform tenant admin BDD: mint → register → login; suspend blocks auth;
//! OAuth rotate bumps `config_version` on GET.

use std::collections::HashMap;
use std::net::TcpStream;
use std::sync::Once;
use std::time::Duration;

use brrtrouter::typed::TypedHandlerRequest;
use http::Method;

use sesame_idam_identity_login_service::controllers::{
    auth_login, auth_register, platform_tenant_create, platform_tenant_get,
    platform_tenant_oauth_rotate, platform_tenant_oauth_upsert, platform_tenant_status_patch,
};
use sesame_idam_identity_login_service_gen::handlers::auth_login::Request as LoginRequest;
use sesame_idam_identity_login_service_gen::handlers::auth_register::Request as RegisterRequest;
use sesame_idam_identity_login_service_gen::handlers::platform_tenant_create::Request as CreateRequest;
use sesame_idam_identity_login_service_gen::handlers::platform_tenant_get::Request as GetRequest;
use sesame_idam_identity_login_service_gen::handlers::platform_tenant_oauth_rotate::Request as RotateRequest;
use sesame_idam_identity_login_service_gen::handlers::platform_tenant_oauth_upsert::Request as OauthUpsertRequest;
use sesame_idam_identity_login_service_gen::handlers::platform_tenant_status_patch::Request as StatusPatchRequest;

static INIT: Once = Once::new();

fn db_available() -> bool {
    let host = std::env::var("TEST_DB_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("TEST_DB_PORT").unwrap_or_else(|_| "5432".to_string());
    let addr = format!("{host}:{port}");
    let reachable = addr
        .parse()
        .ok()
        .and_then(|sa| TcpStream::connect_timeout(&sa, Duration::from_millis(500)).ok())
        .is_some();
    if !reachable {
        return false;
    }

    INIT.call_once(|| {
        std::env::set_var("DB_POOL_MAX", "2");
        std::env::set_var("DB_HOST", &host);
        std::env::set_var("DB_PORT", &port);
        std::env::set_var(
            "DB_USER",
            std::env::var("TEST_DB_USER").unwrap_or_else(|_| "sesame_idam".to_string()),
        );
        std::env::set_var(
            "DB_PASS",
            std::env::var("TEST_DB_PASS")
                .unwrap_or_else(|_| "dev_password_change_in_prod".to_string()),
        );
        std::env::set_var(
            "DB_NAME",
            std::env::var("TEST_DB_NAME").unwrap_or_else(|_| "sesame_idam".to_string()),
        );
    });
    true
}

fn unique_slug(prefix: &str) -> String {
    let id = uuid::Uuid::new_v4().simple().to_string();
    format!("{prefix}-{id}")
}

fn create_request(slug: &str, display_name: &str) -> TypedHandlerRequest<CreateRequest> {
    TypedHandlerRequest {
        method: Method::POST,
        path: "/platform/tenants".to_string(),
        handler_name: "platform_tenant_create".to_string(),
        path_params: HashMap::new(),
        query_params: HashMap::new(),
        data: CreateRequest {
            slug: slug.to_string(),
            display_name: display_name.to_string(),
            activate: Some(true),
            provisioning_mode: None,
        },
        jwt_claims: None,
    }
}

fn get_request(slug: &str) -> TypedHandlerRequest<GetRequest> {
    TypedHandlerRequest {
        method: Method::GET,
        path: format!("/platform/tenants/{slug}"),
        handler_name: "platform_tenant_get".to_string(),
        path_params: HashMap::from([("slug".to_string(), slug.to_string())]),
        query_params: HashMap::new(),
        data: GetRequest {
            slug: slug.to_string(),
        },
        jwt_claims: None,
    }
}

fn status_patch_request(slug: &str, status: &str) -> TypedHandlerRequest<StatusPatchRequest> {
    TypedHandlerRequest {
        method: Method::PATCH,
        path: format!("/platform/tenants/{slug}/status"),
        handler_name: "platform_tenant_status_patch".to_string(),
        path_params: HashMap::from([("slug".to_string(), slug.to_string())]),
        query_params: HashMap::new(),
        data: StatusPatchRequest {
            slug: slug.to_string(),
            status: status.to_string(),
        },
        jwt_claims: None,
    }
}

fn oauth_upsert_request(slug: &str) -> TypedHandlerRequest<OauthUpsertRequest> {
    TypedHandlerRequest {
        method: Method::PUT,
        path: format!("/platform/tenants/{slug}/oauth/google"),
        handler_name: "platform_tenant_oauth_upsert".to_string(),
        path_params: HashMap::from([
            ("slug".to_string(), slug.to_string()),
            ("provider".to_string(), "google".to_string()),
        ]),
        query_params: HashMap::new(),
        data: OauthUpsertRequest {
            slug: slug.to_string(),
            provider: "google".to_string(),
            client_id: "bdd-client-id".to_string(),
            redirect_uris: vec!["http://localhost/oauth/callback".to_string()],
            secret_env_key: "BDD_OAUTH_SECRET_KEY".to_string(),
            client_id_env_key: None,
            enabled: Some(true),
        },
        jwt_claims: None,
    }
}

fn oauth_rotate_request(slug: &str) -> TypedHandlerRequest<RotateRequest> {
    TypedHandlerRequest {
        method: Method::POST,
        path: format!("/platform/tenants/{slug}/oauth/google/rotate"),
        handler_name: "platform_tenant_oauth_rotate".to_string(),
        path_params: HashMap::from([
            ("slug".to_string(), slug.to_string()),
            ("provider".to_string(), "google".to_string()),
        ]),
        query_params: HashMap::new(),
        data: RotateRequest {
            slug: slug.to_string(),
            provider: "google".to_string(),
            rotated_by: "bdd@example.com".to_string(),
        },
        jwt_claims: None,
    }
}

fn register_request(
    tenant: &str,
    email: &str,
    password: &str,
) -> TypedHandlerRequest<RegisterRequest> {
    TypedHandlerRequest {
        method: Method::POST,
        path: "/auth/register".to_string(),
        handler_name: "auth_register".to_string(),
        path_params: HashMap::new(),
        query_params: HashMap::new(),
        data: RegisterRequest {
            email: email.to_string(),
            first_name: None,
            last_name: None,
            password: password.to_string(),
            phone: None,
            username: None,
            x_tenant_id: tenant.to_string(),
        },
        jwt_claims: None,
    }
}

fn login_request(tenant: &str, email: &str, password: &str) -> TypedHandlerRequest<LoginRequest> {
    TypedHandlerRequest {
        method: Method::POST,
        path: "/auth/login".to_string(),
        handler_name: "auth_login".to_string(),
        path_params: HashMap::new(),
        query_params: HashMap::new(),
        data: LoginRequest {
            email: email.to_string(),
            organization_id: None,
            password: password.to_string(),
            x_tenant_id: tenant.to_string(),
        },
        jwt_claims: None,
    }
}

/// Scenario: platform mint → register → login succeeds.
#[test]
fn platform_mint_register_login() {
    if !db_available() {
        println!("SKIP: Postgres not available");
        return;
    }

    let slug = unique_slug("bdd-platform");
    let create = platform_tenant_create::handle(create_request(&slug, "BDD Platform Tenant"));
    assert_eq!(create.status, 201, "create failed: {:?}", create.body);

    let email = format!("user_{}@example.com", uuid::Uuid::new_v4());
    let password = "SecureP@ss123!";

    let reg = auth_register::handle(register_request(&slug, &email, password));
    assert_eq!(reg.status, 201, "register failed: {:?}", reg.body);

    let login = auth_login::handle(login_request(&slug, &email, password));
    assert_eq!(login.status, 200, "login failed: {:?}", login.body);
    assert!(login.body["access_token"].is_string());
}

/// Scenario: unknown slug rejected before platform mint.
#[test]
fn unknown_slug_before_mint_rejected() {
    if !db_available() {
        println!("SKIP: Postgres not available");
        return;
    }

    let slug = unique_slug("unprovisioned");
    let email = format!("nobody_{}@example.com", uuid::Uuid::new_v4());

    let reg = auth_register::handle(register_request(&slug, &email, "SecureP@ss123!"));
    assert_eq!(reg.status, 404);
    assert_eq!(reg.body["error"], "tenant_unknown");
}

/// Scenario: suspend via PATCH blocks login.
#[test]
fn suspend_blocks_login() {
    if !db_available() {
        println!("SKIP: Postgres not available");
        return;
    }

    let slug = unique_slug("bdd-suspend");
    let create = platform_tenant_create::handle(create_request(&slug, "Suspend Test"));
    assert_eq!(create.status, 201);

    let email = format!("suspend_{}@example.com", uuid::Uuid::new_v4());
    let password = "SecureP@ss123!";
    let reg = auth_register::handle(register_request(&slug, &email, password));
    assert_eq!(reg.status, 201);

    let patch = platform_tenant_status_patch::handle(status_patch_request(&slug, "suspended"));
    assert_eq!(patch.status, 200, "suspend failed: {:?}", patch.body);

    let login = auth_login::handle(login_request(&slug, &email, password));
    assert_eq!(login.status, 403);
    assert_eq!(login.body["error"], "tenant_not_active");
}

/// Scenario: OAuth rotate bumps `config_version` visible on GET.
#[test]
fn oauth_rotate_bumps_config_version() {
    if !db_available() {
        println!("SKIP: Postgres not available");
        return;
    }

    let slug = unique_slug("bdd-oauth");
    let create = platform_tenant_create::handle(create_request(&slug, "OAuth Rotate Test"));
    assert_eq!(create.status, 201);

    let upsert = platform_tenant_oauth_upsert::handle(oauth_upsert_request(&slug));
    assert_eq!(upsert.status, 200, "oauth upsert failed: {:?}", upsert.body);
    assert_eq!(upsert.body["config_version"], 1);

    let rotate = platform_tenant_oauth_rotate::handle(oauth_rotate_request(&slug));
    assert_eq!(rotate.status, 200, "rotate failed: {:?}", rotate.body);
    assert_eq!(rotate.body["config_version"], 2);

    let get = platform_tenant_get::handle(get_request(&slug));
    assert_eq!(get.status, 200);
    let providers = get.body["oauth_providers"]
        .as_array()
        .expect("oauth_providers");
    let google = providers
        .iter()
        .find(|p| p["provider"] == "google")
        .expect("google provider");
    assert_eq!(google["config_version"], 2);
}
