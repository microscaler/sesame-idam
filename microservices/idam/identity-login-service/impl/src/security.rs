/// Security provider initialization for the service.
///
/// Registers JWKS bearer and static API-key providers from `config.yaml`,
/// matching the generated `gen/main.rs` pattern so deployed impl crates
/// do not fail with "Security provider not found" on protected routes.
use std::sync::Arc;

use brrtrouter::security::{JwksBearerProvider, JwtAlgorithm};
use brrtrouter::server::AppService;
use brrtrouter::spec::SecurityScheme;

use sesame_common::api_key::register_api_key_scheme;
use sesame_common::{config::AppConfig, SesameTokenStatusChecker};

/// Initialize security providers from the application configuration.
///
/// # Errors
///
/// Returns an error string when an API key is empty, short or a development value, or the token-status checker
/// cannot be built.
pub fn init_security(
    service: &mut AppService,
    app_config: &AppConfig,
) -> std::result::Result<(), String> {
    let sec_cfg = app_config.security.as_ref();
    let schemes = service.security_schemes.clone();

    for (scheme_name, scheme) in schemes {
        match scheme {
            SecurityScheme::ApiKey { .. } => {
                // ADR-015 hardening: keys only from config or the environment, never a built-in fallback.
                let configured = sec_cfg
                    .and_then(|s| s.api_keys.as_ref())
                    .and_then(|cfgs| cfgs.get(&scheme_name))
                    .and_then(|cfg| cfg.key.clone());
                register_api_key_scheme(service, &scheme_name, configured.as_deref())?;
            }
            SecurityScheme::Http { scheme, .. } if scheme.eq_ignore_ascii_case("bearer") => {
                if let Some(jwks_map) = sec_cfg.and_then(|s| s.jwks.as_ref()) {
                    if let Some(jwks) = jwks_map.get(&scheme_name) {
                        let mut provider = JwksBearerProvider::new(&jwks.jwks_url)
                            .allowed_algorithms(&[JwtAlgorithm::EdDSA])
                            .token_status_checker(Arc::new(SesameTokenStatusChecker::from_env()?));
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
            }
            _ => {}
        }
    }

    Ok(())
}
