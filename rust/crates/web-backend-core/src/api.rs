use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use tokio::sync::Mutex;

use crate::state::{
    BackendApiSchema, QueueClaimRequest, QueueItem, QueueItemCreateRequest, WebBackendStore,
};

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<Mutex<WebBackendStore>>,
}

pub fn app(store: WebBackendStore) -> Router {
    let state = AppState {
        store: Arc::new(Mutex::new(store)),
    };

    Router::new()
        .route("/healthz", get(healthz))
        .route("/v1/schema", get(schema))
        .route("/v1/state", get(state_snapshot))
        .route("/v1/queue", get(queue))
        .route("/v1/queue/items", post(create_queue_item))
        .route("/v1/queue/items/:id/claim", post(claim_queue_item))
        .with_state(state)
}

async fn healthz() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "ok": true,
        "service": "claw-webd",
        "honestyNote": "Local backend core only. No claim of a full live web product.",
    }))
}

async fn schema(
    State(state): State<AppState>,
) -> Result<Json<BackendApiSchema>, (StatusCode, String)> {
    let store = state.store.lock().await;
    let snapshot = store.snapshot().map_err(internal_error)?;
    Ok(Json(snapshot.schema))
}

async fn state_snapshot(
    State(state): State<AppState>,
) -> Result<Json<crate::state::BackendSnapshot>, (StatusCode, String)> {
    let store = state.store.lock().await;
    let snapshot = store.snapshot().map_err(internal_error)?;
    Ok(Json(snapshot))
}

async fn queue(
    State(state): State<AppState>,
) -> Result<Json<crate::state::OperatorQueue>, (StatusCode, String)> {
    let store = state.store.lock().await;
    let queue = store.load_queue().map_err(internal_error)?;
    Ok(Json(queue))
}

async fn create_queue_item(
    State(state): State<AppState>,
    Json(request): Json<QueueItemCreateRequest>,
) -> Result<(StatusCode, Json<QueueItem>), (StatusCode, String)> {
    let store = state.store.lock().await;
    let item = store.create_queue_item(request).map_err(map_store_error)?;
    Ok((StatusCode::CREATED, Json(item)))
}

async fn claim_queue_item(
    Path(item_id): Path<String>,
    State(state): State<AppState>,
    Json(request): Json<QueueClaimRequest>,
) -> Result<Json<QueueItem>, (StatusCode, String)> {
    let store = state.store.lock().await;
    let item = store
        .claim_queue_item(&item_id, request)
        .map_err(map_store_error)?;
    Ok(Json(item))
}

fn internal_error(error: crate::state::StoreError) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}

fn map_store_error(error: crate::state::StoreError) -> (StatusCode, String) {
    match error {
        crate::state::StoreError::Validation(message) => (StatusCode::BAD_REQUEST, message),
        crate::state::StoreError::NotFound(message) => (StatusCode::NOT_FOUND, message),
        other => internal_error(other),
    }
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Method, Request, StatusCode};
    use tower::util::ServiceExt;

    use crate::state::{StorePaths, WebBackendStore};

    use super::app;

    fn temp_workspace(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "web-backend-core-api-{name}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time")
                .as_millis()
        ))
    }

    #[tokio::test]
    async fn exposes_health_and_state_routes() {
        let root = temp_workspace("routes");
        let app = app(WebBackendStore::new(
            StorePaths::from_workspace_root(&root),
            "127.0.0.1:8787",
        ));

        let health = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/healthz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("health response");
        assert_eq!(health.status(), StatusCode::OK);

        let state = app
            .oneshot(
                Request::builder()
                    .uri("/v1/state")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("state response");
        assert_eq!(state.status(), StatusCode::OK);
        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn can_create_queue_items_via_http() {
        let root = temp_workspace("create");
        let app = app(WebBackendStore::new(
            StorePaths::from_workspace_root(&root),
            "127.0.0.1:8787",
        ));

        let request = Request::builder()
            .method(Method::POST)
            .uri("/v1/queue/items")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "title": "Inspect review bundle",
                    "kind": "review",
                    "source_path": ".demo-artifacts/repo-analysis-demo/run/operator-dashboard.html"
                })
                .to_string(),
            ))
            .unwrap();

        let response = app.oneshot(request).await.expect("queue create response");
        assert_eq!(response.status(), StatusCode::CREATED);
        let _ = std::fs::remove_dir_all(root);
    }
}
