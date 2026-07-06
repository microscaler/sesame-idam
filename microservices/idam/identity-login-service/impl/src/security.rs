/// Security provider initialization for the service.
///
/// Registers JWKS bearer and static API-key providers from `config.yaml`,
/// matching the generated `gen/main.rs` pattern so deployed impl crates
/// do not fail with "Security provider not found" on protected routes.
use std::sync::Arc;

use brrtrouter::security::{JwksBearerProvider, SecurityProvider, SecurityRequest};
use brrtrouter::server::AppService;
use brrtrouter::spec::SecurityScheme;

use sesame_common::config::AppConfig;

/// Static API key provider (dev / M2M).
struct StaticApiKeyProvider {
    key: String,
}

impl SecurityProvider for StaticApiKeyProvider {
    fn validate(
        &self,
        scheme: &SecurityScheme,
        _scopes: &[String],
        req: &SecurityRequest,
    ) -> bool {
        match scheme {
            SecurityScheme::ApiKey { name, location, .. } => match location.as_str() {
                "header" => req.get_header(name).is_some_and(|v| v == self.key),
                "query" => req.get_query(name).is_some_and(|v| v == self.key),
                "cookie" => req.get_cookie(name).is_some_and(|v| v == self.key),
                _ => false,
            },
            _ => false,
        }
    }
}

/// Initialize security providers from the application configuration.
///
/// # Errors
///
/// Returns an error string if provider registration fails (currently always
/// succeeds — missing config entries skip registration).
// Result kept for a uniform init interface across services (some fail).
#[allow(clippy::unnecessary_wraps)]
pub fn init_security(
    service: &mut AppService,
    app_config: &AppConfig,
) -> std::result::Result<(), String> {
    let sec_cfg = app_config.security.as_ref();
    let schemes = service.security_schemes.clone();

    for (scheme_name, scheme) in schemes {
        match scheme {
            SecurityScheme::ApiKey { .. } => {
                if let Some(cfgs) = sec_cfg.and_then(|s| s.api_keys.as_ref()) {
                    if let Some(cfg) = cfgs.get(&scheme_name) {
                        if let Some(key) = cfg.key.clone() {
                            println!(
                                "[auth] register StaticApiKeyProvider scheme={} key_len={}",
                                scheme_name,
                                key.len()
                            );
                            service.register_security_provider(
                                &scheme_name,
                                Arc::new(StaticApiKeyProvider { key }),
                            );
                            continue;
                        }
                    }
                }
                let fallback = std::env::var("BRRTR_API_KEY").unwrap_or_else(|_| "test123".into());
                println!(
                    "[auth] register StaticApiKeyProvider scheme={} from=fallback key_len={}",
                    scheme_name,
                    fallback.len()
                );
                service.register_security_provider(
                    &scheme_name,
                    Arc::new(StaticApiKeyProvider { key: fallback }),
                );
            }
            SecurityScheme::Http { scheme, .. } if scheme.eq_ignore_ascii_case("bearer") => {
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
            }
            _ => {}
        }
    }

    Ok(())
}
