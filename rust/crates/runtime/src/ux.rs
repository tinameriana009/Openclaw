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
pub struct FinalAnswer {
    pub body: String,
    pub citations: Vec<Citation>,
    pub confidence: Option<ConfidenceNote>,
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
            let citations = self
                .citations
                .iter()
                .map(|citation| {
                    let locator_suffix = citation
                        .locator
                        .as_deref()
                        .map(|locator| format!(" — {locator}"))
                        .unwrap_or_default();
                    format!(
                        "- [{}] {} · {}{}",
                        citation.label,
                        citation.provenance.as_str(),
                        citation.title,
                        locator_suffix
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            sections.push(format!("Sources\n{citations}"));
        }

        if let Some(confidence) = &self.confidence {
            let mut confidence_lines = vec![format!(
                "Confidence: {} — {}",
                confidence.level.as_str(),
                confidence.summary.trim()
            )];
            if !confidence.gaps.is_empty() {
                confidence_lines.push("Open questions:".to_string());
                confidence_lines.extend(confidence.gaps.iter().map(|gap| format!("- {gap}")));
            }
            sections.push(confidence_lines.join("\n"));
        }

        if let Some(trace_id) = &self.trace_id {
            sections.push(format!("Trace id: {trace_id}"));
        }

        sections
            .into_iter()
            .filter(|section| !section.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Citation, ConfidenceLevel, ConfidenceNote, EvidenceProvenance, ExecutionProfile,
        FinalAnswer,
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
            trace_id: Some("trace-123".to_string()),
        };

        let rendered = answer.render_text();
        assert!(rendered.contains("Sources"));
        assert!(rendered.contains("[L1] local · runtime/src/config.rs"));
        assert!(rendered.contains("Confidence: medium"));
        assert!(rendered.contains("Trace id: trace-123"));
    }
}
