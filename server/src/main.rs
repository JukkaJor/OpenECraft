//! OpenECraft parts server: hosts a global parts database for all OpenECraft
//! installations (desktop and web). Runs independently of the editor.
//!
//! ```text
//! openecraft-server --bind 0.0.0.0:8090 --data parts-db.json --token <secret>
//! ```
//!
//! Without `--token` (or `OPENECRAFT_TOKEN`) the server is read-only.

mod routes;
mod store;

use clap::Parser;
use openecraft_parts::api::DEFAULT_PORT;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

#[derive(Parser)]
#[command(version, about = "Global parts database server for OpenECraft")]
struct Args {
    /// Address to listen on.
    #[arg(long, default_value_t = SocketAddr::from(([127, 0, 0, 1], DEFAULT_PORT)))]
    bind: SocketAddr,
    /// Database file (created with the example catalog if missing).
    #[arg(long, default_value = "parts-db.json")]
    data: PathBuf,
    /// Access token required for changes. Without a token the server is read-only.
    #[arg(long, env = "OPENECRAFT_TOKEN", hide_env_values = true)]
    token: Option<String>,
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let args = Args::parse();
    let store = match store::Store::open(&args.data) {
        Ok(s) => s,
        Err(e) => {
            log::error!("cannot open parts database: {e}");
            std::process::exit(1);
        }
    };
    log::info!("{} parts loaded from {} (revision {})", store.parts.len(), args.data.display(), store.revision);
    let token = args.token.filter(|t| !t.trim().is_empty());
    if token.is_none() {
        log::warn!("no access token set: the server is read-only");
    }
    let state = routes::AppState { store: Arc::new(RwLock::new(store)), token };
    let listener = match tokio::net::TcpListener::bind(args.bind).await {
        Ok(l) => l,
        Err(e) => {
            log::error!("cannot listen on {}: {e}", args.bind);
            std::process::exit(1);
        }
    };
    log::info!("listening on http://{}", args.bind);
    let shutdown = async {
        tokio::signal::ctrl_c().await.ok();
        log::info!("shutting down");
    };
    if let Err(e) = axum::serve(listener, routes::router(state)).with_graceful_shutdown(shutdown).await {
        log::error!("server error: {e}");
        std::process::exit(1);
    }
}
