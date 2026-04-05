use std::collections::BTreeMap;

use crate::config::{RuntimeWebResearchConfig, RuntimeWebResearchMode};
use crate::corpus::{CorpusBackend, RetrievalHit, RetrievalResult};
use crate::json::JsonValue;
use crate::trace::{TraceEvent, TraceEventType};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WebAccessMode {
    Off,
    Ask,
    On,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebPolicy {
    pub mode: WebAccessMode,
    pub max_fetches: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebAccessDecision {
    Denied,
    RequiresApproval,
    Allowed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HybridExecutionContext {
    pub web_policy: WebPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceKind {
    Local,
    Web,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvidenceRecord {
    pub kind: EvidenceKind,
    pub id: String,
    pub title: String,
    pub locator: String,
    pub snippet: String,
    pub score: Option<f64>,
    pub metadata: BTreeMap<String, JsonValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebEvidenceInput {
    pub id: String,
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub fetched_at_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalEvidenceSummary {
    pub total_hits: usize,
    pub distinct_documents: usize,
    pub best_score: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EscalationHeuristicInput {
    pub local_summary: LocalEvidenceSummary,
    pub requires_external_freshness: bool,
    pub user_requested_web: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EscalationReason {
    UserRequestedWeb,
    NoLocalEvidence,
    WeakLocalEvidence,
    FreshnessRequired,
    LocalEvidenceSufficient,
    PolicyDenied,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EscalationOutcome {
    pub decision: WebAccessDecision,
    pub reason: EscalationReason,
}

impl WebAccessMode {
    #[must_use]
    pub fn allows_web(self) -> bool {
        !matches!(self, Self::Off)
    }
}

impl From<RuntimeWebResearchMode> for WebAccessMode {
    fn from(value: RuntimeWebResearchMode) -> Self {
        match value {
            RuntimeWebResearchMode::Off => Self::Off,
            RuntimeWebResearchMode::Ask => Self::Ask,
            RuntimeWebResearchMode::On => Self::On,
        }
    }
}

impl WebPolicy {
    #[must_use]
    pub fn from_config(config: &RuntimeWebResearchConfig) -> Self {
        Self {
            mode: config.mode.into(),
            max_fetches: config
                .max_fetches
                .and_then(|value| u32::try_from(value).ok()),
        }
    }

    #[must_use]
    pub fn decision(self) -> WebAccessDecision {
        match self.mode {
            WebAccessMode::Off => WebAccessDecision::Denied,
            WebAccessMode::Ask => WebAccessDecision::RequiresApproval,
            WebAccessMode::On => WebAccessDecision::Allowed,
        }
    }

    #[must_use]
    pub fn inherit_for_child(self, child_request: Option<&WebPolicy>) -> Self {
        let requested_mode = child_request.map_or(self.mode, |child| child.mode);
        let requested_fetches = child_request.and_then(|child| child.max_fetches);
        Self {
            mode: self.mode.min(requested_mode),
            max_fetches: min_some_u32(self.max_fetches, requested_fetches),
        }
    }

    #[must_use]
    pub fn context(self) -> HybridExecutionContext {
        HybridExecutionContext { web_policy: self }
    }
}

impl EvidenceRecord {
    #[must_use]
    pub fn from_retrieval_hit(hit: &RetrievalHit) -> Self {
        Self {
            kind: EvidenceKind::Local,
            id: hit.chunk_id.clone(),
            title: hit.path.clone(),
            locator: hit.path.clone(),
            snippet: hit.preview.clone(),
            score: Some(hit.score),
            metadata: BTreeMap::from([
                (
                    "documentId".to_string(),
                    JsonValue::String(hit.document_id.clone()),
                ),
                ("reason".to_string(), JsonValue::String(hit.reason.clone())),
            ]),
        }
    }

    #[must_use]
    pub fn from_web_input(input: WebEvidenceInput) -> Self {
        let mut metadata = BTreeMap::new();
        if let Some(fetched_at_ms) = input.fetched_at_ms {
            metadata.insert(
                "fetchedAtMs".to_string(),
                JsonValue::Number(i64::try_from(fetched_at_ms).unwrap_or(i64::MAX)),
            );
        }
        Self {
            kind: EvidenceKind::Web,
            id: input.id,
            title: input.title,
            locator: input.url,
            snippet: input.snippet,
            score: None,
            metadata,
        }
    }

    #[must_use]
    pub fn citation_label(&self) -> String {
        match self.kind {
            EvidenceKind::Local => format!("[local] {}", self.locator),
            EvidenceKind::Web => format!("[web] {}", self.locator),
        }
    }

    #[must_use]
    pub fn to_json_value(&self) -> JsonValue {
        JsonValue::Object(BTreeMap::from([
            (
                "kind".to_string(),
                JsonValue::String(match self.kind {
                    EvidenceKind::Local => "local".to_string(),
                    EvidenceKind::Web => "web".to_string(),
                }),
            ),
            ("id".to_string(), JsonValue::String(self.id.clone())),
            ("title".to_string(), JsonValue::String(self.title.clone())),
            (
                "locator".to_string(),
                JsonValue::String(self.locator.clone()),
            ),
            (
                "snippet".to_string(),
                JsonValue::String(self.snippet.clone()),
            ),
            (
                "score".to_string(),
                self.score
                    .map(|value| JsonValue::Number((value * 1000.0).round() as i64))
                    .unwrap_or(JsonValue::Null),
            ),
            (
                "metadata".to_string(),
                JsonValue::Object(self.metadata.clone()),
            ),
        ]))
    }
}

#[must_use]
pub fn normalize_local_evidence(result: &RetrievalResult) -> Vec<EvidenceRecord> {
    result
        .hits
        .iter()
        .map(EvidenceRecord::from_retrieval_hit)
        .collect()
}

#[must_use]
pub fn format_citations(records: &[EvidenceRecord]) -> Vec<String> {
    records.iter().map(EvidenceRecord::citation_label).collect()
}

#[must_use]
pub fn summarize_local_evidence(result: &RetrievalResult) -> LocalEvidenceSummary {
    let mut distinct_documents = std::collections::BTreeSet::new();
    let mut best_score = None;
    for hit in &result.hits {
        distinct_documents.insert(hit.document_id.clone());
        best_score = Some(best_score.map_or(hit.score, |current: f64| current.max(hit.score)));
    }
    LocalEvidenceSummary {
        total_hits: result.hits.len(),
        distinct_documents: distinct_documents.len(),
        best_score,
    }
}

#[must_use]
pub fn evaluate_web_escalation(
    policy: WebPolicy,
    input: EscalationHeuristicInput,
) -> EscalationOutcome {
    let should_escalate = if input.user_requested_web {
        Some(EscalationReason::UserRequestedWeb)
    } else if input.local_summary.total_hits == 0 {
        Some(EscalationReason::NoLocalEvidence)
    } else if input.requires_external_freshness {
        Some(EscalationReason::FreshnessRequired)
    } else if is_local_evidence_weak(input.local_summary) {
        Some(EscalationReason::WeakLocalEvidence)
    } else {
        None
    };

    match should_escalate {
        None => EscalationOutcome {
            decision: WebAccessDecision::Denied,
            reason: EscalationReason::LocalEvidenceSufficient,
        },
        Some(_reason) if !policy.mode.allows_web() => EscalationOutcome {
            decision: WebAccessDecision::Denied,
            reason: EscalationReason::PolicyDenied,
        },
        Some(reason) => EscalationOutcome {
            decision: policy.decision(),
            reason,
        },
    }
}

#[must_use]
pub fn is_local_evidence_weak(summary: LocalEvidenceSummary) -> bool {
    summary.total_hits == 0
        || summary.best_score.is_some_and(|score| score < 0.55)
        || (summary.best_score.is_some_and(|score| score < 0.72)
            && summary.distinct_documents <= 1
            && summary.total_hits <= 2)
}

#[must_use]
pub fn local_evidence_trace_event(
    sequence: u32,
    timestamp_ms: u64,
    result: &RetrievalResult,
) -> TraceEvent {
    let evidence = normalize_local_evidence(result)
        .into_iter()
        .map(|record| record.to_json_value())
        .collect();
    TraceEvent::new(
        sequence,
        TraceEventType::RetrievalCompleted,
        timestamp_ms,
        BTreeMap::from([
            (
                "evidenceKind".to_string(),
                JsonValue::String("local".to_string()),
            ),
            (
                "backend".to_string(),
                JsonValue::String(
                    match result.backend {
                        CorpusBackend::Lexical => "lexical",
                        CorpusBackend::Hybrid => "hybrid",
                        CorpusBackend::Semantic => "semantic",
                    }
                    .to_string(),
                ),
            ),
            ("query".to_string(), JsonValue::String(result.query.clone())),
            (
                "elapsedMs".to_string(),
                JsonValue::Number(i64::try_from(result.elapsed_ms).unwrap_or(i64::MAX)),
            ),
            ("records".to_string(), JsonValue::Array(evidence)),
        ]),
    )
}

#[must_use]
pub fn web_evidence_trace_event(
    sequence: u32,
    timestamp_ms: u64,
    records: &[EvidenceRecord],
) -> TraceEvent {
    TraceEvent::new(
        sequence,
        TraceEventType::WebEvidenceAdded,
        timestamp_ms,
        BTreeMap::from([
            (
                "evidenceKind".to_string(),
                JsonValue::String("web".to_string()),
            ),
            (
                "records".to_string(),
                JsonValue::Array(records.iter().map(EvidenceRecord::to_json_value).collect()),
            ),
        ]),
    )
}

fn min_some_u32(parent: Option<u32>, requested: Option<u32>) -> Option<u32> {
    match (parent, requested) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        evaluate_web_escalation, format_citations, is_local_evidence_weak,
        local_evidence_trace_event, normalize_local_evidence, summarize_local_evidence,
        web_evidence_trace_event, EscalationHeuristicInput, EscalationReason, EvidenceKind,
        EvidenceRecord, LocalEvidenceSummary, WebAccessDecision, WebAccessMode, WebEvidenceInput,
        WebPolicy,
    };
    use crate::config::{RuntimeWebResearchConfig, RuntimeWebResearchMode};
    use crate::corpus::{CorpusBackend, RetrievalHit, RetrievalResult};
    use crate::json::JsonValue;
    use crate::trace::TraceEventType;

    fn retrieval_result_with_hits(hits: Vec<RetrievalHit>) -> RetrievalResult {
        RetrievalResult {
            query: "policy".to_string(),
            backend: CorpusBackend::Lexical,
            elapsed_ms: 11,
            hits,
        }
    }

    fn sample_hit(score: f64, document_id: &str, path: &str) -> RetrievalHit {
        RetrievalHit {
            chunk_id: format!("chunk-{document_id}"),
            document_id: document_id.to_string(),
            path: path.to_string(),
            score,
            reason: "keyword match".to_string(),
            preview: "matched text".to_string(),
        }
    }

    #[test]
    fn web_policy_obeys_off_ask_on_semantics_and_child_inheritance() {
        let parent = WebPolicy::from_config(&RuntimeWebResearchConfig {
            mode: RuntimeWebResearchMode::Ask,
            max_fetches: Some(4),
        });
        let child = parent.clone().inherit_for_child(Some(&WebPolicy {
            mode: WebAccessMode::On,
            max_fetches: Some(9),
        }));

        assert_eq!(
            WebPolicy::from_config(&RuntimeWebResearchConfig::default()).decision(),
            WebAccessDecision::Denied
        );
        assert_eq!(parent.decision(), WebAccessDecision::RequiresApproval);
        assert_eq!(
            WebPolicy {
                mode: WebAccessMode::On,
                max_fetches: Some(2)
            }
            .decision(),
            WebAccessDecision::Allowed
        );
        assert_eq!(child.mode, WebAccessMode::Ask);
        assert_eq!(child.max_fetches, Some(4));
    }

    #[test]
    fn normalizes_local_and_web_evidence_and_formats_citations() {
        let local = normalize_local_evidence(&retrieval_result_with_hits(vec![sample_hit(
            0.91,
            "doc-1",
            "docs/policy.md",
        )]));
        let web = EvidenceRecord::from_web_input(WebEvidenceInput {
            id: "web-1".to_string(),
            title: "Release notes".to_string(),
            url: "https://example.test/release".to_string(),
            snippet: "fresh info".to_string(),
            fetched_at_ms: Some(1234),
        });

        assert_eq!(local[0].kind, EvidenceKind::Local);
        assert_eq!(web.kind, EvidenceKind::Web);
        assert_eq!(
            format_citations(&[local[0].clone(), web.clone()]),
            vec![
                "[local] docs/policy.md".to_string(),
                "[web] https://example.test/release".to_string()
            ]
        );
        assert_eq!(
            web.metadata.get("fetchedAtMs"),
            Some(&JsonValue::Number(1234))
        );
    }

    #[test]
    fn separates_local_and_web_evidence_in_trace_events() {
        let local_event = local_evidence_trace_event(
            1,
            100,
            &retrieval_result_with_hits(vec![sample_hit(0.91, "doc-1", "docs/policy.md")]),
        );
        let web_event = web_evidence_trace_event(
            2,
            101,
            &[EvidenceRecord::from_web_input(WebEvidenceInput {
                id: "web-1".to_string(),
                title: "Release notes".to_string(),
                url: "https://example.test/release".to_string(),
                snippet: "fresh info".to_string(),
                fetched_at_ms: None,
            })],
        );

        assert_eq!(local_event.event_type, TraceEventType::RetrievalCompleted);
        assert_eq!(web_event.event_type, TraceEventType::WebEvidenceAdded);
        assert_eq!(
            local_event.data.get("evidenceKind"),
            Some(&JsonValue::String("local".to_string()))
        );
        assert_eq!(
            web_event.data.get("evidenceKind"),
            Some(&JsonValue::String("web".to_string()))
        );
    }

    #[test]
    fn escalation_heuristics_are_conservative_and_policy_aware() {
        let no_local = EscalationHeuristicInput {
            local_summary: LocalEvidenceSummary {
                total_hits: 0,
                distinct_documents: 0,
                best_score: None,
            },
            requires_external_freshness: false,
            user_requested_web: false,
        };
        let strong_local = EscalationHeuristicInput {
            local_summary: LocalEvidenceSummary {
                total_hits: 4,
                distinct_documents: 2,
                best_score: Some(0.89),
            },
            requires_external_freshness: false,
            user_requested_web: false,
        };
        let freshness_needed = EscalationHeuristicInput {
            local_summary: LocalEvidenceSummary {
                total_hits: 2,
                distinct_documents: 1,
                best_score: Some(0.61),
            },
            requires_external_freshness: true,
            user_requested_web: false,
        };

        let denied = evaluate_web_escalation(
            WebPolicy {
                mode: WebAccessMode::Off,
                max_fetches: None,
            },
            no_local,
        );
        let allowed = evaluate_web_escalation(
            WebPolicy {
                mode: WebAccessMode::On,
                max_fetches: Some(3),
            },
            no_local,
        );
        let ask = evaluate_web_escalation(
            WebPolicy {
                mode: WebAccessMode::Ask,
                max_fetches: Some(3),
            },
            freshness_needed,
        );
        let stay_local = evaluate_web_escalation(
            WebPolicy {
                mode: WebAccessMode::On,
                max_fetches: Some(3),
            },
            strong_local,
        );

        assert_eq!(denied.decision, WebAccessDecision::Denied);
        assert_eq!(denied.reason, EscalationReason::PolicyDenied);
        assert_eq!(allowed.decision, WebAccessDecision::Allowed);
        assert_eq!(allowed.reason, EscalationReason::NoLocalEvidence);
        assert_eq!(ask.decision, WebAccessDecision::RequiresApproval);
        assert_eq!(ask.reason, EscalationReason::FreshnessRequired);
        assert_eq!(stay_local.decision, WebAccessDecision::Denied);
        assert_eq!(stay_local.reason, EscalationReason::LocalEvidenceSufficient);
    }

    #[test]
    fn summarizes_and_scores_local_evidence_strength() {
        let result = retrieval_result_with_hits(vec![
            sample_hit(0.42, "doc-1", "docs/a.md"),
            sample_hit(0.67, "doc-1", "docs/a.md"),
            sample_hit(0.63, "doc-2", "docs/b.md"),
        ]);

        let summary = summarize_local_evidence(&result);
        assert_eq!(summary.total_hits, 3);
        assert_eq!(summary.distinct_documents, 2);
        assert_eq!(summary.best_score, Some(0.67));
        assert!(!is_local_evidence_weak(summary));
    }
}
