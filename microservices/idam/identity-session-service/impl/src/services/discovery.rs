//! OIDC discovery document construction.

fn issuer_url() -> String {
    std::env::var("SESAME_JWT_ISSUER")
        .unwrap_or_else(|_| "https://id.sesameidentity.dev.local".into())
}

fn auth_base_url() -> String {
    std::env::var("SESAME_AUTH_PUBLIC_URL")
        .unwrap_or_else(|_| "https://auth.sesameidentity.dev.local".into())
}

fn api_base_url() -> String {
    std::env::var("SESAME_API_PUBLIC_URL")
        .unwrap_or_else(|_| "https://api.sesameidentity.dev.local".into())
}

/// Build the `OpenID` Connect discovery document per our `OpenAPI` example.
pub struct OpenIdDiscovery {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub token_endpoint_auth_methods_supported: Vec<String>,
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
    pub claims_supported: Vec<String>,
}

/// Load discovery metadata from environment with spec-aligned defaults.
#[must_use]
pub fn openid_configuration() -> OpenIdDiscovery {
    OpenIdDiscovery {
        issuer: issuer_url(),
        authorization_endpoint: format!("{}/oauth/authorize", auth_base_url()),
        token_endpoint: format!("{}/oauth/token", api_base_url()),
        token_endpoint_auth_methods_supported: vec![
            "none".into(),
            "client_secret_basic".into(),
            "client_secret_post".into(),
        ],
        jwks_uri: format!("{}/.well-known/jwks.json", issuer_url()),
        userinfo_endpoint: format!("{}/oauth/userinfo", api_base_url()),
        scopes_supported: vec!["openid".into(), "email".into(), "profile".into()],
        response_types_supported: vec!["code".into()],
        response_modes_supported: vec!["query".into()],
        grant_types_supported: vec!["authorization_code".into(), "refresh_token".into()],
        subject_types_supported: vec!["public".into()],
        id_token_signing_alg_values_supported: vec!["EdDSA".into()],
        userinfo_signing_alg_values_supported: vec![],
        userinfo_encryption_alg_values_supported: vec![],
        userinfo_encryption_enc_values_supported: vec![],
        code_challenge_methods_supported: vec!["S256".into()],
        claims_supported: vec![
            "iss".into(),
            "sub".into(),
            "aud".into(),
            "exp".into(),
            "iat".into(),
            "auth_time".into(),
            "nonce".into(),
            "azp".into(),
            "email".into(),
            "email_verified".into(),
            "preferred_username".into(),
        ],
    }
}
