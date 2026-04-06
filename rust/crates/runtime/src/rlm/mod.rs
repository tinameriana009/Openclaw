mod finalization;
mod helpers;
mod types;

use std::collections::BTreeMap;
use std::path::Path;

use crate::budget::{BudgetSliceRequest, RuntimeBudget, RuntimeBudgetUsage};
use crate::corpus::{CorpusManifest, RetrievalHit, RetrievalResult};
use crate::hybrid::{
    evaluate_web_escalation, is_local_evidence_weak, summarize_local_evidence,
    web_evidence_trace_event, EscalationHeuristicInput, EscalationReason, WebAccessDecision,
    WebAccessMode, WebPolicy,
};
use crate::json::JsonValue;
use crate::trace::{TraceEventType, TraceFinalStatus, TraceLedger};
pub use finalization::{export_trace, render_trace_summary};
use finalization::{finalize_empty_stop, finalize_failed_run, finalize_successful_run};
use helpers::{
    build_child_prompt, build_iteration_query, effective_child_web_policy, escalation_reason_label,
    map_chunk, mode_label, next_iteration_stop_reason, now_ms, push_trace_event, slice_text,
    task_mentions_freshness, task_requests_web, web_access_decision_label, web_policy_label,
};
use telemetry::SessionTracer;
pub use types::*;

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
                    stop_reason = if child_outputs.is_empty() {
                        RecursiveStopReason::NoChildCapacity
                    } else {
                        RecursiveStopReason::Completed
                    };
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
                    WebAccessDecision::Allowed
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
                selected_chunk_ids: artifacts.selected_chunk_ids.clone(),
            });
            seen_chunk_ids.extend(artifacts.selected_chunk_ids);
            last_escalation = Some(artifacts.escalation);
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
        let query = build_iteration_query(task, child_outputs);
        push_trace_event(
            trace,
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
            trace,
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

        let selected_chunk_ids = retrieval
            .hits
            .iter()
            .map(|hit| hit.chunk_id.clone())
            .filter(|chunk_id| !seen_chunk_ids.contains(chunk_id))
            .take(3)
            .collect::<Vec<_>>();
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
    use crate::corpus::{CorpusBackend, CorpusChunk, CorpusDocument, CorpusKind};
    use crate::hybrid::{EvidenceKind, EvidenceRecord};
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

        assert!(matches!(
            result.stop_reason.as_str(),
            "completed" | "no_new_context"
        ));
        assert_eq!(result.child_outputs.len(), 2);
        assert_eq!(result.usage.iterations, 2);
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
                prompt_tokens: 20,
                completion_tokens: 10,
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
        assert!(result
            .final_answer
            .contains("[W1] Example release notes"));
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
