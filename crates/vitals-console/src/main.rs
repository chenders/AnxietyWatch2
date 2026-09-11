//! The vitals-console HTTP server binary.
//!
//! Reads `VITALS_CONSOLE_BIND`, `VITALSD_SOCKET`, `DATABASE_URL` (read API),
//! `VITALS_CONSOLE_DATABASE_URL` (write API), the private-network allowlist,
//! and the external console-admin capability from the environment, builds the
//! application state, and serves the console routes.

use axum::{Router, middleware, routing::get, routing::post};
use std::path::PathBuf;
use std::{env, net::SocketAddr, sync::Arc};
use tower_http::services::{ServeDir, ServeFile};
use vitals_console::api::air11::{
    air11_action_handler, air11_handler, air11_network_guard, default_allowed_networks,
    load_admin_token, parse_allowed_networks,
};
use vitals_console::api::clock::clock_handler;
use vitals_console::api::display::display_handler;
use vitals_console::api::host::host_handler;
use vitals_console::api::ops::SysOps;
use vitals_console::api::postgres::{latest_handler, nightly_handler};
use vitals_console::api::query::query_handler;
use vitals_console::api::recovery::recovery_handler;
use vitals_console::api::services::services_handler;
use vitals_console::handlers::{
    AppState, attempts_handler, dashboard_handler, envelope_handler, health_handler,
    plaintext_test_handler, register_handler, test_page_handler, version_handler,
};
fn configured_networks() -> Vec<vitals_console::api::air11::AllowedNetwork> {
    match env::var("VITALS_CONSOLE_ALLOWED_NETWORKS") {
        Ok(raw) => match parse_allowed_networks(&raw) {
            Ok(networks) => networks,
            Err(error) => {
                eprintln!("vitals-console: invalid VITALS_CONSOLE_ALLOWED_NETWORKS: {error}");
                std::process::exit(64);
            }
        },
        Err(_) => default_allowed_networks(),
    }
}

fn air11_capabilities() -> (bool, Option<Arc<String>>, Option<Arc<String>>) {
    let actions_enabled = env::var("VITALS_CONSOLE_AIR11_ACTIONS").is_ok_and(|v| v == "1");
    let admin_token = match load_admin_token() {
        Ok(token) => token.map(Arc::new),
        Err(error) => {
            eprintln!("vitals-console: AirSense admin actions disabled: {error}");
            None
        }
    };
    if actions_enabled && admin_token.is_none() {
        eprintln!(
            "vitals-console: AirSense admin actions requested but no external admin capability is available"
        );
    }
    let action_token = if actions_enabled && admin_token.is_some() {
        let mut bytes = [0_u8; 32];
        if let Err(error) = getrandom::getrandom(&mut bytes) {
            eprintln!("vitals-console: cannot create AirSense action capability: {error}");
            std::process::exit(70);
        }
        Some(Arc::new(hex::encode(bytes)))
    } else {
        None
    };
    (actions_enabled, action_token, admin_token)
}

#[expect(
    clippy::too_many_lines,
    reason = "startup wiring keeps network, credential, database, and route policy together"
)]
#[tokio::main]
async fn main() {
    let bind_addr = env::var("VITALS_CONSOLE_BIND").unwrap_or_else(|_| "0.0.0.0:8480".to_string());
    let socket_path =
        env::var("VITALSD_SOCKET").unwrap_or_else(|_| "/run/vitals/vitalsd.sock".to_string());
    let test_routes_enabled = env::var("VITALS_CONSOLE_TEST_ROUTES").is_ok_and(|v| v == "1");
    if test_routes_enabled
        && !(bind_addr.starts_with("127.0.0.1:") || bind_addr.starts_with("[::1]:"))
    {
        eprintln!(
            "vitals-console: test routes require loopback bind; VITALS_CONSOLE_BIND={bind_addr}"
        );
        std::process::exit(64);
    }
    let assets_path = env::var("VITALS_CONSOLE_ASSETS").map_or_else(
        |_| PathBuf::from("crates/vitals-console/assets"),
        PathBuf::from,
    );
    let allowed_networks = configured_networks();
    let (air11_actions_enabled, air11_action_token, air11_admin_token) = air11_capabilities();

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
        db_ops: std::sync::Arc::new(vitals_console::api::postgres::PgDbOps { db_url }),
        air11_actions_enabled,
        air11_action_token,
        air11_admin_token,
        air11_allowed_networks: Arc::new(allowed_networks),
    };

    println!("Starting vitals-console on {bind_addr} (vitalsd socket: {socket_path})");
    if test_routes_enabled {
        println!("WARNING: test routes enabled (VITALS_CONSOLE_TEST_ROUTES=1)");
    }

    let air11_routes = Router::new()
        .route("/api/air11", get(air11_handler).post(air11_action_handler))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            air11_network_guard,
        ));

    let app = Router::new()
        .merge(vitals_console::client_api::router::<AppState>())
        .merge(air11_routes)
        .route("/dashboard", get(dashboard_handler))
        .route("/dashboard/", get(dashboard_handler))
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
        .route("/api/envelope", post(envelope_handler))
        .route("/api/attempts", get(attempts_handler));

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

    let app = app
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
