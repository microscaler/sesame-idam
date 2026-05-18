// Implementation crate main entry point
// This file is generated as a starting point.
// You can modify this file freely - it will NOT be auto-regenerated.

use sesame_idam_identity_login_service_gen::registry;
mod audit;

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

    // Create memory tracking middleware
    let memory = std::sync::Arc::new(brrtrouter::middleware::MemoryMiddleware::new());
    brrtrouter::middleware::memory::start_memory_monitor(memory.clone());

    // Register handlers from generated crate
    unsafe {
        registry::register_from_spec(&mut dispatcher, &routes);
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
