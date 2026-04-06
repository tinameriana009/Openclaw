use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::budget::{RuntimeBudget, RuntimeBudgetUsage};
use crate::corpus::CorpusChunk;
use crate::hybrid::{EscalationReason, WebAccessDecision, WebAccessMode, WebPolicy};
use crate::json::JsonValue;
use crate::trace::{TraceEvent, TraceEventType, TraceLedger};

use super::types::{
    ChildSubqueryOutput, RecursiveContextSlice, RecursiveExecutionMode, RecursiveStopReason,
};

pub(super) fn map_chunk(path: &str, chunk: &CorpusChunk) -> RecursiveContextSlice {
    RecursiveContextSlice {
        chunk_id: chunk.chunk_id.clone(),
        document_id: chunk.document_id.clone(),
        path: path.to_string(),
        ordinal: chunk.ordinal,
        start_offset: chunk.start_offset,
        end_offset: chunk.end_offset,
        preview: chunk.text_preview.clone(),
        metadata: chunk.metadata.clone(),
    }
}

pub(super) fn slice_text(metadata: &BTreeMap<String, JsonValue>, preview: &str) -> String {
    metadata
        .get("text")
        .and_then(JsonValue::as_str)
        .filter(|text| !text.is_empty())
        .unwrap_or(preview)
        .to_string()
}

pub(super) fn build_iteration_query(task: &str, child_outputs: &[ChildSubqueryOutput]) -> String {
    if let Some(last) = child_outputs.last() {
        format!("{task} {} {}", last.answer, last.citations.join(" "))
    } else {
        task.to_string()
    }
}

pub(super) fn build_child_prompt(
    task: &str,
    iteration: u32,
    prior_outputs: &[ChildSubqueryOutput],
    slices: &[RecursiveContextSlice],
) -> String {
    let mut prompt =
        format!("Task: {task}\nIteration: {iteration}\nUse only the provided slices.\n");
    if !prior_outputs.is_empty() {
        prompt.push_str("Prior child findings:\n");
        for output in prior_outputs {
            prompt.push_str("- ");
            prompt.push_str(&output.answer);
            prompt.push('\n');
        }
    }
    for slice in slices {
        let text = slice_text(&slice.metadata, &slice.preview);
        prompt.push_str("\n");
        prompt.push_str(&format!(
            "[{}] {}#{} ({}-{})\n{}\n",
            slice.chunk_id, slice.path, slice.ordinal, slice.start_offset, slice.end_offset, text
        ));
    }
    prompt
}

pub(super) fn next_iteration_stop_reason(
    budget: &RuntimeBudget,
    usage: &RuntimeBudgetUsage,
) -> Option<RecursiveStopReason> {
    if let Some(limit) = budget.max_iterations {
        if usage.iterations >= limit {
            return Some(RecursiveStopReason::IterationCap);
        }
    }
    if let Some(limit) = budget.max_subcalls {
        if usage.subcalls >= limit {
            return Some(RecursiveStopReason::SubcallCap);
        }
    }
    if let Some(limit) = budget.max_runtime_ms {
        if usage.runtime_ms >= limit {
            return Some(RecursiveStopReason::Timeout);
        }
    }
    if let Some(limit) = budget.max_prompt_tokens {
        if usage.prompt_tokens >= limit {
            return Some(RecursiveStopReason::PromptTokenCap);
        }
    }
    if let Some(limit) = budget.max_completion_tokens {
        if usage.completion_tokens >= limit {
            return Some(RecursiveStopReason::CompletionTokenCap);
        }
    }
    if let Some(limit) = budget.max_cost_usd {
        if usage.cost_usd >= limit {
            return Some(RecursiveStopReason::CostCap);
        }
    }
    None
}

pub(super) fn push_trace_event(
    trace: &mut TraceLedger,
    event_type: TraceEventType,
    data: BTreeMap<String, JsonValue>,
) {
    let sequence = u32::try_from(trace.events.len() + 1).unwrap_or(u32::MAX);
    trace
        .events
        .push(TraceEvent::new(sequence, event_type, now_ms(), data));
}

pub(super) fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should be after epoch")
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

pub(super) fn mode_label(mode: RecursiveExecutionMode) -> &'static str {
    match mode {
        RecursiveExecutionMode::Direct => "direct",
        RecursiveExecutionMode::Rag => "rag",
        RecursiveExecutionMode::Rlm => "rlm",
    }
}

pub(super) fn task_requests_web(task: &str) -> bool {
    let lowered = task.to_ascii_lowercase();
    ["web", "online", "internet", "search the web", "browse"]
        .iter()
        .any(|needle| lowered.contains(needle))
}

pub(super) fn task_mentions_freshness(task: &str) -> bool {
    let lowered = task.to_ascii_lowercase();
    [
        "latest",
        "current",
        "today",
        "recent",
        "newest",
        "up-to-date",
        "fresh",
    ]
    .iter()
    .any(|needle| lowered.contains(needle))
}

pub(super) fn effective_child_web_policy(
    parent: WebPolicy,
    escalation: crate::hybrid::EscalationOutcome,
) -> WebPolicy {
    let max_fetches = parent.max_fetches;
    match escalation.decision {
        WebAccessDecision::Denied => WebPolicy {
            mode: WebAccessMode::Off,
            max_fetches: Some(0),
        },
        WebAccessDecision::RequiresApproval => parent.inherit_for_child(Some(&WebPolicy {
            mode: WebAccessMode::Ask,
            max_fetches,
        })),
        WebAccessDecision::Allowed => parent.inherit_for_child(Some(&WebPolicy {
            mode: WebAccessMode::On,
            max_fetches,
        })),
    }
}

pub(super) fn web_policy_label(mode: WebAccessMode) -> &'static str {
    match mode {
        WebAccessMode::Off => "off",
        WebAccessMode::Ask => "ask",
        WebAccessMode::On => "on",
    }
}

pub(super) fn web_access_decision_label(decision: WebAccessDecision) -> &'static str {
    match decision {
        WebAccessDecision::Denied => "denied",
        WebAccessDecision::RequiresApproval => "requires_approval",
        WebAccessDecision::Allowed => "allowed",
    }
}

pub(super) fn escalation_reason_label(reason: EscalationReason) -> &'static str {
    match reason {
        EscalationReason::UserRequestedWeb => "user_requested_web",
        EscalationReason::NoLocalEvidence => "no_local_evidence",
        EscalationReason::WeakLocalEvidence => "weak_local_evidence",
        EscalationReason::FreshnessRequired => "freshness_required",
        EscalationReason::LocalEvidenceSufficient => "local_evidence_sufficient",
        EscalationReason::PolicyDenied => "policy_denied",
    }
}
