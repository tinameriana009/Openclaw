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

impl From<io::Error> for StoreError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for StoreError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

#[derive(Debug, Clone)]
pub struct WebBackendStore {
    paths: StorePaths,
    bind_address: String,
}

impl WebBackendStore {
    #[must_use]
    pub fn new(paths: StorePaths, bind_address: impl Into<String>) -> Self {
        Self {
            paths,
            bind_address: bind_address.into(),
        }
    }

    pub fn ensure_storage(&self) -> Result<(), StoreError> {
        fs::create_dir_all(&self.paths.storage_root)?;
        if !self.paths.queue_file.exists() {
            self.write_queue(&OperatorQueue { items: Vec::new() })?;
        }
        if !self.paths.runtime_bridge_file.exists() {
            fs::write(
                &self.paths.runtime_bridge_file,
                serde_json::to_string_pretty(&serde_json::json!({
                    "schemaVersion": 1,
                    "generatedAtUtc": now_utc_string(),
                    "honestyNote": "Local backend snapshot only. This is a bounded runtime bridge, not a claim of a full live web product.",
                    "latestSession": null,
                    "recentTraces": [],
                }))?,
            )?;
        }
        Ok(())
    }

    pub fn snapshot(&self) -> Result<BackendSnapshot, StoreError> {
        self.ensure_storage()?;
        Ok(BackendSnapshot {
            service: ServiceInfo {
                name: "claw-webd".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                honesty_note: "Local-only backend foundation. It exposes persisted operator state and runtime snapshots, but does not claim a complete live web app.".to_string(),
            },
            schema: BackendApiSchema {
                version: "v1".to_string(),
                endpoints: vec![
                    "/healthz".to_string(),
                    "/v1/schema".to_string(),
                    "/v1/state".to_string(),
                    "/v1/queue".to_string(),
                    "/v1/queue/items".to_string(),
                    "/v1/queue/items/:id/claim".to_string(),
                ],
            },
            config: ServiceConfig {
                bind_address: self.bind_address.clone(),
                storage_root: relative_or_absolute(&self.paths.workspace_root, &self.paths.storage_root),
            },
            paths: BackendPaths {
                storage_root: relative_or_absolute(&self.paths.workspace_root, &self.paths.storage_root),
                queue_file: relative_or_absolute(&self.paths.workspace_root, &self.paths.queue_file),
                runtime_bridge_file: relative_or_absolute(&self.paths.workspace_root, &self.paths.runtime_bridge_file),
            },
            runtime_bridge: self.load_runtime_bridge()?,
            queue: self.load_queue()?,
        })
    }

    pub fn load_queue(&self) -> Result<OperatorQueue, StoreError> {
        self.ensure_storage()?;
        let contents = fs::read_to_string(&self.paths.queue_file)?;
        Ok(serde_json::from_str(&contents)?)
    }

    pub fn create_queue_item(
        &self,
        request: QueueItemCreateRequest,
    ) -> Result<QueueItem, StoreError> {
        if request.title.trim().is_empty() {
            return Err(StoreError::Validation(
                "queue item title must not be empty".to_string(),
            ));
        }
        if request.kind.trim().is_empty() {
            return Err(StoreError::Validation(
                "queue item kind must not be empty".to_string(),
            ));
        }
        let mut queue = self.load_queue()?;
        let item = QueueItem {
            id: format!("item-{}", unix_timestamp()),
            title: request.title.trim().to_string(),
            kind: request.kind.trim().to_string(),
            status: QueueItemStatus::Queued,
            created_at_utc: now_utc_string(),
            claimed_by: None,
            note: request.note.filter(|value| !value.trim().is_empty()),
            source_path: request.source_path.filter(|value| !value.trim().is_empty()),
        };
        queue.items.push(item.clone());
        self.write_queue(&queue)?;
        Ok(item)
    }

    pub fn claim_queue_item(
        &self,
        item_id: &str,
        request: QueueClaimRequest,
    ) -> Result<QueueItem, StoreError> {
        if request.claimed_by.trim().is_empty() {
            return Err(StoreError::Validation(
                "claimed_by must not be empty".to_string(),
            ));
        }
        let mut queue = self.load_queue()?;
        let item = queue
            .items
            .iter_mut()
            .find(|item| item.id == item_id)
            .ok_or_else(|| StoreError::NotFound(format!("queue item not found: {item_id}")))?;
        item.status = QueueItemStatus::Claimed;
        item.claimed_by = Some(request.claimed_by.trim().to_string());
        let updated = item.clone();
        self.write_queue(&queue)?;
        Ok(updated)
    }

    fn write_queue(&self, queue: &OperatorQueue) -> Result<(), StoreError> {
        fs::write(&self.paths.queue_file, serde_json::to_string_pretty(queue)?)?;
        Ok(())
    }

    fn load_runtime_bridge(&self) -> Result<RuntimeBridgeSnapshot, StoreError> {
        self.ensure_storage()?;
        let contents = fs::read_to_string(&self.paths.runtime_bridge_file)?;
        let value: serde_json::Value = serde_json::from_str(&contents)?;
        let latest_session = value.get("latestSession");
        let recent_traces = value
            .get("recentTraces")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();
        Ok(RuntimeBridgeSnapshot {
            latest_session_id: latest_session
                .and_then(|session| session.get("sessionId"))
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned),
            latest_session_path: latest_session
                .and_then(|session| session.get("path"))
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned),
            recent_trace_ids: recent_traces
                .into_iter()
                .filter_map(|trace| {
                    trace
                        .get("traceId")
                        .and_then(serde_json::Value::as_str)
                        .map(ToOwned::to_owned)
                })
                .collect(),
            source_file: Some(relative_or_absolute(
                &self.paths.workspace_root,
                &self.paths.runtime_bridge_file,
            )),
            generated_at_utc: value
                .get("generatedAtUtc")
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned),
            status: if latest_session.is_some() || value.get("recentTraces").is_some() {
                "loaded".to_string()
            } else {
                "placeholder".to_string()
            },
        })
    }
}

fn relative_or_absolute(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map(|value| value.display().to_string())
        .unwrap_or_else(|_| path.display().to_string())
}

fn unix_timestamp() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn now_utc_string() -> String {
    unix_timestamp().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_workspace(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("web-backend-core-{name}-{}", unix_timestamp()))
    }

    #[test]
    fn creates_storage_and_placeholder_files() {
        let root = temp_workspace("storage");
        let store = WebBackendStore::new(StorePaths::from_workspace_root(&root), "127.0.0.1:8787");
        store.ensure_storage().expect("storage should initialize");
        assert!(root.join(".claw/backend/operator-queue.json").exists());
        assert!(root.join(".claw/backend/runtime-bridge.json").exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn can_create_and_claim_queue_items() {
        let root = temp_workspace("queue");
        let store = WebBackendStore::new(StorePaths::from_workspace_root(&root), "127.0.0.1:8787");
        let item = store
            .create_queue_item(QueueItemCreateRequest {
                title: "Review approval packet".to_string(),
                kind: "review".to_string(),
                note: Some("first pass".to_string()),
                source_path: Some(".claw/web-approvals/index.json".to_string()),
            })
            .expect("queue create should succeed");
        assert_eq!(item.status, QueueItemStatus::Queued);

        let claimed = store
            .claim_queue_item(
                &item.id,
                QueueClaimRequest {
                    claimed_by: "operator-a".to_string(),
                },
            )
            .expect("claim should succeed");
        assert_eq!(claimed.status, QueueItemStatus::Claimed);
        assert_eq!(claimed.claimed_by.as_deref(), Some("operator-a"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn snapshot_includes_service_and_queue_state() {
        let root = temp_workspace("snapshot");
        let store = WebBackendStore::new(StorePaths::from_workspace_root(&root), "127.0.0.1:8787");
        store.ensure_storage().expect("storage should initialize");
        let snapshot = store.snapshot().expect("snapshot should succeed");
        assert_eq!(snapshot.service.name, "claw-webd");
        assert_eq!(snapshot.schema.version, "v1");
        assert!(snapshot
            .paths
            .queue_file
            .ends_with(".claw/backend/operator-queue.json"));
        let _ = fs::remove_dir_all(root);
    }
}
