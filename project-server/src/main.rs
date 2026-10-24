//! OpenECraft project server: stores projects for several users. Every page and
//! the project-wide data are locked separately, so users work at the same time on
//! different pages of a project. Runs independently of the editor.
//!
//! ```text
//! openecraft-project-server --bind 0.0.0.0:8091 --data projects --token <secret>
//! ```
//!
//! Without `--token` (or `OPENECRAFT_PROJECT_TOKEN`) and without user accounts
//! everyone who reaches the server can open and change its projects. Users (viewer,
//! editor, administrator; optionally limited to some projects) sign in with a
//! password; `--admin-user` creates the first administrator.

mod routes;
mod sessions;
mod store;
mod users;

use clap::Parser;
use openecraft_core::collab::DEFAULT_PORT;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Parser)]
#[command(version, about = "Project server for OpenECraft: shared projects with page locks")]
struct Args {
    /// Address to listen on.
    #[arg(long, default_value_t = SocketAddr::from(([127, 0, 0, 1], DEFAULT_PORT)))]
    bind: SocketAddr,
    /// Directory of the project files (created if missing).
    #[arg(long, default_value = "projects")]
    data: PathBuf,
    /// Access token required for every request.
    #[arg(long, env = "OPENECRAFT_PROJECT_TOKEN", hide_env_values = true)]
    token: Option<String>,
    /// Creates (or resets) an administrator account with the password from
    /// OPENECRAFT_PROJECT_ADMIN_PASSWORD.
    #[arg(long)]
    admin_user: Option<String>,
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let args = Args::parse();
    let store = match store::Store::open(&args.data) {
        Ok(s) => s,
        Err(e) => {
            log::error!("cannot open project directory: {e}");
            std::process::exit(1);
        }
    };
    log::info!("{} projects loaded from {}", store.projects.len(), args.data.display());
    let mut users = match users::Users::open(&args.data) {
        Ok(u) => u,
        Err(e) => {
            log::error!("cannot read the user accounts: {e}");
            std::process::exit(1);
        }
    };
    if let Some(name) = args.admin_user.filter(|n| !n.trim().is_empty()) {
        let password = std::env::var("OPENECRAFT_PROJECT_ADMIN_PASSWORD").unwrap_or_default();
        if password.is_empty() {
            log::error!("--admin-user needs the password in OPENECRAFT_PROJECT_ADMIN_PASSWORD");
            std::process::exit(1);
        }
        if let Err(e) = users.set_admin(&name, &password).map_err(|e| e.to_string()).and_then(|_| users.save().map_err(|e| e.to_string())) {
            log::error!("cannot store the administrator: {e}");
            std::process::exit(1);
        }
        log::info!("administrator {name} set");
    }
    let token = args.token.filter(|t| !t.trim().is_empty());
    if token.is_none() && users.accounts.is_empty() {
        log::warn!("no access token and no user accounts: everyone who reaches the server can change its projects");
    }
    let server = routes::Server { store, sessions: Default::default(), users };
    let state = routes::AppState { server: Arc::new(Mutex::new(server)), token };
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
