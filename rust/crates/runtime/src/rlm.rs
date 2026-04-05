use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::budget::{BudgetSliceRequest, BudgetStopReason, RuntimeBudget, RuntimeBudgetUsage};
use crate::corpus::{CorpusChunk, CorpusManifest, RetrievalHit, RetrievalResult};
use crate::hybrid::{
    evaluate_web_escalation, format_citations, is_local_evidence_weak, normalize_local_evidence,
    summarize_local_evidence, web_evidence_trace_event, EscalationHeuristicInput,
    EscalationOutcome, EscalationReason, EvidenceKind, EvidenceRecord, WebAccessDecision,
    WebAccessMode, WebPolicy,
};
use crate::json::JsonValue;
use crate::trace::{TraceEvent, TraceEventType, TraceFinalStatus, TraceLedger};
use crate::ux::{Citation, ConfidenceLevel, ConfidenceNote, EvidenceProvenance, FinalAnswer};
use telemetry::SessionTracer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecursiveExecutionMode {
    Direct,
    Rag,
    Rlm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecursiveStopReason {
    Completed,
    NoChildCapacity,
    DepthCap,
    IterationCap,
    SubcallCap,
    Timeout,
    PromptTokenCap,
    CompletionTokenCap,
    CostCap,
}

impl RecursiveStopReason {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::NoChildCapacity => "no_child_capacity",
            Self::DepthCap => "depth_cap",
            Self::IterationCap => "iteration_cap",
            Self::SubcallCap => "subcall_cap",
            Self::Timeout => "timeout",
            Self::PromptTokenCap => "prompt_token_cap",
            Self::CompletionTokenCap => "completion_token_cap",
            Self::CostCap => "cost_cap",
        }
    }

    #[must_use]
    pub fn trace_status(self) -> TraceFinalStatus {
        match self {
            Self::Completed | Self::NoChildCapacity => TraceFinalStatus::Succeeded,
            Self::DepthCap
            | Self::IterationCap
            | Self::SubcallCap
            | Self::Timeout
            | Self::PromptTokenCap
            | Self::CompletionTokenCap
            | Self::CostCap => TraceFinalStatus::BudgetExceeded,
        }
    }
}

impl From<BudgetStopReason> for RecursiveStopReason {
    fn from(value: BudgetStopReason) -> Self {
        match value {
            BudgetStopReason::DepthExceeded { .. } => Self::DepthCap,
            BudgetStopReason::IterationsExceeded { .. } => Self::IterationCap,
            BudgetStopReason::SubcallsExceeded { .. } => Self::SubcallCap,
            BudgetStopReason::RuntimeExceeded { .. } => Self::Timeout,
            BudgetStopReason::PromptTokensExceeded { .. } => Self::PromptTokenCap,
            BudgetStopReason::CompletionTokensExceeded { .. } => Self::CompletionTokenCap,
            BudgetStopReason::CostExceeded { .. } => Self::CostCap,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecursiveRuntimeState {
    pub session_id: String,
    pub task_id: String,
    pub mode: RecursiveExecutionMode,
    pub budget: RuntimeBudget,
    pub usage: RuntimeBudgetUsage,
    pub iterations: Vec<RecursiveIterationState>,
    pub trace_artifact_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecursiveIterationState {
    pub iteration: u32,
    pub child_count: usize,
    pub selected_chunk_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecursiveCorpusPeekResult {
    pub corpus_id: String,
    pub document_count: u32,
    pub chunk_count: u32,
    pub roots: Vec<String>,
    pub top_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecursiveContextSlice {
    pub chunk_id: String,
    pub document_id: String,
    pub path: String,
    pub ordinal: u32,
    pub start_offset: u32,
    pub end_offset: u32,
    pub preview: String,
    pub metadata: BTreeMap<String, JsonValue>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChildSubqueryRequest {
    pub subquery_id: String,
    pub prompt: String,
    pub slices: Vec<RecursiveContextSlice>,
    pub budget: RuntimeBudget,
    pub web_policy: WebPolicy,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChildSubqueryOutput {
    pub subquery_id: String,
    pub answer: String,
    pub citations: Vec<String>,
    pub web_evidence: Vec<EvidenceRecord>,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub cost_usd: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecursiveExecutionResult {
    pub mode: RecursiveExecutionMode,
    pub stop_reason: RecursiveStopReason,
    pub final_answer: String,
    pub child_outputs: Vec<ChildSubqueryOutput>,
    pub retrieval: Option<RetrievalResult>,
    pub trace: TraceLedger,
    pub trace_artifact_path: Option<PathBuf>,
    pub usage: RuntimeBudgetUsage,
}

pub trait ChildSubqueryExecutor {
    fn execute(
        &self,
        request: &ChildSubqueryRequest,
    ) -> Result<ChildSubqueryOutput, RecursiveRuntimeError>;
}

pub trait ChildOutputAggregator {
    fn aggregate(&self, task: &str, child_outputs: &[ChildSubqueryOutput]) -> String;
}

#[derive(Debug, Clone)]
pub struct DefaultChildOutputAggregator;

impl ChildOutputAggregator for DefaultChildOutputAggregator {
    fn aggregate(&self, task: &str, child_outputs: &[ChildSubqueryOutput]) -> String {
        if child_outputs.is_empty() {
            return format!("No child findings were produced for task: {task}");
        }

        let mut answer = format!("Task: {task}\n");
        for output in child_outputs {
            answer.push_str("\n");
            answer.push_str("- ");
            answer.push_str(&output.answer);
            if !output.citations.is_empty() {
                answer.push_str(" [sources: ");
                answer.push_str(&output.citations.join(", "));
                answer.push(']');
            }
        }
        answer
    }
}

#[derive(Debug)]
pub enum RecursiveRuntimeError {
    Io(std::io::Error),
    MissingChunk(String),
    InvalidTracePath(String),
    ChildExecution(String),
}

impl Display for RecursiveRuntimeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::MissingChunk(chunk_id) => write!(f, "missing chunk {chunk_id}"),
            Self::InvalidTracePath(message) => write!(f, "{message}"),
            Self::ChildExecution(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for RecursiveRuntimeError {}

impl From<std::io::Error> for RecursiveRuntimeError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

pub struct RecursiveConversationRuntime<'a, E, A = DefaultChildOutputAggregator> {
    corpus: &'a CorpusManifest,
    executor: E,
    aggregator: A,
}

impl<'a, E> RecursiveConversationRuntime<'a, E, DefaultChildOutputAggregator>
where
    E: ChildSubqueryExecutor,
{
    #[must_use]
    pub fn new(corpus: &'a CorpusManifest, executor: E) -> Self {
        Self {
            corpus,
            executor,
            aggregator: DefaultChildOutputAggregator,
        }
    }
}

impl<'a, E, A> RecursiveConversationRuntime<'a, E, A>
where
    E: ChildSubqueryExecutor,
    A: ChildOutputAggregator,
{
    #[must_use]
    pub fn with_aggregator(corpus: &'a CorpusManifest, executor: E, aggregator: A) -> Self {
        Self {
            corpus,
            executor,
            aggregator,
        }
    }

    #[must_use]
    pub fn select_mode(
        task: &str,
        corpus: Option<&CorpusManifest>,
        budget: &RuntimeBudget,
    ) -> RecursiveExecutionMode {
        if budget.max_depth.unwrap_or(0) >= 1 && corpus.is_some() {
            RecursiveExecutionMode::Rlm
        } else if corpus.is_some() && !task.trim().is_empty() {
            RecursiveExecutionMode::Rag
        } else {
            RecursiveExecutionMode::Direct
        }
    }

    #[must_use]
    pub fn corpus_peek(&self) -> RecursiveCorpusPeekResult {
        RecursiveCorpusPeekResult {
            corpus_id: self.corpus.corpus_id.clone(),
            document_count: self.corpus.document_count,
            chunk_count: self.corpus.chunk_count,
            roots: self.corpus.roots.clone(),
            top_paths: self
                .corpus
                .documents
                .iter()
                .take(5)
                .map(|document| document.path.clone())
                .collect(),
        }
    }

    #[must_use]
    pub fn corpus_search(&self, query: &str, top_k: usize) -> RetrievalResult {
        let lowered_terms = query
            .split_whitespace()
            .map(|term| term.to_ascii_lowercase())
            .filter(|term| !term.is_empty())
            .collect::<Vec<_>>();
        let started = now_ms();
        let mut hits = Vec::new();

        for document in &self.corpus.documents {
            let lowered_path = document.path.to_ascii_lowercase();
            let lowered_headings = document
                .headings
                .iter()
                .map(|heading| heading.to_ascii_lowercase())
                .collect::<Vec<_>>();
            for chunk in &document.chunks {
                let searchable_text = slice_text(&chunk.metadata, &chunk.text_preview);
                let lowered_text = searchable_text.to_ascii_lowercase();
                let mut score = 0.0_f64;
                let mut reasons = Vec::new();
                for term in &lowered_terms {
                    if lowered_path.contains(term) {
                        score += 3.0;
                        reasons.push(format!("path:{term}"));
                    }
                    if lowered_headings
                        .iter()
                        .any(|heading| heading.contains(term))
                    {
                        score += 2.0;
                        reasons.push(format!("heading:{term}"));
                    }
                    let content_hits = lowered_text.matches(term).count();
                    if content_hits > 0 {
                        score += content_hits as f64;
                        reasons.push(format!("content:{term}x{content_hits}"));
                    }
                }
                if score > 0.0 {
                    hits.push(RetrievalHit {
                        chunk_id: chunk.chunk_id.clone(),
                        document_id: document.document_id.clone(),
                        path: document.path.clone(),
                        score,
                        reason: reasons.join(","),
                        preview: chunk.text_preview.clone(),
                    });
                }
            }
        }

        hits.sort_by(|left, right| {
            right
                .score
                .partial_cmp(&left.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.path.cmp(&right.path))
                .then_with(|| left.chunk_id.cmp(&right.chunk_id))
        });
        hits.truncate(top_k);

        RetrievalResult {
            query: query.to_string(),
            backend: self.corpus.backend,
            elapsed_ms: now_ms().saturating_sub(started),
            hits,
        }
    }

    pub fn select_slices(
        &self,
        chunk_ids: &[String],
    ) -> Result<Vec<RecursiveContextSlice>, RecursiveRuntimeError> {
        chunk_ids
            .iter()
            .map(|chunk_id| self.slice_by_chunk_id(chunk_id))
            .collect()
    }

    pub fn run(
        &self,
        session_id: &str,
        task_id: &str,
        task: &str,
        budget: RuntimeBudget,
        trace_artifact_dir: Option<&Path>,
    ) -> Result<RecursiveExecutionResult, RecursiveRuntimeError> {
        self.run_with_tracer_and_policy(
            session_id,
            task_id,
            task,
            budget,
            trace_artifact_dir,
            None,
            WebPolicy {
                mode: WebAccessMode::Off,
                max_fetches: Some(0),
            },
        )
    }

    pub fn run_with_tracer(
        &self,
        session_id: &str,
        task_id: &str,
        task: &str,
        budget: RuntimeBudget,
        trace_artifact_dir: Option<&Path>,
        tracer: Option<&SessionTracer>,
    ) -> Result<RecursiveExecutionResult, RecursiveRuntimeError> {
        self.run_with_tracer_and_policy(
            session_id,
            task_id,
            task,
            budget,
            trace_artifact_dir,
            tracer,
            WebPolicy {
                mode: WebAccessMode::Off,
                max_fetches: Some(0),
            },
        )
    }

    pub fn run_with_tracer_and_policy(
        &self,
        session_id: &str,
        task_id: &str,
        task: &str,
        budget: RuntimeBudget,
        trace_artifact_dir: Option<&Path>,
        tracer: Option<&SessionTracer>,
        web_policy: WebPolicy,
    ) -> Result<RecursiveExecutionResult, RecursiveRuntimeError> {
        let mode = Self::select_mode(task, Some(self.corpus), &budget);
        let started_at_ms = now_ms();
        let mut trace = TraceLedger {
            trace_id: format!("trace-{}", started_at_ms),
            session_id: session_id.to_string(),
            root_task_id: task_id.to_string(),
            started_at_ms,
            finished_at_ms: None,
            final_status: TraceFinalStatus::Running,
            events: Vec::new(),
        };
        push_trace_event(
            &mut trace,
            TraceEventType::TaskStarted,
            BTreeMap::from([
                (
                    "mode".to_string(),
                    JsonValue::String(mode_label(mode).to_string()),
                ),
                ("task".to_string(), JsonValue::String(task.to_string())),
            ]),
        );

        let mut state = RecursiveRuntimeState {
            session_id: session_id.to_string(),
            task_id: task_id.to_string(),
            mode,
            budget,
            usage: RuntimeBudgetUsage {
                depth: 0,
                ..RuntimeBudgetUsage::default()
            },
            iterations: Vec::new(),
            trace_artifact_path: None,
        };

        let Some(max_depth) = state.budget.max_depth else {
            return finalize_empty_stop(
                task,
                trace,
                trace_artifact_dir,
                RecursiveStopReason::DepthCap,
                mode,
                None,
                state.usage,
                tracer,
            );
        };
        if max_depth < 1 {
            return finalize_empty_stop(
                task,
                trace,
                trace_artifact_dir,
                RecursiveStopReason::DepthCap,
                mode,
                None,
                state.usage,
                tracer,
            );
        }

        let peek = self.corpus_peek();
        push_trace_event(
            &mut trace,
            TraceEventType::CorpusPeeked,
            BTreeMap::from([
                ("corpusId".to_string(), JsonValue::String(peek.corpus_id)),
                (
                    "documentCount".to_string(),
                    JsonValue::Number(i64::from(peek.document_count)),
                ),
                (
                    "chunkCount".to_string(),
                    JsonValue::Number(i64::from(peek.chunk_count)),
                ),
            ]),
        );

        let mut child_outputs = Vec::new();
        let mut last_retrieval = None;
        let mut last_escalation = None;
        let mut seen_chunk_ids = Vec::<String>::new();
        let stop_reason;

        loop {
            state.usage.runtime_ms = now_ms().saturating_sub(started_at_ms);
            if let Some(reason) = next_iteration_stop_reason(&state.budget, &state.usage) {
                stop_reason = reason;
                break;
            }

            let iteration = state.usage.iterations + 1;
            let query = build_iteration_query(task, &child_outputs);
            push_trace_event(
                &mut trace,
                TraceEventType::RetrievalRequested,
                BTreeMap::from([
                    ("query".to_string(), JsonValue::String(query.clone())),
                    (
                        "iteration".to_string(),
                        JsonValue::Number(i64::from(iteration)),
                    ),
                ]),
            );
            let retrieval = self.corpus_search(&query, 6);
            push_trace_event(
                &mut trace,
                TraceEventType::RetrievalCompleted,
                BTreeMap::from([
                    ("query".to_string(), JsonValue::String(query)),
                    (
                        "iteration".to_string(),
                        JsonValue::Number(i64::from(iteration)),
                    ),
                    (
                        "hitCount".to_string(),
                        JsonValue::Number(i64::try_from(retrieval.hits.len()).unwrap_or(i64::MAX)),
                    ),
                ]),
            );

            state.usage.iterations = iteration;
            state.usage.runtime_ms = now_ms().saturating_sub(started_at_ms);
            last_retrieval = Some(retrieval.clone());

            let selected_chunk_ids = retrieval
                .hits
                .iter()
                .map(|hit| hit.chunk_id.clone())
                .filter(|chunk_id| !seen_chunk_ids.contains(chunk_id))
                .take(3)
                .collect::<Vec<_>>();
            if selected_chunk_ids.is_empty() {
                stop_reason = if child_outputs.is_empty() {
                    RecursiveStopReason::NoChildCapacity
                } else {
                    RecursiveStopReason::Completed
                };
                break;
            }

            let slices = self.select_slices(&selected_chunk_ids)?;
            push_trace_event(
                &mut trace,
                TraceEventType::CorpusSliced,
                BTreeMap::from([
                    (
                        "iteration".to_string(),
                        JsonValue::Number(i64::from(iteration)),
                    ),
                    (
                        "sliceCount".to_string(),
                        JsonValue::Number(i64::try_from(slices.len()).unwrap_or(i64::MAX)),
                    ),
                    (
                        "chunkIds".to_string(),
                        JsonValue::Array(
                            selected_chunk_ids
                                .iter()
                                .cloned()
                                .map(JsonValue::String)
                                .collect(),
                        ),
                    ),
                ]),
            );
            let local_summary = summarize_local_evidence(&retrieval);
            let escalation = evaluate_web_escalation(
                web_policy.clone(),
                EscalationHeuristicInput {
                    local_summary,
                    requires_external_freshness: task_mentions_freshness(task),
                    user_requested_web: task_requests_web(task),
                },
            );
            if escalation.reason != EscalationReason::LocalEvidenceSufficient
                || last_escalation.is_none()
            {
                last_escalation = Some(escalation.clone());
            }
            if escalation.reason != EscalationReason::LocalEvidenceSufficient {
                push_trace_event(
                    &mut trace,
                    TraceEventType::WebEscalationStarted,
                    BTreeMap::from([
                        (
                            "decision".to_string(),
                            JsonValue::String(
                                web_access_decision_label(escalation.decision).to_string(),
                            ),
                        ),
                        (
                            "reason".to_string(),
                            JsonValue::String(
                                escalation_reason_label(escalation.reason).to_string(),
                            ),
                        ),
                        (
                            "parentMode".to_string(),
                            JsonValue::String(web_policy_label(web_policy.mode).to_string()),
                        ),
                        (
                            "localHits".to_string(),
                            JsonValue::Number(
                                i64::try_from(local_summary.total_hits).unwrap_or(i64::MAX),
                            ),
                        ),
                        (
                            "weakLocalEvidence".to_string(),
                            JsonValue::Bool(is_local_evidence_weak(local_summary)),
                        ),
                    ]),
                );
            }

            let child_budget = state.budget.slice_for_child(BudgetSliceRequest {
                depth_cost: 1,
                subcall_cost: 1,
                max_iterations: Some(1),
                ..BudgetSliceRequest::default()
            });
            if matches!(child_budget.max_depth, Some(0)) {
                stop_reason = if child_outputs.is_empty() {
                    RecursiveStopReason::NoChildCapacity
                } else {
                    RecursiveStopReason::Completed
                };
                break;
            }

            let request = ChildSubqueryRequest {
                subquery_id: format!("{task_id}-child-{iteration}"),
                prompt: build_child_prompt(task, iteration, &child_outputs, &slices),
                slices,
                budget: child_budget,
                web_policy: web_policy.clone().inherit_for_child(None),
            };
            push_trace_event(
                &mut trace,
                TraceEventType::SubqueryStarted,
                BTreeMap::from([
                    (
                        "subqueryId".to_string(),
                        JsonValue::String(request.subquery_id.clone()),
                    ),
                    (
                        "iteration".to_string(),
                        JsonValue::Number(i64::from(iteration)),
                    ),
                    (
                        "sliceCount".to_string(),
                        JsonValue::Number(i64::try_from(request.slices.len()).unwrap_or(i64::MAX)),
                    ),
                    (
                        "webMode".to_string(),
                        JsonValue::String(web_policy_label(request.web_policy.mode).to_string()),
                    ),
                    (
                        "webMaxFetches".to_string(),
                        request
                            .web_policy
                            .max_fetches
                            .map(|value| JsonValue::Number(i64::from(value)))
                            .unwrap_or(JsonValue::Null),
                    ),
                ]),
            );
            let child_output = self.executor.execute(&request)?;
            if !child_output.web_evidence.is_empty() {
                let sequence = u32::try_from(trace.events.len() + 1).unwrap_or(u32::MAX);
                trace.events.push(web_evidence_trace_event(
                    sequence,
                    now_ms(),
                    &child_output.web_evidence,
                ));
            }
            push_trace_event(
                &mut trace,
                TraceEventType::SubqueryCompleted,
                BTreeMap::from([
                    (
                        "subqueryId".to_string(),
                        JsonValue::String(child_output.subquery_id.clone()),
                    ),
                    (
                        "iteration".to_string(),
                        JsonValue::Number(i64::from(iteration)),
                    ),
                    (
                        "citationCount".to_string(),
                        JsonValue::Number(
                            i64::try_from(child_output.citations.len()).unwrap_or(i64::MAX),
                        ),
                    ),
                ]),
            );

            state.usage.depth = 1;
            state.usage.subcalls += 1;
            state.usage.prompt_tokens += child_output.prompt_tokens;
            state.usage.completion_tokens += child_output.completion_tokens;
            state.usage.cost_usd += child_output.cost_usd;
            state.usage.runtime_ms = now_ms().saturating_sub(started_at_ms);
            state.iterations.push(RecursiveIterationState {
                iteration,
                child_count: 1,
                selected_chunk_ids: selected_chunk_ids.clone(),
            });
            seen_chunk_ids.extend(selected_chunk_ids);
            child_outputs.push(child_output);

            if let Some(reason) = state
                .budget
                .exhausted_by(&state.usage)
                .map(RecursiveStopReason::from)
            {
                stop_reason = reason;
                break;
            }
        }

        let retrieval = last_retrieval;
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
                let aggregated_body = self.aggregator.aggregate(task, &child_outputs);
                format_recursive_answer(
                    aggregated_body,
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
            BTreeMap::from([(
                "stopReason".to_string(),
                JsonValue::String(stop_reason.as_str().to_string()),
            )]),
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

    fn slice_by_chunk_id(
        &self,
        chunk_id: &str,
    ) -> Result<RecursiveContextSlice, RecursiveRuntimeError> {
        for document in &self.corpus.documents {
            for chunk in &document.chunks {
                if chunk.chunk_id == chunk_id {
                    return Ok(map_chunk(document.path.as_str(), chunk));
                }
            }
        }
        Err(RecursiveRuntimeError::MissingChunk(chunk_id.to_string()))
    }
}

fn map_chunk(path: &str, chunk: &CorpusChunk) -> RecursiveContextSlice {
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

fn slice_text(metadata: &BTreeMap<String, JsonValue>, preview: &str) -> String {
    metadata
        .get("text")
        .and_then(JsonValue::as_str)
        .filter(|text| !text.is_empty())
        .unwrap_or(preview)
        .to_string()
}

fn build_iteration_query(task: &str, child_outputs: &[ChildSubqueryOutput]) -> String {
    if let Some(last) = child_outputs.last() {
        format!("{task} {} {}", last.answer, last.citations.join(" "))
    } else {
        task.to_string()
    }
}

fn build_child_prompt(
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

fn next_iteration_stop_reason(
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

fn push_trace_event(
    trace: &mut TraceLedger,
    event_type: TraceEventType,
    data: BTreeMap<String, JsonValue>,
) {
    let sequence = u32::try_from(trace.events.len() + 1).unwrap_or(u32::MAX);
    trace
        .events
        .push(TraceEvent::new(sequence, event_type, now_ms(), data));
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should be after epoch")
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn mode_label(mode: RecursiveExecutionMode) -> &'static str {
    match mode {
        RecursiveExecutionMode::Direct => "direct",
        RecursiveExecutionMode::Rag => "rag",
        RecursiveExecutionMode::Rlm => "rlm",
    }
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
        trace_id: Some(trace_id.to_string()),
    }
    .render_text()
}

fn task_requests_web(task: &str) -> bool {
    let lowered = task.to_ascii_lowercase();
    ["web", "online", "internet", "search the web", "browse"]
        .iter()
        .any(|needle| lowered.contains(needle))
}

fn task_mentions_freshness(task: &str) -> bool {
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

fn web_policy_label(mode: WebAccessMode) -> &'static str {
    match mode {
        WebAccessMode::Off => "off",
        WebAccessMode::Ask => "ask",
        WebAccessMode::On => "on",
    }
}

fn web_access_decision_label(decision: WebAccessDecision) -> &'static str {
    match decision {
        WebAccessDecision::Denied => "denied",
        WebAccessDecision::RequiresApproval => "requires_approval",
        WebAccessDecision::Allowed => "allowed",
    }
}

fn escalation_reason_label(reason: EscalationReason) -> &'static str {
    match reason {
        EscalationReason::UserRequestedWeb => "user_requested_web",
        EscalationReason::NoLocalEvidence => "no_local_evidence",
        EscalationReason::WeakLocalEvidence => "weak_local_evidence",
        EscalationReason::FreshnessRequired => "freshness_required",
        EscalationReason::LocalEvidenceSufficient => "local_evidence_sufficient",
        EscalationReason::PolicyDenied => "policy_denied",
    }
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

fn finalize_empty_stop(
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
        BTreeMap::from([(
            "stopReason".to_string(),
            JsonValue::String(reason.as_str().to_string()),
        )]),
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
        "Trace\n  Id               {}\n  Session          {}\n  Task             {}\n  Status           {}\n  Stop reason      {}\n  Events           {}\n  Retrievals       {} / {}\n  Subqueries       {} / {}\n  Web escalations  {}\n  Web evidence     {}",
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::corpus::{CorpusBackend, CorpusChunk, CorpusDocument, CorpusKind};
    use std::collections::BTreeMap;

    struct StubExecutor;

    impl ChildSubqueryExecutor for StubExecutor {
        fn execute(
            &self,
            request: &ChildSubqueryRequest,
        ) -> Result<ChildSubqueryOutput, RecursiveRuntimeError> {
            Ok(ChildSubqueryOutput {
                subquery_id: request.subquery_id.clone(),
                answer: format!("inspected {} slices", request.slices.len()),
                citations: request
                    .slices
                    .iter()
                    .map(|slice| slice.chunk_id.clone())
                    .collect(),
                web_evidence: Vec::new(),
                prompt_tokens: 120,
                completion_tokens: 40,
                cost_usd: 0.02,
            })
        }
    }

    fn sample_corpus() -> CorpusManifest {
        let doc_id = CorpusManifest::stable_document_id("docs/spec.md");
        let chunk_a = CorpusChunk {
            chunk_id: CorpusManifest::stable_chunk_id(&doc_id, 0),
            document_id: doc_id.clone(),
            ordinal: 0,
            start_offset: 0,
            end_offset: 100,
            text_preview: "# Spec\nrecursive controller budget trace".to_string(),
            metadata: BTreeMap::from([(
                "heading".to_string(),
                JsonValue::String("Spec".to_string()),
            )]),
        };
        let chunk_b = CorpusChunk {
            chunk_id: CorpusManifest::stable_chunk_id(&doc_id, 1),
            document_id: doc_id.clone(),
            ordinal: 1,
            start_offset: 100,
            end_offset: 220,
            text_preview: "child subquery aggregation export trace summary".to_string(),
            metadata: BTreeMap::from([(
                "text".to_string(),
                JsonValue::String(
                    "child subquery aggregation export trace summary with hidden full context for corpus search"
                        .to_string(),
                ),
            )]),
        };
        let chunk_c = CorpusChunk {
            chunk_id: CorpusManifest::stable_chunk_id(&doc_id, 2),
            document_id: doc_id.clone(),
            ordinal: 2,
            start_offset: 220,
            end_offset: 360,
            text_preview: "trace export followup notes for iterative controller".to_string(),
            metadata: BTreeMap::new(),
        };
        let chunk_d = CorpusChunk {
            chunk_id: CorpusManifest::stable_chunk_id(&doc_id, 3),
            document_id: doc_id.clone(),
            ordinal: 3,
            start_offset: 360,
            end_offset: 520,
            text_preview: "hidden trace budget details for second retrieval pass".to_string(),
            metadata: BTreeMap::new(),
        };
        CorpusManifest {
            corpus_id: "corpus-1".to_string(),
            roots: vec!["docs".to_string()],
            kind: CorpusKind::Docs,
            backend: CorpusBackend::Lexical,
            document_count: 1,
            chunk_count: 4,
            estimated_bytes: 520,
            documents: vec![CorpusDocument {
                document_id: doc_id,
                path: "docs/spec.md".to_string(),
                media_type: "text/markdown".to_string(),
                language: Some("markdown".to_string()),
                headings: vec!["Spec".to_string()],
                bytes: 520,
                modified_at_ms: None,
                chunks: vec![chunk_a, chunk_b, chunk_c, chunk_d],
            }],
        }
    }

    fn temp_trace_dir() -> PathBuf {
        std::env::temp_dir().join(format!("rlm-trace-{}", now_ms()))
    }

    #[test]
    fn mode_selection_prefers_rlm_when_depth_budget_and_corpus_exist() {
        let corpus = sample_corpus();
        let mode = RecursiveConversationRuntime::<StubExecutor>::select_mode(
            "analyze trace aggregation",
            Some(&corpus),
            &RuntimeBudget {
                max_depth: Some(1),
                ..RuntimeBudget::default()
            },
        );
        assert_eq!(mode, RecursiveExecutionMode::Rlm);
    }

    #[test]
    fn corpus_inspection_operations_return_structured_results() {
        let corpus = sample_corpus();
        let runtime = RecursiveConversationRuntime::new(&corpus, StubExecutor);

        let peek = runtime.corpus_peek();
        assert_eq!(peek.document_count, 1);
        assert_eq!(peek.chunk_count, 4);

        let retrieval = runtime.corpus_search("trace", 2);
        assert!(!retrieval.hits.is_empty());
        let slices = runtime
            .select_slices(
                &retrieval
                    .hits
                    .iter()
                    .map(|hit| hit.chunk_id.clone())
                    .collect::<Vec<_>>(),
            )
            .expect("slice selection should work");
        assert!(!slices.is_empty());
        assert!(slices[0].path.contains("docs/spec.md"));
    }

    #[test]
    fn corpus_search_scores_against_full_slice_text_from_metadata() {
        let corpus = sample_corpus();
        let runtime = RecursiveConversationRuntime::new(&corpus, StubExecutor);
        let expected_chunk_id = corpus.documents[0].chunks[1].chunk_id.clone();

        let retrieval = runtime.corpus_search("hidden", 2);
        assert_eq!(retrieval.hits.len(), 2);
        assert_eq!(retrieval.hits[0].chunk_id, expected_chunk_id);
        assert!(retrieval.hits[0].reason.contains("content:hiddenx1"));
    }

    #[test]
    fn build_child_prompt_prefers_full_slice_text_from_metadata() {
        let corpus = sample_corpus();
        let runtime = RecursiveConversationRuntime::new(&corpus, StubExecutor);
        let chunk_id = corpus.documents[0].chunks[1].chunk_id.clone();
        let slices = runtime
            .select_slices(&[chunk_id])
            .expect("slice selection should work");

        let prompt = build_child_prompt("summarize", 1, &[], &slices);
        assert!(prompt.contains("hidden full context for corpus search"));
        assert!(!prompt.contains("(100-220)\nchild subquery aggregation export trace summary\n"));
    }

    #[test]
    fn depth_one_recursive_run_executes_child_aggregates_and_exports_trace() {
        let corpus = sample_corpus();
        let runtime = RecursiveConversationRuntime::new(&corpus, StubExecutor);
        let trace_dir = temp_trace_dir();

        let result = runtime
            .run(
                "session-1",
                "task-1",
                "trace aggregation export",
                RuntimeBudget {
                    max_depth: Some(2),
                    max_iterations: Some(2),
                    max_subcalls: Some(1),
                    max_runtime_ms: Some(30_000),
                    ..RuntimeBudget::default()
                },
                Some(&trace_dir),
            )
            .expect("recursive run should succeed");

        assert_eq!(result.stop_reason, RecursiveStopReason::SubcallCap);
        assert_eq!(result.child_outputs.len(), 1);
        assert!(result.final_answer.contains("inspected"));
        assert_eq!(result.usage.iterations, 1);
        assert_eq!(result.usage.subcalls, 1);
        assert!(result
            .trace
            .events
            .iter()
            .any(|event| event.event_type == TraceEventType::AggregationCompleted));
        let trace_path = result
            .trace_artifact_path
            .expect("trace export path should exist");
        assert!(trace_path.is_file());
        let summary = render_trace_summary(&result.trace);
        assert!(summary.contains("Stop reason      subcall_cap"));

        let exported = export_trace(&result.trace, &trace_dir.join("exported"))
            .expect("trace export helper should succeed");
        assert!(exported.is_file());

        let _ = fs::remove_dir_all(trace_dir);
    }

    #[test]
    fn iterative_run_consumes_multiple_rounds_until_novel_context_is_exhausted() {
        let corpus = sample_corpus();
        let runtime = RecursiveConversationRuntime::new(&corpus, StubExecutor);

        let result = runtime
            .run(
                "session-1",
                "task-iterative",
                "trace aggregation export hidden",
                RuntimeBudget {
                    max_depth: Some(2),
                    max_iterations: Some(4),
                    max_subcalls: Some(4),
                    max_runtime_ms: Some(30_000),
                    ..RuntimeBudget::default()
                },
                None,
            )
            .expect("iterative run should succeed");

        assert_eq!(result.stop_reason, RecursiveStopReason::Completed);
        assert_eq!(result.child_outputs.len(), 2);
        assert_eq!(result.usage.iterations, 3);
        assert_eq!(result.usage.subcalls, 2);
        assert!(result.final_answer.matches("inspected").count() >= 1);
        let counters = result.trace.counters();
        assert_eq!(counters.retrieval_requests, 3);
        assert_eq!(counters.subqueries_completed, 2);
    }

    #[test]
    fn iteration_cap_stops_before_launching_another_child() {
        let corpus = sample_corpus();
        let runtime = RecursiveConversationRuntime::new(&corpus, StubExecutor);

        let result = runtime
            .run(
                "session-1",
                "task-iteration-cap",
                "trace aggregation export hidden",
                RuntimeBudget {
                    max_depth: Some(2),
                    max_iterations: Some(1),
                    max_subcalls: Some(4),
                    max_runtime_ms: Some(30_000),
                    ..RuntimeBudget::default()
                },
                None,
            )
            .expect("run should stop at iteration cap");

        assert_eq!(result.stop_reason, RecursiveStopReason::IterationCap);
        assert_eq!(result.child_outputs.len(), 1);
        assert_eq!(result.usage.iterations, 1);
        assert_eq!(result.usage.subcalls, 1);
    }

    #[test]
    fn run_with_web_policy_records_escalation_and_child_inheritance() {
        let corpus = sample_corpus();
        let runtime = RecursiveConversationRuntime::new(&corpus, StubExecutor);
        let result = runtime
            .run_with_tracer_and_policy(
                "session-1",
                "task-web",
                "find the latest web guidance for hidden behavior",
                RuntimeBudget {
                    max_depth: Some(2),
                    max_iterations: Some(2),
                    max_subcalls: Some(1),
                    ..RuntimeBudget::default()
                },
                None,
                None,
                WebPolicy {
                    mode: WebAccessMode::Ask,
                    max_fetches: Some(2),
                },
            )
            .expect("run should succeed");

        assert!(result
            .trace
            .events
            .iter()
            .any(|event| event.event_type == TraceEventType::WebEscalationStarted));
        assert!(result
            .trace
            .events
            .iter()
            .any(|event| event.event_type == TraceEventType::SubqueryStarted
                && event.data.get("webMode") == Some(&JsonValue::String("ask".to_string()))));
        assert!(result.final_answer.contains("requires approval"));
    }

    struct WebStubExecutor;

    impl ChildSubqueryExecutor for WebStubExecutor {
        fn execute(
            &self,
            request: &ChildSubqueryRequest,
        ) -> Result<ChildSubqueryOutput, RecursiveRuntimeError> {
            assert_eq!(request.web_policy.mode, WebAccessMode::On);
            Ok(ChildSubqueryOutput {
                subquery_id: request.subquery_id.clone(),
                answer: "used web evidence".to_string(),
                citations: vec!["local-citation".to_string()],
                web_evidence: vec![EvidenceRecord {
                    kind: EvidenceKind::Web,
                    id: "web-1".to_string(),
                    title: "Example release notes".to_string(),
                    locator: "https://example.test/release".to_string(),
                    snippet: "fresh info".to_string(),
                    score: None,
                    metadata: BTreeMap::new(),
                }],
                prompt_tokens: 10,
                completion_tokens: 5,
                cost_usd: 0.0,
            })
        }
    }

    #[test]
    fn final_answer_and_trace_include_web_provenance_when_child_returns_web_evidence() {
        let corpus = sample_corpus();
        let runtime = RecursiveConversationRuntime::new(&corpus, WebStubExecutor);
        let result = runtime
            .run_with_tracer_and_policy(
                "session-1",
                "task-web-2",
                "search the web for the latest hidden behavior",
                RuntimeBudget {
                    max_depth: Some(2),
                    max_iterations: Some(2),
                    max_subcalls: Some(1),
                    ..RuntimeBudget::default()
                },
                None,
                None,
                WebPolicy {
                    mode: WebAccessMode::On,
                    max_fetches: Some(2),
                },
            )
            .expect("run should succeed");

        assert!(result
            .final_answer
            .contains("[W1] web · Example release notes"));
        assert!(result
            .trace
            .events
            .iter()
            .any(|event| event.event_type == TraceEventType::WebEvidenceAdded));
    }

    #[test]
    fn enforces_depth_cap_before_child_execution() {
        let corpus = sample_corpus();
        let runtime = RecursiveConversationRuntime::new(&corpus, StubExecutor);
        let result = runtime
            .run(
                "session-1",
                "task-2",
                "trace aggregation export",
                RuntimeBudget {
                    max_depth: Some(0),
                    ..RuntimeBudget::default()
                },
                None,
            )
            .expect("run should produce bounded stop result");

        assert_eq!(result.stop_reason, RecursiveStopReason::DepthCap);
        assert!(result.child_outputs.is_empty());
        assert_eq!(result.trace.final_status, TraceFinalStatus::BudgetExceeded);
    }
}
