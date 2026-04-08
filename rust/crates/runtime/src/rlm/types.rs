use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::budget::{BudgetStopReason, RuntimeBudget, RuntimeBudgetUsage};
use crate::corpus::{CorpusManifest, RetrievalResult};
use crate::hybrid::{EscalationOutcome, EvidenceRecord, WebExecutionOutcome, WebPolicy};
use crate::json::JsonValue;
use crate::trace::{TraceFinalStatus, TraceLedger};
use crate::ux::ExecutionProfile;

use super::prepare_recursive_task_run;

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
    pub web_execution: Option<WebExecutionOutcome>,
    pub web_execution_note: Option<String>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecursiveRunArtifacts {
    pub telemetry_path: PathBuf,
    pub trace_dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecursiveTaskWorkspace<'a> {
    pub cwd: &'a Path,
    pub session_id: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecursiveProfileTaskRequest<'a> {
    pub workspace: RecursiveTaskWorkspace<'a>,
    pub task_id: &'a str,
    pub task: &'a str,
    pub profile: ExecutionProfile,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreparedRecursiveTaskRun {
    pub session_id: String,
    pub task_id: String,
    pub task: String,
    pub budget: RuntimeBudget,
    pub telemetry_path: PathBuf,
    pub trace_dir: PathBuf,
    pub web_policy: WebPolicy,
}

impl PreparedRecursiveTaskRun {
    #[must_use]
    pub fn as_request(&self) -> RecursiveTaskRunRequest<'_> {
        RecursiveTaskRunRequest {
            session_id: &self.session_id,
            task_id: &self.task_id,
            task: &self.task,
            budget: self.budget,
            telemetry_path: self.telemetry_path.clone(),
            trace_dir: self.trace_dir.clone(),
            web_policy: self.web_policy.clone(),
        }
    }

    pub fn run_with<'a, E, A>(
        &self,
        runtime: &RecursiveConversationRuntime<'a, E, A>,
    ) -> Result<(RecursiveExecutionResult, RecursiveRunArtifacts), RecursiveRuntimeError>
    where
        E: ChildSubqueryExecutor,
        A: ChildOutputAggregator,
    {
        runtime.run_task(self.as_request())
    }
}

pub trait RecursiveRuntimeFactory<'a> {
    type Executor: ChildSubqueryExecutor;
    type Aggregator: ChildOutputAggregator;

    fn build_runtime(
        &self,
        corpus: &'a CorpusManifest,
    ) -> RecursiveConversationRuntime<'a, Self::Executor, Self::Aggregator>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecursiveTaskEnvelope<'a, F> {
    pub runtime: F,
    pub corpus: &'a CorpusManifest,
    pub task_id: &'a str,
    pub task: &'a str,
    pub profile: ExecutionProfile,
}

impl<'a, F> RecursiveTaskEnvelope<'a, F>
where
    F: RecursiveRuntimeFactory<'a> + RecursiveTaskWorkspaceProvider<'a>,
{
    #[must_use]
    pub fn prepare(&self) -> PreparedRecursiveTaskRun {
        prepare_recursive_task_run(RecursiveProfileTaskRequest {
            workspace: RecursiveTaskWorkspace {
                cwd: self.workspace_cwd(),
                session_id: self.workspace_session_id(),
            },
            task_id: self.task_id,
            task: self.task,
            profile: self.profile,
        })
    }

    pub fn run(
        &self,
    ) -> Result<(RecursiveExecutionResult, RecursiveRunArtifacts), RecursiveRuntimeError> {
        let prepared = self.prepare();
        let runtime = self.runtime.build_runtime(self.corpus);
        prepared.run_with(&runtime)
    }

    fn workspace_cwd(&self) -> &'a Path {
        self.runtime.workspace().cwd
    }

    fn workspace_session_id(&self) -> &'a str {
        self.runtime.workspace().session_id
    }
}

pub trait RecursiveTaskWorkspaceProvider<'a> {
    fn workspace(&self) -> RecursiveTaskWorkspace<'a>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecursiveTaskRunRequest<'a> {
    pub session_id: &'a str,
    pub task_id: &'a str,
    pub task: &'a str,
    pub budget: RuntimeBudget,
    pub telemetry_path: PathBuf,
    pub trace_dir: PathBuf,
    pub web_policy: WebPolicy,
}

pub trait ChildSubqueryExecutor {
    fn execute(
        &self,
        request: &ChildSubqueryRequest,
    ) -> Result<ChildSubqueryOutput, RecursiveRuntimeError>;
}

pub type ChildExecutionFallbackFormatter =
    Arc<dyn Fn(&RecursiveRuntimeError) -> String + Send + Sync>;
pub type ChildExecutionFallbackRenderer =
    Arc<dyn Fn(&ChildSubqueryRequest, &str) -> ChildSubqueryOutput + Send + Sync>;

pub struct FallbackChildSubqueryExecutor<E> {
    primary: E,
    model: String,
    unavailable_reason: Arc<dyn Fn() -> Option<String> + Send + Sync>,
    error_formatter: ChildExecutionFallbackFormatter,
    fallback_renderer: ChildExecutionFallbackRenderer,
}

impl<E> FallbackChildSubqueryExecutor<E> {
    #[must_use]
    pub fn new(
        primary: E,
        model: impl Into<String>,
        unavailable_reason: Arc<dyn Fn() -> Option<String> + Send + Sync>,
        error_formatter: ChildExecutionFallbackFormatter,
        fallback_renderer: ChildExecutionFallbackRenderer,
    ) -> Self {
        Self {
            primary,
            model: model.into(),
            unavailable_reason,
            error_formatter,
            fallback_renderer,
        }
    }

    #[must_use]
    pub fn model(&self) -> &str {
        &self.model
    }
}

impl<E> ChildSubqueryExecutor for FallbackChildSubqueryExecutor<E>
where
    E: ChildSubqueryExecutor,
{
    fn execute(
        &self,
        request: &ChildSubqueryRequest,
    ) -> Result<ChildSubqueryOutput, RecursiveRuntimeError> {
        self.primary.execute(request).or_else(|error| {
            let reason =
                (self.unavailable_reason)().unwrap_or_else(|| (self.error_formatter)(&error));
            Ok((self.fallback_renderer)(request, &reason))
        })
    }
}

pub trait ChildOutputAggregator {
    fn aggregate(&self, task: &str, child_outputs: &[ChildSubqueryOutput]) -> String;
}

#[derive(Debug, Clone)]
pub struct DefaultChildOutputAggregator;

fn extract_labeled_section(answer: &str, heading: &str) -> Option<String> {
    let mut in_section = false;
    let mut lines = Vec::new();
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
                lines.push(remainder.to_string());
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
                lines.push(normalized.to_string());
            }
        }
    }

    (!lines.is_empty()).then(|| lines.join(" "))
}

impl ChildOutputAggregator for DefaultChildOutputAggregator {
    fn aggregate(&self, task: &str, child_outputs: &[ChildSubqueryOutput]) -> String {
        if child_outputs.is_empty() {
            return format!("No child findings were produced for task: {task}");
        }

        let mut answer = format!("Task: {task}\n");
        let mut validation_steps = Vec::new();
        let mut remaining_gaps = Vec::new();
        for output in child_outputs {
            answer.push_str("\n");
            answer.push_str("- ");
            answer.push_str(&output.answer);
            if !output.citations.is_empty() {
                answer.push_str(" [sources: ");
                answer.push_str(&output.citations.join(", "));
                answer.push(']');
            }
            if let Some(step) = extract_labeled_section(&output.answer, "validation loop") {
                if !validation_steps.contains(&step) {
                    validation_steps.push(step);
                }
            }
            if let Some(gap) = extract_labeled_section(&output.answer, "remaining gaps") {
                if !remaining_gaps.contains(&gap) {
                    remaining_gaps.push(gap);
                }
            }
        }
        if !validation_steps.is_empty() {
            answer.push_str("\n\nValidation steps to run next:\n");
            for step in validation_steps.iter().take(3) {
                answer.push_str("- ");
                answer.push_str(step);
                answer.push('\n');
            }
        }
        if !remaining_gaps.is_empty() {
            answer.push_str("\nRemaining gaps still called out:\n");
            for gap in remaining_gaps.iter().take(3) {
                answer.push_str("- ");
                answer.push_str(gap);
                answer.push('\n');
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_child(answer: &str, citations: &[&str]) -> ChildSubqueryOutput {
        ChildSubqueryOutput {
            subquery_id: "child-1".to_string(),
            answer: answer.to_string(),
            citations: citations.iter().map(|value| (*value).to_string()).collect(),
            web_evidence: Vec::new(),
            web_execution: None,
            web_execution_note: None,
            prompt_tokens: 0,
            completion_tokens: 0,
            cost_usd: 0.0,
        }
    }

    #[test]
    fn default_aggregator_surfaces_validation_steps_and_remaining_gaps() {
        let aggregator = DefaultChildOutputAggregator;
        let answer = aggregator.aggregate(
            "tighten recursive planner follow-up",
            &[
                sample_child(
                    "Findings: trace export is stable\nValidation loop: run cargo test -p runtime m5_regression\nRemaining gaps: adaptive retry policy is still heuristic-first",
                    &["chunk-1"],
                ),
                sample_child(
                    "Findings: blender handoff bundle is clearer\nValidation loop: open the staged bundle in Blender and compare counts with validation-baseline.md\nRemaining gaps: Blender still needs manual in-app verification",
                    &["chunk-2"],
                ),
            ],
        );

        assert!(answer.contains("Validation steps to run next"));
        assert!(answer.contains("run cargo test -p runtime m5_regression"));
        assert!(answer.contains("Remaining gaps still called out"));
        assert!(answer.contains("adaptive retry policy is still heuristic-first"));
        assert!(answer.contains("Blender still needs manual in-app verification"));
    }
}
