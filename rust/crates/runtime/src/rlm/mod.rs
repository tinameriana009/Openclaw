mod finalization;
mod helpers;
mod types;

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use crate::budget::{BudgetSliceRequest, RuntimeBudget, RuntimeBudgetUsage};
use crate::corpus::{search_corpus_manifest, CorpusManifest, RetrievalHit, RetrievalResult};
use crate::hybrid::{
    evaluate_web_escalation, is_local_evidence_weak, local_evidence_trace_event,
    summarize_local_evidence, web_evidence_trace_event, EscalationHeuristicInput, EscalationReason,
    WebAccessDecision, WebAccessMode, WebPolicy,
};
use crate::json::JsonValue;
use crate::trace::{TraceEventType, TraceFinalStatus, TraceLedger};
pub use finalization::{export_trace, render_trace_summary};
use finalization::{finalize_empty_stop, finalize_failed_run, finalize_successful_run};
use helpers::{
    build_child_prompt, build_iteration_plan, child_output_novelty_metrics,
    effective_child_web_policy, escalation_reason_label, map_chunk, mode_label,
    next_iteration_stop_reason, no_iteration_artifacts_stop_reason, now_ms, push_trace_event,
    should_stop_for_convergence, task_mentions_freshness, task_requests_web,
    web_access_decision_label, web_policy_label,
};
use telemetry::{JsonlTelemetrySink, SessionTracer};
pub use types::*;

#[must_use]
pub fn prepare_recursive_task_run(
    request: RecursiveProfileTaskRequest<'_>,
) -> PreparedRecursiveTaskRun {
    let resolved = request.profile.resolve();
    PreparedRecursiveTaskRun {
        session_id: request.workspace.session_id.to_string(),
        task_id: request.task_id.to_string(),
        task: request.task.to_string(),
        budget: RuntimeBudget {
            max_depth: resolved
                .rlm
                .max_depth
                .and_then(|value| u32::try_from(value).ok()),
            max_iterations: resolved
                .rlm
                .max_iterations
                .and_then(|value| u32::try_from(value).ok()),
            max_subcalls: resolved
                .rlm
                .max_subcalls
                .and_then(|value| u32::try_from(value).ok()),
            max_runtime_ms: resolved.rlm.max_runtime_ms,
            max_prompt_tokens: None,
            max_completion_tokens: None,
            max_cost_usd: None,
        },
        telemetry_path: request
            .workspace
            .cwd
            .join(".claw")
            .join("telemetry")
            .join("recursive-runtime.jsonl"),
        trace_dir: request.workspace.cwd.join(".claw").join("trace"),
        web_policy: WebPolicy::from_config(&resolved.web_research),
    }
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
    pub fn run_task(
        &self,
        request: RecursiveTaskRunRequest<'_>,
    ) -> Result<(RecursiveExecutionResult, RecursiveRunArtifacts), RecursiveRuntimeError> {
        let tracer = SessionTracer::new(
            request.session_id,
            Arc::new(
                JsonlTelemetrySink::new(&request.telemetry_path).map_err(|error| {
                    RecursiveRuntimeError::ChildExecution(format!(
                        "failed to initialize recursive runtime telemetry sink: {error}"
                    ))
                })?,
            ),
        );
        let result = self.run_with_tracer_and_policy(
            request.session_id,
            request.task_id,
            request.task,
            request.budget,
            Some(&request.trace_dir),
            Some(&tracer),
            request.web_policy,
        )?;
        Ok((
            result,
            RecursiveRunArtifacts {
                telemetry_path: request.telemetry_path,
                trace_dir: request.trace_dir,
            },
        ))
    }
}

fn select_diverse_chunk_ids(
    hits: &[RetrievalHit],
    seen_chunk_ids: &[String],
    limit: usize,
) -> Vec<String> {
    let mut selected = Vec::new();
    let mut selected_docs = std::collections::BTreeSet::new();

    for hit in hits {
        if selected.len() >= limit {
            break;
        }
        if seen_chunk_ids.contains(&hit.chunk_id) || selected_docs.contains(&hit.document_id) {
            continue;
        }
        selected_docs.insert(hit.document_id.clone());
        selected.push(hit.chunk_id.clone());
    }

    for hit in hits {
        if selected.len() >= limit {
            break;
        }
        if seen_chunk_ids.contains(&hit.chunk_id) || selected.contains(&hit.chunk_id) {
            continue;
        }
        selected.push(hit.chunk_id.clone());
    }

    selected
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
    pub fn child_executor(&self) -> &E {
        &self.executor
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
        let started = now_ms();
        let mut result = search_corpus_manifest(self.corpus, query, top_k, None);
        result.elapsed_ms = now_ms().saturating_sub(started);
        result
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
            artifact_kind: crate::trace::TRACE_ARTIFACT_KIND.to_string(),
            schema_version: crate::trace::TRACE_SCHEMA_VERSION,
            compat_version: crate::trace::TRACE_COMPAT_VERSION.to_string(),
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
            let artifacts = match self.prepare_iteration(
                task,
                iteration,
                &child_outputs,
                &seen_chunk_ids,
                web_policy.clone(),
                &mut trace,
            )? {
                Some(artifacts) => artifacts,
                None => {
                    stop_reason = no_iteration_artifacts_stop_reason(&child_outputs);
                    break;
                }
            };

            state.usage.iterations = iteration;
            state.usage.runtime_ms = now_ms().saturating_sub(started_at_ms);
            last_retrieval = Some(artifacts.retrieval.clone());

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

            let child_web_policy =
                effective_child_web_policy(web_policy.clone(), artifacts.escalation.clone());
            let request = ChildSubqueryRequest {
                subquery_id: format!("{task_id}-child-{iteration}"),
                prompt: build_child_prompt(task, iteration, &child_outputs, &artifacts.slices),
                slices: artifacts.slices,
                budget: child_budget,
                web_policy: child_web_policy,
                web_research_query: matches!(
                    artifacts.escalation.decision,
                    WebAccessDecision::Allowed | WebAccessDecision::RequiresApproval
                )
                .then(|| task.to_string()),
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

            let child_output = match self.executor.execute(&request) {
                Ok(output) => output,
                Err(error) => {
                    return finalize_failed_run(
                        task,
                        trace,
                        trace_artifact_dir,
                        RecursiveStopReason::ChildFailed,
                        mode,
                        last_retrieval,
                        child_outputs,
                        state.usage,
                        tracer,
                        error.to_string(),
                    );
                }
            };

            if !child_output.web_evidence.is_empty() {
                let sequence = u32::try_from(trace.events.len() + 1).unwrap_or(u32::MAX);
                trace.events.push(web_evidence_trace_event(
                    sequence,
                    now_ms(),
                    &child_output.web_evidence,
                ));
            }
            if let Some(execution) = child_output.web_execution.as_ref() {
                push_trace_event(
                    &mut trace,
                    TraceEventType::WebExecutionCompleted,
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
                            "status".to_string(),
                            JsonValue::String(execution.status.as_str().to_string()),
                        ),
                        ("approved".to_string(), JsonValue::Bool(execution.approved)),
                        ("degraded".to_string(), JsonValue::Bool(execution.degraded)),
                        (
                            "evidenceCount".to_string(),
                            JsonValue::Number(i64::from(execution.evidence_count)),
                        ),
                        (
                            "query".to_string(),
                            execution
                                .query
                                .clone()
                                .map(JsonValue::String)
                                .unwrap_or(JsonValue::Null),
                        ),
                        (
                            "note".to_string(),
                            execution
                                .note
                                .clone()
                                .map(JsonValue::String)
                                .unwrap_or(JsonValue::Null),
                        ),
                    ]),
                );
            }
            let web_evidence_count = child_output.web_evidence.len();
            let mut outputs_with_current = child_outputs.clone();
            outputs_with_current.push(child_output.clone());
            let (
                novel_citation_count,
                novel_web_evidence_count,
                novel_answer_token_count,
                materially_novel,
            ) = child_output_novelty_metrics(&outputs_with_current).unwrap_or((
                child_output.citations.len(),
                web_evidence_count,
                0,
                true,
            ));
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
                    (
                        "webEvidenceCount".to_string(),
                        JsonValue::Number(i64::try_from(web_evidence_count).unwrap_or(i64::MAX)),
                    ),
                    (
                        "novelCitationCount".to_string(),
                        JsonValue::Number(i64::try_from(novel_citation_count).unwrap_or(i64::MAX)),
                    ),
                    (
                        "novelWebEvidenceCount".to_string(),
                        JsonValue::Number(
                            i64::try_from(novel_web_evidence_count).unwrap_or(i64::MAX),
                        ),
                    ),
                    (
                        "novelAnswerTokenCount".to_string(),
                        JsonValue::Number(
                            i64::try_from(novel_answer_token_count).unwrap_or(i64::MAX),
                        ),
                    ),
                    (
                        "materiallyNovel".to_string(),
                        JsonValue::Bool(materially_novel),
                    ),
                    (
                        "answerPreview".to_string(),
                        JsonValue::String(child_output.answer.chars().take(180).collect()),
                    ),
                    (
                        "citations".to_string(),
                        JsonValue::Array(
                            child_output
                                .citations
                                .iter()
                                .cloned()
                                .map(JsonValue::String)
                                .collect(),
                        ),
                    ),
                    (
                        "webCollectionDegraded".to_string(),
                        JsonValue::Bool(
                            child_output
                                .web_execution
                                .as_ref()
                                .is_some_and(|execution| execution.degraded),
                        ),
                    ),
                    (
                        "webExecutionStatus".to_string(),
                        child_output
                            .web_execution
                            .as_ref()
                            .map(|execution| {
                                JsonValue::String(execution.status.as_str().to_string())
                            })
                            .unwrap_or(JsonValue::Null),
                    ),
                    (
                        "webApprovalState".to_string(),
                        child_output
                            .web_execution
                            .as_ref()
                            .map(|execution| {
                                JsonValue::String(
                                    if execution.approved {
                                        "approved"
                                    } else {
                                        "not_approved"
                                    }
                                    .to_string(),
                                )
                            })
                            .unwrap_or(JsonValue::Null),
                    ),
                    (
                        "webQuery".to_string(),
                        request
                            .web_research_query
                            .clone()
                            .map(JsonValue::String)
                            .unwrap_or(JsonValue::Null),
                    ),
                    (
                        "webExecutionNote".to_string(),
                        child_output
                            .web_execution_note
                            .clone()
                            .map(JsonValue::String)
                            .unwrap_or(JsonValue::Null),
                    ),
                    (
                        "webEscalationDecision".to_string(),
                        JsonValue::String(
                            web_access_decision_label(artifacts.escalation.decision).to_string(),
                        ),
                    ),
                    (
                        "webEscalationReason".to_string(),
                        JsonValue::String(
                            escalation_reason_label(artifacts.escalation.reason).to_string(),
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
                selected_chunk_ids: artifacts.selected_chunk_ids.clone(),
            });
            seen_chunk_ids.extend(artifacts.selected_chunk_ids);
            last_escalation = Some(artifacts.escalation);
            child_outputs.push(child_output);

            if should_stop_for_convergence(&child_outputs) {
                stop_reason = RecursiveStopReason::Converged;
                break;
            }

            if let Some(reason) = state
                .budget
                .exhausted_by(&state.usage)
                .map(RecursiveStopReason::from)
            {
                stop_reason = reason;
                break;
            }
        }

        let aggregated_body =
            (!child_outputs.is_empty()).then(|| self.aggregator.aggregate(task, &child_outputs));

        finalize_successful_run(
            task,
            trace,
            trace_artifact_dir,
            stop_reason,
            mode,
            last_retrieval,
            last_escalation,
            child_outputs,
            state,
            tracer,
            aggregated_body,
        )
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

    fn prepare_iteration(
        &self,
        task: &str,
        iteration: u32,
        child_outputs: &[ChildSubqueryOutput],
        seen_chunk_ids: &[String],
        web_policy: WebPolicy,
        trace: &mut TraceLedger,
    ) -> Result<Option<IterationArtifacts>, RecursiveRuntimeError> {
        let plan = build_iteration_plan(task, child_outputs);
        push_trace_event(
            trace,
            TraceEventType::RetrievalRequested,
            BTreeMap::from([
                ("query".to_string(), JsonValue::String(plan.query.clone())),
                (
                    "iteration".to_string(),
                    JsonValue::Number(i64::from(iteration)),
                ),
                (
                    "plannerStrategy".to_string(),
                    JsonValue::String(plan.strategy.to_string()),
                ),
                (
                    "plannerRationale".to_string(),
                    JsonValue::String(plan.rationale.to_string()),
                ),
                (
                    "plannerAnchorTerms".to_string(),
                    JsonValue::Array(
                        plan.anchor_terms
                            .iter()
                            .cloned()
                            .map(JsonValue::String)
                            .collect(),
                    ),
                ),
                (
                    "plannerGapTerms".to_string(),
                    JsonValue::Array(
                        plan.gap_terms
                            .iter()
                            .cloned()
                            .map(JsonValue::String)
                            .collect(),
                    ),
                ),
                (
                    "plannerValidationTerms".to_string(),
                    JsonValue::Array(
                        plan.validation_terms
                            .iter()
                            .cloned()
                            .map(JsonValue::String)
                            .collect(),
                    ),
                ),
            ]),
        );
        let retrieval = self.corpus_search(&plan.query, 6);
        let sequence = u32::try_from(trace.events.len() + 1).unwrap_or(u32::MAX);
        let mut retrieval_event = local_evidence_trace_event(sequence, now_ms(), &retrieval);
        retrieval_event.data.insert(
            "iteration".to_string(),
            JsonValue::Number(i64::from(iteration)),
        );
        trace.events.push(retrieval_event);

        let selected_chunk_ids = select_diverse_chunk_ids(&retrieval.hits, seen_chunk_ids, 3);
        if selected_chunk_ids.is_empty() {
            return Ok(None);
        }

        let slices = self.select_slices(&selected_chunk_ids)?;
        push_trace_event(
            trace,
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
        if escalation.reason != EscalationReason::LocalEvidenceSufficient {
            push_trace_event(
                trace,
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
                        JsonValue::String(escalation_reason_label(escalation.reason).to_string()),
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

        Ok(Some(IterationArtifacts {
            retrieval,
            selected_chunk_ids,
            slices,
            escalation,
        }))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::corpus::{
        CorpusBackend, CorpusChunk, CorpusDocument, CorpusKind, CorpusRootSummary,
    };
    use crate::hybrid::{EvidenceKind, EvidenceRecord};
    use crate::ExecutionProfile;
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::{Mutex, OnceLock};

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
                web_execution: Some(crate::WebExecutionOutcome::not_requested()),
                web_execution_note: None,
                prompt_tokens: 120,
                completion_tokens: 40,
                cost_usd: 0.02,
            })
        }
    }

    struct FlakyExecutor {
        calls: std::sync::Mutex<u32>,
    }

    impl ChildSubqueryExecutor for FlakyExecutor {
        fn execute(
            &self,
            request: &ChildSubqueryRequest,
        ) -> Result<ChildSubqueryOutput, RecursiveRuntimeError> {
            let mut calls = self.calls.lock().expect("lock should succeed");
            *calls += 1;
            if *calls >= 2 {
                return Err(RecursiveRuntimeError::ChildExecution(
                    "child executor crashed on follow-up".to_string(),
                ));
            }
            Ok(ChildSubqueryOutput {
                subquery_id: request.subquery_id.clone(),
                answer: "first child succeeded".to_string(),
                citations: vec![request.slices[0].chunk_id.clone()],
                web_evidence: Vec::new(),
                web_execution: Some(crate::WebExecutionOutcome::not_requested()),
                web_execution_note: None,
                prompt_tokens: 80,
                completion_tokens: 20,
                cost_usd: 0.01,
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
            artifact_kind: crate::corpus::CORPUS_ARTIFACT_KIND.to_string(),
            schema_version: crate::corpus::CORPUS_SCHEMA_VERSION,
            compat_version: crate::corpus::CORPUS_COMPAT_VERSION.to_string(),
            corpus_id: "corpus-1".to_string(),
            roots: vec!["docs".to_string()],
            kind: CorpusKind::Docs,
            backend: CorpusBackend::Lexical,
            document_count: 1,
            chunk_count: 4,
            estimated_bytes: 520,
            root_summaries: vec![CorpusRootSummary {
                root: "docs".to_string(),
                document_count: 1,
                chunk_count: 4,
            }],
            skip_summary: crate::corpus::CorpusSkipSummary::empty(),
            documents: vec![CorpusDocument {
                document_id: doc_id,
                source_root: "docs".to_string(),
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

    fn multi_doc_hits() -> Vec<RetrievalHit> {
        vec![
            RetrievalHit {
                chunk_id: "chunk-a1".to_string(),
                document_id: "doc-a".to_string(),
                source_root: "docs".to_string(),
                path: "docs/a.md".to_string(),
                score: 12.0,
                reason: "phrase:body".to_string(),
                matched_terms: vec!["alpha".to_string()],
                preview: "alpha".to_string(),
            },
            RetrievalHit {
                chunk_id: "chunk-a2".to_string(),
                document_id: "doc-a".to_string(),
                source_root: "docs".to_string(),
                path: "docs/a.md".to_string(),
                score: 11.5,
                reason: "coverage:3/3".to_string(),
                matched_terms: vec!["alpha".to_string(), "trace".to_string()],
                preview: "alpha-2".to_string(),
            },
            RetrievalHit {
                chunk_id: "chunk-b1".to_string(),
                document_id: "doc-b".to_string(),
                source_root: "docs".to_string(),
                path: "docs/b.md".to_string(),
                score: 10.0,
                reason: "content:beta".to_string(),
                matched_terms: vec!["beta".to_string()],
                preview: "beta".to_string(),
            },
            RetrievalHit {
                chunk_id: "chunk-c1".to_string(),
                document_id: "doc-c".to_string(),
                source_root: "docs".to_string(),
                path: "docs/c.md".to_string(),
                score: 9.0,
                reason: "content:gamma".to_string(),
                matched_terms: vec!["gamma".to_string()],
                preview: "gamma".to_string(),
            },
        ]
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
    fn diverse_chunk_selection_prefers_distinct_documents_before_fill_in() {
        let selected = select_diverse_chunk_ids(&multi_doc_hits(), &[], 3);
        assert_eq!(selected, vec!["chunk-a1", "chunk-b1", "chunk-c1"]);

        let selected_with_seen =
            select_diverse_chunk_ids(&multi_doc_hits(), &["chunk-a1".to_string()], 3);
        assert_eq!(selected_with_seen, vec!["chunk-a2", "chunk-b1", "chunk-c1"]);
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
        assert!(summary.contains("Web executions"));
        assert!(summary.contains("Web pending"));
        assert!(summary.contains("Web no evidence"));
        assert!(summary.contains("Web degraded"));

        let exported = export_trace(&result.trace, &trace_dir.join("exported"))
            .expect("trace export helper should succeed");
        assert!(exported.is_file());

        let _ = fs::remove_dir_all(trace_dir);
    }

    #[test]
    fn iterative_run_stops_when_novel_context_is_exhausted() {
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

        assert_eq!(result.stop_reason, RecursiveStopReason::NoNewContext);
        assert_eq!(result.child_outputs.len(), 2);
        assert_eq!(result.usage.iterations, 2);
        assert_eq!(result.usage.subcalls, 2);
        assert!(result.final_answer.matches("inspected").count() >= 1);
        let counters = result.trace.counters();
        assert_eq!(counters.retrieval_requests, 3);
        assert_eq!(counters.subqueries_completed, 2);
        assert!(result.trace.events.iter().any(|event| {
            event.event_type == TraceEventType::StopConditionReached
                && event.data.get("stopReason")
                    == Some(&JsonValue::String("no_new_context".to_string()))
                && event.data.get("childCount") == Some(&JsonValue::Number(2))
                && event.data.get("completedIterations") == Some(&JsonValue::Number(2))
                && event.data.get("subcalls") == Some(&JsonValue::Number(2))
        }));
    }

    #[test]
    fn repeated_child_results_are_treated_as_convergence() {
        let corpus = sample_corpus();
        let runtime = RecursiveConversationRuntime::new(&corpus, ConvergingExecutor);

        let result = runtime
            .run(
                "session-1",
                "task-converged",
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
            .expect("run should stop on convergence");

        assert_eq!(result.stop_reason, RecursiveStopReason::Converged);
        assert_eq!(result.child_outputs.len(), 2);
        assert_eq!(result.usage.iterations, 2);
        assert_eq!(result.usage.subcalls, 2);
    }

    #[test]
    fn low_novelty_follow_up_results_also_converge() {
        let corpus = sample_corpus();
        let runtime = RecursiveConversationRuntime::new(
            &corpus,
            LowNoveltyExecutor {
                calls: std::sync::Mutex::new(0),
            },
        );

        let result = runtime
            .run(
                "session-1",
                "task-low-novelty",
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
            .expect("run should stop on low novelty");

        assert_eq!(result.stop_reason, RecursiveStopReason::Converged);
        assert_eq!(result.child_outputs.len(), 2);
        assert!(result.trace.events.iter().any(|event| {
            event.event_type == TraceEventType::SubqueryCompleted
                && event.data.get("iteration") == Some(&JsonValue::Number(2))
                && event.data.get("materiallyNovel") == Some(&JsonValue::Bool(false))
                && event.data.get("novelCitationCount") == Some(&JsonValue::Number(0))
        }));
    }

    #[test]
    fn retrieval_trace_includes_planner_metadata() {
        let corpus = sample_corpus();
        let runtime = RecursiveConversationRuntime::new(&corpus, StubExecutor);

        let result = runtime
            .run(
                "session-1",
                "task-planner-trace",
                "investigate recursive scheduler provenance gaps",
                RuntimeBudget {
                    max_depth: Some(2),
                    max_iterations: Some(2),
                    max_subcalls: Some(2),
                    max_runtime_ms: Some(30_000),
                    ..RuntimeBudget::default()
                },
                None,
            )
            .expect("run should succeed");

        let retrieval_event = result
            .trace
            .events
            .iter()
            .find(|event| event.event_type == TraceEventType::RetrievalRequested)
            .expect("retrieval request event should exist");

        assert_eq!(
            retrieval_event.data.get("plannerStrategy"),
            Some(&JsonValue::String("bootstrap".to_string()))
        );
        assert!(matches!(
            retrieval_event.data.get("plannerAnchorTerms"),
            Some(JsonValue::Array(values)) if !values.is_empty()
        ));
        assert!(matches!(
            retrieval_event.data.get("plannerGapTerms"),
            Some(JsonValue::Array(values)) if values.is_empty()
        ));
        assert!(matches!(
            retrieval_event.data.get("plannerValidationTerms"),
            Some(JsonValue::Array(values)) if values.is_empty()
        ));
        assert!(retrieval_event.data.contains_key("plannerRationale"));
    }

    #[test]
    fn child_prompt_includes_validation_loop_and_prior_evidence() {
        let slices = vec![RecursiveContextSlice {
            chunk_id: "chunk-1".to_string(),
            document_id: "doc-1".to_string(),
            path: "docs/spec.md".to_string(),
            ordinal: 0,
            start_offset: 0,
            end_offset: 42,
            preview: "preview".to_string(),
            metadata: BTreeMap::from([(
                "text".to_string(),
                JsonValue::String("actual text from slice".to_string()),
            )]),
        }];
        let prior_outputs = vec![ChildSubqueryOutput {
            subquery_id: "child-1".to_string(),
            answer: "first pass found a likely workflow".to_string(),
            citations: vec!["chunk-1".to_string()],
            web_evidence: Vec::new(),
            web_execution: Some(crate::WebExecutionOutcome::not_requested()),
            web_execution_note: Some("needs manual validation".to_string()),
            prompt_tokens: 0,
            completion_tokens: 0,
            cost_usd: 0.0,
        }];

        let prompt = build_child_prompt("audit the workflow", 2, &prior_outputs, &slices);

        assert!(prompt.contains("Validation loop"));
        assert!(prompt.contains("Remaining gaps"));
        assert!(prompt.contains("avoid repeating"));
        assert!(prompt.contains("chunk-1"));
        assert!(prompt.contains("needs manual validation"));
        assert!(prompt.contains("Do not claim completion"));
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
    fn prompt_token_cap_stops_before_launching_a_follow_up_child() {
        let corpus = sample_corpus();
        let runtime = RecursiveConversationRuntime::new(&corpus, StubExecutor);

        let result = runtime
            .run(
                "session-1",
                "task-prompt-cap",
                "trace aggregation export hidden",
                RuntimeBudget {
                    max_depth: Some(2),
                    max_iterations: Some(4),
                    max_subcalls: Some(4),
                    max_prompt_tokens: Some(120),
                    ..RuntimeBudget::default()
                },
                None,
            )
            .expect("run should stop at prompt token cap");

        assert_eq!(result.stop_reason, RecursiveStopReason::PromptTokenCap);
        assert_eq!(result.child_outputs.len(), 1);
        assert_eq!(result.usage.prompt_tokens, 120);
        assert_eq!(result.trace.final_status, TraceFinalStatus::BudgetExceeded);
    }

    #[test]
    fn child_failure_returns_partial_result_and_records_failure_trace() {
        let corpus = sample_corpus();
        let runtime = RecursiveConversationRuntime::new(
            &corpus,
            FlakyExecutor {
                calls: std::sync::Mutex::new(0),
            },
        );

        let result = runtime
            .run(
                "session-1",
                "task-flaky",
                "trace aggregation export hidden",
                RuntimeBudget {
                    max_depth: Some(2),
                    max_iterations: Some(4),
                    max_subcalls: Some(4),
                    ..RuntimeBudget::default()
                },
                None,
            )
            .expect("run should degrade gracefully on child failure");

        assert_eq!(result.stop_reason, RecursiveStopReason::ChildFailed);
        assert_eq!(result.child_outputs.len(), 1);
        assert!(result.final_answer.contains("Partial recursive findings"));
        assert_eq!(result.trace.final_status, TraceFinalStatus::Failed);
        assert!(result.trace.events.iter().any(|event| event.event_type
            == TraceEventType::TaskFailed
            && event.data.get("message")
                == Some(&JsonValue::String(
                    "child executor crashed on follow-up".to_string(),
                ))));
        assert!(result.trace.events.iter().any(|event| {
            event.event_type == TraceEventType::StopConditionReached
                && event.data.get("stopReason")
                    == Some(&JsonValue::String("child_failed".to_string()))
        }));
        assert!(result.usage.subcalls >= 1);
        assert!(result.usage.iterations >= 1);
    }

    #[test]
    fn run_with_web_policy_records_escalation_and_child_inheritance() {
        let corpus = sample_corpus();
        captured_web_modes()
            .lock()
            .expect("lock should succeed")
            .clear();
        let runtime = RecursiveConversationRuntime::new(&corpus, CapturingExecutor);
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
        assert_eq!(
            captured_web_modes()
                .lock()
                .expect("lock should succeed")
                .as_slice(),
            &[WebAccessMode::Ask]
        );
        assert!(result.final_answer.contains("requires approval"));
    }

    static CAPTURED_WEB_MODES: OnceLock<Mutex<Vec<WebAccessMode>>> = OnceLock::new();

    fn captured_web_modes() -> &'static Mutex<Vec<WebAccessMode>> {
        CAPTURED_WEB_MODES.get_or_init(|| Mutex::new(Vec::new()))
    }

    struct CapturingExecutor;

    impl ChildSubqueryExecutor for CapturingExecutor {
        fn execute(
            &self,
            request: &ChildSubqueryRequest,
        ) -> Result<ChildSubqueryOutput, RecursiveRuntimeError> {
            captured_web_modes()
                .lock()
                .expect("lock should succeed")
                .push(request.web_policy.mode);
            Ok(ChildSubqueryOutput {
                subquery_id: request.subquery_id.clone(),
                answer: format!(
                    "executed with web mode {}",
                    web_policy_label(request.web_policy.mode)
                ),
                citations: request
                    .slices
                    .iter()
                    .map(|slice| slice.chunk_id.clone())
                    .collect(),
                web_evidence: Vec::new(),
                web_execution: Some(crate::WebExecutionOutcome::no_evidence(
                    request
                        .web_research_query
                        .clone()
                        .unwrap_or_else(|| "unknown".to_string()),
                    format!(
                        "captured executor ran with web mode {} and query {:?}",
                        web_policy_label(request.web_policy.mode),
                        request.web_research_query
                    ),
                )),
                web_execution_note: Some(format!(
                    "captured executor ran with web mode {} and query {:?}",
                    web_policy_label(request.web_policy.mode),
                    request.web_research_query
                )),
                prompt_tokens: 20,
                completion_tokens: 10,
                cost_usd: 0.0,
            })
        }
    }

    struct AskAwareExecutor;

    impl ChildSubqueryExecutor for AskAwareExecutor {
        fn execute(
            &self,
            request: &ChildSubqueryRequest,
        ) -> Result<ChildSubqueryOutput, RecursiveRuntimeError> {
            let (web_execution, web_execution_note) =
                if matches!(request.web_policy.mode, WebAccessMode::Ask) {
                    let query = request
                        .web_research_query
                        .clone()
                        .unwrap_or_else(|| "missing-query".to_string());
                    let note = format!(
                        "approval required before using the web for query {:?}",
                        request.web_research_query
                    );
                    (
                        Some(crate::WebExecutionOutcome::approval_required(
                            query,
                            note.clone(),
                        )),
                        Some(note),
                    )
                } else {
                    (Some(crate::WebExecutionOutcome::not_requested()), None)
                };
            Ok(ChildSubqueryOutput {
                subquery_id: request.subquery_id.clone(),
                answer: format!(
                    "ask-aware executor ran with web mode {} and query {:?}",
                    web_policy_label(request.web_policy.mode),
                    request.web_research_query
                ),
                citations: request
                    .slices
                    .iter()
                    .map(|slice| slice.chunk_id.clone())
                    .collect(),
                web_evidence: Vec::new(),
                web_execution,
                web_execution_note,
                prompt_tokens: 12,
                completion_tokens: 4,
                cost_usd: 0.0,
            })
        }
    }

    struct ConvergingExecutor;

    impl ChildSubqueryExecutor for ConvergingExecutor {
        fn execute(
            &self,
            request: &ChildSubqueryRequest,
        ) -> Result<ChildSubqueryOutput, RecursiveRuntimeError> {
            Ok(ChildSubqueryOutput {
                subquery_id: request.subquery_id.clone(),
                answer: "stable answer".to_string(),
                citations: vec!["stable-citation".to_string()],
                web_evidence: Vec::new(),
                web_execution: Some(crate::WebExecutionOutcome::not_requested()),
                web_execution_note: None,
                prompt_tokens: 15,
                completion_tokens: 5,
                cost_usd: 0.0,
            })
        }
    }

    struct LowNoveltyExecutor {
        calls: std::sync::Mutex<u32>,
    }

    impl ChildSubqueryExecutor for LowNoveltyExecutor {
        fn execute(
            &self,
            request: &ChildSubqueryRequest,
        ) -> Result<ChildSubqueryOutput, RecursiveRuntimeError> {
            let mut calls = self.calls.lock().expect("lock should succeed");
            *calls += 1;
            let answer = if *calls == 1 {
                "stable answer".to_string()
            } else {
                "stable answer again".to_string()
            };
            Ok(ChildSubqueryOutput {
                subquery_id: request.subquery_id.clone(),
                answer,
                citations: vec!["stable-citation".to_string()],
                web_evidence: Vec::new(),
                web_execution: Some(crate::WebExecutionOutcome::not_requested()),
                web_execution_note: Some("reran validation without new evidence".to_string()),
                prompt_tokens: 15,
                completion_tokens: 5,
                cost_usd: 0.0,
            })
        }
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
                web_execution: Some(crate::WebExecutionOutcome::succeeded(
                    request
                        .web_research_query
                        .clone()
                        .unwrap_or_else(|| "unknown".to_string()),
                    1,
                    Some("attached bounded external evidence".to_string()),
                )),
                web_execution_note: Some("attached bounded external evidence".to_string()),
                prompt_tokens: 10,
                completion_tokens: 5,
                cost_usd: 0.0,
            })
        }
    }

    #[test]
    fn local_only_flows_force_child_web_policy_off_even_when_parent_allows_web() {
        let corpus = sample_corpus();
        captured_web_modes()
            .lock()
            .expect("lock should succeed")
            .clear();
        let runtime = RecursiveConversationRuntime::new(&corpus, CapturingExecutor);
        let result = runtime
            .run_with_tracer_and_policy(
                "session-1",
                "task-local-only",
                "summarize hidden behavior from the local corpus",
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
                    max_fetches: Some(3),
                },
            )
            .expect("run should succeed");

        assert_eq!(
            captured_web_modes()
                .lock()
                .expect("lock should succeed")
                .as_slice(),
            &[WebAccessMode::Off]
        );
        assert!(!result
            .trace
            .events
            .iter()
            .any(|event| event.event_type == TraceEventType::WebEscalationStarted));
        assert!(result.trace.events.iter().any(|event| {
            event.event_type == TraceEventType::RetrievalCompleted
                && event.data.get("evidenceKind")
                    == Some(&JsonValue::String("local".to_string()))
                && matches!(event.data.get("records"), Some(JsonValue::Array(records)) if !records.is_empty())
        }));
    }

    #[test]
    fn trace_marks_degraded_web_collection_when_approved_subquery_returns_no_web_evidence() {
        let corpus = sample_corpus();
        let runtime = RecursiveConversationRuntime::new(&corpus, CapturingExecutor);
        let result = runtime
            .run_with_tracer_and_policy(
                "session-1",
                "task-web-missing",
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
            .contains("no web evidence was attached by the child executor"));
        assert!(result
            .final_answer
            .contains("captured executor ran with web mode on"));
        assert!(result.trace.events.iter().any(|event| {
            event.event_type == TraceEventType::SubqueryCompleted
                && event.data.get("webEvidenceCount") == Some(&JsonValue::Number(0))
                && event.data.get("webCollectionDegraded") == Some(&JsonValue::Bool(true))
                && event.data.get("webQuery")
                    == Some(&JsonValue::String(
                        "search the web for the latest hidden behavior".to_string()
                    ))
                && matches!(event.data.get("webExecutionNote"), Some(JsonValue::String(note)) if note.contains("captured executor ran with web mode on"))
        }));
        assert!(result.trace.events.iter().any(|event| {
            event.event_type == TraceEventType::WebExecutionCompleted
                && event.data.get("status") == Some(&JsonValue::String("no_evidence".to_string()))
                && event.data.get("approved") == Some(&JsonValue::Bool(true))
                && event.data.get("degraded") == Some(&JsonValue::Bool(true))
        }));
    }

    #[test]
    fn ask_mode_preserves_web_query_and_approval_required_trace_state() {
        let corpus = sample_corpus();
        let runtime = RecursiveConversationRuntime::new(&corpus, AskAwareExecutor);
        let result = runtime
            .run_with_tracer_and_policy(
                "session-1",
                "task-web-approval",
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
                    mode: WebAccessMode::Ask,
                    max_fetches: Some(2),
                },
            )
            .expect("run should succeed");

        assert!(result
            .final_answer
            .contains("approval-required subqueries=1"));
        assert!(result
            .final_answer
            .contains("approval required before using the web for query Some(\"search the web for the latest hidden behavior\")"));
        assert!(result.trace.events.iter().any(|event| {
            event.event_type == TraceEventType::SubqueryCompleted
                && event.data.get("webQuery")
                    == Some(&JsonValue::String(
                        "search the web for the latest hidden behavior".to_string(),
                    ))
                && event.data.get("webApprovalState")
                    == Some(&JsonValue::String("not_approved".to_string()))
        }));
        assert!(result.trace.events.iter().any(|event| {
            event.event_type == TraceEventType::WebExecutionCompleted
                && event.data.get("status")
                    == Some(&JsonValue::String("approval_required".to_string()))
                && event.data.get("approved") == Some(&JsonValue::Bool(false))
                && event.data.get("degraded") == Some(&JsonValue::Bool(true))
                && event.data.get("query")
                    == Some(&JsonValue::String(
                        "search the web for the latest hidden behavior".to_string(),
                    ))
        }));
        let summary = render_trace_summary(&result.trace);
        assert!(summary.contains("Operator state   awaiting approval"));
        assert!(summary.contains("Pending queries  search the web for the latest hidden behavior"));
        assert!(summary.contains(
            "Next step        approve web queries: search the web for the latest hidden behavior"
        ));
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

        println!("FINAL ANSWER:\n{}", result.final_answer);
        assert!(result.final_answer.contains("Web sources"));
        assert!(result.final_answer.contains("[W1] Example release notes"));
        assert!(result
            .final_answer
            .contains("Web execution summary: web-aware subqueries=1, approved subqueries=1"));
        assert!(result
            .trace
            .events
            .iter()
            .any(|event| event.event_type == TraceEventType::WebEvidenceAdded));
        assert!(result.trace.events.iter().any(|event| {
            event.event_type == TraceEventType::WebExecutionCompleted
                && event.data.get("status") == Some(&JsonValue::String("succeeded".to_string()))
                && event.data.get("evidenceCount") == Some(&JsonValue::Number(1))
                && event.data.get("degraded") == Some(&JsonValue::Bool(false))
        }));
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

    #[test]
    fn fallback_child_executor_uses_unavailable_reason_when_present() {
        let request = ChildSubqueryRequest {
            subquery_id: "subq-fallback".to_string(),
            prompt: "summarize".to_string(),
            slices: Vec::new(),
            budget: RuntimeBudget::default(),
            web_policy: WebPolicy {
                mode: WebAccessMode::Off,
                max_fetches: Some(0),
            },
            web_research_query: None,
        };
        let executor = FallbackChildSubqueryExecutor::new(
            AlwaysFailExecutor,
            "claude-sonnet-4-6",
            std::sync::Arc::new(|| Some("backend unavailable".to_string())),
            std::sync::Arc::new(|error| format!("formatted: {error}")),
            std::sync::Arc::new(|request, reason| ChildSubqueryOutput {
                subquery_id: request.subquery_id.clone(),
                answer: format!("fallback via {reason}"),
                citations: Vec::new(),
                web_evidence: Vec::new(),
                web_execution: Some(crate::WebExecutionOutcome::not_requested()),
                web_execution_note: None,
                prompt_tokens: 0,
                completion_tokens: 0,
                cost_usd: 0.0,
            }),
        );

        let output = executor
            .execute(&request)
            .expect("fallback wrapper should recover");
        assert_eq!(executor.model(), "claude-sonnet-4-6");
        assert_eq!(output.answer, "fallback via backend unavailable");
    }

    #[test]
    fn fallback_child_executor_formats_runtime_error_when_backend_reason_absent() {
        let request = ChildSubqueryRequest {
            subquery_id: "subq-fallback".to_string(),
            prompt: "summarize".to_string(),
            slices: Vec::new(),
            budget: RuntimeBudget::default(),
            web_policy: WebPolicy {
                mode: WebAccessMode::Off,
                max_fetches: Some(0),
            },
            web_research_query: None,
        };
        let executor = FallbackChildSubqueryExecutor::new(
            AlwaysFailExecutor,
            "claude-sonnet-4-6",
            std::sync::Arc::new(|| None),
            std::sync::Arc::new(|error| format!("formatted: {error}")),
            std::sync::Arc::new(|request, reason| ChildSubqueryOutput {
                subquery_id: request.subquery_id.clone(),
                answer: reason.to_string(),
                citations: Vec::new(),
                web_evidence: Vec::new(),
                web_execution: Some(crate::WebExecutionOutcome::not_requested()),
                web_execution_note: None,
                prompt_tokens: 0,
                completion_tokens: 0,
                cost_usd: 0.0,
            }),
        );

        let output = executor
            .execute(&request)
            .expect("fallback wrapper should recover");
        assert_eq!(output.answer, "formatted: child blew up");
    }

    #[test]
    fn shared_recursive_task_runner_owns_telemetry_and_trace_artifacts() {
        let corpus = sample_corpus();
        let runtime = RecursiveConversationRuntime::new(&corpus, StubExecutor);
        let workspace_root = temp_trace_dir()
            .parent()
            .expect("trace dir parent")
            .to_path_buf();
        let prepared = prepare_recursive_task_run(RecursiveProfileTaskRequest {
            workspace: RecursiveTaskWorkspace {
                cwd: &workspace_root,
                session_id: "session-task-runner",
            },
            task_id: "task-task-runner",
            task: "trace aggregation export",
            profile: ExecutionProfile::Balanced,
        });

        let (result, artifacts) = runtime
            .run_task(prepared.as_request())
            .expect("shared task runner should succeed");

        assert_eq!(artifacts.telemetry_path, prepared.telemetry_path);
        assert_eq!(artifacts.trace_dir, prepared.trace_dir);
        assert!(artifacts.telemetry_path.is_file());
        assert!(result
            .trace_artifact_path
            .as_ref()
            .is_some_and(|path| path.starts_with(&artifacts.trace_dir)));
        let telemetry = std::fs::read_to_string(&artifacts.telemetry_path)
            .expect("telemetry log should be readable");
        assert!(!telemetry.trim().is_empty());
        assert!(telemetry.contains("session-task-runner"));
    }

    #[test]
    fn prepared_recursive_task_run_centralizes_profile_budget_and_paths() {
        let workspace_root = temp_trace_dir()
            .parent()
            .expect("trace dir parent")
            .to_path_buf();
        let prepared = prepare_recursive_task_run(RecursiveProfileTaskRequest {
            workspace: RecursiveTaskWorkspace {
                cwd: &workspace_root,
                session_id: "session-prepared",
            },
            task_id: "task-prepared",
            task: "investigate grounded execution",
            profile: ExecutionProfile::Research,
        });

        assert_eq!(prepared.session_id, "session-prepared");
        assert_eq!(prepared.task_id, "task-prepared");
        assert_eq!(prepared.task, "investigate grounded execution");
        assert_eq!(
            prepared.telemetry_path,
            workspace_root
                .join(".claw")
                .join("telemetry")
                .join("recursive-runtime.jsonl")
        );
        assert_eq!(
            prepared.trace_dir,
            workspace_root.join(".claw").join("trace")
        );
        assert_eq!(prepared.budget.max_depth, Some(4));
        assert_eq!(prepared.web_policy.mode, WebAccessMode::Ask);

        let borrowed = prepared.as_request();
        assert_eq!(borrowed.session_id, "session-prepared");
        assert_eq!(borrowed.task_id, "task-prepared");
        assert_eq!(borrowed.task, "investigate grounded execution");
        assert_eq!(
            borrowed.telemetry_path,
            workspace_root
                .join(".claw")
                .join("telemetry")
                .join("recursive-runtime.jsonl")
        );
        assert_eq!(
            borrowed.trace_dir,
            workspace_root.join(".claw").join("trace")
        );
    }

    #[test]
    fn recursive_task_envelope_reuses_shared_prepare_and_run_flow() {
        #[derive(Clone)]
        struct StubRuntimeFactory<'a> {
            workspace: RecursiveTaskWorkspace<'a>,
        }

        impl<'a> RecursiveTaskWorkspaceProvider<'a> for StubRuntimeFactory<'a> {
            fn workspace(&self) -> RecursiveTaskWorkspace<'a> {
                self.workspace.clone()
            }
        }

        impl<'a> RecursiveRuntimeFactory<'a> for StubRuntimeFactory<'a> {
            type Executor = StubExecutor;
            type Aggregator = DefaultChildOutputAggregator;

            fn build_runtime(
                &self,
                corpus: &'a CorpusManifest,
            ) -> RecursiveConversationRuntime<'a, Self::Executor, Self::Aggregator> {
                RecursiveConversationRuntime::new(corpus, StubExecutor)
            }
        }

        let corpus = sample_corpus();
        let workspace_root = temp_trace_dir()
            .parent()
            .expect("trace dir parent")
            .to_path_buf();
        let request = RecursiveTaskEnvelope {
            runtime: StubRuntimeFactory {
                workspace: RecursiveTaskWorkspace {
                    cwd: &workspace_root,
                    session_id: "session-envelope",
                },
            },
            corpus: &corpus,
            task_id: "task-envelope",
            task: "summarize recursive orchestration",
            profile: ExecutionProfile::Balanced,
        };

        let prepared = request.prepare();
        assert_eq!(prepared.session_id, "session-envelope");
        assert_eq!(prepared.task_id, "task-envelope");

        let (result, artifacts) = request.run().expect("shared envelope run should succeed");
        assert!(artifacts.telemetry_path.is_file());
        assert_eq!(result.stop_reason, RecursiveStopReason::NoChildCapacity);
    }

    struct AlwaysFailExecutor;

    impl ChildSubqueryExecutor for AlwaysFailExecutor {
        fn execute(
            &self,
            _request: &ChildSubqueryRequest,
        ) -> Result<ChildSubqueryOutput, RecursiveRuntimeError> {
            Err(RecursiveRuntimeError::ChildExecution(
                "child blew up".to_string(),
            ))
        }
    }
}
