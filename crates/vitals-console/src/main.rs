//! The vitals-console HTTP server binary.
//!
//! Reads `VITALS_CONSOLE_BIND`, `VITALSD_SOCKET`, `DATABASE_URL` (read API)
//! and `VITALS_CONSOLE_DATABASE_URL` (write API) from the environment, builds
//! the application state, and serves the console routes.

use axum::{Router, routing::get, routing::post};
use std::env;
use std::path::PathBuf;
use tower_http::services::{ServeDir, ServeFile};
use vitals_console::api::clock::clock_handler;
use vitals_console::api::display::display_handler;
use vitals_console::api::host::host_handler;
use vitals_console::api::ops::SysOps;
use vitals_console::api::postgres::{latest_handler, nightly_handler};
use vitals_console::api::query::query_handler;
use vitals_console::api::recovery::recovery_handler;
use vitals_console::api::services::services_handler;
use vitals_console::handlers::{
    AppState, attempts_handler, envelope_handler, health_handler, plaintext_test_handler,
    register_handler, test_page_handler, version_handler,
};

#[tokio::main]
async fn main() {
    let bind_addr =
        env::var("VITALS_CONSOLE_BIND").unwrap_or_else(|_| "127.0.0.1:8480".to_string());
    let socket_path =
        env::var("VITALSD_SOCKET").unwrap_or_else(|_| "/run/vitals/vitalsd.sock".to_string());
    let test_routes_enabled = env::var("VITALS_CONSOLE_TEST_ROUTES")
        .map(|v| v == "1")
        .unwrap_or(false);
    let assets_path = env::var("VITALS_CONSOLE_ASSETS")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("crates/vitals-console/assets"));

    // Write-API pool: the service must still serve every read without it.
    let db = match env::var("VITALS_CONSOLE_DATABASE_URL") {
        Ok(url) => match sqlx::PgPool::connect(&url).await {
            Ok(pool) => {
                eprintln!("vitals-console: write-API database connected");
                Some(pool)
            }
            Err(e) => {
                eprintln!("vitals-console: write-API database connection failed: {e}");
                None
            }
        },
        Err(_) => {
            eprintln!(
                "vitals-console: VITALS_CONSOLE_DATABASE_URL unset; envelope/attempts serve a named 503"
            );
            None
        }
    };

    let db_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "host=/var/run/postgresql user=vitals dbname=vitals".to_string());

    let state = AppState {
        socket_path: PathBuf::from(&socket_path),
        db,
        test_routes_enabled,
        db_url: db_url.clone(),
        sys_ops: SysOps::real(),
        db_ops: std::sync::Arc::new(vitals_console::api::postgres::PgDbOps { db_url }),
    };

    println!("Starting vitals-console on {bind_addr} (vitalsd socket: {socket_path})");
    if test_routes_enabled {
        println!("WARNING: test routes enabled (VITALS_CONSOLE_TEST_ROUTES=1)");
    }

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
        .route("/api/envelope/register", post(register_handler))
        .route("/api/envelope", post(envelope_handler))
        .route("/api/attempts", get(attempts_handler))
        .route("/test", get(test_page_handler))
        .route("/api/envelope/plaintext-test", post(plaintext_test_handler))
        .nest_service(
            "/console",
            ServeDir::new(&assets_path)
                .append_index_html_on_directories(true)
                .fallback(ServeFile::new(assets_path.join("index.html"))),
        )
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
