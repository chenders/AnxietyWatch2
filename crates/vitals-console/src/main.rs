//! The vitals-console HTTP server binary.
//!
//! Reads `VITALS_CONSOLE_BIND`, `VITALSD_SOCKET`, `DATABASE_URL` (read API)
//! and `VITALS_CONSOLE_DATABASE_URL` (write API) from the environment, builds
//! the application state, and serves the console routes.

use axum::{Router, middleware, routing::get, routing::post};
use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::services::{ServeDir, ServeFile};
use vitals_console::api::clock::clock_handler;
use vitals_console::api::display::display_handler;
use vitals_console::api::host::host_handler;
use vitals_console::api::ops::SysOps;
use vitals_console::api::ops::service_operation_handler;
use vitals_console::api::postgres::{latest_handler, nightly_handler};
use vitals_console::api::query::query_handler;
use vitals_console::api::recovery::recovery_handler;
use vitals_console::api::services::services_handler;
use vitals_console::handlers::{
    AppState, attempts_handler, dashboard_handler, envelope_handler, health_handler,
    health_mute_handler, operator_override_handler, plaintext_test_handler, register_handler,
    source_declaration_handler, test_page_handler, version_handler,
};
use vitals_console::security::{
    MAX_REQUEST_BODY_BYTES, login_handler, logout_handler, security_handler, security_middleware,
};

#[tokio::main]
#[allow(
    clippy::too_many_lines,
    reason = "startup keeps bind, route, and middleware assembly in one auditable sequence"
)]
async fn main() {
    let bind_addr =
        env::var("VITALS_CONSOLE_BIND").unwrap_or_else(|_| "127.0.0.1:8480".to_string());
    let socket_path =
        env::var("VITALSD_SOCKET").unwrap_or_else(|_| "/run/vitals/vitalsd.sock".to_string());
    let test_routes_enabled = env::var("VITALS_CONSOLE_TEST_ROUTES").is_ok_and(|v| v == "1");
    let loopback_bind = bind_addr.starts_with("127.0.0.1:") || bind_addr.starts_with("[::1]:");
    if !loopback_bind {
        eprintln!(
            "vitals-console: bind blocked: VITALS_CONSOLE_BIND={bind_addr} is not loopback; Caddy/WireGuard is the only production ingress"
        );
        std::process::exit(64);
    }
    let assets_path = env::var("VITALS_CONSOLE_ASSETS").map_or_else(
        |_| PathBuf::from("crates/vitals-console/assets"),
        PathBuf::from,
    );

    // Write-API pool: the service must still serve every read without it.
    let db = if let Ok(url) = env::var("VITALS_CONSOLE_DATABASE_URL") {
        match sqlx::PgPool::connect(&url).await {
            Ok(pool) => {
                eprintln!("vitals-console: write-API database connected");
                Some(pool)
            }
            Err(e) => {
                eprintln!("vitals-console: write-API database connection failed: {e}");
                None
            }
        }
    } else {
        eprintln!(
            "vitals-console: VITALS_CONSOLE_DATABASE_URL unset; envelope/attempts serve a named 503"
        );
        None
    };

    let db_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "host=/var/run/postgresql user=vitals dbname=vitals".to_string());

    let state = AppState {
        socket_path: PathBuf::from(&socket_path),
        db,
        test_routes_enabled,
        db_url: db_url.clone(),
        sys_ops: SysOps::real(),
        host_history: vitals_console::api::host::HostHistory::default(),
        db_ops: std::sync::Arc::new(vitals_console::api::postgres::PgDbOps { db_url }),
        security: std::sync::Arc::new(vitals_console::security::SecurityState::from_env()),
        ops_token: env::var("VITALS_CONSOLE_OPS_TOKEN")
            .ok()
            .filter(|token| !token.is_empty()),
    };

    println!("Starting vitals-console on {bind_addr} (vitalsd socket: {socket_path})");
    if test_routes_enabled {
        println!("WARNING: test routes enabled (VITALS_CONSOLE_TEST_ROUTES=1)");
    }

    let app = Router::new()
        .merge(vitals_console::client_api::router::<AppState>())
        .route("/dashboard", get(dashboard_handler))
        .route("/dashboard/", get(dashboard_handler))
        .route("/api/health", get(health_handler))
        .route("/api/health/mutes", post(health_mute_handler))
        .route("/api/config/declarations", post(source_declaration_handler))
        .route("/api/config/overrides", post(operator_override_handler))
        .route("/api/version", get(version_handler))
        .route("/api/query", get(query_handler))
        .route("/api/latest", get(latest_handler))
        .route("/api/nightly", get(nightly_handler))
        .route("/api/services", get(services_handler))
        .route(
            "/api/services/:unit/:action",
            post(service_operation_handler),
        )
        .route("/api/host", get(host_handler))
        .route("/api/display", get(display_handler))
        .route("/api/recovery", get(recovery_handler))
        .route("/api/clock", get(clock_handler))
        .route("/api/envelope", post(envelope_handler))
        .route("/api/attempts", get(attempts_handler))
        .route("/api/admin/login", post(login_handler))
        .route("/api/admin/logout", post(logout_handler))
        .route("/api/admin/security", get(security_handler));

    // Amendment 09 retires caller-chosen secret registration and plaintext
    // helpers from production. They exist only on an explicitly enabled
    // loopback test listener and never form a mobile compatibility path.
    let app = if test_routes_enabled {
        app.route("/api/envelope/register", post(register_handler))
            .route("/test", get(test_page_handler))
            .route("/api/envelope/plaintext-test", post(plaintext_test_handler))
    } else {
        app
    };

    let security = state.security.clone();
    let mobile_database = state.db.clone();
    let app = app
        .nest_service(
            "/console",
            ServeDir::new(&assets_path)
                .append_index_html_on_directories(true)
                .fallback(ServeFile::new(assets_path.join("index.html"))),
        )
        .with_state(state)
        .layer(axum::Extension(mobile_database))
        .layer(RequestBodyLimitLayer::new(MAX_REQUEST_BODY_BYTES))
        .layer(middleware::from_fn_with_state(
            security,
            security_middleware,
        ));

    let listener = match tokio::net::TcpListener::bind(&bind_addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("vitals-console: cannot bind {bind_addr}: {e}");
            std::process::exit(64);
        }
    };
    if let Err(e) = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    {
        eprintln!("vitals-console: server exited: {e}");
        std::process::exit(70);
    }
}
