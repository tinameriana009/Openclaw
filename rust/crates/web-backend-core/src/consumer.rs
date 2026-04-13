use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::state::{BackendSnapshot, QueueItemStatus, StoreError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsumerExportReport {
    pub api_base_url: String,
    pub output_path: String,
    pub queue_item_count: usize,
    pub claimed_item_count: usize,
    pub generated_from_schema: String,
}

pub async fn export_static_status_page(
    api_base_url: &str,
    output_path: impl AsRef<Path>,
) -> Result<ConsumerExportReport, StoreError> {
    let api_base_url = api_base_url.trim_end_matches('/');
    let state_url = format!("{api_base_url}/v1/state");
    let response = reqwest::get(&state_url)
        .await
        .map_err(|error| StoreError::Validation(format!("failed to fetch {state_url}: {error}")))?;
    let response = response
        .error_for_status()
        .map_err(|error| StoreError::Validation(format!("backend api request failed for {state_url}: {error}")))?;
    let snapshot = response
        .json::<BackendSnapshot>()
        .await
        .map_err(|error| StoreError::Validation(format!("failed to decode backend snapshot from {state_url}: {error}")))?;

    let output_path = output_path.as_ref();
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(output_path, render_static_status_page(&snapshot, api_base_url))?;

    Ok(ConsumerExportReport {
        api_base_url: api_base_url.to_string(),
        output_path: output_path.display().to_string(),
        queue_item_count: snapshot.queue.items.len(),
        claimed_item_count: snapshot
            .queue
            .items
            .iter()
            .filter(|item| {
                matches!(
                    item.status,
                    QueueItemStatus::Claimed
                        | QueueItemStatus::InReview
                        | QueueItemStatus::HandoffReady
                        | QueueItemStatus::Completed
                )
            })
            .count(),
        generated_from_schema: snapshot.schema.version,
    })
}

pub fn render_static_status_page(snapshot: &BackendSnapshot, api_base_url: &str) -> String {
    let mut queue_rows = String::new();
    if snapshot.queue.items.is_empty() {
        queue_rows.push_str(
            "<tr><td colspan=\"6\"><em>No queue items yet. The backend is live, but currently empty.</em></td></tr>",
        );
    } else {
        for item in &snapshot.queue.items {
            let _ = write!(
                queue_rows,
                "<tr><td><code>{}</code></td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                escape_html(&item.id),
                escape_html(&item.title),
                escape_html(&item.kind),
                escape_html(&format!("{:?}", item.status)),
                escape_html(item.claimed_by.as_deref().unwrap_or("-")),
                escape_html(item.source_path.as_deref().unwrap_or("-")),
            );
        }
    }

    let trace_list = if snapshot.runtime_bridge.recent_trace_ids.is_empty() {
        "<li>None captured in the imported bridge snapshot.</li>".to_string()
    } else {
        snapshot
            .runtime_bridge
            .recent_trace_ids
            .iter()
            .map(|trace_id| format!("<li><code>{}</code></li>", escape_html(trace_id)))
            .collect::<Vec<_>>()
            .join("")
    };

    format!(
        "<!DOCTYPE html>
<html lang=\"en\">
<head>
  <meta charset=\"utf-8\" />
  <title>Claw Local Backend Status</title>
  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" />
  <style>
    :root {{ color-scheme: light dark; }}
    body {{ font-family: Inter, system-ui, sans-serif; margin: 2rem auto; max-width: 1100px; padding: 0 1rem 4rem; line-height: 1.5; }}
    .honesty {{ border-left: 4px solid #d97706; padding: 0.75rem 1rem; background: rgba(217, 119, 6, 0.12); }}
    .grid {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(220px, 1fr)); gap: 1rem; margin: 1.5rem 0; }}
    .card {{ border: 1px solid rgba(127,127,127,0.35); border-radius: 12px; padding: 1rem; }}
    table {{ width: 100%; border-collapse: collapse; margin-top: 1rem; }}
    th, td {{ text-align: left; padding: 0.65rem; border-bottom: 1px solid rgba(127,127,127,0.25); vertical-align: top; }}
    code {{ font-family: ui-monospace, SFMono-Regular, monospace; }}
    .muted {{ opacity: 0.8; }}
  </style>
</head>
<body>
  <h1>Claw Local Backend Status</h1>
  <p class=\"muted\">Generated from <code>{api_base_url}/v1/state</code>.</p>
  <div class=\"honesty\">
    <strong>Bounded surface only.</strong>
    <div>{honesty_note}</div>
  </div>

  <div class=\"grid\">
    <section class=\"card\">
      <h2>Service</h2>
      <p><strong>{service_name}</strong> v{service_version}</p>
      <p>Bind address: <code>{bind_address}</code></p>
      <p>Schema: <code>{schema_version}</code></p>
    </section>
    <section class=\"card\">
      <h2>Runtime bridge</h2>
      <p>Status: <strong>{runtime_status}</strong></p>
      <p>Latest session: <code>{latest_session}</code></p>
      <p>Bridge file: <code>{runtime_bridge_file}</code></p>
    </section>
    <section class=\"card\">
      <h2>Queue summary</h2>
      <p>Revision: <strong>{queue_revision}</strong></p>
      <p>Items: <strong>{queue_count}</strong></p>
      <p>Updated: <code>{queue_updated}</code></p>
    </section>
  </div>

  <section>
    <h2>Recent traces</h2>
    <ul>{trace_list}</ul>
  </section>

  <section>
    <h2>Operator queue</h2>
    <table>
      <thead>
        <tr><th>ID</th><th>Title</th><th>Kind</th><th>Status</th><th>Claimed by</th><th>Source path</th></tr>
      </thead>
      <tbody>{queue_rows}</tbody>
    </table>
  </section>
</body>
</html>",
        api_base_url = escape_html(api_base_url),
        honesty_note = escape_html(&snapshot.service.honesty_note),
        service_name = escape_html(&snapshot.service.name),
        service_version = escape_html(&snapshot.service.version),
        bind_address = escape_html(&snapshot.config.bind_address),
        schema_version = escape_html(&snapshot.schema.version),
        runtime_status = escape_html(&snapshot.runtime_bridge.status),
        latest_session = escape_html(snapshot.runtime_bridge.latest_session_id.as_deref().unwrap_or("none")),
        runtime_bridge_file = escape_html(&snapshot.paths.runtime_bridge_file),
        queue_revision = snapshot.queue.revision,
        queue_count = snapshot.queue.items.len(),
        queue_updated = escape_html(&snapshot.queue.updated_at_utc),
        trace_list = trace_list,
        queue_rows = queue_rows,
    )
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{
        BackendApiSchema, BackendPaths, OperatorInboxSnapshot, OperatorQueue, QueueItem,
        QueueItemStatus, RuntimeBridgeSnapshot, ServiceConfig, ServiceInfo,
    };

    fn sample_snapshot() -> BackendSnapshot {
        BackendSnapshot {
            service: ServiceInfo {
                name: "claw-webd".into(),
                version: "0.1.0".into(),
                honesty_note: "Local-only backend foundation.".into(),
            },
            schema: BackendApiSchema {
                version: "v1".into(),
                endpoints: vec!["/v1/state".into()],
            },
            config: ServiceConfig {
                bind_address: "127.0.0.1:8787".into(),
                storage_root: ".claw/backend".into(),
            },
            paths: BackendPaths {
                storage_root: ".claw/backend".into(),
                queue_file: ".claw/backend/operator-queue.json".into(),
                runtime_bridge_file: ".claw/backend/runtime-bridge.json".into(),
                operator_inbox_file: ".claw/backend/operator-inbox.json".into(),
            },
            runtime_bridge: RuntimeBridgeSnapshot {
                latest_session_id: Some("session-123".into()),
                latest_session_path: Some(".claw/sessions/session-123.jsonl".into()),
                recent_trace_ids: vec!["trace-a".into(), "trace-b".into()],
                source_file: Some(".claw/backend/runtime-bridge.json".into()),
                generated_at_utc: Some("123456".into()),
                status: "loaded".into(),
            },
            queue: OperatorQueue {
                schema_version: 1,
                revision: 3,
                updated_at_utc: "123456".into(),
                items: vec![QueueItem {
                    id: "item-1".into(),
                    title: "Review imported bundle".into(),
                    kind: "repo-analysis-demo".into(),
                    status: QueueItemStatus::Claimed,
                    created_at_utc: "123450".into(),
                    claimed_by: Some("operator-a".into()),
                    note: None,
                    source_path: Some(".demo-artifacts/run/operator-handoff.json".into()),
                }],
            },
            operator_inbox: OperatorInboxSnapshot {
                source_file: Some(".claw/backend/operator-inbox.json".into()),
                review_index_file: Some(".claw/web-approvals/index.review.json".into()),
                generated_at_utc: Some("123456".into()),
                synced_at_utc: Some("123457".into()),
                status: "loaded".into(),
                entry_count: 1,
                entries: Vec::new(),
                honesty_note: "Backend-cached operator inbox snapshot only.".into(),
            },
        }
    }

    #[test]
    fn render_static_status_page_includes_queue_and_honesty_note() {
        let html = render_static_status_page(&sample_snapshot(), "http://127.0.0.1:8787");
        assert!(html.contains("Claw Local Backend Status"));
        assert!(html.contains("Bounded surface only."));
        assert!(html.contains("Review imported bundle"));
        assert!(html.contains("operator-a"));
        assert!(html.contains("trace-a"));
    }

    #[test]
    fn render_static_status_page_escapes_html() {
        let mut snapshot = sample_snapshot();
        snapshot.queue.items[0].title = "<script>alert(1)</script>".into();
        let html = render_static_status_page(&snapshot, "http://127.0.0.1:8787");
        assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
        assert!(!html.contains("<script>alert(1)</script>"));
    }
}
