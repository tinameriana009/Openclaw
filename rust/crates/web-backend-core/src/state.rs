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
    pub operator_inbox_file: String,
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
    #[serde(default)]
    pub expected_revision: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QueueNoteRequest {
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QueueTransitionRequest {
    pub to_status: QueueItemStatus,
    #[serde(default)]
    pub claimed_by: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub expected_revision: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackendSnapshot {
    pub service: ServiceInfo,
    pub schema: BackendApiSchema,
    pub config: ServiceConfig,
    pub paths: BackendPaths,
    pub runtime_bridge: RuntimeBridgeSnapshot,
    pub queue: OperatorQueue,
    pub operator_inbox: OperatorInboxSnapshot,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyncInboxReport {
    pub synced_at_utc: String,
    pub inbox_source_file: String,
    pub review_index_file: Option<String>,
    pub imported_entries: u64,
    pub queue_revision: u64,
    pub operator_inbox_file: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OperatorInboxSnapshot {
    pub source_file: Option<String>,
    pub review_index_file: Option<String>,
    pub generated_at_utc: Option<String>,
    pub synced_at_utc: Option<String>,
    pub status: String,
    pub entry_count: u64,
    pub entries: Vec<OperatorInboxEntry>,
    pub honesty_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OperatorInboxEntry {
    pub item_id: String,
    pub trace_id: Option<String>,
    pub queue_item_id: Option<String>,
    pub status: String,
    pub queue_bucket: Option<String>,
    pub queue_label: Option<String>,
    pub queue_priority: Option<u64>,
    pub queue_status: Option<QueueItemStatus>,
    pub operator_state: Option<String>,
    pub next_step: Option<String>,
    pub review_json_path: Option<String>,
    pub review_html_path: Option<String>,
    pub review_status_path: Option<String>,
    pub approval_packet: Option<String>,
    pub session_id: Option<String>,
    pub corpus_id: Option<String>,
    pub pending_query_count: u64,
    pub replay_count: u64,
    pub source_updated_at_ms: Option<u64>,
    pub first_surfaced_at_ms: Option<u64>,
    pub last_surfaced_at_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorePaths {
    pub workspace_root: PathBuf,
    pub storage_root: PathBuf,
    pub queue_file: PathBuf,
    pub runtime_bridge_file: PathBuf,
    pub operator_inbox_file: PathBuf,
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
            operator_inbox_file: storage_root.join("operator-inbox.json"),
        }
    }
}

#[derive(Debug)]
pub enum StoreError {
    Io(io::Error),
    Json(serde_json::Error),
    Validation(String),
    NotFound(String),
    Conflict(String),
}

impl Display for StoreError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::Json(error) => write!(f, "{error}"),
            Self::Validation(message) | Self::NotFound(message) | Self::Conflict(message) => write!(f, "{message}"),
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
            self.write_queue(&OperatorQueue {
                schema_version: 1,
                revision: 0,
                updated_at_utc: now_utc_string(),
                items: Vec::new(),
            })?;
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
        if !self.paths.operator_inbox_file.exists() {
            fs::write(
                &self.paths.operator_inbox_file,
                serde_json::to_string_pretty(&serde_json::json!({
                    "schemaVersion": 1,
                    "generatedAtUtc": now_utc_string(),
                    "syncedAtUtc": null,
                    "sourceFile": null,
                    "reviewIndexFile": null,
                    "honestyNote": "Backend-cached operator inbox snapshot only. It is synced from static review artifacts on demand; there is no live watcher or browser control plane.",
                    "entries": [],
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
                    "/healthz".into(),
                    "/v1/schema".into(),
                    "/v1/state".into(),
                    "/v1/queue".into(),
                    "/v1/queue/items".into(),
                    "/v1/queue/items/:id/claim".into(),
                    "/v1/queue/items/:id/unclaim".into(),
                    "/v1/queue/items/:id/complete".into(),
                    "/v1/queue/items/:id/drop".into(),
                    "/v1/queue/items/:id/reopen".into(),
                    "/v1/operator/inbox".into(),
                    "/v1/operator/inbox/sync".into(),
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
                operator_inbox_file: relative_or_absolute(&self.paths.workspace_root, &self.paths.operator_inbox_file),
            },
            runtime_bridge: self.load_runtime_bridge()?,
            queue: self.load_queue()?,
            operator_inbox: self.load_operator_inbox()?,
        })
    }

    pub fn load_queue(&self) -> Result<OperatorQueue, StoreError> {
        self.ensure_storage()?;
        Ok(serde_json::from_str(&fs::read_to_string(
            &self.paths.queue_file,
        )?)?)
    }

    pub fn load_operator_inbox(&self) -> Result<OperatorInboxSnapshot, StoreError> {
        self.ensure_storage()?;
        let value: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&self.paths.operator_inbox_file)?)?;
        let entries = value
            .get("entries")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(|entry| OperatorInboxEntry {
                item_id: entry
                    .get("itemId")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
                trace_id: entry
                    .get("traceId")
                    .and_then(serde_json::Value::as_str)
                    .map(ToOwned::to_owned),
                queue_item_id: entry
                    .get("queueItemId")
                    .and_then(serde_json::Value::as_str)
                    .map(ToOwned::to_owned),
                status: entry
                    .get("status")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("pending")
                    .to_string(),
                queue_bucket: entry
                    .get("queueBucket")
                    .and_then(serde_json::Value::as_str)
                    .map(ToOwned::to_owned),
                queue_label: entry
                    .get("queueLabel")
                    .and_then(serde_json::Value::as_str)
                    .map(ToOwned::to_owned),
                queue_priority: entry
                    .get("queuePriority")
                    .and_then(serde_json::Value::as_u64),
                queue_status: entry
                    .get("queueStatus")
                    .cloned()
                    .and_then(|status| serde_json::from_value(status).ok()),
                operator_state: entry
                    .get("operatorState")
                    .and_then(serde_json::Value::as_str)
                    .map(ToOwned::to_owned),
                next_step: entry
                    .get("nextStep")
                    .and_then(serde_json::Value::as_str)
                    .map(ToOwned::to_owned),
                review_json_path: entry
                    .get("reviewJsonPath")
                    .and_then(serde_json::Value::as_str)
                    .map(ToOwned::to_owned),
                review_html_path: entry
                    .get("reviewHtmlPath")
                    .and_then(serde_json::Value::as_str)
                    .map(ToOwned::to_owned),
                review_status_path: entry
                    .get("reviewStatusPath")
                    .and_then(serde_json::Value::as_str)
                    .map(ToOwned::to_owned),
                approval_packet: entry
                    .get("approvalPacket")
                    .and_then(serde_json::Value::as_str)
                    .map(ToOwned::to_owned),
                session_id: entry
                    .get("sessionId")
                    .and_then(serde_json::Value::as_str)
                    .map(ToOwned::to_owned),
                corpus_id: entry
                    .get("corpusId")
                    .and_then(serde_json::Value::as_str)
                    .map(ToOwned::to_owned),
                pending_query_count: entry
                    .get("pendingQueryCount")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0),
                replay_count: entry
                    .get("replayCount")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0),
                source_updated_at_ms: entry
                    .get("sourceUpdatedAtMs")
                    .and_then(serde_json::Value::as_u64),
                first_surfaced_at_ms: entry
                    .get("firstSurfacedAtMs")
                    .and_then(serde_json::Value::as_u64),
                last_surfaced_at_ms: entry
                    .get("lastSurfacedAtMs")
                    .and_then(serde_json::Value::as_u64),
            })
            .collect::<Vec<_>>();
        Ok(OperatorInboxSnapshot {
            source_file: value
                .get("sourceFile")
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned),
            review_index_file: value
                .get("reviewIndexFile")
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned),
            generated_at_utc: value
                .get("generatedAtUtc")
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned),
            synced_at_utc: value
                .get("syncedAtUtc")
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned),
            status: if entries.is_empty() {
                "empty".into()
            } else {
                "loaded".into()
            },
            entry_count: entries.len() as u64,
            entries,
            honesty_note: value
                .get("honestyNote")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("Backend-cached operator inbox snapshot only.")
                .to_string(),
        })
    }

    pub fn create_queue_item(
        &self,
        request: QueueItemCreateRequest,
    ) -> Result<QueueItem, StoreError> {
        if request.title.trim().is_empty() {
            return Err(StoreError::Validation(
                "queue item title must not be empty".into(),
            ));
        }
        if request.kind.trim().is_empty() {
            return Err(StoreError::Validation(
                "queue item kind must not be empty".into(),
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
            note: request.note.and_then(trimmed),
            source_path: request.source_path.and_then(trimmed),
        };
        queue.items.push(item.clone());
        queue.revision += 1;
        queue.updated_at_utc = now_utc_string();
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
                "claimed_by must not be empty".into(),
            ));
        }
        self.update_queue_item(item_id, |item| {
            if matches!(item.status, QueueItemStatus::Completed | QueueItemStatus::Dropped) {
                return Err(StoreError::Validation(
                    "cannot claim a completed or dropped queue item; reopen it first".into(),
                ));
            }
            item.status = QueueItemStatus::Claimed;
            item.claimed_by = Some(request.claimed_by.trim().to_string());
            Ok(())
        })
    }

    pub fn unclaim_queue_item(
        &self,
        item_id: &str,
        request: QueueNoteRequest,
    ) -> Result<QueueItem, StoreError> {
        self.update_queue_item(item_id, |item| {
            if matches!(item.status, QueueItemStatus::Completed | QueueItemStatus::Dropped) {
                return Err(StoreError::Validation(
                    "cannot unclaim a completed or dropped queue item; reopen it first".into(),
                ));
            }
            item.status = QueueItemStatus::Queued;
            item.claimed_by = None;
            item.note = normalize_note(request.note.clone(), item.note.clone());
            Ok(())
        })
    }

    pub fn complete_queue_item(
        &self,
        item_id: &str,
        request: QueueNoteRequest,
    ) -> Result<QueueItem, StoreError> {
        self.update_queue_item(item_id, |item| {
            if item.status == QueueItemStatus::Dropped {
                return Err(StoreError::Validation(
                    "cannot complete a dropped queue item; reopen it first".into(),
                ));
            }
            item.status = QueueItemStatus::Completed;
            item.note = normalize_note(request.note.clone(), item.note.clone());
            Ok(())
        })
    }

    pub fn drop_queue_item(
        &self,
        item_id: &str,
        request: QueueNoteRequest,
    ) -> Result<QueueItem, StoreError> {
        self.update_queue_item(item_id, |item| {
            item.status = QueueItemStatus::Dropped;
            item.claimed_by = None;
            item.note = normalize_note(request.note.clone(), item.note.clone());
            Ok(())
        })
    }

    pub fn reopen_queue_item(
        &self,
        item_id: &str,
        request: QueueNoteRequest,
    ) -> Result<QueueItem, StoreError> {
        self.update_queue_item(item_id, |item| {
            item.status = QueueItemStatus::Queued;
            item.claimed_by = None;
            item.note = normalize_note(request.note.clone(), item.note.clone());
            Ok(())
        })
    }

    pub fn import_repo_analysis_bundle(
        &self,
        bundle_dir: impl AsRef<Path>,
    ) -> Result<ImportBundleReport, StoreError> {
        self.ensure_storage()?;
        let bundle_dir = bundle_dir.as_ref();
        if !bundle_dir.is_dir() {
            return Err(StoreError::NotFound(format!(
                "bundle directory not found: {}",
                bundle_dir.display()
            )));
        }
        let runtime_bridge_path = bundle_dir.join("runtime-bridge.json");
        let handoff_path = bundle_dir.join("operator-handoff.json");
        let review_status_path = bundle_dir.join("review-status.json");
        let continuity_path = bundle_dir.join("continuity-status.json");
        for required in [
            &runtime_bridge_path,
            &handoff_path,
            &review_status_path,
            &continuity_path,
        ] {
            if !required.exists() {
                return Err(StoreError::NotFound(format!(
                    "required bundle file missing: {}",
                    required.display()
                )));
            }
        }
        let runtime_bridge: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&runtime_bridge_path)?)?;
        let handoff: serde_json::Value = serde_json::from_str(&fs::read_to_string(&handoff_path)?)?;
        let review_status: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&review_status_path)?)?;
        let continuity_status: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&continuity_path)?)?;
        let workflow = handoff
            .get("workflow")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("repo-analysis-demo")
            .to_string();
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
        fs::write(
            &self.paths.runtime_bridge_file,
            serde_json::to_string_pretty(&imported_bridge)?,
        )?;
        let queue_status = derive_queue_status(&review_status, &continuity_status);
        let current_owner = continuity_status
            .get("currentOwner")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned);
        let source_path = relative_or_absolute(&self.paths.workspace_root, &handoff_path);
        let title = format!(
            "Review {} bundle {}",
            workflow,
            bundle_dir
                .file_name()
                .and_then(|v| v.to_str())
                .unwrap_or("latest")
        );
        let note = Some(format!(
            "Imported from staged operator bundle; reviewStatus={}, handoffState={}",
            review_status
                .get("status")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unknown"),
            continuity_status
                .get("handoffState")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unknown")
        ));
        let mut queue = self.load_queue()?;
        let queue_item_id;
        if let Some(existing) = queue
            .items
            .iter_mut()
            .find(|item| item.source_path.as_deref() == Some(source_path.as_str()))
        {
            existing.title = title;
            existing.kind = workflow.clone();
            existing.status = queue_status.clone();
            existing.claimed_by = current_owner.clone();
            existing.note = note.clone();
            queue_item_id = existing.id.clone();
        } else {
            let item = QueueItem {
                id: format!("item-{}", unix_timestamp()),
                title,
                kind: workflow.clone(),
                status: queue_status.clone(),
                created_at_utc: imported_at_utc.clone(),
                claimed_by: current_owner.clone(),
                note,
                source_path: Some(source_path.clone()),
            };
            queue_item_id = item.id.clone();
            queue.items.push(item);
        }
        queue.revision += 1;
        queue.updated_at_utc = imported_at_utc.clone();
        self.write_queue(&queue)?;
        Ok(ImportBundleReport {
            workflow,
            run_dir: run_dir_display,
            imported_at_utc,
            queue_item_id,
            queue_status,
            runtime_bridge_file: relative_or_absolute(
                &self.paths.workspace_root,
                &self.paths.runtime_bridge_file,
            ),
        })
    }

    pub fn sync_web_approval_inbox(&self) -> Result<SyncInboxReport, StoreError> {
        self.ensure_storage()?;
        let inbox_source_path = self
            .paths
            .workspace_root
            .join(".claw")
            .join("web-approvals")
            .join("inbox-state.json");
        if !inbox_source_path.exists() {
            return Err(StoreError::NotFound(format!(
                "web approval inbox artifact not found: {}",
                inbox_source_path.display()
            )));
        }
        let review_index_path = self
            .paths
            .workspace_root
            .join(".claw")
            .join("web-approvals")
            .join("index.review.json");
        let inbox_value: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&inbox_source_path)?)?;
        let review_index_value = if review_index_path.exists() {
            Some(serde_json::from_str::<serde_json::Value>(
                &fs::read_to_string(&review_index_path)?,
            )?)
        } else {
            None
        };
        let synced_at_utc = now_utc_string();
        let mut queue = self.load_queue()?;
        let review_entries = review_index_value
            .as_ref()
            .and_then(|value| value.get("entries"))
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut synced_entries = Vec::new();
        for entry in inbox_value
            .get("entries")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default()
        {
            let source_path = entry
                .get("reviewJsonPath")
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned)
                .or_else(|| {
                    entry
                        .get("approvalPacket")
                        .and_then(serde_json::Value::as_str)
                        .map(ToOwned::to_owned)
                });
            let matched_review_entry = find_review_entry(
                &review_entries,
                entry.get("traceId").and_then(serde_json::Value::as_str),
                source_path.as_deref(),
            );
            let queue_status = matched_review_entry
                .and_then(|review| derive_queue_status_from_review_entry(review).ok())
                .unwrap_or_else(|| derive_queue_status_from_inbox_entry(&entry));
            let claimed_by = matched_review_entry
                .and_then(|review| review.get("claimedBy").and_then(serde_json::Value::as_str))
                .map(ToOwned::to_owned);
            let title = build_inbox_queue_title(
                entry.get("traceId").and_then(serde_json::Value::as_str),
                entry.get("queueLabel").and_then(serde_json::Value::as_str),
                entry.get("task").and_then(serde_json::Value::as_str),
            );
            let note = Some(format!(
                "Synced from static web approval inbox; queueBucket={}, operatorState={}",
                entry
                    .get("queueBucket")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("unknown"),
                entry
                    .get("operatorState")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("unknown")
            ));
            let queue_item_id = if let Some(source_path) = source_path.as_deref() {
                if let Some(existing) = queue
                    .items
                    .iter_mut()
                    .find(|item| item.source_path.as_deref() == Some(source_path))
                {
                    existing.title = title.clone();
                    existing.kind = "web-approval-review".to_string();
                    existing.status = queue_status.clone();
                    existing.claimed_by = claimed_by.clone();
                    existing.note = note.clone();
                    existing.id.clone()
                } else {
                    let item = QueueItem {
                        id: format!("item-{}", unix_timestamp()),
                        title: title.clone(),
                        kind: "web-approval-review".to_string(),
                        status: queue_status.clone(),
                        created_at_utc: synced_at_utc.clone(),
                        claimed_by: claimed_by.clone(),
                        note: note.clone(),
                        source_path: Some(source_path.to_string()),
                    };
                    let id = item.id.clone();
                    queue.items.push(item);
                    id
                }
            } else {
                format!("unsynced-{}", unix_timestamp())
            };
            let mut synced_entry = entry;
            synced_entry["queueItemId"] = serde_json::Value::String(queue_item_id);
            synced_entry["queueStatus"] = serde_json::to_value(queue_status)?;
            synced_entries.push(synced_entry);
        }
        queue.revision += 1;
        queue.updated_at_utc = synced_at_utc.clone();
        self.write_queue(&queue)?;
        let persisted_inbox = serde_json::json!({
            "schemaVersion": 1,
            "generatedAtUtc": inbox_value.get("generatedAtUtc").and_then(serde_json::Value::as_u64).map(|value| value.to_string()).unwrap_or_else(now_utc_string),
            "syncedAtUtc": synced_at_utc,
            "sourceFile": relative_or_absolute(&self.paths.workspace_root, &inbox_source_path),
            "reviewIndexFile": review_index_value.as_ref().map(|_| relative_or_absolute(&self.paths.workspace_root, &review_index_path)),
            "honestyNote": "Backend-cached operator inbox snapshot only. It is synced from static review artifacts on demand; there is no live watcher or browser control plane.",
            "entries": synced_entries,
        });
        fs::write(
            &self.paths.operator_inbox_file,
            serde_json::to_string_pretty(&persisted_inbox)?,
        )?;
        Ok(SyncInboxReport {
            synced_at_utc,
            inbox_source_file: relative_or_absolute(&self.paths.workspace_root, &inbox_source_path),
            review_index_file: review_index_value
                .as_ref()
                .map(|_| relative_or_absolute(&self.paths.workspace_root, &review_index_path)),
            imported_entries: persisted_inbox
                .get("entries")
                .and_then(serde_json::Value::as_array)
                .map(|entries| entries.len())
                .unwrap_or(0) as u64,
            queue_revision: queue.revision,
            operator_inbox_file: relative_or_absolute(
                &self.paths.workspace_root,
                &self.paths.operator_inbox_file,
            ),
        })
    }

    fn update_queue_item<F>(&self, item_id: &str, mut mutate: F) -> Result<QueueItem, StoreError>
    where
        F: FnMut(&mut QueueItem) -> Result<(), StoreError>,
    {
        let mut queue = self.load_queue()?;
        let item = queue
            .items
            .iter_mut()
            .find(|item| item.id == item_id)
            .ok_or_else(|| StoreError::NotFound(format!("queue item not found: {item_id}")))?;
        mutate(item)?;
        let updated = item.clone();
        queue.revision += 1;
        queue.updated_at_utc = now_utc_string();
        self.write_queue(&queue)?;
        Ok(updated)
    }

    fn write_queue(&self, queue: &OperatorQueue) -> Result<(), StoreError> {
        fs::write(&self.paths.queue_file, serde_json::to_string_pretty(queue)?)?;
        Ok(())
    }

    fn load_runtime_bridge(&self) -> Result<RuntimeBridgeSnapshot, StoreError> {
        self.ensure_storage()?;
        let value: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&self.paths.runtime_bridge_file)?)?;
        let latest_session = value
            .get("runtimeBridge")
            .and_then(|bridge| bridge.get("latestSession"))
            .or_else(|| value.get("latestSession"));
        let recent_traces = value
            .get("runtimeBridge")
            .and_then(|bridge| bridge.get("recentTraces"))
            .or_else(|| value.get("recentTraces"))
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();
        Ok(RuntimeBridgeSnapshot {
            latest_session_id: latest_session
                .and_then(|s| s.get("sessionId"))
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned),
            latest_session_path: latest_session
                .and_then(|s| s.get("path"))
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
            status: if latest_session.is_some()
                || value.get("recentTraces").is_some()
                || value.get("runtimeBridge").is_some()
            {
                "loaded".into()
            } else {
                "placeholder".into()
            },
        })
    }
}

fn derive_queue_status(
    review_status: &serde_json::Value,
    continuity_status: &serde_json::Value,
) -> QueueItemStatus {
    match review_status
        .get("status")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("pending-review")
    {
        "review-complete" => QueueItemStatus::Completed,
        "review-in-progress" => QueueItemStatus::InReview,
        "dropped" => QueueItemStatus::Dropped,
        _ => match continuity_status
            .get("handoffState")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("awaiting-first-operator")
        {
            "claimed" => QueueItemStatus::Claimed,
            "in-review" => QueueItemStatus::InReview,
            "handoff-ready" => QueueItemStatus::HandoffReady,
            "completed" => QueueItemStatus::Completed,
            "dropped" => QueueItemStatus::Dropped,
            _ => QueueItemStatus::Queued,
        },
    }
}

fn derive_queue_status_from_inbox_entry(entry: &serde_json::Value) -> QueueItemStatus {
    match entry
        .get("queueBucket")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("queued")
    {
        "ready-to-review" => QueueItemStatus::Queued,
        "ready-to-rerun" => QueueItemStatus::Claimed,
        "needs-trace-recovery" | "waiting-on-context" => QueueItemStatus::InReview,
        "archived" => QueueItemStatus::Completed,
        _ => QueueItemStatus::Queued,
    }
}

fn derive_queue_status_from_review_entry(
    entry: &serde_json::Value,
) -> Result<QueueItemStatus, StoreError> {
    Ok(
        match entry
            .get("operatorState")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("queued")
        {
            "rerun captured for review" => QueueItemStatus::Queued,
            "rerun completed without saved trace artifact" => QueueItemStatus::Claimed,
            "needs trace recovery" => QueueItemStatus::InReview,
            "approved" | "review complete" => QueueItemStatus::Completed,
            other if other.contains("handoff") => QueueItemStatus::HandoffReady,
            _ => derive_queue_status_from_inbox_entry(entry),
        },
    )
}

fn find_review_entry<'a>(
    entries: &'a [serde_json::Value],
    trace_id: Option<&str>,
    source_path: Option<&str>,
) -> Option<&'a serde_json::Value> {
    entries.iter().find(|entry| {
        trace_id.is_some_and(|trace_id| {
            entry.get("traceId").and_then(serde_json::Value::as_str) == Some(trace_id)
        }) || source_path.is_some_and(|source_path| {
            entry
                .get("reviewJsonPath")
                .and_then(serde_json::Value::as_str)
                == Some(source_path)
        }) || source_path.is_some_and(|source_path| {
            entry
                .get("approvalPacket")
                .and_then(serde_json::Value::as_str)
                == Some(source_path)
        })
    })
}

fn build_inbox_queue_title(
    trace_id: Option<&str>,
    queue_label: Option<&str>,
    task: Option<&str>,
) -> String {
    let base = task.unwrap_or("Inspect synced web approval review");
    match (queue_label, trace_id) {
        (Some(label), Some(trace_id)) => format!("{label}: {base} ({trace_id})"),
        (Some(label), None) => format!("{label}: {base}"),
        (None, Some(trace_id)) => format!("{base} ({trace_id})"),
        (None, None) => base.to_string(),
    }
}

fn relative_or_absolute(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map(|v| v.display().to_string())
        .unwrap_or_else(|_| path.display().to_string())
}
fn trimmed(value: String) -> Option<String> {
    let t = value.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
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
        store.ensure_storage().unwrap();
        assert!(root.join(".claw/backend/operator-queue.json").exists());
        assert!(root.join(".claw/backend/runtime-bridge.json").exists());
        assert!(root.join(".claw/backend/operator-inbox.json").exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn can_create_and_claim_queue_items() {
        let root = temp_workspace("queue");
        let store = WebBackendStore::new(StorePaths::from_workspace_root(&root), "127.0.0.1:8787");
        let item = store
            .create_queue_item(QueueItemCreateRequest {
                title: "Review approval packet".into(),
                kind: "review".into(),
                note: Some("first pass".into()),
                source_path: Some(".claw/web-approvals/index.json".into()),
            })
            .unwrap();
        let claimed = store
            .claim_queue_item(
                &item.id,
                QueueClaimRequest {
                    claimed_by: "operator-a".into(),
                    expected_revision: Some(1),
                },
            )
            .unwrap();
        assert_eq!(claimed.status, QueueItemStatus::Claimed);
        assert_eq!(claimed.claimed_by.as_deref(), Some("operator-a"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn can_unclaim_complete_drop_and_reopen_queue_items() {
        let root = temp_workspace("queue-mutations");
        let store = WebBackendStore::new(StorePaths::from_workspace_root(&root), "127.0.0.1:8787");
        let item = store
            .create_queue_item(QueueItemCreateRequest {
                title: "Review approval packet".into(),
                kind: "review".into(),
                note: Some("first pass".into()),
                source_path: None,
            })
            .unwrap();
        let claimed = store
            .claim_queue_item(
                &item.id,
                QueueClaimRequest {
                    claimed_by: "operator-a".into(),
                },
            )
            .unwrap();
        assert_eq!(claimed.status, QueueItemStatus::Claimed);

        let unclaimed = store
            .unclaim_queue_item(
                &item.id,
                QueueNoteRequest {
                    note: Some("released for later".into()),
                },
            )
            .unwrap();
        assert_eq!(unclaimed.status, QueueItemStatus::Queued);
        assert_eq!(unclaimed.claimed_by, None);
        assert_eq!(unclaimed.note.as_deref(), Some("released for later"));

        let completed = store
            .complete_queue_item(
                &item.id,
                QueueNoteRequest {
                    note: Some("done locally".into()),
                },
            )
            .unwrap();
        assert_eq!(completed.status, QueueItemStatus::Completed);

        let reopened = store
            .reopen_queue_item(
                &item.id,
                QueueNoteRequest {
                    note: Some("follow-up needed".into()),
                },
            )
            .unwrap();
        assert_eq!(reopened.status, QueueItemStatus::Queued);
        assert_eq!(reopened.claimed_by, None);

        let dropped = store
            .drop_queue_item(
                &item.id,
                QueueNoteRequest {
                    note: Some("not actionable".into()),
                },
            )
            .unwrap();
        assert_eq!(dropped.status, QueueItemStatus::Dropped);
        assert_eq!(dropped.claimed_by, None);
        assert_eq!(dropped.note.as_deref(), Some("not actionable"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_claiming_terminal_items_without_reopen() {
        let root = temp_workspace("queue-terminal-validation");
        let store = WebBackendStore::new(StorePaths::from_workspace_root(&root), "127.0.0.1:8787");
        let item = store
            .create_queue_item(QueueItemCreateRequest {
                title: "Done item".into(),
                kind: "review".into(),
                note: None,
                source_path: None,
            })
            .unwrap();
        store
            .complete_queue_item(
                &item.id,
                QueueNoteRequest {
                    note: Some("done".into()),
                },
            )
            .unwrap();
        let error = store
            .claim_queue_item(
                &item.id,
                QueueClaimRequest {
                    claimed_by: "operator-b".into(),
                },
            )
            .unwrap_err();
        assert!(matches!(error, StoreError::Validation(_)));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_stale_claim_revisions_and_terminal_claims() {
        let root = temp_workspace("claim-guards");
        let store = WebBackendStore::new(StorePaths::from_workspace_root(&root), "127.0.0.1:8787");
        let item = store
            .create_queue_item(QueueItemCreateRequest {
                title: "Review approval packet".into(),
                kind: "review".into(),
                note: None,
                source_path: None,
            })
            .unwrap();

        let stale = store
            .claim_queue_item(
                &item.id,
                QueueClaimRequest {
                    claimed_by: "operator-a".into(),
                    expected_revision: Some(0),
                },
            )
            .expect_err("stale revision should fail");
        assert!(matches!(stale, StoreError::Conflict(message) if message.contains("revision mismatch")));

        let dropped = store
            .transition_queue_item(
                &item.id,
                QueueTransitionRequest {
                    to_status: QueueItemStatus::Dropped,
                    claimed_by: None,
                    note: Some("no longer needed".into()),
                    expected_revision: Some(1),
                },
            )
            .unwrap();
        assert_eq!(dropped.status, QueueItemStatus::Dropped);

        let terminal = store
            .claim_queue_item(
                &item.id,
                QueueClaimRequest {
                    claimed_by: "operator-a".into(),
                    expected_revision: Some(2),
                },
            )
            .expect_err("terminal item should not be claimable");
        assert!(matches!(terminal, StoreError::Conflict(message) if message.contains("terminal")));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn enforces_explicit_queue_transition_rules() {
        let root = temp_workspace("transition-rules");
        let store = WebBackendStore::new(StorePaths::from_workspace_root(&root), "127.0.0.1:8787");
        let item = store
            .create_queue_item(QueueItemCreateRequest {
                title: "Review approval packet".into(),
                kind: "review".into(),
                note: None,
                source_path: None,
            })
            .unwrap();

        let invalid = store
            .transition_queue_item(
                &item.id,
                QueueTransitionRequest {
                    to_status: QueueItemStatus::Completed,
                    claimed_by: None,
                    note: None,
                    expected_revision: Some(1),
                },
            )
            .expect_err("queued -> completed should fail");
        assert!(matches!(invalid, StoreError::Conflict(message) if message.contains("invalid queue transition")));

        let claimed = store
            .transition_queue_item(
                &item.id,
                QueueTransitionRequest {
                    to_status: QueueItemStatus::Claimed,
                    claimed_by: Some("operator-a".into()),
                    note: None,
                    expected_revision: Some(1),
                },
            )
            .unwrap();
        assert_eq!(claimed.status, QueueItemStatus::Claimed);
        assert_eq!(claimed.claimed_by.as_deref(), Some("operator-a"));

        let review = store
            .transition_queue_item(
                &item.id,
                QueueTransitionRequest {
                    to_status: QueueItemStatus::InReview,
                    claimed_by: None,
                    note: Some("triage started".into()),
                    expected_revision: Some(2),
                },
            )
            .unwrap();
        assert_eq!(review.status, QueueItemStatus::InReview);
        assert_eq!(review.note.as_deref(), Some("triage started"));

        let backward = store
            .transition_queue_item(
                &item.id,
                QueueTransitionRequest {
                    to_status: QueueItemStatus::Queued,
                    claimed_by: None,
                    note: None,
                    expected_revision: Some(3),
                },
            )
            .expect_err("backward transition should fail");
        assert!(matches!(backward, StoreError::Conflict(message) if message.contains("invalid queue transition")));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn snapshot_includes_service_queue_and_operator_inbox_state() {
        let root = temp_workspace("snapshot");
        let store = WebBackendStore::new(StorePaths::from_workspace_root(&root), "127.0.0.1:8787");
        store.ensure_storage().unwrap();
        let snapshot = store.snapshot().unwrap();
        assert_eq!(snapshot.service.name, "claw-webd");
        assert_eq!(snapshot.schema.version, "v1");
        assert_eq!(snapshot.operator_inbox.status, "empty");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn can_import_repo_analysis_bundle_into_backend_state() {
        let root = temp_workspace("import");
        let bundle = root.join(".demo-artifacts/repo-analysis-demo/20260412T030700Z");
        fs::create_dir_all(&bundle).unwrap();
        fs::write(bundle.join("runtime-bridge.json"), serde_json::json!({"schemaVersion":2,"latestSession":{"sessionId":"session-123","path":".claw/sessions/session-123.jsonl"},"recentTraces":[{"traceId":"trace-1","path":".claw/trace/trace-1.json"}]}).to_string()).unwrap();
        fs::write(
            bundle.join("operator-handoff.json"),
            serde_json::json!({"workflow":"repo-analysis-demo","operatorNextStep":"review it"})
                .to_string(),
        )
        .unwrap();
        fs::write(
            bundle.join("review-status.json"),
            serde_json::json!({"status":"pending-review"}).to_string(),
        )
        .unwrap();
        fs::write(
            bundle.join("continuity-status.json"),
            serde_json::json!({"handoffState":"awaiting-first-operator","currentOwner":null})
                .to_string(),
        )
        .unwrap();
        let store = WebBackendStore::new(StorePaths::from_workspace_root(&root), "127.0.0.1:8787");
        let report = store.import_repo_analysis_bundle(&bundle).unwrap();
        assert_eq!(report.workflow, "repo-analysis-demo");
        assert_eq!(report.queue_status, QueueItemStatus::Queued);
        let snapshot = store.snapshot().unwrap();
        assert_eq!(
            snapshot.runtime_bridge.latest_session_id.as_deref(),
            Some("session-123")
        );
        assert_eq!(snapshot.queue.items.len(), 1);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn can_sync_static_web_approval_inbox_into_backend_state() {
        let root = temp_workspace("sync-web-approval");
        let approvals_dir = root.join(".claw/web-approvals");
        fs::create_dir_all(&approvals_dir).unwrap();
        fs::write(
            approvals_dir.join("inbox-state.json"),
            serde_json::json!({
                "generatedAtUtc": 123456,
                "entries": [{
                    "itemId": "inbox-trace-1",
                    "traceId": "trace-1",
                    "status": "queued",
                    "queueBucket": "ready-to-review",
                    "queueLabel": "Ready to review rerun",
                    "queuePriority": 1,
                    "operatorState": "rerun captured for review",
                    "nextStep": "inspect review json",
                    "reviewJsonPath": ".claw/web-approvals/trace-1.review.json",
                    "reviewHtmlPath": ".claw/web-approvals/trace-1.review.html",
                    "reviewStatusPath": ".claw/web-approvals/trace-1.review-status.json",
                    "approvalPacket": ".claw/web-approvals/trace-1.json",
                    "sessionId": "session-1",
                    "corpusId": "corpus-1",
                    "pendingQueryCount": 2,
                    "replayCount": 1,
                    "sourceUpdatedAtMs": 123460,
                    "firstSurfacedAtMs": 123450,
                    "lastSurfacedAtMs": 123470
                }]
            })
            .to_string(),
        )
        .unwrap();
        fs::write(
            approvals_dir.join("index.review.json"),
            serde_json::json!({
                "entries": [{
                    "traceId": "trace-1",
                    "reviewJsonPath": ".claw/web-approvals/trace-1.review.json",
                    "approvalPacket": ".claw/web-approvals/trace-1.json",
                    "operatorState": "rerun captured for review"
                }]
            })
            .to_string(),
        )
        .unwrap();
        let store = WebBackendStore::new(StorePaths::from_workspace_root(&root), "127.0.0.1:8787");
        let report = store.sync_web_approval_inbox().unwrap();
        assert_eq!(report.imported_entries, 1);
        let snapshot = store.load_operator_inbox().unwrap();
        assert_eq!(snapshot.entry_count, 1);
        assert_eq!(snapshot.entries[0].trace_id.as_deref(), Some("trace-1"));
        assert_eq!(
            snapshot.entries[0].queue_status,
            Some(QueueItemStatus::Queued)
        );
        let queue = store.load_queue().unwrap();
        assert_eq!(queue.items.len(), 1);
        assert_eq!(queue.items[0].kind, "web-approval-review");
        let _ = fs::remove_dir_all(root);
    }
}
