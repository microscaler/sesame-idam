//! Local SAML SSO broker + OAuth provider mocks for Sesame-IDAM contract testing.
//!
//! Replaces internet-dependent DummyIDP for desktop dev:
//! - SSOReady-compatible `/v1/saml/redirect` + `/v1/saml/redeem`
//! - Local IdP simulator (`/idp/login`, `/idp/simulate`)
//! - Google / Microsoft OAuth token + profile endpoints under `/mock/*`
//!
//! Pact contracts: `pacts/Sesame-SSO-Broker.json`, `Sesame-OAuth-Google.json`, `Sesame-OAuth-Microsoft.json`

use axum::{
    body::Bytes,
    extract::{Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Form, Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing::{info, warn};
use uuid::Uuid;

use crate::{health_check, logging_middleware};

const DEFAULT_TEST_ORG: &str = "a1000001-0001-4000-8000-000000000001";
const DEFAULT_TEST_EMAIL: &str = "buyer@testshipper.local";
const PACT_TEST_ACCESS_CODE: &str = "saml_access_code_test";

/// In-memory broker state (organizations, pending logins, issued access codes).
#[derive(Clone, Debug)]
pub struct BrokerState {
    pub base_url: String,
    pub app_redirect_url: String,
    pub api_key: String,
    pub organizations: Arc<RwLock<HashMap<String, OrgRecord>>>,
    pub login_sessions: Arc<RwLock<HashMap<String, LoginSession>>>,
    pub access_codes: Arc<RwLock<HashMap<String, AccessCodeRecord>>>,
    pub oauth_codes: Arc<RwLock<HashMap<String, OAuthCodeRecord>>>,
}

#[derive(Clone, Debug, Serialize)]
pub struct OrgRecord {
    pub external_id: String,
    pub broker_org_id: String,
    pub domains: Vec<String>,
    pub users: Vec<String>,
}

#[derive(Clone, Debug)]
struct LoginSession {
    org_external_id: String,
    created_at: std::time::Instant,
}

#[derive(Clone, Debug)]
struct AccessCodeRecord {
    email: String,
    org_external_id: String,
    broker_org_id: String,
    redeemed: bool,
}

#[derive(Clone, Debug)]
struct OAuthCodeRecord {
    provider: String,
    email: String,
    provider_user_id: String,
}

impl BrokerState {
    #[must_use]
    pub fn new(base_url: impl Into<String>, app_redirect_url: impl Into<String>) -> Self {
        let base_url = base_url.into();
        let mut organizations = HashMap::new();
        organizations.insert(
            DEFAULT_TEST_ORG.to_string(),
            OrgRecord {
                external_id: DEFAULT_TEST_ORG.to_string(),
                broker_org_id: "org_test_001".to_string(),
                domains: vec!["testshipper.local".to_string()],
                users: vec![
                    DEFAULT_TEST_EMAIL.to_string(),
                    "it-admin@testshipper.local".to_string(),
                ],
            },
        );
        // Hauliage demo shipper org (AME Corp) — Track A MVP
        organizations.insert(
            "b2000002-0002-4000-8000-000000000002".to_string(),
            OrgRecord {
                external_id: "b2000002-0002-4000-8000-000000000002".to_string(),
                broker_org_id: "org_amecorp_001".to_string(),
                domains: vec!["amecorp.dev".to_string()],
                users: vec!["shipper@amecorp.dev".to_string()],
            },
        );

        Self {
            base_url,
            app_redirect_url: app_redirect_url.into(),
            api_key: std::env::var("SESAME_BROKER_API_KEY")
                .unwrap_or_else(|_| "ssoready_sk_test".to_string()),
            organizations: Arc::new(RwLock::new(organizations)),
            login_sessions: Arc::new(RwLock::new(HashMap::new())),
            access_codes: Arc::new(RwLock::new(HashMap::new())),
            oauth_codes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn authorize_api_key(&self, headers: &HeaderMap) -> bool {
        let Some(auth) = headers.get(header::AUTHORIZATION) else {
            return false;
        };
        let Ok(value) = auth.to_str() else {
            return false;
        };
        let expected = format!("Bearer {}", self.api_key);
        value == expected
    }
}

#[derive(Debug, Deserialize)]
pub struct SamlRedirectRequest {
    pub organization_external_id: Option<String>,
    #[serde(rename = "organizationExternalId")]
    pub organization_external_id_camel: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SamlRedirectResponse {
    #[serde(rename = "redirectUrl")]
    pub redirect_url: String,
}

#[derive(Debug, Deserialize)]
pub struct SamlRedeemRequest {
    pub saml_access_code: Option<String>,
    #[serde(rename = "samlAccessCode")]
    pub saml_access_code_camel: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SamlRedeemResponse {
    pub email: String,
    #[serde(rename = "organizationId")]
    pub organization_id: String,
    #[serde(rename = "organizationExternalId")]
    pub organization_external_id: String,
}

#[derive(Debug, Deserialize)]
pub struct IdpSimulateRequest {
    pub organization_external_id: Option<String>,
    #[serde(rename = "organizationExternalId")]
    pub organization_external_id_camel: Option<String>,
    pub email: String,
}

#[derive(Debug, Serialize)]
pub struct IdpSimulateResponse {
    #[serde(rename = "samlAccessCode")]
    pub saml_access_code: String,
    #[serde(rename = "organizationExternalId")]
    pub organization_external_id: String,
    pub email: String,
    #[serde(rename = "redirectUrl")]
    pub redirect_url: String,
}

#[derive(Debug, Deserialize)]
pub struct IdpLoginQuery {
    pub session: String,
}

#[derive(Debug, Deserialize)]
pub struct IdpLoginForm {
    pub session: String,
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct OAuthAuthorizeQuery {
    pub redirect_uri: String,
    pub state: String,
    pub client_id: Option<String>,
    /// Deterministic test user for CI (skips HTML form).
    pub email: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OAuthTokenForm {
    pub code: String,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub redirect_uri: Option<String>,
    pub grant_type: Option<String>,
}

fn org_external_id(req: &SamlRedirectRequest) -> Option<String> {
    req.organization_external_id
        .clone()
        .or_else(|| req.organization_external_id_camel.clone())
}

fn access_code_value(req: &SamlRedeemRequest) -> Option<String> {
    req.saml_access_code
        .clone()
        .or_else(|| req.saml_access_code_camel.clone())
}

async fn issue_access_code(
    state: &BrokerState,
    org_external_id: &str,
    email: &str,
) -> Result<String, StatusCode> {
    let orgs = state.organizations.read().await;
    let Some(org) = orgs.get(org_external_id) else {
        return Err(StatusCode::NOT_FOUND);
    };
    if !org.users.iter().any(|u| u.eq_ignore_ascii_case(email)) {
        warn!(email, org_external_id, "IdP login rejected: user not in org");
        return Err(StatusCode::FORBIDDEN);
    }
    let broker_org_id = org.broker_org_id.clone();
    drop(orgs);

    let code = format!("saml_access_code_{}", Uuid::new_v4().simple());
    let mut codes = state.access_codes.write().await;
    codes.insert(
        code.clone(),
        AccessCodeRecord {
            email: email.to_ascii_lowercase(),
            org_external_id: org_external_id.to_string(),
            broker_org_id,
            redeemed: false,
        },
    );
    Ok(code)
}

/// POST /v1/saml/redirect — SSOReady-compatible redirect URL generation.
pub async fn saml_redirect(
    State(state): State<BrokerState>,
    headers: HeaderMap,
    Json(body): Json<SamlRedirectRequest>,
) -> Result<Json<SamlRedirectResponse>, StatusCode> {
    if !state.authorize_api_key(&headers) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let org_id = org_external_id(&body).ok_or(StatusCode::BAD_REQUEST)?;
    if !state.organizations.read().await.contains_key(&org_id) {
        return Err(StatusCode::NOT_FOUND);
    }

    let session_id = Uuid::new_v4().to_string();
    state.login_sessions.write().await.insert(
        session_id.clone(),
        LoginSession {
            org_external_id: org_id,
            created_at: std::time::Instant::now(),
        },
    );

    let redirect_url = format!("{}/idp/login?session={session_id}", state.base_url.trim_end_matches('/'));
    info!(%redirect_url, "SAML redirect issued");
    Ok(Json(SamlRedirectResponse { redirect_url }))
}

/// POST /v1/saml/redeem — exchange access code for authenticated identity.
pub async fn saml_redeem(
    State(state): State<BrokerState>,
    headers: HeaderMap,
    Json(body): Json<SamlRedeemRequest>,
) -> Result<Json<SamlRedeemResponse>, StatusCode> {
    if !state.authorize_api_key(&headers) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let code = access_code_value(&body).ok_or(StatusCode::BAD_REQUEST)?;

    // Pact fixture code for contract tests
    if code == PACT_TEST_ACCESS_CODE {
        return Ok(Json(SamlRedeemResponse {
            email: DEFAULT_TEST_EMAIL.to_string(),
            organization_id: "org_test_001".to_string(),
            organization_external_id: DEFAULT_TEST_ORG.to_string(),
        }));
    }

    let mut codes = state.access_codes.write().await;
    let Some(record) = codes.get_mut(&code) else {
        return Err(StatusCode::BAD_REQUEST);
    };
    if record.redeemed {
        return Err(StatusCode::BAD_REQUEST);
    }
    record.redeemed = true;
    let response = SamlRedeemResponse {
        email: record.email.clone(),
        organization_id: record.broker_org_id.clone(),
        organization_external_id: record.org_external_id.clone(),
    };
    Ok(Json(response))
}

/// POST /idp/simulate — CI-friendly login without browser or public internet.
pub async fn idp_simulate(
    State(state): State<BrokerState>,
    Json(body): Json<IdpSimulateRequest>,
) -> Result<Json<IdpSimulateResponse>, StatusCode> {
    let org_id = body
        .organization_external_id
        .or(body.organization_external_id_camel)
        .ok_or(StatusCode::BAD_REQUEST)?;
    let code = issue_access_code(&state, &org_id, &body.email).await?;
    let redirect_url = format!(
        "{}?saml_access_code={}",
        state.app_redirect_url, code
    );
    Ok(Json(IdpSimulateResponse {
        saml_access_code: code,
        organization_external_id: org_id,
        email: body.email.to_ascii_lowercase(),
        redirect_url,
    }))
}

/// GET /idp/login — minimal HTML IdP login form (local-only).
pub async fn idp_login_get(
    State(state): State<BrokerState>,
    Query(query): Query<IdpLoginQuery>,
) -> Result<Html<String>, StatusCode> {
    let sessions = state.login_sessions.read().await;
    let Some(session) = sessions.get(&query.session) else {
        return Err(StatusCode::NOT_FOUND);
    };
    let orgs = state.organizations.read().await;
    let Some(org) = orgs.get(&session.org_external_id) else {
        return Err(StatusCode::NOT_FOUND);
    };
    let options: String = org
        .users
        .iter()
        .map(|u| format!("<option value=\"{u}\">{u}</option>"))
        .collect();
    let html = format!(
        r#"<!DOCTYPE html>
<html><head><title>Sesame Pact IdP (local)</title></head>
<body>
  <h1>Sesame SAML Test IdP</h1>
  <p>Organization: {org_id}</p>
  <form method="post" action="/idp/login">
    <input type="hidden" name="session" value="{session}" />
    <label>Employee email:</label>
    <select name="email">{options}</select>
    <button type="submit">Sign in with SAML</button>
  </form>
</body></html>"#,
        org_id = org.external_id,
        session = query.session,
        options = options
    );
    Ok(Html(html))
}

/// POST /idp/login — complete browser IdP flow → redirect to app callback.
pub async fn idp_login_post(
    State(state): State<BrokerState>,
    Form(form): Form<IdpLoginForm>,
) -> Result<Response, StatusCode> {
    let org_external_id = {
        let sessions = state.login_sessions.read().await;
        let Some(session) = sessions.get(&form.session) else {
            return Err(StatusCode::NOT_FOUND);
        };
        session.org_external_id.clone()
    };
    let code = issue_access_code(&state, &org_external_id, &form.email).await?;
    let target = format!("{}?saml_access_code={}", state.app_redirect_url, code);
    Ok(Redirect::temporary(&target).into_response())
}

// --- OAuth mocks (Google / Microsoft) ---

async fn oauth_authorize(
    state: BrokerState,
    provider: &'static str,
    default_email: &str,
    default_sub: &str,
    query: OAuthAuthorizeQuery,
) -> Result<Response, StatusCode> {
    let email = query.email.as_deref().unwrap_or(default_email);
    let code = format!("{provider}_code_{}", Uuid::new_v4().simple());
    {
        let mut codes = state.oauth_codes.write().await;
        codes.insert(
            code.clone(),
            OAuthCodeRecord {
                provider: provider.to_string(),
                email: email.to_ascii_lowercase(),
                provider_user_id: default_sub.to_string(),
            },
        );
    }
    let separator = if query.redirect_uri.contains('?') { '&' } else { '?' };
    let target = format!(
        "{redirect_uri}{separator}code={code}&state={state}",
        redirect_uri = query.redirect_uri,
        separator = separator,
        code = code,
        state = urlencoding::encode(&query.state),
    );
    Ok(Redirect::temporary(&target).into_response())
}

pub async fn google_authorize(
    State(state): State<BrokerState>,
    Query(query): Query<OAuthAuthorizeQuery>,
) -> Result<Response, StatusCode> {
    oauth_authorize(
        state,
        "google",
        "alice@gmail.com",
        "google-sub-001",
        query,
    )
    .await
}

pub async fn microsoft_authorize(
    State(state): State<BrokerState>,
    Query(query): Query<OAuthAuthorizeQuery>,
) -> Result<Response, StatusCode> {
    oauth_authorize(
        state,
        "microsoft",
        "bob@outlook.com",
        "microsoft-sub-001",
        query,
    )
    .await
}

pub async fn google_token(
    State(state): State<BrokerState>,
    body: Bytes,
) -> Result<Json<Value>, StatusCode> {
    let form: OAuthTokenForm = serde_urlencoded::from_bytes(&body).map_err(|_| StatusCode::BAD_REQUEST)?;
    oauth_token_response(&state, "google", &form).await
}

pub async fn microsoft_token(
    State(state): State<BrokerState>,
    body: Bytes,
) -> Result<Json<Value>, StatusCode> {
    let form: OAuthTokenForm = serde_urlencoded::from_bytes(&body).map_err(|_| StatusCode::BAD_REQUEST)?;
    oauth_token_response(&state, "microsoft", &form).await
}

async fn oauth_token_response(
    state: &BrokerState,
    provider: &str,
    form: &OAuthTokenForm,
) -> Result<Json<Value>, StatusCode> {
    let fixture_code = match provider {
        "google" => "google_test_code",
        "microsoft" => "microsoft_test_code",
        _ => return Err(StatusCode::BAD_REQUEST),
    };
    let fixture_token = match provider {
        "google" => "google_mock_access_token",
        "microsoft" => "microsoft_mock_access_token",
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    if form.code == fixture_code {
        return Ok(Json(json!({
            "access_token": fixture_token,
            "token_type": "Bearer",
            "expires_in": 3600
        })));
    }
    let codes = state.oauth_codes.read().await;
    if codes.contains_key(&form.code) {
        return Ok(Json(json!({
            "access_token": format!("{provider}_token_for_{}", form.code),
            "token_type": "Bearer",
            "expires_in": 3600
        })));
    }
    Err(StatusCode::BAD_REQUEST)
}

#[allow(dead_code)]
pub async fn google_token_form(
    State(state): State<BrokerState>,
    Form(form): Form<OAuthTokenForm>,
) -> Result<Json<Value>, StatusCode> {
    oauth_token_response(&state, "google", &form).await
}

pub async fn google_userinfo(headers: HeaderMap) -> Result<Json<Value>, StatusCode> {
    let auth = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if auth == "Bearer google_mock_access_token" || auth.starts_with("Bearer google_token_for_") {
        return Ok(Json(json!({
            "sub": "google-sub-001",
            "email": "alice@gmail.com",
            "email_verified": true
        })));
    }
    Err(StatusCode::UNAUTHORIZED)
}

pub async fn microsoft_me(headers: HeaderMap) -> Result<Json<Value>, StatusCode> {
    let auth = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if auth == "Bearer microsoft_mock_access_token" || auth.starts_with("Bearer microsoft_token_for_")
    {
        return Ok(Json(json!({
            "id": "microsoft-sub-001",
            "mail": "bob@outlook.com",
            "userPrincipalName": "bob@outlook.com"
        })));
    }
    Err(StatusCode::UNAUTHORIZED)
}

/// Admin/test helper — register an organization for SAML tests.
pub async fn admin_register_org(
    State(state): State<BrokerState>,
    Json(body): Json<Value>,
) -> Result<Json<OrgRecord>, StatusCode> {
    let external_id = body
        .get("externalId")
        .or_else(|| body.get("external_id"))
        .and_then(|v| v.as_str())
        .ok_or(StatusCode::BAD_REQUEST)?;
    let domains: Vec<String> = body
        .get("domains")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let users: Vec<String> = body
        .get("users")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_else(|| vec![DEFAULT_TEST_EMAIL.to_string()]);

    let record = OrgRecord {
        external_id: external_id.to_string(),
        broker_org_id: format!("org_{}", &external_id[..8.min(external_id.len())]),
        domains,
        users,
    };
    state
        .organizations
        .write()
        .await
        .insert(external_id.to_string(), record.clone());
    Ok(Json(record))
}

/// Build the combined Sesame IdAM broker router.
#[must_use]
pub fn build_app(state: BrokerState) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/v1/saml/redirect", post(saml_redirect))
        .route("/v1/saml/redeem", post(saml_redeem))
        .route("/idp/simulate", post(idp_simulate))
        .route("/idp/login", get(idp_login_get).post(idp_login_post))
        .route("/admin/organizations", post(admin_register_org))
        .route(
            "/mock/google/o/oauth2/v2/auth",
            get(google_authorize),
        )
        .route("/mock/google/token", post(google_token))
        .route(
            "/mock/google/userinfo",
            get(google_userinfo),
        )
        .route(
            "/mock/microsoft/common/oauth2/v2.0/authorize",
            get(microsoft_authorize),
        )
        .route("/mock/microsoft/token", post(microsoft_token))
        .route("/mock/microsoft/me", get(microsoft_me))
        .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()))
        .layer(axum::middleware::from_fn(logging_middleware))
        .with_state(state)
}

/// Default listen port for the Sesame IdAM pact broker.
pub const DEFAULT_PORT: u16 = 9190;
