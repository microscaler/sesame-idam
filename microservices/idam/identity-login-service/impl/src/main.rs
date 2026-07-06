// Implementation crate main entry point
// This file is generated as a starting point.
// You can modify this file freely - it will NOT be auto-regenerated.

use sesame_idam_identity_login_service_gen::registry;
// All application modules live in the lib crate (see lib.rs) so BDD tests
// and the migrator can reuse them.
use sesame_idam_identity_login_service::{controllers, jwt, security, services};

use brrtrouter::dispatcher::Dispatcher;
use brrtrouter::typed::spawn_typed_with_stack_size_and_name;

use security::init_security;
use sesame_common::config::load_config;

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

// Application config structs are in config.rs

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

    // Create memory tracking middleware
    let memory = std::sync::Arc::new(brrtrouter::middleware::MemoryMiddleware::new());
    brrtrouter::middleware::memory::start_memory_monitor(memory.clone());

    // Register & Overwrite pattern (hauliage ADR 0001): register all gen
    // stubs first, then overwrite implemented routes with impl controllers.
    unsafe {
        registry::register_from_spec(&mut dispatcher, &routes);
        for route in &routes {
            match route.handler_name.as_ref() {
                "auth_login" => {
                    let tx = spawn_typed_with_stack_size_and_name(
                        controllers::auth_login::AuthLoginController,
                        20480,
                        Some(route.handler_name.as_ref()),
                    );
                    dispatcher.add_route(route.clone(), tx);
                }
                "auth_register" => {
                    let tx = spawn_typed_with_stack_size_and_name(
                        controllers::auth_register::AuthRegisterController,
                        20480,
                        Some(route.handler_name.as_ref()),
                    );
                    dispatcher.add_route(route.clone(), tx);
                }
                _ => {} // gen stubs serve everything else for now
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

    // Load application config for security initialization
    let app_config = load_config(&args.config).map_err(|e| {
        eprintln!("[config][error] {e}");
        io::Error::other(e)
    })?;

    // Initialize security providers from config
    if let Err(e) = init_security(&mut service, &app_config) {
        eprintln!("[auth][error] {e}");
        return Err(io::Error::other(e));
    }

    // Validate JWT TTL configuration at startup (HACK-301: prevent zero-TTL DoS).
    // Load from env vars (with defaults); config.yaml values are optional.
    let jwt_access = app_config
        .jwt
        .as_ref()
        .and_then(|j| j.access_token.as_ref());
    let ttl_config = jwt::ttl::TtlConfig::from_env_and_config(
        jwt_access.and_then(|a| a.normal_ttl_secs),
        jwt_access.and_then(|a| a.elevated_ttl_secs),
        jwt_access.and_then(|a| a.admin_ttl_secs),
        jwt_access.and_then(|a| a.platform_ttl_secs),
        jwt_access.and_then(|a| a.refresh_ttl_days),
    );
    println!(
        "[jwt] TTL config loaded: normal={}s elevated={}s admin={}s platform={}s refresh={}d",
        ttl_config.normal_secs,
        ttl_config.elevated_secs,
        ttl_config.admin_secs,
        ttl_config.platform_secs,
        ttl_config.refresh_days
    );
    jwt::ttl::validate_minimum_ttl(&ttl_config);
    jwt::ttl::validate_refresh_exceeds_access(&ttl_config);

    // Concatenate Lifeguard's prometheus text (DB metrics, pool stats) into
    // BRRTRouter's scrape response so a single /metrics endpoint covers both
    // the HTTP layer and the Postgres layer.
    service.set_extra_prometheus(Some(std::sync::Arc::new(|| {
        lifeguard::metrics::prometheus_scrape_text()
    })));

    // Warm Lifeguard on the main OS thread before may-scheduled HTTP handlers:
    // lazy pool init inside a may coroutine can deadlock the runtime
    // (WorkerPool::new + may_postgres::connect on a may worker).
    let _ = sesame_idam_database::db();

    // Initialize the JWT signer eagerly so a malformed signing key fails at
    // startup rather than on the first login.
    let _ = &*services::token_issuer::SIGNER;

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
