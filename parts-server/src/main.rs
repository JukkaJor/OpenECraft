//! OpenECraft parts server: the data portal of all OpenECraft installations (desktop
//! and web). Hosts manufacturer catalogs with faceted search, part history and
//! lifecycle, attachments (datasheets, models, macros), users with roles and a web
//! page for browsing. Runs independently of the editor.
//!
//! ```text
//! openecraft-parts-server --bind 0.0.0.0:8090 --data parts-db.sqlite --token <secret>
//! openecraft-parts-server --storage json --data parts-db.json
//! ```
//!
//! Without `--token` (or `OPENECRAFT_TOKEN`) and without user accounts the server is
//! read-only.

mod auth;
mod routes;
mod storage;
mod web;

use clap::Parser;
use openecraft_parts::api::DEFAULT_PORT;
use openecraft_parts::portal::{Role, User};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use storage::{StorageKind, Store};

#[derive(Parser)]
#[command(version, about = "Data portal (global parts database) server for OpenECraft")]
struct Args {
    /// Address to listen on.
    #[arg(long, default_value_t = SocketAddr::from(([127, 0, 0, 1], DEFAULT_PORT)))]
    bind: SocketAddr,
    /// Storage backend. Default: JSON for a `.json` data file, SQLite otherwise.
    #[arg(long, value_enum)]
    storage: Option<StorageKind>,
    /// Database file (created with the example catalog if missing).
    #[arg(long, default_value = "parts-db.sqlite")]
    data: PathBuf,
    /// Copy all data of this database into a new, empty `--data` database first
    /// (e.g. `--migrate-from parts-db.json` when changing to SQLite).
    #[arg(long)]
    migrate_from: Option<PathBuf>,
    /// Administrator token. Without a token changes need user accounts.
    #[arg(long, env = "OPENECRAFT_TOKEN", hide_env_values = true)]
    token: Option<String>,
    /// Create this administrator account (or reset its password) at startup; the
    /// password is read from OPENECRAFT_ADMIN_PASSWORD.
    #[arg(long)]
    admin_user: Option<String>,
}

fn fail(msg: String) -> ! {
    log::error!("{msg}");
    std::process::exit(1);
}

fn migrate(from: &std::path::Path, kind: StorageKind, to: &std::path::Path) {
    use storage::open_backend;
    let source = open_backend(StorageKind::resolve(None, from), from).unwrap_or_else(|e| fail(format!("cannot open {}: {e}", from.display())));
    let mut target = open_backend(kind, to).unwrap_or_else(|e| fail(format!("cannot open {}: {e}", to.display())));
    if target.revision() != 0 {
        fail(format!("{} is not empty: migrate into a new database", to.display()));
    }
    match storage::migrate(source.as_ref(), target.as_mut()) {
        Ok(n) => log::info!("copied {n} parts from {} to {}", from.display(), to.display()),
        Err(e) => fail(format!("migration failed: {e}")),
    }
}

fn add_admin(store: &mut Store, name: &str) {
    let password = std::env::var("OPENECRAFT_ADMIN_PASSWORD").unwrap_or_default();
    if password.is_empty() {
        fail("--admin-user needs the password in OPENECRAFT_ADMIN_PASSWORD".into());
    }
    let mut account = store.account(name).cloned().unwrap_or_default();
    account.user = User { name: name.trim().into(), role: Role::Admin, manufacturer: String::new() };
    account.password = auth::hash_password(&password).unwrap_or_else(|e| fail(e));
    account.sessions.clear();
    if let Err(e) = store.put_account(account) {
        fail(format!("cannot save user {name}: {e}"));
    }
    log::info!("administrator {name} ready");
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let args = Args::parse();
    let kind = StorageKind::resolve(args.storage, &args.data);
    if let Some(from) = &args.migrate_from {
        migrate(from, kind, &args.data);
    }
    let mut store = Store::open(kind, &args.data).unwrap_or_else(|e| fail(format!("cannot open parts database: {e}")));
    if let Some(name) = &args.admin_user {
        add_admin(&mut store, name);
    }
    match store.collect_garbage() {
        Ok(0) => {}
        Ok(n) => log::info!("removed {n} unused files"),
        Err(e) => log::warn!("unused files not removed: {e}"),
    }
    log::info!(
        "{} parts loaded from {} ({}, revision {})",
        store.count().unwrap_or(0),
        args.data.display(),
        store.kind(),
        store.revision
    );
    let token = args.token.filter(|t| !t.trim().is_empty());
    if token.is_none() && store.accounts().is_empty() {
        log::warn!("no access token and no users: the server is read-only");
    }
    let state = routes::AppState { store: Arc::new(Mutex::new(store)), token };
    let listener = tokio::net::TcpListener::bind(args.bind).await.unwrap_or_else(|e| fail(format!("cannot listen on {}: {e}", args.bind)));
    log::info!("listening on http://{}", args.bind);
    let shutdown = async {
        tokio::signal::ctrl_c().await.ok();
        log::info!("shutting down");
    };
    if let Err(e) = axum::serve(listener, routes::router(state)).with_graceful_shutdown(shutdown).await {
        fail(format!("server error: {e}"));
    }
}
