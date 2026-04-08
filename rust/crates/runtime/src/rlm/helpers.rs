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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RetrievalPlan {
    pub query: String,
    pub strategy: &'static str,
    pub rationale: &'static str,
    pub anchor_terms: Vec<String>,
    pub gap_terms: Vec<String>,
    pub validation_terms: Vec<String>,
}

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

fn is_stopword(token: &str) -> bool {
    matches!(
        token,
        "about"
            | "after"
            | "also"
            | "around"
            | "because"
            | "before"
            | "being"
            | "between"
            | "check"
            | "child"
            | "cite"
            | "concrete"
            | "could"
            | "evidence"
            | "export"
            | "finding"
            | "findings"
            | "first"
            | "found"
            | "from"
            | "grounded"
            | "happen"
            | "hidden"
            | "inspection"
            | "inspected"
            | "iteration"
            | "local"
            | "manual"
            | "missing"
            | "needs"
            | "next"
            | "note"
            | "observed"
            | "operator"
            | "pass"
            | "prior"
            | "produced"
            | "remaining"
            | "repeat"
            | "repeating"
            | "response"
            | "search"
            | "should"
            | "slice"
            | "slices"
            | "state"
            | "step"
            | "still"
            | "summary"
            | "support"
            | "task"
            | "that"
            | "their"
            | "them"
            | "there"
            | "these"
            | "they"
            | "this"
            | "trace"
            | "validation"
            | "verify"
            | "what"
            | "when"
            | "with"
            | "without"
            | "workflow"
    )
}

fn collect_terms<'a>(sources: impl IntoIterator<Item = &'a str>, limit: usize) -> Vec<String> {
    let mut terms = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for source in sources {
        for token in source
            .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '-' && ch != '_')
            .filter(|token| token.len() >= 5)
            .map(|token| {
                token
                    .trim_matches('-')
                    .trim_matches('_')
                    .to_ascii_lowercase()
            })
        {
            if token.is_empty() || is_stopword(&token) || !seen.insert(token.clone()) {
                continue;
            }
            terms.push(token);
            if terms.len() >= limit {
                return terms;
            }
        }
    }
    terms
}

fn collect_anchor_terms(task: &str, child_outputs: &[ChildSubqueryOutput]) -> Vec<String> {
    collect_terms(
        std::iter::once(task).chain(
            child_outputs
                .iter()
                .rev()
                .map(|output| output.answer.as_str()),
        ),
        6,
    )
}

fn extract_section_terms(answer: &str, heading: &str, limit: usize) -> Vec<String> {
    let mut captured = Vec::new();
    let mut in_section = false;
    for raw_line in answer.lines() {
        let trimmed = raw_line.trim();
        let normalized = trimmed
            .trim_start_matches(|ch: char| {
                ch.is_ascii_digit() || ch == '.' || ch == ')' || ch == '-'
            })
            .trim();
        let lower = normalized.to_ascii_lowercase();
        if lower.starts_with(heading) {
            in_section = true;
            let remainder = normalized
                .split_once(':')
                .map(|(_, tail)| tail.trim())
                .unwrap_or("");
            if !remainder.is_empty() {
                captured.push(remainder.to_string());
            }
            continue;
        }
        if in_section {
            if lower.starts_with("findings:")
                || lower.starts_with("evidence used:")
                || lower.starts_with("validation loop:")
                || lower.starts_with("remaining gaps:")
            {
                break;
            }
            if !normalized.is_empty() {
                captured.push(normalized.to_string());
            }
        }
    }
    collect_terms(captured.iter().map(String::as_str), limit)
}

pub(super) fn build_iteration_plan(
    task: &str,
    child_outputs: &[ChildSubqueryOutput],
) -> RetrievalPlan {
    if child_outputs.is_empty() {
        return RetrievalPlan {
            query: task.to_string(),
            strategy: "bootstrap",
            rationale: "start from the original task before any child evidence exists",
            anchor_terms: collect_anchor_terms(task, child_outputs),
            gap_terms: Vec::new(),
            validation_terms: Vec::new(),
        };
    }

    let anchor_terms = collect_anchor_terms(task, child_outputs);
    let gap_terms = child_outputs
        .last()
        .map(|last| extract_section_terms(&last.answer, "remaining gaps", 4))
        .unwrap_or_default();
    let validation_terms = child_outputs
        .last()
        .map(|last| extract_section_terms(&last.answer, "validation loop", 4))
        .unwrap_or_default();
    let mut query_parts = vec![task.to_string()];
    if let Some(last) = child_outputs.last() {
        query_parts.push(last.answer.clone());
        if !last.citations.is_empty() {
            query_parts.push(last.citations.join(" "));
        }
    }
    if !gap_terms.is_empty() {
        query_parts.push(format!("remaining gaps {}", gap_terms.join(" ")));
    }
    if !validation_terms.is_empty() {
        query_parts.push(format!("validation {}", validation_terms.join(" ")));
    }
    if !anchor_terms.is_empty() {
        query_parts.push(anchor_terms.join(" "));
    }

    let strategy = if !gap_terms.is_empty() {
        "gap_targeted_followup"
    } else if child_outputs.len() >= 2 {
        "gap_followup"
    } else {
        "evidence_followup"
    };
    let rationale = if !gap_terms.is_empty() {
        "re-query by carrying forward explicit remaining-gap and validation-loop terms from the last child response instead of only replaying the summary"
    } else if child_outputs.len() >= 2 {
        "re-query with prior findings plus stable anchor terms to chase remaining gaps instead of only echoing the last answer"
    } else {
        "re-query with the first child result and stable anchor terms to broaden evidence coverage"
    };

    RetrievalPlan {
        query: query_parts.join(" "),
        strategy,
        rationale,
        anchor_terms,
        gap_terms,
        validation_terms,
    }
}

pub(super) fn build_child_prompt(
    task: &str,
    iteration: u32,
    prior_outputs: &[ChildSubqueryOutput],
    slices: &[RecursiveContextSlice],
) -> String {
    let mut prompt = format!(
        "Task: {task}\nIteration: {iteration}\nUse only the provided slices. Stay grounded in observed evidence.\n\nRequired response shape:\n1. Findings: concise answer tied to the task.\n2. Evidence used: cite the slice ids or citations that support the findings.\n3. Validation loop: name one concrete check, repo inspection, build/test step, or operator verification that should happen next.\n4. Remaining gaps: state what is still uncertain or missing.\n"
    );
    if !prior_outputs.is_empty() {
        prompt.push_str("\nPrior child findings (avoid repeating them unless you are correcting or validating them):\n");
        for output in prior_outputs {
            prompt.push_str("- ");
            prompt.push_str(&output.answer);
            if !output.citations.is_empty() {
                prompt.push_str(" [citations: ");
                prompt.push_str(&output.citations.join(", "));
                prompt.push(']');
            }
            if let Some(note) = output.web_execution_note.as_deref() {
                prompt.push_str(" [note: ");
                prompt.push_str(note);
                prompt.push(']');
            }
            prompt.push('\n');
        }
    }
    prompt.push_str("\nAvailable slices:\n");
    for slice in slices {
        let text = slice_text(&slice.metadata, &slice.preview);
        prompt.push_str("\n");
        prompt.push_str(&format!(
            "[{}] {}#{} ({}-{})\n{}\n",
            slice.chunk_id, slice.path, slice.ordinal, slice.start_offset, slice.end_offset, text
        ));
    }
    prompt.push_str(
        "\nDo not claim completion just because you produced a summary. Prefer verifiable next steps and explicitly say when the slices are not enough.\n",
    );
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

pub(super) fn child_output_signature(output: &ChildSubqueryOutput) -> String {
    let normalized_answer = output
        .answer
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let mut citations = output.citations.clone();
    citations.sort();
    let mut web_ids = output
        .web_evidence
        .iter()
        .map(|record| record.id.clone())
        .collect::<Vec<_>>();
    web_ids.sort();
    format!(
        "answer={normalized_answer}|citations={}|web={}|web_note={}",
        citations.join(","),
        web_ids.join(","),
        output.web_execution_note.as_deref().unwrap_or("")
    )
}

fn normalized_answer_tokens(answer: &str) -> Vec<String> {
    answer
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| token.len() >= 4)
        .map(|token| token.to_ascii_lowercase())
        .collect()
}

pub(super) fn child_output_novelty_metrics(
    child_outputs: &[ChildSubqueryOutput],
) -> Option<(usize, usize, usize, bool)> {
    let (last, prior) = child_outputs.split_last()?;
    let previous = prior.last()?;

    let prior_citations = prior
        .iter()
        .flat_map(|output| output.citations.iter().cloned())
        .collect::<std::collections::BTreeSet<_>>();
    let prior_web_ids = prior
        .iter()
        .flat_map(|output| output.web_evidence.iter().map(|record| record.id.clone()))
        .collect::<std::collections::BTreeSet<_>>();
    let previous_answer_tokens = normalized_answer_tokens(&previous.answer)
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>();
    let current_answer_tokens = normalized_answer_tokens(&last.answer)
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>();

    let novel_citations = last
        .citations
        .iter()
        .filter(|citation| !prior_citations.contains(*citation))
        .count();
    let novel_web_ids = last
        .web_evidence
        .iter()
        .filter(|record| !prior_web_ids.contains(&record.id))
        .count();
    let novel_answer_tokens = current_answer_tokens
        .difference(&previous_answer_tokens)
        .count();
    let materially_novel = novel_citations > 0 || novel_web_ids > 0 || novel_answer_tokens >= 3;

    Some((
        novel_citations,
        novel_web_ids,
        novel_answer_tokens,
        materially_novel,
    ))
}

pub(super) fn should_stop_for_convergence(child_outputs: &[ChildSubqueryOutput]) -> bool {
    let Some((last, prior)) = child_outputs.split_last() else {
        return false;
    };
    if prior
        .last()
        .is_some_and(|previous| child_output_signature(previous) == child_output_signature(last))
    {
        return true;
    }

    child_output_novelty_metrics(child_outputs)
        .is_some_and(|(_, _, _, materially_novel)| !materially_novel)
}

pub(super) fn no_iteration_artifacts_stop_reason(
    child_outputs: &[ChildSubqueryOutput],
) -> RecursiveStopReason {
    if child_outputs.is_empty() {
        RecursiveStopReason::NoChildCapacity
    } else {
        RecursiveStopReason::NoNewContext
    }
}

pub(super) fn stop_event_data(
    reason: RecursiveStopReason,
    child_outputs: &[ChildSubqueryOutput],
    usage: &RuntimeBudgetUsage,
) -> BTreeMap<String, JsonValue> {
    BTreeMap::from([
        (
            "stopReason".to_string(),
            JsonValue::String(reason.as_str().to_string()),
        ),
        (
            "childCount".to_string(),
            JsonValue::Number(i64::try_from(child_outputs.len()).unwrap_or(i64::MAX)),
        ),
        (
            "completedIterations".to_string(),
            JsonValue::Number(i64::from(usage.iterations)),
        ),
        (
            "subcalls".to_string(),
            JsonValue::Number(i64::from(usage.subcalls)),
        ),
    ])
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hybrid::WebExecutionOutcome;

    fn sample_child_output(answer: &str) -> ChildSubqueryOutput {
        ChildSubqueryOutput {
            subquery_id: "child-1".to_string(),
            answer: answer.to_string(),
            citations: vec!["chunk-1".to_string()],
            web_evidence: Vec::new(),
            web_execution: Some(WebExecutionOutcome::not_requested()),
            web_execution_note: None,
            prompt_tokens: 0,
            completion_tokens: 0,
            cost_usd: 0.0,
        }
    }

    #[test]
    fn no_iteration_artifacts_distinguishes_bootstrap_vs_exhausted_context() {
        assert_eq!(
            no_iteration_artifacts_stop_reason(&[]),
            RecursiveStopReason::NoChildCapacity
        );
        assert_eq!(
            no_iteration_artifacts_stop_reason(&[sample_child_output("done")]),
            RecursiveStopReason::NoNewContext
        );
    }

    #[test]
    fn stop_event_data_captures_counts_for_trace_consistency() {
        let usage = RuntimeBudgetUsage {
            iterations: 2,
            subcalls: 2,
            ..RuntimeBudgetUsage::default()
        };
        let data = stop_event_data(
            RecursiveStopReason::Converged,
            &[sample_child_output("stable answer")],
            &usage,
        );

        assert_eq!(
            data.get("stopReason"),
            Some(&JsonValue::String("converged".to_string()))
        );
        assert_eq!(data.get("childCount"), Some(&JsonValue::Number(1)));
        assert_eq!(data.get("completedIterations"), Some(&JsonValue::Number(2)));
        assert_eq!(data.get("subcalls"), Some(&JsonValue::Number(2)));
    }

    #[test]
    fn bootstrap_iteration_plan_starts_from_task() {
        let plan = build_iteration_plan("audit recursive runtime planner gaps", &[]);

        assert_eq!(plan.strategy, "bootstrap");
        assert_eq!(plan.query, "audit recursive runtime planner gaps");
        assert!(plan.anchor_terms.contains(&"recursive".to_string()));
        assert!(plan.anchor_terms.contains(&"runtime".to_string()));
        assert!(plan.anchor_terms.contains(&"planner".to_string()));
    }

    #[test]
    fn follow_up_iteration_plan_carries_anchor_terms_forward() {
        let first = sample_child_output(
            "first pass found recursive scheduler weakness and attestation verification gap",
        );
        let second = sample_child_output(
            "second pass confirmed scheduler weakness but still needs provenance bundle validation",
        );
        let plan = build_iteration_plan(
            "audit recursive scheduler and provenance bundle",
            &[first, second],
        );

        assert_eq!(plan.strategy, "gap_followup");
        assert!(plan
            .query
            .contains("second pass confirmed scheduler weakness"));
        assert!(plan.anchor_terms.contains(&"scheduler".to_string()));
        assert!(plan.anchor_terms.contains(&"provenance".to_string()));
        assert!(plan.anchor_terms.contains(&"bundle".to_string()));
    }

    #[test]
    fn gap_targeted_iteration_plan_extracts_remaining_gaps_and_validation_terms() {
        let first = sample_child_output(
            "Findings: grounded answer\nValidation loop: run cargo test -p runtime recursive_trace\nRemaining gaps: planner policy still lacks adaptive retry budgeting and stress coverage",
        );
        let plan = build_iteration_plan("audit recursive planner maturity", &[first]);

        assert_eq!(plan.strategy, "gap_targeted_followup");
        assert!(plan.gap_terms.contains(&"planner".to_string()));
        assert!(plan.gap_terms.contains(&"adaptive".to_string()));
        assert!(plan.validation_terms.contains(&"cargo".to_string()));
        assert!(plan.validation_terms.contains(&"runtime".to_string()));
        assert!(plan.query.contains("remaining gaps"));
        assert!(plan.query.contains("validation"));
    }
}
