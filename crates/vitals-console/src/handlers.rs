use axum::{Json, extract::State};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use vitals_core::liveness::{Evidence, evaluate};
use vitals_registry::ALL_SOURCES;

use crate::client::get_health;
use crate::models::{DaemonReachable, HealthView, SourceHealthView, VersionView};

#[derive(Clone)]
pub struct AppState {
    pub socket_path: PathBuf,
}

pub async fn health_handler(State(state): State<AppState>) -> Json<HealthView> {
    let daemon_health = get_health(&state.socket_path).await;

    let (vitalsd, uptime, degraded, heartbeat_pulsing, daemon_sources) = match daemon_health {
        Ok(h) => (
            DaemonReachable::Reachable,
            Some(h.uptime_secs),
            h.degraded,
            Some(h.heartbeat_pulsing),
            Some(h.sources),
        ),
        Err(e) => (
            DaemonReachable::Unreachable(e.to_string()),
            None,
            vec![],
            None,
            None,
        ),
    };

    let now_s = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let mut source_views = Vec::new();
    for source in ALL_SOURCES {
        let expected_cadence = source.expected_cadence();
        let core_cadence = match expected_cadence {
            vitals_registry::Cadence::EverySeconds(s) => {
                vitals_core::liveness::Cadence::EverySeconds(s)
            }
            vitals_registry::Cadence::OnDemand => vitals_core::liveness::Cadence::OnDemand,
        };
        let daemon_info = daemon_sources
            .as_ref()
            .and_then(|sources| sources.iter().find(|s| s.source == source.id()));

        let mut evidence = Evidence {
            last_attempt_epoch_s: None,
            last_attempt_failure: None,
            last_success_epoch_s: None,
            inflight: None,
            learned_interval_s: None,
        };

        let mut observation_age_s = None;
        let mut raw_liveness = None;

        if let Some(info) = daemon_info {
            raw_liveness = Some(info.liveness);
            if let Some(age) = info.last_sample_age_secs {
                observation_age_s = Some(age);
                let timestamp = now_s.saturating_sub(age as i64);
                evidence.last_attempt_epoch_s = Some(timestamp);
                evidence.last_success_epoch_s = Some(timestamp);
            }
        }

        let assertion = evaluate(now_s, core_cadence, &evidence);

        let state_str = match assertion.state {
            vitals_core::liveness::HealthState::Live => "Live",
            vitals_core::liveness::HealthState::Failing => "Failing",
            vitals_core::liveness::HealthState::Unknown => "Unknown",
        };

        source_views.push(SourceHealthView {
            id: source.id(),
            name: format!("{:?}", source),
            state: state_str.to_string(),
            reason: assertion.reason,
            degrade_deadline_epoch_s: assertion.degrade_deadline_epoch_s,
            observation_age_s,
            raw_liveness,
        });
    }

    Json(HealthView {
        vitalsd,
        uptime,
        degraded,
        heartbeat_pulsing,
        sources: source_views,
    })
}

pub async fn version_handler() -> Json<VersionView> {
    Json(VersionView {
        version: env!("CARGO_PKG_VERSION"),
        git: option_env!("GIT_DESCRIBE"),
        note: "attempts schema: not yet migrated here",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, body::Body, http::Request as AxumRequest, routing::get};
    use http_body_util::BodyExt; // for `collect`
    use serde_json::Value;
    use std::time::Duration;
    use tempfile::tempdir;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::UnixListener;
    use tower::util::ServiceExt; // for `oneshot`
    use vitals_proto::{Health, Liveness, Response, SourceHealth, check_frame_len, encode_frame};
    use vitals_registry::VitalsSource;

    async fn spawn_stub_daemon(socket_path: PathBuf, canned_health: Health) {
        let listener = UnixListener::bind(&socket_path).unwrap();
        tokio::spawn(async move {
            if let Ok((mut stream, _)) = listener.accept().await {
                let mut len_buf = [0u8; 4];
                if stream.read_exact(&mut len_buf).await.is_ok() {
                    let len = check_frame_len(u32::from_le_bytes(len_buf)).unwrap();
                    let mut payload = vec![0u8; len];
                    if stream.read_exact(&mut payload).await.is_ok() {
                        let resp = Response::Health {
                            health: Box::new(canned_health),
                        };
                        let out = encode_frame(&resp).unwrap();
                        let _ = stream.write_all(&out).await;
                    }
                }
            }
        });
    }

    fn default_health() -> Health {
        Health {
            uptime_secs: 100,
            sources: vec![],
            total_samples: 0,
            spool_bytes: 0,
            lag_bytes: 0,
            lag_age_seconds: 0,
            last_advanced_at: None,
            subscribers: vec![],
            heartbeat: vitals_proto::HeartbeatInputs {
                monitoring_window_active: true,
                primary_sample_age_secs: Some(10),
                tier_evaluation_age_secs: Some(5),
                drain_progressing: true,
            },
            heartbeat_pulsing: true,
            degraded: vec![],
        }
    }

    #[tokio::test]
    async fn test_unreachable_socket() {
        let dir = tempdir().unwrap();
        let socket_path = dir.path().join("vitalsd.sock");

        let app = Router::new()
            .route("/api/health", get(health_handler))
            .with_state(AppState { socket_path });

        let req = AxumRequest::builder()
            .uri("/api/health")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), 200);

        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let json: Value = serde_json::from_slice(&body).unwrap();

        // (d) unreachable socket produces the unreachable marker, not an empty list
        assert!(json["vitalsd"].get("unreachable").is_some());

        let sources = json["sources"].as_array().unwrap();
        assert!(
            !sources.is_empty(),
            "must not return an empty list of sources"
        );
    }

    #[tokio::test]
    async fn test_source_mapping() {
        let dir = tempdir().unwrap();
        let socket_path = dir.path().join("vitalsd.sock");

        let mut health = default_health();
        health.sources = vec![
            SourceHealth {
                source: VitalsSource::Emay.id(),
                liveness: Liveness::Live,
                last_sample_age_secs: Some(10),
                samples: 100,
            },
            SourceHealth {
                source: VitalsSource::PolarH10.id(),
                liveness: Liveness::Stale,
                last_sample_age_secs: Some(1000),
                samples: 50,
            },
            SourceHealth {
                source: VitalsSource::Healthkit.id(),
                liveness: Liveness::NeverSeen,
                last_sample_age_secs: None,
                samples: 0,
            },
        ];

        spawn_stub_daemon(socket_path.clone(), health).await;
        // give the listener a moment to bind
        tokio::time::sleep(Duration::from_millis(50)).await;

        let app = Router::new()
            .route("/api/health", get(health_handler))
            .with_state(AppState { socket_path });

        let req = AxumRequest::builder()
            .uri("/api/health")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), 200);

        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let json: Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["vitalsd"], "reachable");

        let sources = json["sources"].as_array().unwrap();

        // Find Emay (a)
        let emay = sources.iter().find(|s| s["name"] == "Emay").unwrap();
        assert_eq!(emay["state"], "Live");
        assert_eq!(emay["observation_age_s"], 10);
        assert!(emay["degrade_deadline_epoch_s"].is_number());
        assert_eq!(emay["reason"], "10s ago, expect <= 60s"); // (e) reason strings name value + threshold

        // Find PolarH10 (b)
        let polar = sources.iter().find(|s| s["name"] == "PolarH10").unwrap();
        assert_eq!(polar["state"], "Unknown"); // never FAILING
        assert_eq!(polar["observation_age_s"], 1000);
        assert_eq!(polar["reason"], "no attempt for 1000s, expect <= 60s"); // (e)

        // Find Healthkit (b)
        let hk = sources.iter().find(|s| s["name"] == "Healthkit").unwrap();
        assert_eq!(hk["state"], "Unknown");
        assert_eq!(hk["reason"], "never observed");

        // Find AdUa651ble (c) registry source absent from reply
        let ad = sources.iter().find(|s| s["name"] == "AdUa651ble").unwrap();
        assert_eq!(ad["state"], "Unknown");
        assert_eq!(ad["reason"], "never observed");
    }
}
