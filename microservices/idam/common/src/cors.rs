//! Shared CORS installation for every sesame service (Gate A5).
//!
//! The gen-crate mains carried this wiring but the deployed `impl` binaries
//! never did — meaning NO CORS enforcement in the running services. This
//! module is the single implementation each `impl/src/main.rs` calls.
//!
//! Origin policy is CONFIG, not code:
//! - `cors.origins` in the service's `config.yaml` (dev defaults), overridden
//!   per environment by `CORS_ALLOWED_ORIGINS` (comma-separated exact
//!   origins) — how staging/prod lock to their frontend origins without
//!   touching baked config.
//! - A wildcard (`"*"`) is honoured only for dev; combining it with
//!   `allow_credentials: true` panics at startup (invalid per the CORS spec).

use std::sync::Arc;

use brrtrouter::middleware::{
    build_route_cors_map, CorsMiddleware, CorsMiddlewareBuilder, MetricsMiddleware,
    RouteCorsPolicy,
};
use brrtrouter::spec::RouteMeta;
use http::Method;

use crate::config::AppConfig;

/// Effective origin list: `CORS_ALLOWED_ORIGINS` env (comma-separated) wins;
/// else the config file's `cors.origins`.
fn effective_origins(app_config: &AppConfig) -> Vec<String> {
    if let Ok(v) = std::env::var("CORS_ALLOWED_ORIGINS") {
        let list: Vec<String> = v
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if !list.is_empty() {
            return list;
        }
    }
    app_config
        .cors
        .as_ref()
        .and_then(|c| c.origins.clone())
        .unwrap_or_default()
}

/// Build the CORS middleware from config (+ env override) and the spec's
/// route-level policies. Returns `None` (with a loud log) only when the
/// builder rejects the configuration.
#[must_use]
pub fn build_cors_middleware(
    app_config: &AppConfig,
    routes: &[RouteMeta],
    metrics: &Arc<MetricsMiddleware>,
) -> Option<Arc<CorsMiddleware>> {
    let origins_owned = effective_origins(app_config);
    let origins: Vec<&str> = origins_owned.iter().map(String::as_str).collect();
    if origins.iter().any(|o| *o == "*") {
        tracing::warn!(
            "CORS origins contain a wildcard — dev only; set CORS_ALLOWED_ORIGINS (or cors.origins) to explicit origins before exposure (Gate A5)"
        );
    }

    let cors_cfg = app_config.cors.as_ref();
    let mut builder = CorsMiddlewareBuilder::new();
    if !origins.is_empty() {
        builder = builder.allowed_origins(&origins);
    }
    if let Some(cfg) = cors_cfg {
        if let Some(headers) = cfg.allowed_headers.as_ref() {
            let header_strs: Vec<&str> = headers.iter().map(String::as_str).collect();
            builder = builder.allowed_headers(&header_strs);
        }
        if let Some(methods) = cfg.allowed_methods.as_ref() {
            let method_vec: Vec<Method> = methods
                .iter()
                .filter_map(|m| m.parse::<Method>().ok())
                .collect();
            if !method_vec.is_empty() {
                builder = builder.allowed_methods(&method_vec);
            }
        }
        if let Some(creds) = cfg.allow_credentials {
            builder = builder.allow_credentials(creds);
        }
        if let Some(expose) = cfg.expose_headers.as_ref() {
            let expose_strs: Vec<&str> = expose.iter().map(String::as_str).collect();
            builder = builder.expose_headers(&expose_strs);
        }
        if let Some(age) = cfg.max_age {
            builder = builder.max_age(age);
        }
    }

    match builder.build() {
        Ok(global_cors) => {
            // Merge spec-declared per-route policies with the configured
            // origins (with_origins also validates credential combinations).
            let route_policies = build_route_cors_map(routes);
            let mut merged_policies = std::collections::HashMap::new();
            for (handler_name, policy) in route_policies {
                let merged_policy = match policy {
                    RouteCorsPolicy::Custom(route_config) => {
                        RouteCorsPolicy::Custom(route_config.with_origins(&origins))
                    }
                    other => other,
                };
                merged_policies.insert(handler_name, merged_policy);
            }
            Some(Arc::new(
                CorsMiddleware::with_route_policies(global_cors, merged_policies)
                    .with_metrics_sink(metrics.clone()),
            ))
        }
        Err(e) => {
            tracing::error!(error = ?e, "failed to build CORS middleware — cross-origin requests will NOT be policed");
            None
        }
    }
}
