//! OIDC discovery document construction.

/// Public base URL for Sesame-IDAM (no path suffix).
fn public_base_url() -> String {
    std::env::var("SESAME_IDAM_PUBLIC_URL")
        .or_else(|_| std::env::var("SESAME_JWT_ISSUER"))
        .unwrap_or_else(|_| "https://identity.seasame-idam.microscaler.local".into())
}

fn issuer_url() -> String {
    std::env::var("SESAME_JWT_ISSUER").unwrap_or_else(|_| public_base_url())
}

fn idam_v1_path(path: &str) -> String {
    format!("{}/idam/v1{}", public_base_url(), path)
}

/// Build the `OpenID` Connect discovery document per our `OpenAPI` example.
pub struct OpenIdDiscovery {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub jwks_uri: String,
    pub userinfo_endpoint: String,
    pub scopes_supported: Vec<String>,
    pub response_types_supported: Vec<String>,
    pub response_modes_supported: Vec<String>,
    pub grant_types_supported: Vec<String>,
    pub subject_types_supported: Vec<String>,
    pub id_token_signing_alg_values_supported: Vec<String>,
    pub userinfo_signing_alg_values_supported: Vec<String>,
    pub userinfo_encryption_alg_values_supported: Vec<String>,
    pub userinfo_encryption_enc_values_supported: Vec<String>,
    pub code_challenge_methods_supported: Vec<String>,
}

/// Load discovery metadata from environment with spec-aligned defaults.
#[must_use]
pub fn openid_configuration() -> OpenIdDiscovery {
    OpenIdDiscovery {
        issuer: issuer_url(),
        authorization_endpoint: idam_v1_path("/oauth/authorize"),
        token_endpoint: idam_v1_path("/auth/token"),
        jwks_uri: idam_v1_path("/.well-known/jwks.json"),
        userinfo_endpoint: idam_v1_path("/identity/userinfo"),
        scopes_supported: vec![
            "openid".into(),
            "email".into(),
            "profile".into(),
            "phone".into(),
        ],
        response_types_supported: vec!["code".into(), "id_token".into(), "id_token token".into()],
        response_modes_supported: vec!["query".into(), "fragment".into(), "form_post".into()],
        grant_types_supported: vec![
            "authorization_code".into(),
            "implicit".into(),
            "refresh_token".into(),
            "client_credentials".into(),
            "urn:ietf:params:oauth:grant-type:token-exchange".into(),
        ],
        subject_types_supported: vec!["public".into(), "pairwise".into()],
        id_token_signing_alg_values_supported: vec!["EdDSA".into()],
        userinfo_signing_alg_values_supported: vec![],
        userinfo_encryption_alg_values_supported: vec![],
        userinfo_encryption_enc_values_supported: vec![],
        code_challenge_methods_supported: vec!["S256".into()],
    }
}
