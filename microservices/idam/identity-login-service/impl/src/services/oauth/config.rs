//! Per-tenant OAuth provider configuration.
//!
//! Each tenant (hauliage, pricewhisperer, …) supplies its own Google/Microsoft app
//! credentials. A single Sesame instance serves all tenants; lookup is keyed by
//! `X-Tenant-ID` so one tenant's OAuth app is never used for another.

use std::collections::HashSet;

/// Supported social login providers for the MVP slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SupportedProvider {
    Google,
    Microsoft,
}

impl SupportedProvider {
    /// Parse a path/query provider name (case-insensitive).
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "google" => Some(Self::Google),
            "microsoft" => Some(Self::Microsoft),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Google => "google",
            Self::Microsoft => "microsoft",
        }
    }

    #[must_use]
    pub fn env_suffix(self) -> &'static str {
        match self {
            Self::Google => "GOOGLE",
            Self::Microsoft => "MICROSOFT",
        }
    }
}

/// Client credentials for one OAuth provider.
#[derive(Debug, Clone)]
pub struct ProviderCredentials {
    pub client_id: String,
    pub client_secret: String,
}

/// Tenant-scoped OAuth configuration.
///
/// Loaded exclusively for the requesting tenant — no global fallback (zero bleed).
#[derive(Debug, Clone)]
pub struct TenantOAuthConfig {
    pub tenant_id: String,
    pub google: Option<ProviderCredentials>,
    pub microsoft: Option<ProviderCredentials>,
    pub allowed_redirect_uris: HashSet<String>,
}

impl TenantOAuthConfig {
    /// Load OAuth settings for one tenant from environment variables.
    ///
    /// Env key pattern (tenant slug uppercased, `-` → `_`):
    ///
    /// - `SESAME_OAUTH__{TENANT}__GOOGLE_CLIENT_ID`
    /// - `SESAME_OAUTH__{TENANT}__GOOGLE_CLIENT_SECRET`
    /// - `SESAME_OAUTH__{TENANT}__MICROSOFT_CLIENT_ID`
    /// - `SESAME_OAUTH__{TENANT}__MICROSOFT_CLIENT_SECRET`
    /// - `SESAME_OAUTH__{TENANT}__ALLOWED_REDIRECT_URIS` (comma-separated)
    ///
    /// Example for hauliage:
    /// `SESAME_OAUTH__HAULIAGE__GOOGLE_CLIENT_ID=...`
    #[must_use]
    pub fn for_tenant(tenant_id: &str) -> Self {
        let prefix = tenant_env_prefix(tenant_id);
        let google = tenant_credentials(&prefix, SupportedProvider::Google);
        let microsoft = tenant_credentials(&prefix, SupportedProvider::Microsoft);

        let redirects_key = format!("SESAME_OAUTH__{prefix}__ALLOWED_REDIRECT_URIS");
        let allowed_redirect_uris = std::env::var(&redirects_key)
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToString::to_string)
            .collect();

        Self {
            tenant_id: tenant_id.to_string(),
            google,
            microsoft,
            allowed_redirect_uris,
        }
    }

    /// Credentials for a supported provider, if configured for this tenant.
    #[must_use]
    pub fn credentials_for(&self, provider: SupportedProvider) -> Option<&ProviderCredentials> {
        match provider {
            SupportedProvider::Google => self.google.as_ref(),
            SupportedProvider::Microsoft => self.microsoft.as_ref(),
        }
    }

    /// Returns true when the redirect URI is allowed for this tenant.
    #[must_use]
    pub fn redirect_uri_allowed(&self, redirect_uri: &str) -> bool {
        if self.allowed_redirect_uris.is_empty() {
            // Dev convenience when tenant allowlist unset: localhost / .local only.
            return is_dev_redirect_uri(redirect_uri);
        }
        self.allowed_redirect_uris.contains(redirect_uri)
    }

    /// True when at least one provider is configured for this tenant.
    #[must_use]
    pub fn any_provider_configured(&self) -> bool {
        self.google.is_some() || self.microsoft.is_some()
    }
}

/// Normalize tenant id for env var segment: `hauliage` → `HAULIAGE`, `price-whisperer` → `PRICE_WHISPERER`.
fn tenant_env_prefix(tenant_id: &str) -> String {
    tenant_id
        .trim()
        .chars()
        .map(|c| {
            if c == '-' {
                '_'
            } else {
                c.to_ascii_uppercase()
            }
        })
        .collect()
}

fn tenant_credentials(prefix: &str, provider: SupportedProvider) -> Option<ProviderCredentials> {
    let id_key = format!(
        "SESAME_OAUTH__{prefix}__{}_CLIENT_ID",
        provider.env_suffix()
    );
    let secret_key = format!(
        "SESAME_OAUTH__{prefix}__{}_CLIENT_SECRET",
        provider.env_suffix()
    );
    let client_id = std::env::var(&id_key)
        .ok()
        .filter(|s| !s.trim().is_empty())?;
    let client_secret = std::env::var(&secret_key)
        .ok()
        .filter(|s| !s.trim().is_empty())?;
    Some(ProviderCredentials {
        client_id,
        client_secret,
    })
}

/// Dev-only redirect URI heuristic when tenant allowlist is unset.
#[must_use]
pub fn is_dev_redirect_uri(uri: &str) -> bool {
    uri.starts_with("http://localhost")
        || uri.starts_with("http://127.0.0.1")
        || uri.contains(".local")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_provider_names() {
        assert_eq!(
            SupportedProvider::parse("Google"),
            Some(SupportedProvider::Google)
        );
        assert_eq!(
            SupportedProvider::parse("MICROSOFT"),
            Some(SupportedProvider::Microsoft)
        );
        assert_eq!(SupportedProvider::parse("github"), None);
    }

    #[test]
    fn tenant_env_prefix_normalizes_slug() {
        assert_eq!(tenant_env_prefix("hauliage"), "HAULIAGE");
        assert_eq!(tenant_env_prefix("price-whisperer"), "PRICE_WHISPERER");
    }

    #[test]
    fn dev_redirect_uri_heuristic() {
        assert!(is_dev_redirect_uri("http://localhost:7174/oauth/callback"));
        assert!(is_dev_redirect_uri(
            "http://hauliage.dev.microscaler.local/oauth/callback"
        ));
        assert!(!is_dev_redirect_uri("https://evil.example/callback"));
    }

    #[test]
    fn tenant_configs_are_independent() {
        std::env::set_var(
            "SESAME_OAUTH__HAULIAGE__GOOGLE_CLIENT_ID",
            "haulier-google-id",
        );
        std::env::set_var(
            "SESAME_OAUTH__HAULIAGE__GOOGLE_CLIENT_SECRET",
            "haulier-google-secret",
        );
        std::env::set_var(
            "SESAME_OAUTH__PRICEWHISPERER__GOOGLE_CLIENT_ID",
            "pw-google-id",
        );
        std::env::set_var(
            "SESAME_OAUTH__PRICEWHISPERER__GOOGLE_CLIENT_SECRET",
            "pw-google-secret",
        );

        let hauliage = TenantOAuthConfig::for_tenant("hauliage");
        let pw = TenantOAuthConfig::for_tenant("pricewhisperer");

        assert_eq!(
            hauliage
                .credentials_for(SupportedProvider::Google)
                .map(|c| c.client_id.as_str()),
            Some("haulier-google-id")
        );
        assert_eq!(
            pw.credentials_for(SupportedProvider::Google)
                .map(|c| c.client_id.as_str()),
            Some("pw-google-id")
        );

        std::env::remove_var("SESAME_OAUTH__HAULIAGE__GOOGLE_CLIENT_ID");
        std::env::remove_var("SESAME_OAUTH__HAULIAGE__GOOGLE_CLIENT_SECRET");
        std::env::remove_var("SESAME_OAUTH__PRICEWHISPERER__GOOGLE_CLIENT_ID");
        std::env::remove_var("SESAME_OAUTH__PRICEWHISPERER__GOOGLE_CLIENT_SECRET");
    }
}
