//! Application entry point.
//!
//! Loads configuration, initializes the security chain (`JwksBearerProvider`),
//! registers middleware and handlers, and runs the `BRRTRouter` HTTP server.
//!

// Use jemalloc as the global allocator for better memory performance.
#[cfg(feature = "jemalloc")]
use tikv_jemallocator::Jemalloc;

#[cfg(feature = "jemalloc")]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

// All application modules live in the lib crate (see lib.rs) so the bin,
// tests, and migrator share one compilation of them.
use sesame_common::config::load_config;
use sesame_idam_api_keys::{controllers, security::init_security};

use sesame_idam_api_keys_gen::registry;

use brrtrouter::dispatcher::Dispatcher;
use brrtrouter::middleware::MetricsMiddleware;
use brrtrouter::router::Router;
use brrtrouter::runtime_config::RuntimeConfig;
use brrtrouter::server::AppService;
use brrtrouter::server::HttpServer;
use clap::Parser;
use std::io;
use std::path::PathBuf;

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
                "create_api_key" => {
                    let tx = brrtrouter::typed::spawn_typed_with_stack_size_and_name(
                        controllers::create_api_key::CreateApiKeyController,
                        20480,
                        Some(route.handler_name.as_ref()),
                    );
                    dispatcher.add_route(route.clone(), tx);
                }
                "validate_api_key" => {
                    let tx = brrtrouter::typed::spawn_typed_with_stack_size_and_name(
                        controllers::validate_api_key::ValidateApiKeyController,
                        20480,
                        Some(route.handler_name.as_ref()),
                    );
                    dispatcher.add_route(route.clone(), tx);
                }
                _ => {} // gen stubs serve everything else for now
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

    // Concatenate Lifeguard's prometheus text (DB metrics, pool stats) into
    // BRRTRouter's scrape response so a single /metrics endpoint covers both
    // the HTTP layer and the Postgres layer.
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
