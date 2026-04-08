use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::budget::RuntimeBudgetUsage;
use crate::corpus::RetrievalResult;
use crate::hybrid::{
    evaluate_web_escalation, format_citations, normalize_local_evidence, summarize_local_evidence,
    EscalationHeuristicInput, EscalationOutcome, EvidenceKind, WebAccessDecision, WebAccessMode,
    WebPolicy,
};
use crate::json::JsonValue;
use crate::trace::{TraceEventType, TraceLedger};
use crate::ux::{
    Citation, ConfidenceLevel, ConfidenceNote, EvidenceProvenance, FinalAnswer, WebExecutionDetail,
    WebExecutionSummary,
};
use telemetry::SessionTracer;

use super::helpers::{
    escalation_reason_label, now_ms, push_trace_event, stop_event_data, task_mentions_freshness,
    task_requests_web,
};
use super::types::{
    ChildSubqueryOutput, RecursiveExecutionMode, RecursiveExecutionResult, RecursiveRuntimeError,
    RecursiveRuntimeState, RecursiveStopReason,
};

fn summarize_web_execution(child_outputs: &[ChildSubqueryOutput]) -> Option<WebExecutionSummary> {
    let mut total = 0usize;
    let mut approved = 0usize;
    let mut approval_required = 0usize;
    let mut skipped = 0usize;
    let mut no_evidence = 0usize;
    let mut failed = 0usize;
    let mut degraded = 0usize;
    let mut succeeded = 0usize;
    let mut succeeded_with_fetched_evidence = 0usize;
    let mut details = Vec::new();

    for output in child_outputs {
        let Some(execution) = output.web_execution.as_ref() else {
            continue;
        };
        total += 1;
        if execution.approved {
            approved += 1;
        }
        match execution.status {
            crate::WebExecutionStatus::ApprovalRequired => approval_required += 1,
            crate::WebExecutionStatus::Skipped => skipped += 1,
            crate::WebExecutionStatus::Succeeded => {
                succeeded += 1;
                if execution.evidence_count > 0 {
                    succeeded_with_fetched_evidence += 1;
                }
            }
            crate::WebExecutionStatus::NoEvidence => no_evidence += 1,
            crate::WebExecutionStatus::Failed => failed += 1,
            crate::WebExecutionStatus::NotRequested => {}
        }
        if execution.degraded {
            degraded += 1;
        }
        details.push(WebExecutionDetail {
            subquery_id: output.subquery_id.clone(),
            status: execution.status.as_str().to_string(),
            approval: if execution.approved {
                "approved".to_string()
            } else {
                "not approved".to_string()
            },
            query: execution.query.clone(),
            evidence_count: execution.evidence_count,
            degraded: execution.degraded,
            note: execution
                .note
                .clone()
                .or_else(|| output.web_execution_note.clone()),
        });
    }

    if total == 0 {
        return None;
    }

    Some(WebExecutionSummary {
        total,
        approved,
        approval_required,
        succeeded,
        succeeded_with_fetched_evidence,
        no_evidence,
        failed,
        skipped,
        degraded,
        details,
    })
}

fn format_recursive_answer(
    body: String,
    retrieval: &RetrievalResult,
    child_outputs: &[ChildSubqueryOutput],
    trace_id: &str,
    escalation: &EscalationOutcome,
) -> String {
    let citations = normalize_local_evidence(retrieval)
        .into_iter()
        .take(4)
        .enumerate()
        .map(|(index, record)| Citation {
            label: format!("L{}", index + 1),
            provenance: EvidenceProvenance::Local,
            title: record.title,
            locator: Some(record.locator),
        })
        .chain(
            child_outputs
                .iter()
                .flat_map(|output| output.citations.iter())
                .enumerate()
                .map(|(index, citation)| Citation {
                    label: format!("C{}", index + 1),
                    provenance: EvidenceProvenance::Local,
                    title: citation.clone(),
                    locator: None,
                }),
        )
        .chain(
            child_outputs
                .iter()
                .flat_map(|output| output.web_evidence.iter())
                .filter(|record| record.kind == EvidenceKind::Web)
                .enumerate()
                .map(|(index, record)| Citation {
                    label: format!("W{}", index + 1),
                    provenance: EvidenceProvenance::Web,
                    title: record.title.clone(),
                    locator: Some(record.locator.clone()),
                }),
        )
        .collect::<Vec<_>>();
    let local_summary = summarize_local_evidence(retrieval);
    let child_citations = format_citations(&normalize_local_evidence(retrieval));
    let mut gaps = if child_citations.is_empty() {
        vec!["No local evidence was retrieved.".to_string()]
    } else {
        Vec::new()
    };
    if matches!(escalation.decision, WebAccessDecision::RequiresApproval) {
        gaps.push(format!(
            "Web escalation requires approval ({}) before fresh external evidence can be fetched.",
            escalation_reason_label(escalation.reason)
        ));
    } else if matches!(escalation.decision, WebAccessDecision::Allowed)
        && !child_outputs
            .iter()
            .any(|output| !output.web_evidence.is_empty())
    {
        gaps.push(format!(
            "Web escalation was allowed ({}) but no web evidence was attached by the child executor.",
            escalation_reason_label(escalation.reason)
        ));
    }
    let web = summarize_web_execution(child_outputs);
    if let Some(summary) = web.as_ref() {
        gaps.push(format!(
            "Web execution summary: web-aware subqueries={}, approved subqueries={}, approval-required subqueries={}, successful web fetches={}, subqueries with fetched web evidence={}, no-evidence outcomes={}, failed web outcomes={}, skipped web paths={}, degraded web outcomes={}.",
            summary.total,
            summary.approved,
            summary.approval_required,
            summary.succeeded,
            summary.succeeded_with_fetched_evidence,
            summary.no_evidence,
            summary.failed,
            summary.skipped,
            summary.degraded,
        ));
    }
    for note in child_outputs
        .iter()
        .filter_map(|output| output.web_execution_note.as_deref())
    {
        if !gaps.iter().any(|existing| existing == note) {
            gaps.push(note.to_string());
        }
    }
    let confidence = ConfidenceNote {
        level: if local_summary.total_hits >= 3 {
            ConfidenceLevel::High
        } else if local_summary.total_hits >= 1 {
            ConfidenceLevel::Medium
        } else {
            ConfidenceLevel::Low
        },
        summary: format!(
            "{} local hits across {} document(s); {} child result(s).",
            local_summary.total_hits,
            local_summary.distinct_documents,
            child_outputs.len()
        ),
        gaps,
    };
    FinalAnswer {
        body,
        citations,
        confidence: Some(confidence),
        web,
        trace_id: Some(trace_id.to_string()),
    }
    .render_text()
}

pub(super) fn finalize_successful_run(
    task: &str,
    mut trace: TraceLedger,
    trace_artifact_dir: Option<&Path>,
    stop_reason: RecursiveStopReason,
    mode: RecursiveExecutionMode,
    retrieval: Option<RetrievalResult>,
    last_escalation: Option<EscalationOutcome>,
    child_outputs: Vec<ChildSubqueryOutput>,
    mut state: RecursiveRuntimeState,
    tracer: Option<&SessionTracer>,
    aggregated_body: Option<String>,
) -> Result<RecursiveExecutionResult, RecursiveRuntimeError> {
    let final_answer = if let Some(ref retrieval) = retrieval {
        let escalation = last_escalation
            .clone()
            .unwrap_or_else(|| default_escalation_outcome(task, retrieval));
        if child_outputs.is_empty() {
            format_recursive_answer(
                format!("Recursive execution completed without child calls for task: {task}"),
                retrieval,
                &child_outputs,
                &trace.trace_id,
                &escalation,
            )
        } else {
            format_recursive_answer(
                aggregated_body
                    .unwrap_or_else(|| format!("No child findings were produced for task: {task}")),
                retrieval,
                &child_outputs,
                &trace.trace_id,
                &escalation,
            )
        }
    } else {
        format!("Recursive execution stopped before any child subqueries for task: {task}")
    };
    push_trace_event(
        &mut trace,
        TraceEventType::AggregationCompleted,
        BTreeMap::from([
            (
                "childCount".to_string(),
                JsonValue::Number(i64::try_from(child_outputs.len()).unwrap_or(i64::MAX)),
            ),
            (
                "iterationCount".to_string(),
                JsonValue::Number(i64::try_from(state.iterations.len()).unwrap_or(i64::MAX)),
            ),
            (
                "finalAnswerChars".to_string(),
                JsonValue::Number(i64::try_from(final_answer.len()).unwrap_or(i64::MAX)),
            ),
        ]),
    );
    push_trace_event(
        &mut trace,
        TraceEventType::StopConditionReached,
        stop_event_data(stop_reason, &child_outputs, &state.usage),
    );
    trace.finished_at_ms = Some(now_ms());
    trace.final_status = stop_reason.trace_status();
    let trace_path = export_trace_if_requested(&trace, trace_artifact_dir)?;
    if let Some(tracer) = tracer {
        trace.emit_telemetry(tracer);
    }
    state.trace_artifact_path = trace_path.clone();

    Ok(RecursiveExecutionResult {
        mode,
        stop_reason,
        final_answer,
        child_outputs,
        retrieval,
        trace,
        trace_artifact_path: trace_path,
        usage: state.usage,
    })
}

pub(super) fn finalize_failed_run(
    task: &str,
    mut trace: TraceLedger,
    trace_artifact_dir: Option<&Path>,
    stop_reason: RecursiveStopReason,
    mode: RecursiveExecutionMode,
    retrieval: Option<RetrievalResult>,
    child_outputs: Vec<ChildSubqueryOutput>,
    usage: RuntimeBudgetUsage,
    tracer: Option<&SessionTracer>,
    error_message: String,
) -> Result<RecursiveExecutionResult, RecursiveRuntimeError> {
    push_trace_event(
        &mut trace,
        TraceEventType::TaskFailed,
        BTreeMap::from([
            (
                "stopReason".to_string(),
                JsonValue::String(stop_reason.as_str().to_string()),
            ),
            (
                "message".to_string(),
                JsonValue::String(error_message.clone()),
            ),
        ]),
    );
    push_trace_event(
        &mut trace,
        TraceEventType::StopConditionReached,
        stop_event_data(stop_reason, &child_outputs, &usage),
    );
    trace.finished_at_ms = Some(now_ms());
    trace.final_status = stop_reason.trace_status();
    let trace_path = export_trace_if_requested(&trace, trace_artifact_dir)?;
    if let Some(tracer) = tracer {
        trace.emit_telemetry(tracer);
    }

    let final_answer = if child_outputs.is_empty() {
        format!("Recursive child execution failed for task: {task}\nReason: {error_message}")
    } else if let Some(ref retrieval) = retrieval {
        let body = format!(
            "Partial recursive findings were produced before a child failed for task: {task}\nFailure: {error_message}"
        );
        format_recursive_answer(
            body,
            retrieval,
            &child_outputs,
            &trace.trace_id,
            &default_escalation_outcome(task, retrieval),
        )
    } else {
        format!("Recursive child execution failed for task: {task}\nReason: {error_message}")
    };

    Ok(RecursiveExecutionResult {
        mode,
        stop_reason,
        final_answer,
        child_outputs,
        retrieval,
        trace,
        trace_artifact_path: trace_path,
        usage,
    })
}

fn default_escalation_outcome(task: &str, retrieval: &RetrievalResult) -> EscalationOutcome {
    evaluate_web_escalation(
        WebPolicy {
            mode: WebAccessMode::Off,
            max_fetches: Some(0),
        },
        EscalationHeuristicInput {
            local_summary: summarize_local_evidence(retrieval),
            requires_external_freshness: task_mentions_freshness(task),
            user_requested_web: task_requests_web(task),
        },
    )
}

fn export_trace_if_requested(
    trace: &TraceLedger,
    trace_artifact_dir: Option<&Path>,
) -> Result<Option<PathBuf>, RecursiveRuntimeError> {
    let Some(dir) = trace_artifact_dir else {
        return Ok(None);
    };
    if dir.as_os_str().is_empty() {
        return Err(RecursiveRuntimeError::InvalidTracePath(
            "trace artifact directory cannot be empty".to_string(),
        ));
    }
    fs::create_dir_all(dir)?;
    let path = dir.join(format!("{}.json", trace.trace_id));
    trace
        .write_to_path(&path)
        .map_err(|error| RecursiveRuntimeError::InvalidTracePath(error.to_string()))?;
    Ok(Some(path))
}

pub(super) fn finalize_empty_stop(
    task: &str,
    mut trace: TraceLedger,
    trace_artifact_dir: Option<&Path>,
    reason: RecursiveStopReason,
    mode: RecursiveExecutionMode,
    retrieval: Option<RetrievalResult>,
    usage: RuntimeBudgetUsage,
    tracer: Option<&SessionTracer>,
) -> Result<RecursiveExecutionResult, RecursiveRuntimeError> {
    push_trace_event(
        &mut trace,
        TraceEventType::StopConditionReached,
        stop_event_data(reason, &[], &usage),
    );
    trace.finished_at_ms = Some(now_ms());
    trace.final_status = reason.trace_status();
    let trace_path = export_trace_if_requested(&trace, trace_artifact_dir)?;
    if let Some(tracer) = tracer {
        trace.emit_telemetry(tracer);
    }
    let final_answer = if let Some(ref retrieval) = retrieval {
        format_recursive_answer(
            format!("Recursive execution stopped before any child subqueries for task: {task}"),
            retrieval,
            &[],
            &trace.trace_id,
            &default_escalation_outcome(task, retrieval),
        )
    } else {
        format!("Recursive execution stopped before any child subqueries for task: {task}")
    };
    Ok(RecursiveExecutionResult {
        mode,
        stop_reason: reason,
        final_answer,
        child_outputs: Vec::new(),
        retrieval,
        trace,
        trace_artifact_path: trace_path,
        usage,
    })
}

#[must_use]
pub fn render_trace_summary(trace: &TraceLedger) -> String {
    let stop_reason = trace
        .events
        .iter()
        .rev()
        .find(|event| event.event_type == TraceEventType::StopConditionReached)
        .and_then(|event| event.data.get("stopReason"))
        .and_then(JsonValue::as_str)
        .unwrap_or("unknown");
    let counters = trace.counters();
    format!(
        "Trace\n  Id               {}\n  Session          {}\n  Task             {}\n  Status           {}\n  Stop reason      {}\n  Events           {}\n  Retrievals       {} / {}\n  Subqueries       {} / {}\n  Web escalations  {}\n  Web executions   {}\n  Web approved     {}\n  Web pending      {}\n  Web succeeded    {}\n  Web no evidence  {}\n  Web failed       {}\n  Web skipped      {}\n  Web degraded     {}\n  Web evidence     {}",
        trace.trace_id,
        trace.session_id,
        trace.root_task_id,
        trace.final_status.as_str(),
        stop_reason,
        trace.events.len(),
        counters.retrieval_completions,
        counters.retrieval_requests,
        counters.subqueries_completed,
        counters.subqueries_started,
        counters.web_escalations,
        counters.web_executions_completed,
        counters.web_execution_approved,
        counters.web_execution_approval_required,
        counters.web_execution_succeeded,
        counters.web_execution_no_evidence,
        counters.web_execution_failed,
        counters.web_execution_skipped,
        counters.degraded_web_executions,
        counters.web_evidence_items,
    )
}

pub fn export_trace(
    trace: &TraceLedger,
    destination: &Path,
) -> Result<PathBuf, RecursiveRuntimeError> {
    let destination = if destination.extension().is_none() {
        destination.join(format!("{}.json", trace.trace_id))
    } else {
        destination.to_path_buf()
    };
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    trace
        .write_to_path(&destination)
        .map_err(|error| RecursiveRuntimeError::InvalidTracePath(error.to_string()))?;
    Ok(destination)
}
