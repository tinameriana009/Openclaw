use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use tokio::sync::Mutex;

use crate::state::{
    BackendApiSchema, OperatorInboxSnapshot, QueueClaimRequest, QueueItem, QueueItemCreateRequest,
    QueueNoteRequest, SyncInboxReport, WebBackendStore,
};

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<Mutex<WebBackendStore>>,

    #[tokio::test]
    async fn can_run_queue_lifecycle_mutations_via_http() {
        let root = temp_workspace("mutations");
        let app = app(WebBackendStore::new(
            StorePaths::from_workspace_root(&root),
            "127.0.0.1:8787",
        ));

        let create = Request::builder()
            .method(Method::POST)
            .uri("/v1/queue/items")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::json!({"title": "Inspect review bundle", "kind": "review"}).to_string()))
            .unwrap();
        let response = app.clone().oneshot(create).await.expect("queue create response");
        assert_eq!(response.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.expect("create body");
        let created: crate::state::QueueItem = serde_json::from_slice(&body).expect("created item");

        for (route, expected_status) in [
            (format!("/v1/queue/items/{}/claim", created.id), "claimed"),
            (format!("/v1/queue/items/{}/unclaim", created.id), "queued"),
            (format!("/v1/queue/items/{}/complete", created.id), "completed"),
            (format!("/v1/queue/items/{}/reopen", created.id), "queued"),
            (format!("/v1/queue/items/{}/drop", created.id), "dropped"),
        ] {
            let payload = if route.ends_with("/claim") {
                serde_json::json!({"claimed_by": "operator-a"})
            } else {
                serde_json::json!({"note": format!("mutation via {}", route)})
            };
            let request = Request::builder()
                .method(Method::POST)
                .uri(&route)
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap();
            let response = app.clone().oneshot(request).await.expect("mutation response");
            assert_eq!(response.status(), StatusCode::OK);
            let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.expect("mutation body");
            let item: crate::state::QueueItem = serde_json::from_slice(&body).expect("mutated item");
            assert_eq!(serde_json::to_value(&item).unwrap().get("status").and_then(serde_json::Value::as_str), Some(expected_status));
        }
        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn rejects_claiming_completed_items_until_reopened() {
        let root = temp_workspace("validation");
        let app = app(WebBackendStore::new(
            StorePaths::from_workspace_root(&root),
            "127.0.0.1:8787",
        ));

        let create = Request::builder()
            .method(Method::POST)
            .uri("/v1/queue/items")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::json!({"title": "Inspect review bundle", "kind": "review"}).to_string()))
            .unwrap();
        let response = app.clone().oneshot(create).await.expect("queue create response");
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.expect("create body");
        let created: crate::state::QueueItem = serde_json::from_slice(&body).expect("created item");

        let complete = Request::builder()
            .method(Method::POST)
            .uri(format!("/v1/queue/items/{}/complete", created.id))
            .header("content-type", "application/json")
            .body(Body::from(serde_json::json!({"note": "done"}).to_string()))
            .unwrap();
        let complete_response = app.clone().oneshot(complete).await.expect("complete response");
        assert_eq!(complete_response.status(), StatusCode::OK);

        let claim = Request::builder()
            .method(Method::POST)
            .uri(format!("/v1/queue/items/{}/claim", created.id))
            .header("content-type", "application/json")
            .body(Body::from(serde_json::json!({"claimed_by": "operator-a"}).to_string()))
            .unwrap();
        let claim_response = app.oneshot(claim).await.expect("claim response");
        assert_eq!(claim_response.status(), StatusCode::BAD_REQUEST);
        let _ = std::fs::remove_dir_all(root);
    }
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
        .route("/v1/queue/items/:id/unclaim", post(unclaim_queue_item))
        .route("/v1/queue/items/:id/complete", post(complete_queue_item))
        .route("/v1/queue/items/:id/drop", post(drop_queue_item))
        .route("/v1/queue/items/:id/reopen", post(reopen_queue_item))
        .route("/v1/operator/inbox", get(operator_inbox))
        .route("/v1/operator/inbox/sync", post(sync_operator_inbox))
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

async fn operator_inbox(
    State(state): State<AppState>,
) -> Result<Json<OperatorInboxSnapshot>, (StatusCode, String)> {
    let store = state.store.lock().await;
    let inbox = store.load_operator_inbox().map_err(internal_error)?;
    Ok(Json(inbox))
}

async fn sync_operator_inbox(
    State(state): State<AppState>,
) -> Result<Json<SyncInboxReport>, (StatusCode, String)> {
    let store = state.store.lock().await;
    let report = store.sync_web_approval_inbox().map_err(map_store_error)?;
    Ok(Json(report))
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

async fn unclaim_queue_item(
    Path(item_id): Path<String>,
    State(state): State<AppState>,
    Json(request): Json<QueueNoteRequest>,
) -> Result<Json<QueueItem>, (StatusCode, String)> {
    let store = state.store.lock().await;
    let item = store
        .unclaim_queue_item(&item_id, request)
        .map_err(map_store_error)?;
    Ok(Json(item))
}

async fn complete_queue_item(
    Path(item_id): Path<String>,
    State(state): State<AppState>,
    Json(request): Json<QueueNoteRequest>,
) -> Result<Json<QueueItem>, (StatusCode, String)> {
    let store = state.store.lock().await;
    let item = store
        .complete_queue_item(&item_id, request)
        .map_err(map_store_error)?;
    Ok(Json(item))
}

async fn drop_queue_item(
    Path(item_id): Path<String>,
    State(state): State<AppState>,
    Json(request): Json<QueueNoteRequest>,
) -> Result<Json<QueueItem>, (StatusCode, String)> {
    let store = state.store.lock().await;
    let item = store
        .drop_queue_item(&item_id, request)
        .map_err(map_store_error)?;
    Ok(Json(item))
}

async fn reopen_queue_item(
    Path(item_id): Path<String>,
    State(state): State<AppState>,
    Json(request): Json<QueueNoteRequest>,
) -> Result<Json<QueueItem>, (StatusCode, String)> {
    let store = state.store.lock().await;
    let item = store
        .reopen_queue_item(&item_id, request)
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
    async fn exposes_health_state_and_operator_inbox_routes() {
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
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/v1/state")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("state response");
        assert_eq!(state.status(), StatusCode::OK);

        let inbox = app
            .oneshot(
                Request::builder()
                    .uri("/v1/operator/inbox")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("inbox response");
        assert_eq!(inbox.status(), StatusCode::OK);
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

    #[tokio::test]
    async fn sync_endpoint_imports_static_operator_inbox() {
        let root = temp_workspace("sync");
        let approvals_dir = root.join(".claw/web-approvals");
        std::fs::create_dir_all(&approvals_dir).unwrap();
        std::fs::write(
            approvals_dir.join("inbox-state.json"),
            serde_json::json!({
                "entries": [{
                    "itemId": "inbox-trace-1",
                    "traceId": "trace-1",
                    "status": "queued",
                    "queueBucket": "ready-to-review",
                    "queueLabel": "Ready to review rerun",
                    "operatorState": "rerun captured for review",
                    "nextStep": "inspect review json",
                    "reviewJsonPath": ".claw/web-approvals/trace-1.review.json",
                    "approvalPacket": ".claw/web-approvals/trace-1.json"
                }]
            })
            .to_string(),
        )
        .unwrap();
        let app = app(WebBackendStore::new(
            StorePaths::from_workspace_root(&root),
            "127.0.0.1:8787",
        ));

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/v1/operator/inbox/sync")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("sync response");
        assert_eq!(response.status(), StatusCode::OK);
        let _ = std::fs::remove_dir_all(root);
    }
}
