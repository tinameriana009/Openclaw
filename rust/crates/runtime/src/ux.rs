use std::fmt::{Display, Formatter};

use crate::config::{
    RuntimeRagConfig, RuntimeRlmConfig, RuntimeWebResearchConfig, RuntimeWebResearchMode,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionProfile {
    Fast,
    Balanced,
    Deep,
    Research,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionProfileConfig {
    pub profile: ExecutionProfile,
    pub rag: RuntimeRagConfig,
    pub rlm: RuntimeRlmConfig,
    pub web_research: RuntimeWebResearchConfig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceProvenance {
    Local,
    Web,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Citation {
    pub label: String,
    pub provenance: EvidenceProvenance,
    pub title: String,
    pub locator: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfidenceLevel {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfidenceNote {
    pub level: ConfidenceLevel,
    pub summary: String,
    pub gaps: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebExecutionDetail {
    pub subquery_id: String,
    pub status: String,
    pub approval: String,
    pub query: Option<String>,
    pub evidence_count: u32,
    pub degraded: bool,
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebExecutionSummary {
    pub total: usize,
    pub approved: usize,
    pub approval_required: usize,
    pub succeeded: usize,
    pub succeeded_with_fetched_evidence: usize,
    pub no_evidence: usize,
    pub failed: usize,
    pub skipped: usize,
    pub degraded: usize,
    pub details: Vec<WebExecutionDetail>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannerStep {
    pub iteration: u32,
    pub strategy: String,
    pub rationale: String,
    pub anchor_terms: Vec<String>,
    pub gap_terms: Vec<String>,
    pub validation_terms: Vec<String>,
    pub progress_status: String,
    pub progress_reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannerSummary {
    pub iterations: usize,
    pub latest_strategy: String,
    pub latest_rationale: String,
    pub latest_progress_status: String,
    pub latest_progress_reason: String,
    pub steps: Vec<PlannerStep>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinalAnswer {
    pub body: String,
    pub citations: Vec<Citation>,
    pub confidence: Option<ConfidenceNote>,
    pub planner: Option<PlannerSummary>,
    pub web: Option<WebExecutionSummary>,
    pub trace_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionProfileParseError {
    value: String,
}

impl Display for ExecutionProfileParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "unsupported execution profile {} (expected fast, balanced, deep, or research)",
            self.value
        )
    }
}

impl std::error::Error for ExecutionProfileParseError {}

impl ExecutionProfile {
    pub fn parse(value: &str) -> Result<Self, ExecutionProfileParseError> {
        match value.trim().to_ascii_lowercase().as_str() {
            "fast" => Ok(Self::Fast),
            "balanced" => Ok(Self::Balanced),
            "deep" => Ok(Self::Deep),
            "research" => Ok(Self::Research),
            _ => Err(ExecutionProfileParseError {
                value: value.to_string(),
            }),
        }
    }

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fast => "fast",
            Self::Balanced => "balanced",
            Self::Deep => "deep",
            Self::Research => "research",
        }
    }

    #[must_use]
    pub fn resolve(self) -> ExecutionProfileConfig {
        match self {
            Self::Fast => ExecutionProfileConfig {
                profile: self,
                rag: RuntimeRagConfig {
                    enabled: true,
                    backend: Some("lexical".to_string()),
                    default_corpora: Vec::new(),
                    chunk_bytes: Some(1_200),
                    max_hits: Some(4),
                },
                rlm: RuntimeRlmConfig {
                    enabled: false,
                    max_depth: Some(0),
                    max_iterations: Some(1),
                    max_subcalls: Some(0),
                    max_runtime_ms: Some(5_000),
                    subcall_model: None,
                    trace: false,
                },
                web_research: RuntimeWebResearchConfig {
                    mode: RuntimeWebResearchMode::Off,
                    max_fetches: Some(0),
                },
            },
            Self::Balanced => ExecutionProfileConfig {
                profile: self,
                rag: RuntimeRagConfig {
                    enabled: true,
                    backend: Some("lexical".to_string()),
                    default_corpora: Vec::new(),
                    chunk_bytes: Some(1_800),
                    max_hits: Some(8),
                },
                rlm: RuntimeRlmConfig {
                    enabled: true,
                    max_depth: Some(1),
                    max_iterations: Some(4),
                    max_subcalls: Some(3),
                    max_runtime_ms: Some(15_000),
                    subcall_model: None,
                    trace: true,
                },
                web_research: RuntimeWebResearchConfig {
                    mode: RuntimeWebResearchMode::Off,
                    max_fetches: Some(0),
                },
            },
            Self::Deep => ExecutionProfileConfig {
                profile: self,
                rag: RuntimeRagConfig {
                    enabled: true,
                    backend: Some("lexical".to_string()),
                    default_corpora: Vec::new(),
                    chunk_bytes: Some(2_400),
                    max_hits: Some(12),
                },
                rlm: RuntimeRlmConfig {
                    enabled: true,
                    max_depth: Some(3),
                    max_iterations: Some(8),
                    max_subcalls: Some(8),
                    max_runtime_ms: Some(45_000),
                    subcall_model: None,
                    trace: true,
                },
                web_research: RuntimeWebResearchConfig {
                    mode: RuntimeWebResearchMode::Ask,
                    max_fetches: Some(2),
                },
            },
            Self::Research => ExecutionProfileConfig {
                profile: self,
                rag: RuntimeRagConfig {
                    enabled: true,
                    backend: Some("lexical".to_string()),
                    default_corpora: Vec::new(),
                    chunk_bytes: Some(3_200),
                    max_hits: Some(16),
                },
                rlm: RuntimeRlmConfig {
                    enabled: true,
                    max_depth: Some(4),
                    max_iterations: Some(12),
                    max_subcalls: Some(12),
                    max_runtime_ms: Some(90_000),
                    subcall_model: None,
                    trace: true,
                },
                web_research: RuntimeWebResearchConfig {
                    mode: RuntimeWebResearchMode::Ask,
                    max_fetches: Some(6),
                },
            },
        }
    }
}

impl EvidenceProvenance {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Web => "web",
        }
    }
}

impl ConfidenceLevel {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
        }
    }
}

impl FinalAnswer {
    #[must_use]
    pub fn render_text(&self) -> String {
        let mut sections = vec![self.body.trim().to_string()];

        if !self.citations.is_empty() {
            let local_citations = self
                .citations
                .iter()
                .filter(|citation| citation.provenance == EvidenceProvenance::Local)
                .map(render_citation)
                .collect::<Vec<_>>();
            let web_citations = self
                .citations
                .iter()
                .filter(|citation| citation.provenance == EvidenceProvenance::Web)
                .map(render_citation)
                .collect::<Vec<_>>();
            let provenance_summary = match (local_citations.is_empty(), web_citations.is_empty()) {
                (false, true) => "local only",
                (true, false) => "web only",
                (false, false) => "local + web",
                (true, true) => "unknown",
            };
            let mut evidence_lines = vec![
                "Evidence".to_string(),
                format!("  Provenance       {provenance_summary}"),
            ];
            if !local_citations.is_empty() {
                evidence_lines.push("Local sources".to_string());
                evidence_lines.extend(local_citations);
            }
            if !web_citations.is_empty() {
                evidence_lines.push("Web sources".to_string());
                evidence_lines.extend(web_citations);
            }
            sections.push(evidence_lines.join("\n"));
        }

        if let Some(confidence) = &self.confidence {
            let mut confidence_lines = vec![
                "Confidence".to_string(),
                format!("  Level            {}", confidence.level.as_str()),
                format!("  Summary          {}", confidence.summary.trim()),
            ];
            if !confidence.gaps.is_empty() {
                confidence_lines.push("  Remaining gaps".to_string());
                confidence_lines.extend(confidence.gaps.iter().map(|gap| format!("  - {gap}")));
            }
            sections.push(confidence_lines.join("\n"));
        }

        if let Some(planner) = &self.planner {
            let mut planner_lines = vec![
                "Recursive planning".to_string(),
                format!("  Planned iterations            {}", planner.iterations),
                format!(
                    "  Latest strategy               {}",
                    planner.latest_strategy
                ),
                format!(
                    "  Latest rationale              {}",
                    planner.latest_rationale
                ),
                format!(
                    "  Planner progress              {}",
                    planner.latest_progress_status
                ),
                format!(
                    "  Progress note                 {}",
                    planner.latest_progress_reason
                ),
            ];
            if !planner.steps.is_empty() {
                planner_lines.push("  Iteration details".to_string());
                planner_lines.extend(planner.steps.iter().map(render_planner_step));
            }
            sections.push(planner_lines.join("\n"));
        }

        if let Some(web) = &self.web {
            let mut web_lines = vec![
                "Web execution".to_string(),
                format!("  Web-aware subqueries          {}", web.total),
                format!("  Approved                      {}", web.approved),
                format!("  Approval required             {}", web.approval_required),
                format!("  Succeeded                     {}", web.succeeded),
                format!(
                    "  With fetched evidence         {}",
                    web.succeeded_with_fetched_evidence
                ),
                format!("  No evidence attached          {}", web.no_evidence),
                format!("  Failed                        {}", web.failed),
                format!("  Skipped                       {}", web.skipped),
                format!("  Degraded                      {}", web.degraded),
            ];
            web_lines.extend(render_web_operator_handoff(web));
            if !web.details.is_empty() {
                web_lines.push("  Subquery details".to_string());
                web_lines.extend(web.details.iter().map(render_web_execution_detail));
            }
            sections.push(web_lines.join("\n"));
        }

        if let Some(trace_id) = &self.trace_id {
            sections.push(format!("Trace reference\n  Id               {trace_id}"));
        }

        sections
            .into_iter()
            .filter(|section| !section.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

fn render_planner_step(step: &PlannerStep) -> String {
    let mut parts = vec![format!(
        "  - iteration {}: strategy={}; rationale={}",
        step.iteration, step.strategy, step.rationale
    )];
    if !step.anchor_terms.is_empty() {
        parts.push(format!("anchors={}", step.anchor_terms.join(", ")));
    }
    if !step.gap_terms.is_empty() {
        parts.push(format!("gaps={}", step.gap_terms.join(", ")));
    }
    if !step.validation_terms.is_empty() {
        parts.push(format!("validation={}", step.validation_terms.join(", ")));
    }
    parts.push(format!("progress={}", step.progress_status));
    parts.push(format!("progress_note={}", step.progress_reason));
    parts.join("; ")
}

fn render_web_operator_handoff(web: &WebExecutionSummary) -> Vec<String> {
    let pending_queries = web
        .details
        .iter()
        .filter(|detail| detail.status == "approval_required")
        .filter_map(|detail| detail.query.as_deref())
        .collect::<Vec<_>>();
    let failed_subqueries = web
        .details
        .iter()
        .filter(|detail| detail.status == "failed")
        .map(|detail| detail.subquery_id.as_str())
        .collect::<Vec<_>>();
    let no_evidence_subqueries = web
        .details
        .iter()
        .filter(|detail| detail.status == "no_evidence")
        .map(|detail| detail.subquery_id.as_str())
        .collect::<Vec<_>>();

    let overall = if web.approval_required > 0 {
        "awaiting approval"
    } else if web.failed > 0 {
        "web attempt failed"
    } else if web.no_evidence > 0 {
        "web attempted but unverified"
    } else if web.succeeded_with_fetched_evidence > 0 {
        "bounded web evidence attached"
    } else if web.total > 0 {
        "local-only despite web-aware execution"
    } else {
        "not requested"
    };

    let mut lines = vec![format!("  Operator state                {overall}")];
    if !pending_queries.is_empty() {
        lines.push(format!(
            "  Operator next step            approve web queries: {}",
            pending_queries.join(" | ")
        ));
    } else if !failed_subqueries.is_empty() {
        lines.push(format!(
            "  Operator next step            inspect failed web subqueries: {}",
            failed_subqueries.join(", ")
        ));
    } else if !no_evidence_subqueries.is_empty() {
        lines.push(format!(
            "  Operator next step            review degraded no-evidence subqueries: {}",
            no_evidence_subqueries.join(", ")
        ));
    } else if web.succeeded_with_fetched_evidence > 0 {
        lines.push(
            "  Operator next step            use attached web evidence carefully; browser automation is still not available"
                .to_string(),
        );
    }
    lines
}

fn render_web_execution_detail(detail: &WebExecutionDetail) -> String {
    let mut line = format!(
        "  - {}: status={}; approval={}; evidence={}",
        detail.subquery_id, detail.status, detail.approval, detail.evidence_count
    );
    if detail.degraded {
        line.push_str("; degraded=yes");
    }
    if let Some(query) = detail.query.as_deref() {
        line.push_str(&format!("; query=\"{query}\""));
    }
    if let Some(note) = detail.note.as_deref() {
        line.push_str(&format!("; note={note}"));
    }
    line
}

fn render_citation(citation: &Citation) -> String {
    let locator_suffix = citation
        .locator
        .as_deref()
        .map(|locator| format!(" — {locator}"))
        .unwrap_or_default();
    format!(
        "- [{}] {}{}",
        citation.label, citation.title, locator_suffix
    )
}

#[cfg(test)]
mod tests {
    use super::{
        Citation, ConfidenceLevel, ConfidenceNote, EvidenceProvenance, ExecutionProfile,
        FinalAnswer, PlannerStep, PlannerSummary, WebExecutionDetail, WebExecutionSummary,
    };

    #[test]
    fn profiles_resolve_to_expected_defaults() {
        let fast = ExecutionProfile::Fast.resolve();
        assert!(fast.rag.enabled);
        assert!(!fast.rlm.enabled);
        assert_eq!(fast.rag.max_hits, Some(4));
        assert_eq!(fast.web_research.mode, crate::RuntimeWebResearchMode::Off);

        let research = ExecutionProfile::Research.resolve();
        assert!(research.rag.enabled);
        assert!(research.rlm.enabled);
        assert_eq!(research.rlm.max_depth, Some(4));
        assert_eq!(
            research.web_research.mode,
            crate::RuntimeWebResearchMode::Ask
        );
        assert_eq!(research.web_research.max_fetches, Some(6));
    }

    #[test]
    fn parses_profile_names_case_insensitively() {
        assert_eq!(
            ExecutionProfile::parse("DeEp").expect("profile should parse"),
            ExecutionProfile::Deep
        );
    }

    #[test]
    fn final_answer_rendering_includes_grounding_sections() {
        let answer = FinalAnswer {
            body: "The fix is in the runtime parser.".to_string(),
            citations: vec![
                Citation {
                    label: "L1".to_string(),
                    provenance: EvidenceProvenance::Local,
                    title: "runtime/src/config.rs".to_string(),
                    locator: Some("lines 863-920".to_string()),
                },
                Citation {
                    label: "W1".to_string(),
                    provenance: EvidenceProvenance::Web,
                    title: "Example spec".to_string(),
                    locator: Some("https://example.test/spec".to_string()),
                },
            ],
            confidence: Some(ConfidenceNote {
                level: ConfidenceLevel::Medium,
                summary: "The implementation matches the current fixture coverage.".to_string(),
                gaps: vec!["A real recursive controller is not wired yet.".to_string()],
            }),
            planner: Some(PlannerSummary {
                iterations: 2,
                latest_strategy: "gap_targeted_followup".to_string(),
                latest_rationale: "carry forward explicit remaining-gap terms".to_string(),
                latest_progress_status: "probing_open_gaps".to_string(),
                latest_progress_reason: "recent child outputs still expose concrete unresolved gaps".to_string(),
                steps: vec![PlannerStep {
                    iteration: 1,
                    strategy: "bootstrap".to_string(),
                    rationale: "start from the original task".to_string(),
                    anchor_terms: vec!["parser".to_string(), "runtime".to_string()],
                    gap_terms: vec!["hook parity".to_string()],
                    validation_terms: vec!["run targeted tests".to_string()],
                    progress_status: "expanding_evidence".to_string(),
                    progress_reason: "only one child output exists, so the planner is broadening local evidence coverage".to_string(),
                }],
            }),
            web: Some(WebExecutionSummary {
                total: 2,
                approved: 1,
                approval_required: 1,
                succeeded: 1,
                succeeded_with_fetched_evidence: 1,
                no_evidence: 0,
                failed: 0,
                skipped: 0,
                degraded: 1,
                details: vec![
                    WebExecutionDetail {
                        subquery_id: "subq-1".to_string(),
                        status: "succeeded".to_string(),
                        approval: "approved".to_string(),
                        query: Some("latest release".to_string()),
                        evidence_count: 1,
                        degraded: false,
                        note: Some("fetched from minimal adapter".to_string()),
                    },
                    WebExecutionDetail {
                        subquery_id: "subq-2".to_string(),
                        status: "approval_required".to_string(),
                        approval: "not approved".to_string(),
                        query: Some("current version".to_string()),
                        evidence_count: 0,
                        degraded: true,
                        note: Some("explicit approval required before web fetch".to_string()),
                    },
                ],
            }),
            trace_id: Some("trace-123".to_string()),
        };

        let rendered = answer.render_text();
        assert!(rendered.contains("Evidence"));
        assert!(rendered.contains("Provenance       local + web"));
        assert!(rendered.contains("Local sources"));
        assert!(rendered.contains("[L1] runtime/src/config.rs"));
        assert!(rendered.contains("Web sources"));
        assert!(rendered.contains("[W1] Example spec"));
        assert!(rendered.contains("Confidence\n  Level            medium"));
        assert!(rendered.contains("Recursive planning"));
        assert!(rendered.contains("Latest strategy               gap_targeted_followup"));
        assert!(rendered.contains("Planner progress              probing_open_gaps"));
        assert!(rendered.contains("Progress note                 recent child outputs still expose concrete unresolved gaps"));
        assert!(rendered.contains("anchors=parser, runtime"));
        assert!(rendered.contains("progress=expanding_evidence"));
        assert!(rendered.contains("Web execution"));
        assert!(rendered.contains("Web-aware subqueries          2"));
        assert!(rendered.contains("Approval required             1"));
        assert!(rendered.contains("Operator state                awaiting approval"));
        assert!(
            rendered.contains("Operator next step            approve web queries: current version")
        );
        assert!(rendered.contains("subq-2: status=approval_required; approval=not approved; evidence=0; degraded=yes; query=\"current version\""));
        assert!(rendered.contains("Trace reference\n  Id               trace-123"));
    }
}
