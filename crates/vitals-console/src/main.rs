//! The vitals-console HTTP server binary.
//!
//! Reads `VITALS_CONSOLE_BIND`, `VITALSD_SOCKET`, `VITALS_CONSOLE_DATABASE_URL`,
//! and `VITALS_CONSOLE_TEST_ROUTES` from the environment, builds the
//! application state, and serves the routes defined in `vitals_console`.

use axum::{Router, routing::get, routing::post};
use std::env;
use std::path::PathBuf;
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
    let test_routes_enabled = env::var("VITALS_CONSOLE_TEST_ROUTES").is_ok_and(|v| v == "1");

    // Try to connect to the database; the service must still serve /api/health
    // without a database (spec M0).
    let db = if let Ok(url) = env::var("VITALS_CONSOLE_DATABASE_URL") {
        match sqlx::PgPool::connect(&url).await {
            Ok(pool) => {
                eprintln!("Connected to database");
                Some(pool)
            }
            Err(e) => {
                eprintln!("Warning: database connection failed: {e}");
                None
            }
        }
    } else {
        eprintln!("Note: VITALS_CONSOLE_DATABASE_URL not set; envelope endpoints will return 503");
        None
    };

    let state = AppState {
        socket_path: PathBuf::from(&socket_path),
        db,
        test_routes_enabled,
    };

    println!("Starting vitals-console on {bind_addr} (vitalsd socket: {socket_path})");
    if test_routes_enabled {
        println!("WARNING: test routes enabled (VITALS_CONSOLE_TEST_ROUTES=1)");
    }

    let app = Router::new()
        .route("/api/health", get(health_handler))
        .route("/api/version", get(version_handler))
        .route("/api/envelope/register", post(register_handler))
        .route("/api/envelope", post(envelope_handler))
        .route("/api/attempts", get(attempts_handler))
        .route("/test", get(test_page_handler))
        .route("/api/envelope/plaintext-test", post(plaintext_test_handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .unwrap_or_else(|e| {
            eprintln!("Failed to bind to {bind_addr}: {e}");
            std::process::exit(1);
        });
    axum::serve(listener, app).await.unwrap_or_else(|e| {
        eprintln!("Server error: {e}");
        std::process::exit(1);
    });
}
