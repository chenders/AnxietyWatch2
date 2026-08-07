//! The vitals-console HTTP server binary.
//!
//! Reads `VITALS_CONSOLE_BIND`, `VITALSD_SOCKET`, and `DATABASE_URL` from the
//! environment, builds the application state, and serves the console routes
//! defined in `vitals_console`.

use axum::{Router, routing::get};
use std::env;
use std::path::PathBuf;
use vitals_console::api::clock::clock_handler;
use vitals_console::api::display::display_handler;
use vitals_console::api::host::host_handler;
use vitals_console::api::ops::SysOps;
use vitals_console::api::postgres::{latest_handler, nightly_handler};
use vitals_console::api::query::query_handler;
use vitals_console::api::recovery::recovery_handler;
use vitals_console::api::services::services_handler;
use vitals_console::handlers::{AppState, health_handler, version_handler};

#[tokio::main]
async fn main() {
    let bind_addr =
        env::var("VITALS_CONSOLE_BIND").unwrap_or_else(|_| "127.0.0.1:8480".to_string());
    let socket_path =
        env::var("VITALSD_SOCKET").unwrap_or_else(|_| "/run/vitals/vitalsd.sock".to_string());
    let db_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "host=/var/run/postgresql user=vitals dbname=vitals".to_string());

    let state = AppState {
        socket_path: PathBuf::from(&socket_path),
        db_url: db_url.clone(),
        sys_ops: SysOps::real(),
        db_ops: std::sync::Arc::new(vitals_console::api::postgres::PgDbOps { db_url }),
    };

    println!("Starting vitals-console on {bind_addr} (vitalsd socket: {socket_path})");

    let app = Router::new()
        .route("/api/health", get(health_handler))
        .route("/api/version", get(version_handler))
        .route("/api/query", get(query_handler))
        .route("/api/latest", get(latest_handler))
        .route("/api/nightly", get(nightly_handler))
        .route("/api/services", get(services_handler))
        .route("/api/host", get(host_handler))
        .route("/api/display", get(display_handler))
        .route("/api/recovery", get(recovery_handler))
        .route("/api/clock", get(clock_handler))
        .with_state(state);

    let listener = match tokio::net::TcpListener::bind(&bind_addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("vitals-console: cannot bind {bind_addr}: {e}");
            std::process::exit(64);
        }
    };
    if let Err(e) = axum::serve(listener, app).await {
        eprintln!("vitals-console: server exited: {e}");
        std::process::exit(70);
    }
}
