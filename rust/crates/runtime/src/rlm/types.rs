use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;

use crate::budget::{BudgetStopReason, RuntimeBudget, RuntimeBudgetUsage};
use crate::corpus::{CorpusManifest, RetrievalResult};
use crate::hybrid::{EscalationOutcome, EvidenceRecord, WebPolicy};
use crate::json::JsonValue;
use crate::trace::{TraceFinalStatus, TraceLedger};

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
    NoNewContext,
    Converged,
    ChildFailed,
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
            Self::NoNewContext => "no_new_context",
            Self::Converged => "converged",
            Self::ChildFailed => "child_failed",
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
            Self::Completed | Self::NoChildCapacity | Self::NoNewContext | Self::Converged => {
                TraceFinalStatus::Succeeded
            }
            Self::ChildFailed => TraceFinalStatus::Failed,
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
    pub web_research_query: Option<String>,
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
    pub(super) corpus: &'a CorpusManifest,
    pub(super) executor: E,
    pub(super) aggregator: A,
}

pub(super) struct IterationArtifacts {
    pub(super) retrieval: RetrievalResult,
    pub(super) selected_chunk_ids: Vec<String>,
    pub(super) slices: Vec<RecursiveContextSlice>,
    pub(super) escalation: EscalationOutcome,
}

