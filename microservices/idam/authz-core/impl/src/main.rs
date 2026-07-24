//! Application entry point.
//!
//! Loads configuration, initializes the security chain (`JwksBearerProvider`),
//! registers middleware and handlers, and runs the `BRRTRouter` HTTP server.
//!
//! The service is a pure stateless authorization gate — it does not store
//! any state beyond what the JWKS client caches in memory.

// Use jemalloc as the global allocator for better memory performance.
#[cfg(feature = "jemalloc")]
use tikv_jemallocator::Jemalloc;

#[cfg(feature = "jemalloc")]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

mod authz_span_middleware;
use sesame_idam_authz_core::security::init_security;

use sesame_idam_authz_core_gen::registry;
use std::path::{Path, PathBuf};

use brrtrouter::dispatcher::Dispatcher;
use brrtrouter::middleware::MetricsMiddleware;
use brrtrouter::router::Router;
use brrtrouter::runtime_config::RuntimeConfig;
use brrtrouter::server::AppService;
use brrtrouter::server::HttpServer;
use brrtrouter::spec::{RouteMeta, SecurityScheme};
use clap::Parser;
use std::collections::HashMap;

use sesame_common::config::load_config;
/// Command-line arguments.
#[derive(Parser)]
#[command(
    name = "authz-core",
    about = "Stateless authorization gate for Sesame-IDAM"
)]
struct Args {
    /// Path to the `OpenAPI` spec file.
    #[arg(short, long, default_value = "./doc/openapi.yaml")]
    spec: PathBuf,

    /// Directory for static file serving.
    #[arg(long)]
    static_dir: Option<PathBuf>,

    /// Directory for serving the `OpenAPI` documentation.
    #[arg(long, default_value = "./doc")]
    doc_dir: PathBuf,

    /// Enable hot-reload of the `OpenAPI` spec.
    #[arg(long, default_value_t = false)]
    hot_reload: bool,

    /// Test API key (for development).
    #[arg(long)]
    test_api_key: Option<String>,

    /// Path to the application configuration file.
    #[arg(long, default_value = "./config/config.yaml")]
    config: PathBuf,
}

fn main() -> std::io::Result<()> {
    // Initialize structured logging.
    if let Err(e) =
        brrtrouter::otel::init_logging_with_config(&brrtrouter::otel::LogConfig::from_env())
    {
        eprintln!("[logging][error] failed to init tracing subscriber: {e}");
    }

    let args = Args::parse();

    // Configure coroutine stack size from environment.
    let runtime_config = RuntimeConfig::from_env();
    may::config().set_stack_size(runtime_config.stack_size);
    may::config().set_workers(runtime_config.may_workers);

    // Load OpenAPI spec and extract routes.
    let spec_path = resolve_spec_path(&args.spec);
    let (routes, schemes, _) = load_spec(&spec_path);

    // Create the router.
    let router_arc =
        std::sync::Arc::new(arc_swap::ArcSwap::from_pointee(Router::new(routes.clone())));
    {
        let r = router_arc.load();
        r.dump_routes();
    }

    // Build the dispatcher with middleware.
    let mut dispatcher = Dispatcher::new();
    let metrics = std::sync::Arc::new(MetricsMiddleware::new());
    dispatcher.add_middleware(metrics.clone());

    // Authz request tracing middleware.
    let authz_span = std::sync::Arc::new(authz_span_middleware::AuthzSpanMiddleware::new());
    dispatcher.add_middleware(authz_span);

    // Memory tracking middleware with background monitor.
    let memory = std::sync::Arc::new(brrtrouter::middleware::MemoryMiddleware::new());
    brrtrouter::middleware::memory::start_memory_monitor(memory.clone());

    // Register & Overwrite pattern (hauliage ADR 0001): register all gen
    // stubs first, then overwrite implemented routes with impl controllers.
    unsafe {
        registry::register_from_spec(&mut dispatcher, &routes);
        for route in &routes {
            if route.handler_name.as_ref() == "principal_effective" {
                let tx = brrtrouter::typed::spawn_typed_with_stack_size_and_name(
                    sesame_idam_authz_core::controllers::principal_effective::PrincipalEffectiveController,
                    20480,
                    Some(route.handler_name.as_ref()),
                );
                dispatcher.add_route(route.clone(), tx);
            }
        }
    }

    // Load config before the dispatcher freezes — CORS middleware (Gate A5)
    // must be added pre-wrap. cors.origins + CORS_ALLOWED_ORIGINS override.
    let loaded_config = load_config(&args.config);
    if let Ok(ref app_config) = loaded_config {
        if let Some(cors) =
            sesame_common::cors::build_cors_middleware(app_config, &routes, &metrics)
        {
            dispatcher.add_middleware(cors);
        }
    }

    let dispatcher = std::sync::Arc::new(arc_swap::ArcSwap::from_pointee(dispatcher));

    // Build the application service.
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

    // Initialize security providers.
    match loaded_config {
        Ok(app_config) => {
            if app_config.security.is_some() {
                if let Err(e) = init_security(&mut service, &app_config) {
                    eprintln!("[config][error] security init failed: {e}");
                    return Err(std::io::Error::other(format!("Security init failed: {e}")));
                }
            } else {
                println!("[config] no security config; using service defaults");
            }
        }
        Err(e) => {
            eprintln!("[config][error] {e}");
            return Err(std::io::Error::other(e));
        }
    }

    // Inject Lifeguard's prometheus text (DB metrics, pool stats) into
    // BRRTRouter's /metrics scrape response for a unified endpoint.
    service.set_extra_prometheus(Some(std::sync::Arc::new(|| {
        format!(
            "{}\n{}",
            lifeguard::metrics::prometheus_scrape_text(),
            sesame_common::token_status_prometheus_scrape_text()
        )
    })));

    // Warm Lifeguard on the main OS thread before may-scheduled HTTP handlers:
    // lazy pool init inside a may coroutine can deadlock the runtime.
    let _ = sesame_idam_database::db();

    // Port selection: PORT env var (K8s) > default 8080.
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
        .map_err(|e| std::io::Error::other(format!("Server error: {e:?}")))?;

    Ok(())
}

/// Resolve the `OpenAPI` spec path — relative paths are joined to the crate root.
fn resolve_spec_path(spec: &Path) -> PathBuf {
    if spec.is_relative() {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        base.join(spec)
    } else {
        spec.to_path_buf()
    }
}

/// Load and parse the `OpenAPI` spec, exiting on any error.
fn load_spec(spec_path: &Path) -> (Vec<RouteMeta>, HashMap<String, SecurityScheme>, PathBuf) {
    let spec_str = spec_path.to_str().unwrap_or_else(|| {
        eprintln!("[startup][error] OpenAPI spec path contains invalid UTF-8");
        std::process::exit(1);
    });
    let (routes, schemes, _) = brrtrouter::spec::load_spec_full(spec_str).unwrap_or_else(|e| {
        eprintln!("[startup][error] failed to load OpenAPI spec: {e}");
        std::process::exit(1);
    });
    (routes, schemes, spec_path.to_path_buf())
}
