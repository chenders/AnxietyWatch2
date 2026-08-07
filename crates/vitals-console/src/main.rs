use axum::{Router, routing::get};
use std::env;
use std::path::PathBuf;
use vitals_console::handlers::{AppState, health_handler, version_handler};

#[tokio::main]
async fn main() {
    let bind_addr =
        env::var("VITALS_CONSOLE_BIND").unwrap_or_else(|_| "127.0.0.1:8480".to_string());
    let socket_path =
        env::var("VITALSD_SOCKET").unwrap_or_else(|_| "/run/vitals/vitalsd.sock".to_string());

    let state = AppState {
        socket_path: PathBuf::from(&socket_path),
    };

    println!(
        "Starting vitals-console on {} (vitalsd socket: {})",
        bind_addr, socket_path
    );

    let app = Router::new()
        .route("/api/health", get(health_handler))
        .route("/api/version", get(version_handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&bind_addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
