use std::sync::Arc;

use runtime::{
    ChildSubqueryExecutor, ChildSubqueryOutput, ChildSubqueryRequest, EvidenceRecord,
    RecursiveRuntimeError, WebAccessMode,
};
use tokio::runtime::Runtime;

use crate::{
    max_tokens_for_model, ApiError, InputMessage, MessageRequest, MessageResponse,
    OutputContentBlock, ProviderClient,
};

pub type WebEvidenceCollector = Arc<
    dyn Fn(&ChildSubqueryRequest) -> Result<Vec<EvidenceRecord>, RecursiveRuntimeError>
        + Send
        + Sync,
>;

pub struct ProviderChildExecutor {
    runtime: Runtime,
    client: ProviderClient,
    model: String,
    web_evidence_collector: WebEvidenceCollector,
}

impl ProviderChildExecutor {
    pub fn new(
        client: ProviderClient,
        model: impl Into<String>,
        web_evidence_collector: WebEvidenceCollector,
    ) -> Result<Self, String> {
        let model = model.into();
        let runtime = Runtime::new().map_err(|error| {
            format!("provider runtime initialization failed for model={model}: {error}")
        })?;
        Ok(Self {
            runtime,
            client,
            model,
            web_evidence_collector,
        })
    }

    #[must_use]
    pub fn model(&self) -> &str {
        &self.model
    }
}

impl ChildSubqueryExecutor for ProviderChildExecutor {
    fn execute(
        &self,
        request: &ChildSubqueryRequest,
    ) -> Result<ChildSubqueryOutput, RecursiveRuntimeError> {
        let response = self
            .runtime
            .block_on(async {
                self.client
                    .send_message(&MessageRequest {
                        model: self.model.clone(),
                        max_tokens: max_tokens_for_model(&self.model),
                        messages: vec![InputMessage::user_text(request.prompt.clone())],
                        system: Some(build_corpus_subquery_system_prompt(request)),
                        tools: None,
                        tool_choice: None,
                        stream: false,
                    })
                    .await
            })
            .map_err(map_provider_api_error_to_recursive_error)?;

        build_provider_child_output(
            &self.model,
            request,
            response,
            &*self.web_evidence_collector,
        )
    }
}

#[must_use]
pub fn format_provider_execution_fallback_reason(error: &RecursiveRuntimeError) -> String {
    match error {
        RecursiveRuntimeError::ChildExecution(message) => {
            format!("provider execution failed: {message}")
        }
        other => format!("provider execution failed: {other}"),
    }
}

fn map_provider_api_error_to_recursive_error(error: ApiError) -> RecursiveRuntimeError {
    RecursiveRuntimeError::ChildExecution(match error {
        ApiError::MissingCredentials { provider, env_vars } => format!(
            "missing {provider} credentials during child execution; set {}",
            env_vars.join(" or ")
        ),
        ApiError::ExpiredOAuthToken => {
            "saved OAuth token expired during child execution; re-authenticate and retry"
                .to_string()
        }
        ApiError::Auth(message) => {
            format!("provider auth failed during child execution: {message}")
        }
        other => other.to_string(),
    })
}

fn build_corpus_subquery_system_prompt(request: &ChildSubqueryRequest) -> String {
    let mut prompt = "You are a grounded corpus subquery worker. Answer the task using only the provided corpus slices. If the slices are insufficient, say what is missing briefly. Do not invent sources. Keep the answer concise and directly useful.".to_string();
    match request.web_policy.mode {
        WebAccessMode::Off => {
            prompt.push_str(" Web research is disabled for this subquery, so keep the answer strictly local to the provided slices and do not imply any external verification.");
        }
        WebAccessMode::Ask => {
            prompt.push_str(" External web research would require explicit approval for this subquery. Stay grounded in the provided slices; if fresh or external evidence is needed, say that approval is required before using the web.");
        }
        WebAccessMode::On => {
            if request.web_research_query.is_some() {
                prompt.push_str(" Limited web evidence may be attached separately for this subquery when the runtime has already decided escalation is warranted. Keep your answer explicit about what comes from the provided slices versus any externally fetched confirmation.");
            } else {
                prompt.push_str(" Web access is enabled in principle, but this subquery was not flagged for external fetches. Stay grounded in the provided slices and do not imply web verification.");
            }
        }
    }
    prompt
}

fn build_provider_child_output(
    model: &str,
    request: &ChildSubqueryRequest,
    response: MessageResponse,
    web_evidence_collector: &dyn Fn(
        &ChildSubqueryRequest,
    ) -> Result<Vec<EvidenceRecord>, RecursiveRuntimeError>,
) -> Result<ChildSubqueryOutput, RecursiveRuntimeError> {
    let answer = extract_provider_answer_text(&response);
    if answer.trim().is_empty() {
        return Err(RecursiveRuntimeError::ChildExecution(format!(
            "provider returned an empty child answer (request_id={})",
            response.request_id.as_deref().unwrap_or("unknown")
        )));
    }

    Ok(ChildSubqueryOutput {
        subquery_id: request.subquery_id.clone(),
        answer,
        citations: request
            .slices
            .iter()
            .map(|slice| format!("{} ({})", slice.path, slice.chunk_id))
            .collect(),
        web_evidence: web_evidence_collector(request)?,
        prompt_tokens: response.usage.input_tokens,
        completion_tokens: response.usage.output_tokens,
        cost_usd: response.usage.estimated_cost_usd(model).total_cost_usd(),
    })
}

fn extract_provider_answer_text(response: &MessageResponse) -> String {
    response
        .content
        .iter()
        .filter_map(|block| match block {
            OutputContentBlock::Text { text } => Some(text.trim()),
            _ => None,
        })
        .filter(|text: &&str| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;

    use runtime::{
        ChildSubqueryRequest, RecursiveContextSlice, RecursiveRuntimeError, RuntimeBudget,
        WebAccessMode, WebPolicy,
    };

    use super::*;
    use crate::{AuthSource, MessageResponse, Usage};

    fn sample_request(mode: WebAccessMode, web_research_query: Option<&str>) -> ChildSubqueryRequest {
        ChildSubqueryRequest {
            subquery_id: "subq-1".to_string(),
            prompt: "Summarize the relevant slice".to_string(),
            slices: vec![RecursiveContextSlice {
                chunk_id: "chunk-1".to_string(),
                document_id: "doc-1".to_string(),
                path: "docs/guide.md".to_string(),
                ordinal: 0,
                start_offset: 0,
                end_offset: 12,
                preview: "preview".to_string(),
                metadata: BTreeMap::new(),
            }],
            budget: RuntimeBudget::default(),
            web_policy: WebPolicy {
                mode,
                max_fetches: Some(1),
            },
            web_research_query: web_research_query.map(ToOwned::to_owned),
        }
    }

    fn response_with_text(text: &str) -> MessageResponse {
        MessageResponse {
            id: "msg_123".to_string(),
            kind: "message".to_string(),
            role: "assistant".to_string(),
            content: vec![OutputContentBlock::Text {
                text: text.to_string(),
            }],
            model: "claude-sonnet-4-6".to_string(),
            stop_reason: Some("end_turn".to_string()),
            stop_sequence: None,
            usage: Usage {
                input_tokens: 11,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
                output_tokens: 7,
            },
            request_id: Some("req_123".to_string()),
        }
    }

    #[test]
    fn system_prompt_reflects_web_policy() {
        let off = build_corpus_subquery_system_prompt(&sample_request(WebAccessMode::Off, None));
        assert!(off.contains("Web research is disabled"));

        let ask = build_corpus_subquery_system_prompt(&sample_request(WebAccessMode::Ask, None));
        assert!(ask.contains("would require explicit approval"));

        let on = build_corpus_subquery_system_prompt(&sample_request(
            WebAccessMode::On,
            Some("latest release"),
        ));
        assert!(on.contains("may be attached separately"));
    }

    #[test]
    fn provider_child_output_maps_answer_usage_and_citations() {
        let request = sample_request(WebAccessMode::Off, None);
        let output = build_provider_child_output(
            "claude-sonnet-4-6",
            &request,
            response_with_text("Grounded answer"),
            &|_| Ok(Vec::new()),
        )
        .expect("provider output should build");

        assert_eq!(output.subquery_id, "subq-1");
        assert_eq!(output.answer, "Grounded answer");
        assert_eq!(output.citations, vec!["docs/guide.md (chunk-1)"]);
        assert_eq!(output.prompt_tokens, 11);
        assert_eq!(output.completion_tokens, 7);
        assert!(output.cost_usd >= 0.0);
    }

    #[test]
    fn provider_child_output_rejects_empty_answers() {
        let request = sample_request(WebAccessMode::Off, None);
        let error = build_provider_child_output(
            "claude-sonnet-4-6",
            &request,
            response_with_text("   \n  "),
            &|_| Ok(Vec::new()),
        )
        .expect_err("empty provider answers should fail");

        assert!(matches!(error, RecursiveRuntimeError::ChildExecution(message) if message.contains("empty child answer") && message.contains("req_123")));
    }

    #[test]
    fn provider_child_output_propagates_web_evidence_errors() {
        let request = sample_request(WebAccessMode::On, Some("freshness"));
        let error = build_provider_child_output(
            "claude-sonnet-4-6",
            &request,
            response_with_text("Grounded answer"),
            &|_| Err(RecursiveRuntimeError::ChildExecution("collector failed".to_string())),
        )
        .expect_err("collector errors should surface");

        assert!(matches!(error, RecursiveRuntimeError::ChildExecution(message) if message == "collector failed"));
    }

    #[test]
    fn fallback_reason_wraps_child_errors() {
        let reason = format_provider_execution_fallback_reason(&RecursiveRuntimeError::ChildExecution(
            "network down".to_string(),
        ));
        assert_eq!(reason, "provider execution failed: network down");
    }

    #[test]
    fn executor_can_be_constructed_with_shared_web_evidence_callback() {
        let callback: WebEvidenceCollector = Arc::new(|_| Ok(Vec::new()));
        let client = ProviderClient::Anthropic(crate::AnthropicClient::from_auth(
            AuthSource::ApiKey("test".to_string()),
        ));
        let executor = ProviderChildExecutor::new(client, "claude-sonnet-4-6", callback)
            .expect("executor should build");
        assert_eq!(executor.model(), "claude-sonnet-4-6");
    }
}
