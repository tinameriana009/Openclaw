use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::budget::{BudgetSliceRequest, BudgetStopReason, RuntimeBudget, RuntimeBudgetUsage};
use crate::corpus::{CorpusChunk, CorpusManifest, RetrievalHit, RetrievalResult};
use crate::hybrid::{format_citations, normalize_local_evidence, summarize_local_evidence};
use crate::json::JsonValue;
use crate::trace::{TraceEvent, TraceEventType, TraceFinalStatus, TraceLedger};
use crate::ux::{
    Citation, ConfidenceLevel, ConfidenceNote, EvidenceProvenance, FinalAnswer,
};
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
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChildSubqueryOutput {
    pub subquery_id: String,
    pub answer: String,
    pub citations: Vec<String>,
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
    fn execute(&self, request: &ChildSubqueryRequest) -> Result<ChildSubqueryOutput, RecursiveRuntimeError>;
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
    pub fn select_mode(task: &str, corpus: Option<&CorpusManifest>, budget: &RuntimeBudget) -> RecursiveExecutionMode {
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
                let lowered_preview = chunk.text_preview.to_ascii_lowercase();
                let mut score = 0.0_f64;
                let mut reasons = Vec::new();
                for term in &lowered_terms {
                    if lowered_path.contains(term) {
                        score += 3.0;
                        reasons.push(format!("path:{term}"));
                    }
                    if lowered_headings.iter().any(|heading| heading.contains(term)) {
                        score += 2.0;
                        reasons.push(format!("heading:{term}"));
                    }
                    let content_hits = lowered_preview.matches(term).count();
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

    pub fn select_slices(&self, chunk_ids: &[String]) -> Result<Vec<RecursiveContextSlice>, RecursiveRuntimeError> {
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
        self.run_with_tracer(session_id, task_id, task, budget, trace_artifact_dir, None)
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
                ("mode".to_string(), JsonValue::String(mode_label(mode).to_string())),
                ("task".to_string(), JsonValue::String(task.to_string())),
            ]),
        );

        let usage = RuntimeBudgetUsage {
            depth: 0,
            ..RuntimeBudgetUsage::default()
        };
        let mut state = RecursiveRuntimeState {
            session_id: session_id.to_string(),
            task_id: task_id.to_string(),
            mode,
            budget,
            usage,
            iterations: Vec::new(),
            trace_artifact_path: None,
        };

        let stop_reason = if let Some(limit) = state.budget.max_depth {
            if limit < 1 {
                RecursiveStopReason::DepthCap
            } else {
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
                RecursiveStopReason::Completed
            }
        } else {
            RecursiveStopReason::DepthCap
        };

        if stop_reason == RecursiveStopReason::DepthCap {
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
            state.trace_artifact_path = trace_path.clone();
            return Ok(RecursiveExecutionResult {
                mode,
                stop_reason,
                final_answer: format!("Recursive execution stopped before any child subqueries for task: {task}"),
                child_outputs: Vec::new(),
                retrieval: None,
                trace,
                trace_artifact_path: trace_path,
                usage: state.usage,
            });
        }

        push_trace_event(
            &mut trace,
            TraceEventType::RetrievalRequested,
            BTreeMap::from([("query".to_string(), JsonValue::String(task.to_string()))]),
        );
        let retrieval = self.corpus_search(task, 3);
        push_trace_event(
            &mut trace,
            TraceEventType::RetrievalCompleted,
            BTreeMap::from([
                ("query".to_string(), JsonValue::String(task.to_string())),
                (
                    "hitCount".to_string(),
                    JsonValue::Number(i64::try_from(retrieval.hits.len()).unwrap_or(i64::MAX)),
                ),
            ]),
        );

        state.usage.iterations += 1;
        if let Some(reason) = state.budget.exhausted_by(&state.usage).map(RecursiveStopReason::from) {
            return finalize_budget_stop(task, trace, trace_artifact_dir, reason, retrieval, state.usage, tracer);
        }

        let selected_chunk_ids = retrieval
            .hits
            .iter()
            .map(|hit| hit.chunk_id.clone())
            .collect::<Vec<_>>();
        let slices = self.select_slices(&selected_chunk_ids)?;
        push_trace_event(
            &mut trace,
            TraceEventType::CorpusSliced,
            BTreeMap::from([
                (
                    "sliceCount".to_string(),
                    JsonValue::Number(i64::try_from(slices.len()).unwrap_or(i64::MAX)),
                ),
                (
                    "chunkIds".to_string(),
                    JsonValue::Array(selected_chunk_ids.iter().cloned().map(JsonValue::String).collect()),
                ),
            ]),
        );

        let child_budget = state.budget.slice_for_child(BudgetSliceRequest {
            depth_cost: 1,
            subcall_cost: 1,
            max_iterations: Some(1),
            ..BudgetSliceRequest::default()
        });
        if matches!(child_budget.max_depth, Some(0)) {
            return finalize_early_success(
                task,
                trace,
                trace_artifact_dir,
                RecursiveStopReason::NoChildCapacity,
                retrieval,
                state.usage,
                tracer,
            );
        }

        let request = ChildSubqueryRequest {
            subquery_id: format!("{task_id}-child-1"),
            prompt: build_child_prompt(task, &slices),
            slices,
            budget: child_budget,
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
                    "sliceCount".to_string(),
                    JsonValue::Number(i64::try_from(request.slices.len()).unwrap_or(i64::MAX)),
                ),
            ]),
        );
        let child_output = self.executor.execute(&request)?;
        push_trace_event(
            &mut trace,
            TraceEventType::SubqueryCompleted,
            BTreeMap::from([
                (
                    "subqueryId".to_string(),
                    JsonValue::String(child_output.subquery_id.clone()),
                ),
                (
                    "citationCount".to_string(),
                    JsonValue::Number(i64::try_from(child_output.citations.len()).unwrap_or(i64::MAX)),
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
            iteration: 1,
            child_count: 1,
            selected_chunk_ids,
        });

        if let Some(reason) = state.budget.exhausted_by(&state.usage).map(RecursiveStopReason::from) {
            return finalize_budget_stop(task, trace, trace_artifact_dir, reason, retrieval, state.usage, tracer);
        }

        let aggregated_body = self
            .aggregator
            .aggregate(task, std::slice::from_ref(&child_output));
        let final_answer = format_recursive_answer(
            aggregated_body,
            &retrieval,
            std::slice::from_ref(&child_output),
            &trace.trace_id,
        );
        push_trace_event(
            &mut trace,
            TraceEventType::AggregationCompleted,
            BTreeMap::from([
                (
                    "childCount".to_string(),
                    JsonValue::Number(1),
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
                JsonValue::String(RecursiveStopReason::Completed.as_str().to_string()),
            )]),
        );
        trace.finished_at_ms = Some(now_ms());
        trace.final_status = TraceFinalStatus::Succeeded;
        let trace_path = export_trace_if_requested(&trace, trace_artifact_dir)?;
        if let Some(tracer) = tracer {
            trace.emit_telemetry(tracer);
        }
        state.trace_artifact_path = trace_path.clone();

        Ok(RecursiveExecutionResult {
            mode,
            stop_reason: RecursiveStopReason::Completed,
            final_answer,
            child_outputs: vec![child_output],
            retrieval: Some(retrieval),
            trace,
            trace_artifact_path: trace_path,
            usage: state.usage,
        })
    }

    fn slice_by_chunk_id(&self, chunk_id: &str) -> Result<RecursiveContextSlice, RecursiveRuntimeError> {
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

fn build_child_prompt(task: &str, slices: &[RecursiveContextSlice]) -> String {
    let mut prompt = format!("Task: {task}\nUse only the provided slices.\n");
    for slice in slices {
        prompt.push_str("\n");
        prompt.push_str(&format!(
            "[{}] {}#{} ({}-{})\n{}\n",
            slice.chunk_id, slice.path, slice.ordinal, slice.start_offset, slice.end_offset, slice.preview
        ));
    }
    prompt
}

fn push_trace_event(trace: &mut TraceLedger, event_type: TraceEventType, data: BTreeMap<String, JsonValue>) {
    let sequence = u32::try_from(trace.events.len() + 1).unwrap_or(u32::MAX);
    trace.events.push(TraceEvent::new(sequence, event_type, now_ms(), data));
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
        .collect::<Vec<_>>();
    let local_summary = summarize_local_evidence(retrieval);
    let child_citations = format_citations(&normalize_local_evidence(retrieval));
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
        gaps: if child_citations.is_empty() {
            vec!["No local evidence was retrieved.".to_string()]
        } else {
            Vec::new()
        },
    };
    FinalAnswer {
        body,
        citations,
        confidence: Some(confidence),
        trace_id: Some(trace_id.to_string()),
    }
    .render_text()
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

fn finalize_budget_stop(
    task: &str,
    mut trace: TraceLedger,
    trace_artifact_dir: Option<&Path>,
    reason: RecursiveStopReason,
    retrieval: RetrievalResult,
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
    Ok(RecursiveExecutionResult {
        mode: RecursiveExecutionMode::Rlm,
        stop_reason: reason,
        final_answer: format_recursive_answer(
            format!("Recursive execution stopped for task: {task}"),
            &retrieval,
            &[],
            &trace.trace_id,
        ),
        child_outputs: Vec::new(),
        retrieval: Some(retrieval),
        trace,
        trace_artifact_path: trace_path,
        usage,
    })
}

fn finalize_early_success(
    task: &str,
    mut trace: TraceLedger,
    trace_artifact_dir: Option<&Path>,
    reason: RecursiveStopReason,
    retrieval: RetrievalResult,
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
    Ok(RecursiveExecutionResult {
        mode: RecursiveExecutionMode::Rlm,
        stop_reason: reason,
        final_answer: format_recursive_answer(
            format!("Recursive execution completed without child calls for task: {task}"),
            &retrieval,
            &[],
            &trace.trace_id,
        ),
        child_outputs: Vec::new(),
        retrieval: Some(retrieval),
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

pub fn export_trace(trace: &TraceLedger, destination: &Path) -> Result<PathBuf, RecursiveRuntimeError> {
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
        fn execute(&self, request: &ChildSubqueryRequest) -> Result<ChildSubqueryOutput, RecursiveRuntimeError> {
            Ok(ChildSubqueryOutput {
                subquery_id: request.subquery_id.clone(),
                answer: format!("inspected {} slices", request.slices.len()),
                citations: request.slices.iter().map(|slice| slice.chunk_id.clone()).collect(),
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
            metadata: BTreeMap::new(),
        };
        CorpusManifest {
            corpus_id: "corpus-1".to_string(),
            roots: vec!["docs".to_string()],
            kind: CorpusKind::Docs,
            backend: CorpusBackend::Lexical,
            document_count: 1,
            chunk_count: 2,
            estimated_bytes: 220,
            documents: vec![CorpusDocument {
                document_id: doc_id,
                path: "docs/spec.md".to_string(),
                media_type: "text/markdown".to_string(),
                language: Some("markdown".to_string()),
                headings: vec!["Spec".to_string()],
                bytes: 220,
                modified_at_ms: None,
                chunks: vec![chunk_a, chunk_b],
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
        assert_eq!(peek.chunk_count, 2);

        let retrieval = runtime.corpus_search("trace", 2);
        assert!(!retrieval.hits.is_empty());
        let slices = runtime
            .select_slices(&retrieval.hits.iter().map(|hit| hit.chunk_id.clone()).collect::<Vec<_>>())
            .expect("slice selection should work");
        assert!(!slices.is_empty());
        assert!(slices[0].path.contains("docs/spec.md"));
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

        assert_eq!(result.stop_reason, RecursiveStopReason::Completed);
        assert_eq!(result.child_outputs.len(), 1);
        assert!(result.final_answer.contains("inspected"));
        assert_eq!(result.usage.subcalls, 1);
        assert!(result
            .trace
            .events
            .iter()
            .any(|event| event.event_type == TraceEventType::AggregationCompleted));
        let trace_path = result.trace_artifact_path.expect("trace export path should exist");
        assert!(trace_path.is_file());
        let summary = render_trace_summary(&result.trace);
        assert!(summary.contains("Stop reason      completed"));

        let exported = export_trace(&result.trace, &trace_dir.join("exported"))
            .expect("trace export helper should succeed");
        assert!(exported.is_file());

        let _ = fs::remove_dir_all(trace_dir);
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
