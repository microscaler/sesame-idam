//! `OAuth2` / OIDC helpers for social login (Google, Microsoft).

pub mod config;
pub mod providers;
pub mod state;

pub use config::{ProviderCredentials, SupportedProvider, TenantOAuthConfig};
pub use providers::{build_authorize_url, exchange_code, ProviderProfile};
pub use state::{consume_oauth_state, store_oauth_state, OAuthState};
