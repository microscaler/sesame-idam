//! `dns-probe`: DNS-over-HTTPS lookups of `_sesame-challenge` names for domain verification (story 17.2).
//!
//! Standard `BRRTRouter` bootstrap. The provider list is read and checked at start-up: a deployment with fewer
//! than two distinct operators does not start.

mod controllers;
mod impl_registry;

use brrtrouter::server::{RunAppArgs, RunAppBuilder};
use clap::Parser;
use std::io;
use std::path::PathBuf;

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
    let args = Args::parse();
    if let Err(e) = controllers::lookup_challenge::providers() {
        eprintln!("[startup][error] {e}");
        return Err(io::Error::other(e.to_string()));
    }
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    RunAppBuilder::new()
        .args(RunAppArgs {
            spec: args.spec,
            config: args.config,
            doc_dir: args.doc_dir,
            static_dir: args.static_dir,
            hot_reload: args.hot_reload,
            test_api_key: args.test_api_key,
            manifest_dir,
            default_port: 8080,
            service_name: "dns-probe".into(),
        })
        .register(|dispatcher, routes| unsafe {
            // Base layer answers 501 (never gen example data); impl controllers replace it.
            sesame_common::not_implemented::register_not_implemented(dispatcher, routes)
                .expect("spawn the not-implemented handler");
            impl_registry::register_impl(dispatcher, routes);
        })
        .run()
}
