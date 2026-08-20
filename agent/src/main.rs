use crate::config::AppConfig;
use crate::ledger::worm::WormLedger;
use crate::registry::store::RegistryStore;
use axum::{routing::get, Json, Router};
use rusqlite::Connection;
use std::sync::Arc;

mod config;
mod decide;
mod ingress;
mod ledger;
mod lineage;
mod normaliser;
mod registry;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
}

async fn health_handler() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok"
    }))
}

async fn ready_handler() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ready"
    }))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = AppConfig::load()?;
    let config_arc = Arc::new(config.clone());

    let conn = Connection::open(&config.fmn_db_path)?;
    let _store = RegistryStore::new(conn)?;
    let _worm = WormLedger::new(std::path::PathBuf::from(&config.fmn_worm_path));

    let state = AppState { config: config_arc };

    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/ready", get(ready_handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&config.fmn_bind_addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    fn test_app() -> Router {
        let config = Arc::new(AppConfig {
            fmn_mode: "shadow".to_string(),
            fmn_db_path: ":memory:".to_string(),
            fmn_worm_path: "test.ledger".to_string(),
            fmn_upstream_base_url: "http://localhost".to_string(),
            fmn_upstream_key: "key".to_string(),
            fmn_bind_addr: "127.0.0.1:0".to_string(),
        });
        let state = AppState { config };

        Router::new()
            .route("/health", get(health_handler))
            .route("/ready", get(ready_handler))
            .with_state(state)
    }

    #[tokio::test]
    async fn health_and_ready_endpoints_return_200() {
        let app = test_app();

        let health_res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(health_res.status(), StatusCode::OK);

        let ready_res = app
            .oneshot(
                Request::builder()
                    .uri("/ready")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(ready_res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        let app = test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json, serde_json::json!({"status": "ok"}));
    }

    #[tokio::test]
    async fn test_ready_endpoint() {
        let app = test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/ready")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json, serde_json::json!({"status": "ready"}));
    }
}
