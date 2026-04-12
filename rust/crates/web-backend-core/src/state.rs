use std::fmt::{Display, Formatter};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServiceInfo {
    pub name: String,
    pub version: String,
    pub honesty_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackendApiSchema {
    pub version: String,
    pub endpoints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServiceConfig {
    pub bind_address: String,
    pub storage_root: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackendPaths {
    pub storage_root: String,
    pub queue_file: String,
    pub runtime_bridge_file: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeBridgeSnapshot {
    pub latest_session_id: Option<String>,
    pub latest_session_path: Option<String>,
    pub recent_trace_ids: Vec<String>,
    pub source_file: Option<String>,
    pub generated_at_utc: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OperatorQueue {
    pub schema_version: u64,
    pub revision: u64,
    pub updated_at_utc: String,
    pub items: Vec<QueueItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum QueueItemStatus {
    Queued,
    Claimed,
    InReview,
    HandoffReady,
    Completed,
    Dropped,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QueueItem {
    pub id: String,
    pub title: String,
    pub kind: String,
    pub status: QueueItemStatus,
    pub created_at_utc: String,
    pub claimed_by: Option<String>,
    pub note: Option<String>,
    pub source_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QueueItemCreateRequest {
    pub title: String,
    pub kind: String,
    pub note: Option<String>,
    pub source_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QueueClaimRequest {
    pub claimed_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackendSnapshot {
    pub service: ServiceInfo,
    pub schema: BackendApiSchema,
    pub config: ServiceConfig,
    pub paths: BackendPaths,
    pub runtime_bridge: RuntimeBridgeSnapshot,
    pub queue: OperatorQueue,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImportBundleReport {
    pub workflow: String,
    pub run_dir: String,
    pub imported_at_utc: String,
    pub queue_item_id: String,
    pub queue_status: QueueItemStatus,
    pub runtime_bridge_file: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorePaths {
    pub workspace_root: PathBuf,
    pub storage_root: PathBuf,
    pub queue_file: PathBuf,
    pub runtime_bridge_file: PathBuf,
}

impl StorePaths {
    #[must_use]
    pub fn from_workspace_root(workspace_root: impl Into<PathBuf>) -> Self {
        let workspace_root = workspace_root.into();
        let storage_root = workspace_root.join(".claw").join("backend");
        Self {
            workspace_root: workspace_root.clone(),
            storage_root: storage_root.clone(),
            queue_file: storage_root.join("operator-queue.json"),
            runtime_bridge_file: storage_root.join("runtime-bridge.json"),
        }
    }
}

#[derive(Debug)]
pub enum StoreError {
    Io(io::Error),
    Json(serde_json::Error),
    Validation(String),
    NotFound(String),
}

impl Display for StoreError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::Json(error) => write!(f, "{error}"),
            Self::Validation(message) | Self::NotFound(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for StoreError {}
impl From<io::Error> for StoreError { fn from(value: io::Error) -> Self { Self::Io(value) } }
impl From<serde_json::Error> for StoreError { fn from(value: serde_json::Error) -> Self { Self::Json(value) } }

#[derive(Debug, Clone)]
pub struct WebBackendStore {
    paths: StorePaths,
    bind_address: String,
}

impl WebBackendStore {
    #[must_use]
    pub fn new(paths: StorePaths, bind_address: impl Into<String>) -> Self {
        Self { paths, bind_address: bind_address.into() }
    }

    pub fn ensure_storage(&self) -> Result<(), StoreError> {
        fs::create_dir_all(&self.paths.storage_root)?;
        if !self.paths.queue_file.exists() {
            self.write_queue(&OperatorQueue { schema_version: 1, revision: 0, updated_at_utc: now_utc_string(), items: Vec::new() })?;
        }
        if !self.paths.runtime_bridge_file.exists() {
            fs::write(&self.paths.runtime_bridge_file, serde_json::to_string_pretty(&serde_json::json!({
                "schemaVersion": 1,
                "generatedAtUtc": now_utc_string(),
                "honestyNote": "Local backend snapshot only. This is a bounded runtime bridge, not a claim of a full live web product.",
                "latestSession": null,
                "recentTraces": [],
            }))?)?;
        }
        Ok(())
    }

    pub fn snapshot(&self) -> Result<BackendSnapshot, StoreError> {
        self.ensure_storage()?;
        Ok(BackendSnapshot {
            service: ServiceInfo { name: "claw-webd".to_string(), version: env!("CARGO_PKG_VERSION").to_string(), honesty_note: "Local-only backend foundation. It exposes persisted operator state and runtime snapshots, but does not claim a complete live web app.".to_string() },
            schema: BackendApiSchema { version: "v1".to_string(), endpoints: vec!["/healthz".into(), "/v1/schema".into(), "/v1/state".into(), "/v1/queue".into(), "/v1/queue/items".into(), "/v1/queue/items/:id/claim".into()] },
            config: ServiceConfig { bind_address: self.bind_address.clone(), storage_root: relative_or_absolute(&self.paths.workspace_root, &self.paths.storage_root) },
            paths: BackendPaths { storage_root: relative_or_absolute(&self.paths.workspace_root, &self.paths.storage_root), queue_file: relative_or_absolute(&self.paths.workspace_root, &self.paths.queue_file), runtime_bridge_file: relative_or_absolute(&self.paths.workspace_root, &self.paths.runtime_bridge_file) },
            runtime_bridge: self.load_runtime_bridge()?,
            queue: self.load_queue()?,
        })
    }

    pub fn load_queue(&self) -> Result<OperatorQueue, StoreError> {
        self.ensure_storage()?;
        Ok(serde_json::from_str(&fs::read_to_string(&self.paths.queue_file)?)?)
    }

    pub fn create_queue_item(&self, request: QueueItemCreateRequest) -> Result<QueueItem, StoreError> {
        if request.title.trim().is_empty() { return Err(StoreError::Validation("queue item title must not be empty".into())); }
        if request.kind.trim().is_empty() { return Err(StoreError::Validation("queue item kind must not be empty".into())); }
        let mut queue = self.load_queue()?;
        let item = QueueItem { id: format!("item-{}", unix_timestamp()), title: request.title.trim().to_string(), kind: request.kind.trim().to_string(), status: QueueItemStatus::Queued, created_at_utc: now_utc_string(), claimed_by: None, note: request.note.and_then(trimmed), source_path: request.source_path.and_then(trimmed) };
        queue.items.push(item.clone());
        queue.revision += 1;
        queue.updated_at_utc = now_utc_string();
        self.write_queue(&queue)?;
        Ok(item)
    }

    pub fn claim_queue_item(&self, item_id: &str, request: QueueClaimRequest) -> Result<QueueItem, StoreError> {
        if request.claimed_by.trim().is_empty() { return Err(StoreError::Validation("claimed_by must not be empty".into())); }
        let mut queue = self.load_queue()?;
        let item = queue.items.iter_mut().find(|item| item.id == item_id).ok_or_else(|| StoreError::NotFound(format!("queue item not found: {item_id}")))?;
        item.status = QueueItemStatus::Claimed;
        item.claimed_by = Some(request.claimed_by.trim().to_string());
        let updated = item.clone();
        queue.revision += 1;
        queue.updated_at_utc = now_utc_string();
        self.write_queue(&queue)?;
        Ok(updated)
    }

    pub fn import_repo_analysis_bundle(&self, bundle_dir: impl AsRef<Path>) -> Result<ImportBundleReport, StoreError> {
        self.ensure_storage()?;
        let bundle_dir = bundle_dir.as_ref();
        if !bundle_dir.is_dir() { return Err(StoreError::NotFound(format!("bundle directory not found: {}", bundle_dir.display()))); }
        let runtime_bridge_path = bundle_dir.join("runtime-bridge.json");
        let handoff_path = bundle_dir.join("operator-handoff.json");
        let review_status_path = bundle_dir.join("review-status.json");
        let continuity_path = bundle_dir.join("continuity-status.json");
        for required in [&runtime_bridge_path, &handoff_path, &review_status_path, &continuity_path] {
            if !required.exists() { return Err(StoreError::NotFound(format!("required bundle file missing: {}", required.display()))); }
        }
        let runtime_bridge: serde_json::Value = serde_json::from_str(&fs::read_to_string(&runtime_bridge_path)?)?;
        let handoff: serde_json::Value = serde_json::from_str(&fs::read_to_string(&handoff_path)?)?;
        let review_status: serde_json::Value = serde_json::from_str(&fs::read_to_string(&review_status_path)?)?;
        let continuity_status: serde_json::Value = serde_json::from_str(&fs::read_to_string(&continuity_path)?)?;
        let workflow = handoff.get("workflow").and_then(serde_json::Value::as_str).unwrap_or("repo-analysis-demo").to_string();
        let run_dir_display = relative_or_absolute(&self.paths.workspace_root, bundle_dir);
        let imported_at_utc = now_utc_string();
        let imported_bridge = serde_json::json!({
            "schemaVersion": 1,
            "generatedAtUtc": imported_at_utc,
            "importedFrom": {
                "workflow": workflow,
                "runDir": run_dir_display,
                "runtimeBridgePath": relative_or_absolute(&self.paths.workspace_root, &runtime_bridge_path),
                "reviewStatusPath": relative_or_absolute(&self.paths.workspace_root, &review_status_path),
                "continuityStatusPath": relative_or_absolute(&self.paths.workspace_root, &continuity_path),
                "operatorHandoffPath": relative_or_absolute(&self.paths.workspace_root, &handoff_path),
            },
            "honestyNote": "Imported from a staged static operator bundle. This backend file reflects the latest explicit sync, not a live watcher or a full web app.",
            "bundleReviewStatus": review_status,
            "bundleContinuityStatus": continuity_status,
            "bundleOperatorHandoff": handoff,
            "runtimeBridge": runtime_bridge,
        });
        fs::write(&self.paths.runtime_bridge_file, serde_json::to_string_pretty(&imported_bridge)?)?;
        let queue_status = derive_queue_status(&review_status, &continuity_status);
        let current_owner = continuity_status.get("currentOwner").and_then(serde_json::Value::as_str).map(ToOwned::to_owned);
        let source_path = relative_or_absolute(&self.paths.workspace_root, &handoff_path);
        let title = format!("Review {} bundle {}", workflow, bundle_dir.file_name().and_then(|v| v.to_str()).unwrap_or("latest"));
        let note = Some(format!("Imported from staged operator bundle; reviewStatus={}, handoffState={}", review_status.get("status").and_then(serde_json::Value::as_str).unwrap_or("unknown"), continuity_status.get("handoffState").and_then(serde_json::Value::as_str).unwrap_or("unknown")));
        let mut queue = self.load_queue()?;
        let queue_item_id;
        if let Some(existing) = queue.items.iter_mut().find(|item| item.source_path.as_deref() == Some(source_path.as_str())) {
            existing.title = title;
            existing.kind = workflow.clone();
            existing.status = queue_status.clone();
            existing.claimed_by = current_owner.clone();
            existing.note = note.clone();
            queue_item_id = existing.id.clone();
        } else {
            let item = QueueItem { id: format!("item-{}", unix_timestamp()), title, kind: workflow.clone(), status: queue_status.clone(), created_at_utc: imported_at_utc.clone(), claimed_by: current_owner.clone(), note, source_path: Some(source_path.clone()) };
            queue_item_id = item.id.clone();
            queue.items.push(item);
        }
        queue.revision += 1;
        queue.updated_at_utc = imported_at_utc.clone();
        self.write_queue(&queue)?;
        Ok(ImportBundleReport { workflow, run_dir: run_dir_display, imported_at_utc, queue_item_id, queue_status, runtime_bridge_file: relative_or_absolute(&self.paths.workspace_root, &self.paths.runtime_bridge_file) })
    }

    fn write_queue(&self, queue: &OperatorQueue) -> Result<(), StoreError> {
        fs::write(&self.paths.queue_file, serde_json::to_string_pretty(queue)?)?;
        Ok(())
    }

    fn load_runtime_bridge(&self) -> Result<RuntimeBridgeSnapshot, StoreError> {
        self.ensure_storage()?;
        let value: serde_json::Value = serde_json::from_str(&fs::read_to_string(&self.paths.runtime_bridge_file)?)?;
        let latest_session = value.get("runtimeBridge").and_then(|bridge| bridge.get("latestSession")).or_else(|| value.get("latestSession"));
        let recent_traces = value.get("runtimeBridge").and_then(|bridge| bridge.get("recentTraces")).or_else(|| value.get("recentTraces")).and_then(serde_json::Value::as_array).cloned().unwrap_or_default();
        Ok(RuntimeBridgeSnapshot {
            latest_session_id: latest_session.and_then(|s| s.get("sessionId")).and_then(serde_json::Value::as_str).map(ToOwned::to_owned),
            latest_session_path: latest_session.and_then(|s| s.get("path")).and_then(serde_json::Value::as_str).map(ToOwned::to_owned),
            recent_trace_ids: recent_traces.into_iter().filter_map(|trace| trace.get("traceId").and_then(serde_json::Value::as_str).map(ToOwned::to_owned)).collect(),
            source_file: Some(relative_or_absolute(&self.paths.workspace_root, &self.paths.runtime_bridge_file)),
            generated_at_utc: value.get("generatedAtUtc").and_then(serde_json::Value::as_str).map(ToOwned::to_owned),
            status: if latest_session.is_some() || value.get("recentTraces").is_some() || value.get("runtimeBridge").is_some() { "loaded".into() } else { "placeholder".into() },
        })
    }
}

fn derive_queue_status(review_status: &serde_json::Value, continuity_status: &serde_json::Value) -> QueueItemStatus {
    match review_status.get("status").and_then(serde_json::Value::as_str).unwrap_or("pending-review") {
        "review-complete" => QueueItemStatus::Completed,
        "review-in-progress" => QueueItemStatus::InReview,
        "dropped" => QueueItemStatus::Dropped,
        _ => match continuity_status.get("handoffState").and_then(serde_json::Value::as_str).unwrap_or("awaiting-first-operator") {
            "claimed" => QueueItemStatus::Claimed,
            "in-review" => QueueItemStatus::InReview,
            "handoff-ready" => QueueItemStatus::HandoffReady,
            "completed" => QueueItemStatus::Completed,
            "dropped" => QueueItemStatus::Dropped,
            _ => QueueItemStatus::Queued,
        },
    }
}

fn relative_or_absolute(root: &Path, path: &Path) -> String {
    path.strip_prefix(root).map(|v| v.display().to_string()).unwrap_or_else(|_| path.display().to_string())
}
fn trimmed(value: String) -> Option<String> { let t = value.trim(); if t.is_empty() { None } else { Some(t.to_string()) } }
fn unix_timestamp() -> u128 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() }
fn now_utc_string() -> String { unix_timestamp().to_string() }

#[cfg(test)]
mod tests {
    use super::*;
    fn temp_workspace(name: &str) -> PathBuf { std::env::temp_dir().join(format!("web-backend-core-{name}-{}", unix_timestamp())) }
    #[test]
    fn creates_storage_and_placeholder_files() {
        let root = temp_workspace("storage");
        let store = WebBackendStore::new(StorePaths::from_workspace_root(&root), "127.0.0.1:8787");
        store.ensure_storage().unwrap();
        assert!(root.join(".claw/backend/operator-queue.json").exists());
        assert!(root.join(".claw/backend/runtime-bridge.json").exists());
        let _ = fs::remove_dir_all(root);
    }
    #[test]
    fn can_create_and_claim_queue_items() {
        let root = temp_workspace("queue");
        let store = WebBackendStore::new(StorePaths::from_workspace_root(&root), "127.0.0.1:8787");
        let item = store.create_queue_item(QueueItemCreateRequest { title: "Review approval packet".into(), kind: "review".into(), note: Some("first pass".into()), source_path: Some(".claw/web-approvals/index.json".into()) }).unwrap();
        let claimed = store.claim_queue_item(&item.id, QueueClaimRequest { claimed_by: "operator-a".into() }).unwrap();
        assert_eq!(claimed.status, QueueItemStatus::Claimed);
        assert_eq!(claimed.claimed_by.as_deref(), Some("operator-a"));
        let _ = fs::remove_dir_all(root);
    }
    #[test]
    fn snapshot_includes_service_and_queue_state() {
        let root = temp_workspace("snapshot");
        let store = WebBackendStore::new(StorePaths::from_workspace_root(&root), "127.0.0.1:8787");
        store.ensure_storage().unwrap();
        let snapshot = store.snapshot().unwrap();
        assert_eq!(snapshot.service.name, "claw-webd");
        assert_eq!(snapshot.schema.version, "v1");
        let _ = fs::remove_dir_all(root);
    }
    #[test]
    fn can_import_repo_analysis_bundle_into_backend_state() {
        let root = temp_workspace("import");
        let bundle = root.join(".demo-artifacts/repo-analysis-demo/20260412T030700Z");
        fs::create_dir_all(&bundle).unwrap();
        fs::write(bundle.join("runtime-bridge.json"), serde_json::json!({"schemaVersion":2,"latestSession":{"sessionId":"session-123","path":".claw/sessions/session-123.jsonl"},"recentTraces":[{"traceId":"trace-1","path":".claw/trace/trace-1.json"}]}).to_string()).unwrap();
        fs::write(bundle.join("operator-handoff.json"), serde_json::json!({"workflow":"repo-analysis-demo","operatorNextStep":"review it"}).to_string()).unwrap();
        fs::write(bundle.join("review-status.json"), serde_json::json!({"status":"pending-review"}).to_string()).unwrap();
        fs::write(bundle.join("continuity-status.json"), serde_json::json!({"handoffState":"awaiting-first-operator","currentOwner":null}).to_string()).unwrap();
        let store = WebBackendStore::new(StorePaths::from_workspace_root(&root), "127.0.0.1:8787");
        let report = store.import_repo_analysis_bundle(&bundle).unwrap();
        assert_eq!(report.workflow, "repo-analysis-demo");
        assert_eq!(report.queue_status, QueueItemStatus::Queued);
        let snapshot = store.snapshot().unwrap();
        assert_eq!(snapshot.runtime_bridge.latest_session_id.as_deref(), Some("session-123"));
        assert_eq!(snapshot.queue.items.len(), 1);
        let _ = fs::remove_dir_all(root);
    }
}
