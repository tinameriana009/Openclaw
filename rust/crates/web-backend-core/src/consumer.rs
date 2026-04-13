use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::state::{BackendSnapshot, OperatorInboxEntry, QueueItemStatus, StoreError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsumerExportReport {
    pub api_base_url: String,
    pub output_path: String,
    pub queue_item_count: usize,
    pub claimed_item_count: usize,
    pub review_item_count: usize,
    pub completed_item_count: usize,
    pub inbox_entry_count: u64,
    pub pending_inbox_entry_count: usize,
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
    let response = response.error_for_status().map_err(|error| {
        StoreError::Validation(format!(
            "backend api request failed for {state_url}: {error}"
        ))
    })?;
    let snapshot = response.json::<BackendSnapshot>().await.map_err(|error| {
        StoreError::Validation(format!(
            "failed to decode backend snapshot from {state_url}: {error}"
        ))
    })?;

    let output_path = output_path.as_ref();
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        output_path,
        render_static_status_page(&snapshot, api_base_url),
    )?;

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
        review_item_count: snapshot
            .queue
            .items
            .iter()
            .filter(|item| {
                matches!(
                    item.status,
                    QueueItemStatus::InReview | QueueItemStatus::HandoffReady
                )
            })
            .count(),
        completed_item_count: snapshot
            .queue
            .items
            .iter()
            .filter(|item| matches!(item.status, QueueItemStatus::Completed))
            .count(),
        inbox_entry_count: snapshot.operator_inbox.entry_count,
        pending_inbox_entry_count: snapshot
            .operator_inbox
            .entries
            .iter()
            .filter(|entry| {
                !matches!(
                    entry.queue_status,
                    Some(QueueItemStatus::Completed | QueueItemStatus::Dropped)
                )
            })
            .count(),
        generated_from_schema: snapshot.schema.version,
    })
}

pub fn render_static_status_page(snapshot: &BackendSnapshot, api_base_url: &str) -> String {
    let queue_rows = render_queue_rows(snapshot);
    let inbox_rows = render_inbox_rows(snapshot);
    let trace_list = render_trace_list(snapshot);
    let queue_summary = summarize_queue(snapshot);
    let inbox_summary = summarize_inbox(snapshot);

    format!(
        "<!DOCTYPE html>
<html lang=\"en\">
<head>
  <meta charset=\"utf-8\" />
  <title>Claw Local Backend Review Dashboard</title>
  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" />
  <style>
    :root {{ color-scheme: light dark; }}
    body {{ font-family: Inter, system-ui, sans-serif; margin: 2rem auto; max-width: 1200px; padding: 0 1rem 4rem; line-height: 1.5; }}
    .honesty {{ border-left: 4px solid #d97706; padding: 0.75rem 1rem; background: rgba(217, 119, 6, 0.12); }}
    .grid {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(220px, 1fr)); gap: 1rem; margin: 1.5rem 0; }}
    .card {{ border: 1px solid rgba(127,127,127,0.35); border-radius: 12px; padding: 1rem; }}
    .chip-row {{ display: flex; flex-wrap: wrap; gap: 0.5rem; margin-top: 0.75rem; }}
    .chip {{ border: 1px solid rgba(127,127,127,0.35); border-radius: 999px; padding: 0.2rem 0.65rem; font-size: 0.9rem; }}
    table {{ width: 100%; border-collapse: collapse; margin-top: 1rem; }}
    th, td {{ text-align: left; padding: 0.65rem; border-bottom: 1px solid rgba(127,127,127,0.25); vertical-align: top; }}
    code {{ font-family: ui-monospace, SFMono-Regular, monospace; }}
    .muted {{ opacity: 0.8; }}
    .section {{ margin-top: 2rem; }}
    .tight p {{ margin: 0.35rem 0; }}
  </style>
</head>
<body>
  <h1>Claw Local Backend Review Dashboard</h1>
  <p class=\"muted\">Generated from <code>{api_base_url}/v1/state</code>. Static export only.</p>
  <div class=\"honesty\">
    <strong>Bounded surface only.</strong>
    <div>{honesty_note}</div>
    <div class=\"muted\" style=\"margin-top:0.5rem;\">This page consumes persisted backend state and synced review artifacts. It does not claim live browser controls, sessions, or a full operator web app.</div>
  </div>

  <div class=\"grid\">
    <section class=\"card tight\">
      <h2>Service</h2>
      <p><strong>{service_name}</strong> v{service_version}</p>
      <p>Bind address: <code>{bind_address}</code></p>
      <p>Schema: <code>{schema_version}</code></p>
    </section>
    <section class=\"card tight\">
      <h2>Runtime bridge</h2>
      <p>Status: <strong>{runtime_status}</strong></p>
      <p>Latest session: <code>{latest_session}</code></p>
      <p>Bridge file: <code>{runtime_bridge_file}</code></p>
    </section>
    <section class=\"card tight\">
      <h2>Queue summary</h2>
      <p>Revision: <strong>{queue_revision}</strong></p>
      <p>Items: <strong>{queue_count}</strong></p>
      <p>Updated: <code>{queue_updated}</code></p>
      <div class=\"chip-row\">{queue_summary}</div>
    </section>
    <section class=\"card tight\">
      <h2>Inbox summary</h2>
      <p>Status: <strong>{inbox_status}</strong></p>
      <p>Entries: <strong>{inbox_count}</strong></p>
      <p>Synced: <code>{inbox_synced}</code></p>
      <div class=\"chip-row\">{inbox_summary}</div>
    </section>
    <section class=\"card tight\">
      <h2>Mutation boundary</h2>
      <p>Policy loaded: <strong>{policy_loaded}</strong></p>
      <p>Mutation routes allowed: <strong>{mutation_allowed}</strong></p>
      <p>Required ack header: <code>{required_ack_header}</code></p>
      <p class=\"muted\">{mutation_reason}</p>
    </section>
    <section class=\"card tight\">
      <h2>Storage</h2>
      <p>Queue file: <code>{queue_file}</code></p>
      <p>Inbox file: <code>{operator_inbox_file}</code></p>
      <p>Policy file: <code>{auth_policy_file}</code></p>
    </section>
  </div>

  <section class=\"section\">
    <h2>Recent traces</h2>
    <ul>{trace_list}</ul>
  </section>

  <section class=\"section\">
    <h2>Operator queue</h2>
    <table>
      <thead>
        <tr><th>ID</th><th>Title</th><th>Kind</th><th>Status</th><th>Claimed by</th><th>Note</th><th>Source path</th></tr>
      </thead>
      <tbody>{queue_rows}</tbody>
    </table>
  </section>

  <section class=\"section\">
    <h2>Review inbox snapshot</h2>
    <p class=\"muted\">Synced operator review data only. This is a bounded backend-fed review view, not an interactive inbox.</p>
    <table>
      <thead>
        <tr><th>Item</th><th>Trace</th><th>Queue status</th><th>Operator state</th><th>Next step</th><th>Artifacts</th></tr>
      </thead>
      <tbody>{inbox_rows}</tbody>
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
        queue_summary = queue_summary,
        inbox_status = escape_html(&snapshot.operator_inbox.status),
        inbox_count = snapshot.operator_inbox.entry_count,
        inbox_synced = escape_html(snapshot.operator_inbox.synced_at_utc.as_deref().unwrap_or("never")),
        inbox_summary = inbox_summary,
        policy_loaded = if snapshot.auth_boundary.policy_loaded { "yes" } else { "no" },
        mutation_allowed = if snapshot.auth_boundary.mutation_routes_allowed { "yes" } else { "no" },
        required_ack_header = escape_html(snapshot.auth_boundary.required_local_ack_header.as_deref().unwrap_or("none")),
        mutation_reason = escape_html(&snapshot.auth_boundary.mutation_guard_reason),
        queue_file = escape_html(&snapshot.paths.queue_file),
        operator_inbox_file = escape_html(&snapshot.paths.operator_inbox_file),
        auth_policy_file = escape_html(&snapshot.paths.auth_policy_file),
        trace_list = trace_list,
        queue_rows = queue_rows,
        inbox_rows = inbox_rows,
    )
}

fn summarize_queue(snapshot: &BackendSnapshot) -> String {
    let queued = snapshot
        .queue
        .items
        .iter()
        .filter(|item| matches!(item.status, QueueItemStatus::Queued))
        .count();
    let claimed = snapshot
        .queue
        .items
        .iter()
        .filter(|item| matches!(item.status, QueueItemStatus::Claimed))
        .count();
    let review = snapshot
        .queue
        .items
        .iter()
        .filter(|item| {
            matches!(
                item.status,
                QueueItemStatus::InReview | QueueItemStatus::HandoffReady
            )
        })
        .count();
    let completed = snapshot
        .queue
        .items
        .iter()
        .filter(|item| matches!(item.status, QueueItemStatus::Completed))
        .count();
    let dropped = snapshot
        .queue
        .items
        .iter()
        .filter(|item| matches!(item.status, QueueItemStatus::Dropped))
        .count();

    [
        format!("queued: {queued}"),
        format!("claimed: {claimed}"),
        format!("review/handoff: {review}"),
        format!("completed: {completed}"),
        format!("dropped: {dropped}"),
    ]
    .into_iter()
    .map(|label| format!("<span class=\"chip\">{}</span>", escape_html(&label)))
    .collect::<Vec<_>>()
    .join("")
}

fn summarize_inbox(snapshot: &BackendSnapshot) -> String {
    let pending = snapshot
        .operator_inbox
        .entries
        .iter()
        .filter(|entry| {
            !matches!(
                entry.queue_status,
                Some(QueueItemStatus::Completed | QueueItemStatus::Dropped)
            )
        })
        .count();
    let needs_trace = snapshot
        .operator_inbox
        .entries
        .iter()
        .filter(|entry| entry.operator_state.as_deref() == Some("needs trace recovery"))
        .count();
    let with_review_html = snapshot
        .operator_inbox
        .entries
        .iter()
        .filter(|entry| entry.review_html_path.is_some())
        .count();

    [
        format!("pending: {pending}"),
        format!("needs trace recovery: {needs_trace}"),
        format!("html artifacts: {with_review_html}"),
    ]
    .into_iter()
    .map(|label| format!("<span class=\"chip\">{}</span>", escape_html(&label)))
    .collect::<Vec<_>>()
    .join("")
}

fn render_trace_list(snapshot: &BackendSnapshot) -> String {
    if snapshot.runtime_bridge.recent_trace_ids.is_empty() {
        "<li>None captured in the imported bridge snapshot.</li>".to_string()
    } else {
        snapshot
            .runtime_bridge
            .recent_trace_ids
            .iter()
            .map(|trace_id| format!("<li><code>{}</code></li>", escape_html(trace_id)))
            .collect::<Vec<_>>()
            .join("")
    }
}

fn render_queue_rows(snapshot: &BackendSnapshot) -> String {
    if snapshot.queue.items.is_empty() {
        return "<tr><td colspan=\"7\"><em>No queue items yet. The backend is live, but currently empty.</em></td></tr>".to_string();
    }

    let mut queue_rows = String::new();
    for item in &snapshot.queue.items {
        let _ = write!(
            queue_rows,
            "<tr><td><code>{}</code></td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            escape_html(&item.id),
            escape_html(&item.title),
            escape_html(&item.kind),
            escape_html(&format!("{:?}", item.status)),
            escape_html(item.claimed_by.as_deref().unwrap_or("-")),
            escape_html(item.note.as_deref().unwrap_or("-")),
            escape_html(item.source_path.as_deref().unwrap_or("-")),
        );
    }
    queue_rows
}

fn render_inbox_rows(snapshot: &BackendSnapshot) -> String {
    if snapshot.operator_inbox.entries.is_empty() {
        return "<tr><td colspan=\"6\"><em>No synced review inbox entries yet. Run the inbox sync route/command first if static review artifacts exist.</em></td></tr>".to_string();
    }

    let mut rows = String::new();
    for entry in &snapshot.operator_inbox.entries {
        let _ = write!(
            rows,
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            render_inbox_item_cell(entry),
            escape_html(entry.trace_id.as_deref().unwrap_or("-")),
            escape_html(&render_queue_status_label(entry.queue_status.as_ref())),
            escape_html(
                entry
                    .operator_state
                    .as_deref()
                    .unwrap_or(entry.status.as_str())
            ),
            escape_html(entry.next_step.as_deref().unwrap_or("-")),
            render_artifact_cell(entry),
        );
    }
    rows
}

fn render_inbox_item_cell(entry: &OperatorInboxEntry) -> String {
    let mut parts = vec![format!("<strong>{}</strong>", escape_html(&entry.item_id))];
    if let Some(queue_item_id) = &entry.queue_item_id {
        parts.push(format!(
            "<div class=\"muted\">queue item: <code>{}</code></div>",
            escape_html(queue_item_id)
        ));
    }
    if let Some(queue_label) = &entry.queue_label {
        parts.push(format!(
            "<div class=\"muted\">label: {}</div>",
            escape_html(queue_label)
        ));
    }
    if let Some(queue_bucket) = &entry.queue_bucket {
        parts.push(format!(
            "<div class=\"muted\">bucket: {}</div>",
            escape_html(queue_bucket)
        ));
    }
    parts.join("")
}

fn render_artifact_cell(entry: &OperatorInboxEntry) -> String {
    let mut lines = Vec::new();
    if let Some(path) = &entry.review_json_path {
        lines.push(format!("review json: <code>{}</code>", escape_html(path)));
    }
    if let Some(path) = &entry.review_html_path {
        lines.push(format!("review html: <code>{}</code>", escape_html(path)));
    }
    if let Some(path) = &entry.review_status_path {
        lines.push(format!("review status: <code>{}</code>", escape_html(path)));
    }
    if let Some(path) = &entry.approval_packet {
        lines.push(format!(
            "approval packet: <code>{}</code>",
            escape_html(path)
        ));
    }
    if lines.is_empty() {
        "-".to_string()
    } else {
        lines
            .into_iter()
            .map(|line| format!("<div>{line}</div>"))
            .collect::<Vec<_>>()
            .join("")
    }
}

fn render_queue_status_label(status: Option<&QueueItemStatus>) -> String {
    match status {
        Some(status) => format!("{:?}", status),
        None => "unknown".to_string(),
    }
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
        AuthBoundarySnapshot, BackendApiSchema, BackendPaths, OperatorInboxEntry,
        OperatorInboxSnapshot, OperatorQueue, QueueItem, QueueItemStatus,
        RepoAnalysisIndexSnapshot, RuntimeBridgeSnapshot, ServiceConfig, ServiceInfo,
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
                repo_analysis_index_file: ".claw/backend/repo-analysis-index.json".into(),
                auth_policy_file: ".claw/backend/web-operator-auth-policy.json".into(),
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
                items: vec![
                    QueueItem {
                        id: "item-1".into(),
                        title: "Review imported bundle".into(),
                        kind: "repo-analysis-demo".into(),
                        status: QueueItemStatus::Claimed,
                        created_at_utc: "123450".into(),
                        claimed_by: Some("operator-a".into()),
                        note: Some("triage first".into()),
                        source_path: Some(".demo-artifacts/run/operator-handoff.json".into()),
                    },
                    QueueItem {
                        id: "item-2".into(),
                        title: "Needs trace recovery".into(),
                        kind: "web-approval-review".into(),
                        status: QueueItemStatus::InReview,
                        created_at_utc: "123451".into(),
                        claimed_by: Some("operator-b".into()),
                        note: None,
                        source_path: Some(".claw/web-approvals/trace-a.review.json".into()),
                    },
                ],
            },
            operator_inbox: OperatorInboxSnapshot {
                source_file: Some(".claw/backend/operator-inbox.json".into()),
                review_index_file: Some(".claw/web-approvals/index.review.json".into()),
                generated_at_utc: Some("123456".into()),
                synced_at_utc: Some("123457".into()),
                status: "loaded".into(),
                entry_count: 1,
                entries: vec![OperatorInboxEntry {
                    item_id: "inbox-1".into(),
                    trace_id: Some("trace-a".into()),
                    queue_item_id: Some("item-2".into()),
                    status: "queued".into(),
                    queue_bucket: Some("waiting-on-context".into()),
                    queue_label: Some("Needs trace recovery".into()),
                    queue_priority: Some(1),
                    queue_status: Some(QueueItemStatus::InReview),
                    operator_state: Some("needs trace recovery".into()),
                    next_step: Some("inspect replay artifacts".into()),
                    review_json_path: Some(".claw/web-approvals/trace-a.review.json".into()),
                    review_html_path: Some(".claw/web-approvals/trace-a.review.html".into()),
                    review_status_path: Some(
                        ".claw/web-approvals/trace-a.review-status.json".into(),
                    ),
                    approval_packet: Some(".claw/web-approvals/trace-a.json".into()),
                    session_id: Some("session-123".into()),
                    corpus_id: Some("corpus-123".into()),
                    pending_query_count: 2,
                    replay_count: 1,
                    source_updated_at_ms: Some(123460),
                    first_surfaced_at_ms: Some(123450),
                    last_surfaced_at_ms: Some(123470),
                }],
                honesty_note: "Backend-cached operator inbox snapshot only.".into(),
            },
            auth_boundary: AuthBoundarySnapshot {
                policy_loaded: true,
                policy_source: ".claw/backend/web-operator-auth-policy.json".into(),
                mutation_routes_allowed: false,
                mutation_guard_reason: "Local mutation policy not enabled".into(),
                required_local_ack_header: Some("x-claw-local-operator".into()),
            },
            repo_analysis_index: RepoAnalysisIndexSnapshot {
                source_file: Some(".claw/backend/repo-analysis-index.json".into()),
                generated_at_utc: Some("123456".into()),
                synced_at_utc: Some("123457".into()),
                status: "loaded".into(),
                run_count: 0,
                runs: Vec::new(),
                honesty_note: "Backend-cached repo-analysis index snapshot only.".into(),
            },
        }
    }

    #[test]
    fn render_static_status_page_includes_queue_and_inbox_dashboard_sections() {
        let html = render_static_status_page(&sample_snapshot(), "http://127.0.0.1:8787");
        assert!(html.contains("Claw Local Backend Review Dashboard"));
        assert!(html.contains("Bounded surface only."));
        assert!(html.contains("Review imported bundle"));
        assert!(html.contains("Needs trace recovery"));
        assert!(html.contains("inspect replay artifacts"));
        assert!(html.contains("x-claw-local-operator"));
        assert!(html.contains("trace-a"));
    }

    #[test]
    fn render_static_status_page_escapes_html() {
        let mut snapshot = sample_snapshot();
        snapshot.queue.items[0].title = "<script>alert(1)</script>".into();
        snapshot.operator_inbox.entries[0].next_step = Some("<b>inspect</b>".into());
        let html = render_static_status_page(&snapshot, "http://127.0.0.1:8787");
        assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
        assert!(html.contains("&lt;b&gt;inspect&lt;/b&gt;"));
        assert!(!html.contains("<script>alert(1)</script>"));
    }

    #[test]
    fn render_static_status_page_handles_empty_inbox_and_queue() {
        let mut snapshot = sample_snapshot();
        snapshot.queue.items.clear();
        snapshot.operator_inbox.entries.clear();
        snapshot.operator_inbox.entry_count = 0;
        snapshot.operator_inbox.status = "empty".into();
        let html = render_static_status_page(&snapshot, "http://127.0.0.1:8787");
        assert!(html.contains("No queue items yet"));
        assert!(html.contains("No synced review inbox entries yet"));
    }
}
