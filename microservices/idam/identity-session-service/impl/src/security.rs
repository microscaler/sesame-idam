/// Security provider initialization for the service.
///
/// Loads per-scheme configuration from `config.yaml` and registers
/// `JwksBearerProvider` instances with the `AppService`.
///
/// This module mirrors the security initialization in `gen/main.rs` so
/// the impl crate uses real JWKS-based validation instead of the mock
/// providers that the generated code ships with.
use std::sync::Arc;

use brrtrouter::security::JwksBearerProvider;
use brrtrouter::server::AppService;

use sesame_common::config::AppConfig;

/// Register security providers for all schemes defined in the app config.
///
/// Iterates over the security schemes from the `OpenAPI` spec and matches them
/// against per-scheme config entries. For schemes with JWKS config, creates a
/// `JwksBearerProvider` with the specified `jwks_url`, `iss`, `aud`,
/// `leeway_secs`, and `cache_ttl_secs`.
///
/// # Errors
///
/// Returns an error string if a provider cannot be created.
pub fn init_security(app_config: &AppConfig, service: &mut AppService) -> Result<(), String> {
    let sec_cfg = app_config.security.as_ref();
    let mut schemes = service.security_schemes.clone();
    for (scheme_name, _scheme) in schemes.drain() {
        // Check for per-scheme JWKS config
        if let Some(jwks_map) = sec_cfg.and_then(|s| s.jwks.as_ref()) {
            if let Some(jwks) = jwks_map.get(&scheme_name) {
                let mut provider = JwksBearerProvider::new(&jwks.jwks_url);
                if let Some(iss) = jwks.iss.as_deref() {
                    provider = provider.issuer(iss);
                }
                if let Some(aud) = jwks.aud.as_deref() {
                    provider = provider.audience(aud);
                }
                if let Some(leeway) = jwks.leeway_secs {
                    provider = provider.leeway(leeway);
                }
                if let Some(ttl) = jwks.cache_ttl_secs {
                    provider = provider.cache_ttl(std::time::Duration::from_secs(ttl));
                }
                println!(
                    "[auth] register JwksBearerProvider scheme={} jwks_url={} iss={:?} aud={:?}",
                    scheme_name, jwks.jwks_url, jwks.iss, jwks.aud
                );
                service.register_security_provider(&scheme_name, Arc::new(provider));
            }
        }
        // Fallback: skip this scheme (no JWKS config defined)
    }

    Ok(())
}
