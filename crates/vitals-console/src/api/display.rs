use crate::api::ops::SysOps;
use crate::handlers::AppState;
use axum::{Json, extract::State, http::StatusCode};
use serde::Serialize;

/// One DRM connector's name and link state.
#[derive(Serialize)]
pub struct ConnectorStatus {
    /// Connector name, e.g. `HDMI-A-1`.
    pub name: String,
    /// Connector link state, e.g. `connected` or `disconnected`.
    pub status: String,
}

/// Display stack state: compositor, browser, Wayland socket, and DRM outputs.
#[derive(Serialize)]
pub struct DisplayInfo {
    /// Whether the `cage` compositor process is running.
    pub compositor_alive: bool,
    /// Whether a `chromium` process is running.
    pub browser_alive: bool,
    /// Whether any `/run/user/*/wayland-*` socket exists.
    pub wayland_socket_exists: bool,
    /// One entry per DRM connector found under `/sys/class/drm`.
    pub drm_connectors: Vec<ConnectorStatus>,
}

/// Collect display stack state from processes, sockets, and sysfs.
///
/// # Errors
/// Returns `"not available on this host"` on non-Linux hosts (the handler
/// maps that to 501), or the underlying `stat_vfs` error for other failures.
pub async fn collect_display_info(ops: &SysOps) -> Result<DisplayInfo, String> {
    let stat_res = (ops.stat_vfs)("/");
    if let Err(e) = stat_res
        && e == "not available on this host"
    {
        return Err(e.clone());
    }

    let compositor_alive = (ops.cmd)("pgrep", &["cage"])
        .await
        .is_ok_and(|out| !out.trim().is_empty());
    let browser_alive = (ops.cmd)("pgrep", &["chromium"])
        .await
        .is_ok_and(|out| !out.trim().is_empty());

    let wayland_socket_exists = (ops.glob_exists)("/run/user/*/wayland-*");

    let mut drm_connectors = Vec::new();
    if let Ok(paths) = (ops.glob_paths)("/sys/class/drm/card*-*/status") {
        for path in paths {
            if let Some(name) = path.split('/').nth(4) {
                let status = (ops.read_string)(&path)
                    .await
                    .unwrap_or_else(|_| "unknown".to_string())
                    .trim()
                    .to_string();
                drm_connectors.push(ConnectorStatus {
                    name: name.to_string(),
                    status,
                });
            }
        }
    }

    Ok(DisplayInfo {
        compositor_alive,
        browser_alive,
        wayland_socket_exists,
        drm_connectors,
    })
}

/// GET /api/display
///
/// Serves the display stack state.
///
/// # Errors
/// 501 when the host cannot report display state (non-Linux); 500 on other
/// collection failures.
pub async fn display_handler(
    State(state): State<AppState>,
) -> Result<Json<DisplayInfo>, (StatusCode, String)> {
    collect_display_info(&state.sys_ops)
        .await
        .map(Json)
        .map_err(|e| {
            if e == "not available on this host" {
                (StatusCode::NOT_IMPLEMENTED, e)
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, e)
            }
        })
}
