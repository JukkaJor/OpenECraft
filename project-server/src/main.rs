//! OpenECraft project server: stores projects for several users. Every page and
//! the project-wide data are locked separately, so users work at the same time on
//! different pages of a project. Runs independently of the editor.
//!
//! ```text
//! openecraft-project-server --bind 0.0.0.0:8091 --data projects --token <secret>
//! openecraft-project-server --config project-server.toml
//! ```
//!
//! `--config` reads the settings of the options from a TOML file (`settings`,
//! `project-server.example.toml`); options on the command line win.
//!
//! Without `--token` (or `OPENECRAFT_PROJECT_TOKEN`), user accounts and directories
//! everyone who reaches the server can open and change its projects. Users sign in
//! with a local account or with a directory (Active Directory, LDAP) of the access
//! settings (`--access`, default `access.json` in the data directory). Their rights per
//! project come from their built-in role and the grants to them and their groups;
//! `--admin-user` creates the first administrator.

mod routes;
mod sessions;
mod settings;
mod store;
mod users;

use clap::Parser;
use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let args = match settings::Settings::new(settings::Args::parse()) {
        Ok(s) => s,
        Err(e) => {
            log::error!("{e}");
            std::process::exit(1);
        }
    };
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
    if let Some(name) = args.admin_user {
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
    let access = match openecraft_auth::AccessFile::open(&args.access, Default::default()) {
        Ok(a) => a,
        Err(e) => {
            log::error!("cannot read the access settings: {e}");
            std::process::exit(1);
        }
    };
    for d in &access.config.directories {
        log::info!("sign-in with directory {} ({})", d.id, d.url);
    }
    let token = args.token;
    if token.is_none() && users.accounts.is_empty() && access.config.directories.is_empty() {
        log::warn!("no access token, user accounts or directories: everyone who reaches the server can change its projects");
    }
    let server = routes::Server { store, sessions: Default::default(), users, access };
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
