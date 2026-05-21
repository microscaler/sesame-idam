/// Implementation crate entry point for identity-session-service.
///
/// This service manages JWT signing keys (Ed25519), serves the JWKS endpoint,
/// and handles key rotation lifecycle. It is the authoritative JWKS source for
/// all other Sesame-IDAM consumer services.
///
/// The service is already modularized with:
/// - `key_manager` — JWT signing key generation, rotation, and storage
/// - `controllers` — endpoint handlers (JWKS, admin revoke)
/// - `middleware` — custom middleware (JWKS headers)
/// - `audit` — global audit event emitter
use brrtrouter::typed::spawn_typed_with_stack_size_and_name;
use sesame_idam_identity_session_service_gen::registry;
mod audit;
pub mod config;
mod controllers;
mod jwt;
mod key_manager;
mod middleware;
mod security;

// key_manager module is registered but KeyManager is used via static KEY_MANAGER

use brrtrouter::dispatcher::Dispatcher;

use brrtrouter::middleware::MetricsMiddleware;

use brrtrouter::router::Router;

use brrtrouter::runtime_config::RuntimeConfig;

use brrtrouter::server::AppService;

use brrtrouter::server::HttpServer;
use clap::Parser;
use std::io;
use std::path::PathBuf;

// Use jemalloc as the global allocator for better memory performance.
// This is gated behind the "jemalloc" feature (enabled by default).
// Disable this feature if brrtrouter is providing jemalloc via its own "jemalloc" feature,
// or if you want to use the system allocator: `cargo build --no-default-features`
#[cfg(feature = "jemalloc")]
use tikv_jemallocator::Jemalloc;

#[cfg(feature = "jemalloc")]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

#[derive(Parser)]
struct Args {
    #[arg(short, long, default_value = "./doc/openapi.yaml")]
    spec: PathBuf,
    #[arg(long)]
    static_dir: Option<PathBuf>,
    #[arg(long, default_value = "./doc")]
    doc_dir: PathBuf,
    #[arg(long, default_value_t = false)]
    hot_reload: bool,
    #[arg(long)]
    test_api_key: Option<String>,
    #[arg(long, default_value = "./config/config.yaml")]
    config: PathBuf,
}

fn main() -> io::Result<()> {
    // Initialize structured logging
    if let Err(e) =
        brrtrouter::otel::init_logging_with_config(&brrtrouter::otel::LogConfig::from_env())
    {
        eprintln!("[logging][error] failed to init tracing subscriber: {e}");
    }

    let args = Args::parse();
    // Configure coroutine stack size
    let config = RuntimeConfig::from_env();
    may::config().set_stack_size(config.stack_size);
    may::config().set_workers(config.may_workers);

    // Load OpenAPI spec
    let spec_path = if args.spec.is_relative() {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        base.join(args.spec)
    } else {
        args.spec.clone()
    };

    let spec_str = spec_path.to_str().unwrap_or_else(|| {
        eprintln!("[startup][error] OpenAPI spec path contains invalid UTF-8");
        std::process::exit(1);
    });
    let (routes, schemes, _): (_, _, _) = brrtrouter::spec::load_spec_full(spec_str)
        .unwrap_or_else(|e| {
            eprintln!("[startup][error] failed to load OpenAPI spec: {e}");
            std::process::exit(1);
        });

    let router_arc =
        std::sync::Arc::new(arc_swap::ArcSwap::from_pointee(Router::new(routes.clone())));
    {
        let r = router_arc.load();
        r.dump_routes();
    }

    // Create dispatcher and middleware
    let mut dispatcher = Dispatcher::new();
    let metrics = std::sync::Arc::new(MetricsMiddleware::new());
    dispatcher.add_middleware(metrics.clone());

    // JWKS headers middleware: injects Cache-Control, X-Content-Type-Options, Vary headers
    // on the /.well-known/jwks.json endpoint
    let jwks_headers = std::sync::Arc::new(middleware::jwks_headers::JwksHeadersMiddleware);
    dispatcher.add_middleware(jwks_headers);

    // Create memory tracking middleware
    let memory = std::sync::Arc::new(brrtrouter::middleware::MemoryMiddleware::new());
    brrtrouter::middleware::memory::start_memory_monitor(memory.clone());

    // F5 audit pattern: register_from_spec establishes all gen stubs first,
    // then we override specific routes with impl controllers.
    unsafe {
        registry::register_from_spec(&mut dispatcher, &routes);
        for route in &routes {
            match route.handler_name.as_ref() {
                "jwks" => {
                    let tx = spawn_typed_with_stack_size_and_name(
                        controllers::jwks::JwksController,
                        16384,
                        Some(route.handler_name.as_ref()),
                    );
                    dispatcher.add_route(route.clone(), tx);
                }
                "admin_jwks_revoke" => {
                    let tx = spawn_typed_with_stack_size_and_name(
                        controllers::admin_jwks_revoke::AdminRevokeKeyController,
                        16384,
                        Some(route.handler_name.as_ref()),
                    );
                    dispatcher.add_route(route.clone(), tx);
                }
                _ => {} // fallback to gen stubs for everything else
            }
        }
    }

    let dispatcher = std::sync::Arc::new(arc_swap::ArcSwap::from_pointee(dispatcher));
    let mut service = AppService::new(
        router_arc,
        dispatcher,
        schemes,
        spec_path.clone(),
        args.static_dir.clone(),
        Some(args.doc_dir.clone()),
    );
    service.set_metrics_middleware(metrics);
    service.set_memory_middleware(memory);

    // Load application config and initialize security providers.
    // identity-session-service is the JWKS issuer — it doesn't consume JWTs from
    // itself, so security init is optional. Other consumer services use the
    // same init_security() to validate JWTs against this service's JWKS.
    match config::load_config(&args.config) {
        Ok(app_config) => {
            if app_config.security.is_some() {
                if let Err(e) = security::init_security(&app_config, &mut service) {
                    eprintln!("[config][error] security init failed: {e}");
                    return Err(io::Error::other(format!("Security init failed: {e}")));
                }
            } else {
                println!("[config] no security config; using service defaults");
            }
        }
        Err(e) => {
            eprintln!("[config][error] {e}");
            return Err(io::Error::other(e));
        }
    }

    // Concatenate Lifeguard's prometheus text (DB metrics, pool stats) into
    // BRRTRouter's scrape response so a single /metrics endpoint covers both
    // the HTTP layer and the Postgres layer.
    service.set_extra_prometheus(Some(std::sync::Arc::new(|| {
        lifeguard::metrics::prometheus_scrape_text()
    })));

    // Port selection: PORT env var (K8s) > default 8080
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(8080);
    let addr = if std::env::var("BRRTR_LOCAL").is_ok() {
        format!("127.0.0.1:{port}")
    } else {
        format!("0.0.0.0:{port}")
    };
    println!("🚀 server listening on {addr}");
    let handle = HttpServer(service).start(&addr)?;
    handle
        .run_until_shutdown()
        .map_err(|e| io::Error::other(format!("Server error: {e:?}")))?;

    Ok(())
}
