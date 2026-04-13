use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use tokio::sync::Mutex;

use crate::state::{
    BackendApiSchema, MutationGuard, OperatorInboxSnapshot, QueueClaimRequest, QueueItem,
    QueueItemCreateRequest, QueueTransitionRequest, SyncInboxReport, WebBackendStore,
};

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<Mutex<WebBackendStore>>,
    pub mutation_guard: MutationGuard,
}

pub fn app(store: WebBackendStore) -> Router {
    let mutation_guard = store.mutation_guard().unwrap_or_else(|error| MutationGuard {
        allowed: false,
        reason: format!("failed to evaluate auth boundary policy: {error}"),
        required_ack_header: None,
        policy_loaded: false,
        policy_source: "error".into(),
    });
    let state = AppState {
        store: Arc::new(Mutex::new(store)),
        mutation_guard,
    };

    Router::new()
        .route("/healthz", get(healthz))
        .route("/v1/schema", get(schema))
        .route("/v1/state", get(state_snapshot))
        .route("/v1/queue", get(queue))
        .route("/v1/queue/items", post(create_queue_item))
        .route("/v1/queue/items/:id/claim", post(claim_queue_item))
        .route("/v1/queue/items/:id/transition", post(transition_queue_item))
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
    headers: HeaderMap,
) -> Result<Json<SyncInboxReport>, (StatusCode, String)> {
    require_mutation_permission(&state.mutation_guard, &headers)?;
    let store = state.store.lock().await;
    let report = store.sync_web_approval_inbox().map_err(map_store_error)?;
    Ok(Json(report))
}

async fn create_queue_item(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<QueueItemCreateRequest>,
) -> Result<(StatusCode, Json<QueueItem>), (StatusCode, String)> {
    require_mutation_permission(&state.mutation_guard, &headers)?;
    let store = state.store.lock().await;
    let item = store.create_queue_item(request).map_err(map_store_error)?;
    Ok((StatusCode::CREATED, Json(item)))
}

async fn claim_queue_item(
    Path(item_id): Path<String>,
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<QueueClaimRequest>,
) -> Result<Json<QueueItem>, (StatusCode, String)> {
    require_mutation_permission(&state.mutation_guard, &headers)?;
    let store = state.store.lock().await;
    let item = store
        .claim_queue_item(&item_id, request)
        .map_err(map_store_error)?;
    Ok(Json(item))
}

async fn transition_queue_item(
    Path(item_id): Path<String>,
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<QueueTransitionRequest>,
) -> Result<Json<QueueItem>, (StatusCode, String)> {
    require_mutation_permission(&state.mutation_guard, &headers)?;
    let store = state.store.lock().await;
    let item = store
        .transition_queue_item(&item_id, request)
        .map_err(map_store_error)?;
    Ok(Json(item))
}

fn require_mutation_permission(
    guard: &MutationGuard,
    headers: &HeaderMap,
) -> Result<(), (StatusCode, String)> {
    if !guard.allowed {
        return Err((StatusCode::FORBIDDEN, guard.reason.clone()));
    }
    if let Some(required_header) = &guard.required_ack_header {
        let Some(value) = headers.get(required_header) else {
            return Err((
                StatusCode::FORBIDDEN,
                format!(
                    "missing required local mutation acknowledgment header: {required_header}"
                ),
            ));
        };
        if value.to_str().ok().map(str::trim).filter(|v| !v.is_empty()).is_none() {
            return Err((
                StatusCode::FORBIDDEN,
                format!(
                    "local mutation acknowledgment header must be non-empty: {required_header}"
                ),
            ));
        }
    }
    Ok(())
}

fn internal_error(error: crate::state::StoreError) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}

fn map_store_error(error: crate::state::StoreError) -> (StatusCode, String) {
    match error {
        crate::state::StoreError::Validation(message) => (StatusCode::BAD_REQUEST, message),
        crate::state::StoreError::NotFound(message) => (StatusCode::NOT_FOUND, message),
        crate::state::StoreError::Conflict(message) => (StatusCode::CONFLICT, message),
        other => internal_error(other),
    }
}

#[cfg(test)]
mod tests {
    use axum::body::{to_bytes, Body};
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

    fn write_local_mutation_policy(root: &std::path::Path) {
        let policy_path = root.join(".claw/backend/web-operator-auth-policy.json");
        std::fs::create_dir_all(policy_path.parent().unwrap()).unwrap();
        std::fs::write(
            &policy_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "policyKind": "claw.web-operator-auth-boundary",
                "schemaVersion": 1,
                "backendEnabled": false,
                "deploymentMode": "static-only",
                "anonymousReadAllowed": false,
                "mutationRoutesEnabled": false,
                "sessionCookiesSupported": false,
                "directInternetExposureAllowed": false,
                "trustedProxy": {
                    "required": true,
                    "identityHeaders": ["x-forwarded-user"],
                    "allowClientSuppliedIdentityHeaders": false
                },
                "localOperatorMutations": {
                    "enabled": true,
                    "requireLoopbackBind": true,
                    "requiredAckHeader": "x-claw-local-operator"
                },
                "notes": ["local-only", "no real auth"]
            }))
            .unwrap(),
        )
        .unwrap();
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
    async fn denies_queue_mutations_without_explicit_policy() {
        let root = temp_workspace("create-denied");
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
                    "kind": "review"
                })
                .to_string(),
            ))
            .unwrap();

        let response = app.oneshot(request).await.expect("queue create response");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert!(String::from_utf8(body.to_vec())
            .unwrap()
            .contains("disabled by default"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn can_create_queue_items_via_http_with_explicit_local_policy_and_ack_header() {
        let root = temp_workspace("create-allowed");
        write_local_mutation_policy(&root);
        let app = app(WebBackendStore::new(
            StorePaths::from_workspace_root(&root),
            "127.0.0.1:8787",
        ));

        let request = Request::builder()
            .method(Method::POST)
            .uri("/v1/queue/items")
            .header("content-type", "application/json")
            .header("x-claw-local-operator", "ack")
            .body(Body::from(
                serde_json::json!({
                    "title": "Inspect review bundle",
                    "kind": "review"
                })
                .to_string(),
            ))
            .unwrap();

        let response = app.oneshot(request).await.expect("queue create response");
        assert_eq!(response.status(), StatusCode::CREATED);
        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn denies_sync_without_required_ack_header_even_when_policy_loaded() {
        let root = temp_workspace("sync-missing-header");
        write_local_mutation_policy(&root);
        let approvals_dir = root.join(".claw/web-approvals");
        std::fs::create_dir_all(&approvals_dir).unwrap();
        std::fs::write(
            approvals_dir.join("inbox-state.json"),
            serde_json::json!({"entries": []}).to_string(),
        )
        .unwrap();
        let app = app(WebBackendStore::new(
            StorePaths::from_workspace_root(&root),
            "127.0.0.1:8787",
        ));

        let request = Request::builder()
            .method(Method::POST)
            .uri("/v1/operator/inbox/sync")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.expect("sync response");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert!(String::from_utf8(body.to_vec())
            .unwrap()
            .contains("x-claw-local-operator"));
        let _ = std::fs::remove_dir_all(root);
    }
}
