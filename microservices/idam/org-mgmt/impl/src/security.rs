/// Security provider initialization for the service.
///
/// Loads per-scheme configuration from `config.yaml` and registers
/// `JwksBearerProvider` / tenant CS API-key providers with the `AppService`.
use std::collections::HashMap;
use std::sync::Arc;

use brrtrouter::security::{JwksBearerProvider, JwtAlgorithm, SecurityProvider, SecurityRequest};
use brrtrouter::server::AppService;
use brrtrouter::spec::SecurityScheme;

use sesame_common::{config::AppConfig, SesameTokenStatusChecker};

use sesame_idam_org_mgmt::tenant_cs_auth::{self, TENANT_CS_HEADER};

/// Static API key provider (dev / M2M).
struct StaticApiKeyProvider {
    key: String,
}

impl SecurityProvider for StaticApiKeyProvider {
    fn validate(&self, scheme: &SecurityScheme, _scopes: &[String], req: &SecurityRequest) -> bool {
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

/// Tenant-bound CS key: `X-Tenant-CS-Key` must match `SESAME_TENANT_CS_KEYS[X-Tenant-ID]`.
struct TenantCsKeyProvider {
    keys: HashMap<String, String>,
}

impl SecurityProvider for TenantCsKeyProvider {
    fn validate(&self, scheme: &SecurityScheme, _scopes: &[String], req: &SecurityRequest) -> bool {
        let SecurityScheme::ApiKey { name, location, .. } = scheme else {
            return false;
        };
        if location != "header" {
            return false;
        }
        let presented = req.get_header(name).or_else(|| req.get_header(TENANT_CS_HEADER));
        let tenant = req
            .get_header("X-Tenant-ID")
            .or_else(|| req.get_header("x-tenant-id"));
        tenant_cs_auth::validate_tenant_cs_key(&self.keys, tenant, presented).is_ok()
    }
}

/// Initialize security providers from the application configuration.
///
/// # Errors
///
/// Returns an error string if JWKS token-status checker setup fails.
#[allow(clippy::unnecessary_wraps)]
pub fn init_security(
    service: &mut AppService,
    app_config: &AppConfig,
) -> std::result::Result<(), String> {
    let sec_cfg = app_config.security.as_ref();
    let schemes = service.security_schemes.clone();

    for (scheme_name, scheme) in schemes {
        match &scheme {
            SecurityScheme::ApiKey { .. } if scheme_name == "TenantCsAuth" => {
                match tenant_cs_auth::load_tenant_cs_keys_from_env() {
                    Ok(keys) => {
                        println!(
                            "[auth] register TenantCsKeyProvider scheme={} tenants={}",
                            scheme_name,
                            keys.len()
                        );
                        service.register_security_provider(
                            &scheme_name,
                            Arc::new(TenantCsKeyProvider { keys }),
                        );
                    }
                    Err(_) => {
                        // Register a provider that always fails closed so routes exist
                        // but return 401 until keys are configured.
                        println!(
                            "[auth] TenantCsAuth unconfigured — registering reject-all provider"
                        );
                        service.register_security_provider(
                            &scheme_name,
                            Arc::new(TenantCsKeyProvider {
                                keys: HashMap::new(),
                            }),
                        );
                    }
                }
            }
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
            SecurityScheme::Http { scheme: http_scheme, .. }
                if http_scheme.eq_ignore_ascii_case("bearer") =>
            {
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
