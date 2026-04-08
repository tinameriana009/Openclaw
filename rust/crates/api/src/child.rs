use std::path::Path;
use std::sync::Arc;

use runtime::{
    ChildSubqueryExecutor, ChildSubqueryOutput, ChildSubqueryRequest, ConfigLoader, EvidenceRecord,
    RecursiveRuntimeError, WebAccessMode, WebEvidenceInput, WebExecutionOutcome,
};
use tokio::runtime::Runtime;

use crate::{
    max_tokens_for_model, minimal_web_research, resolve_startup_auth_source, ApiError, AuthSource,
    InputMessage, MessageRequest, MessageResponse, OutputContentBlock, PromptCache, ProviderClient,
};

pub type WebEvidenceCollector = Arc<
    dyn Fn(&ChildSubqueryRequest) -> Result<Vec<EvidenceRecord>, RecursiveRuntimeError>
        + Send
        + Sync,
>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MinimalWebEvidence {
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub fetched: bool,
}

pub type MinimalWebEvidenceFetcher =
    Arc<dyn Fn(&str, usize) -> Result<Vec<MinimalWebEvidence>, String> + Send + Sync>;

pub type ProviderChildAuthResolver =
    Arc<dyn Fn() -> Result<Option<AuthSource>, String> + Send + Sync>;
pub type ProviderFallbackRenderer =
    Arc<dyn Fn(&ChildSubqueryRequest, &str) -> ChildSubqueryOutput + Send + Sync>;
pub type ProviderBackedChildExecutor = runtime::FallbackChildSubqueryExecutor<ProviderChildBackend>;

pub struct ProviderChildExecutor {
    runtime: Runtime,
    client: ProviderClient,
    model: String,
    web_evidence_collector: WebEvidenceCollector,
}

#[derive(Debug, Clone, PartialEq)]
struct CollectedWebContext {
    evidence: Vec<EvidenceRecord>,
    outcome: WebExecutionOutcome,
    note: Option<String>,
}

impl Default for CollectedWebContext {
    fn default() -> Self {
        Self {
            evidence: Vec::new(),
            outcome: WebExecutionOutcome::not_requested(),
            note: None,
        }
    }
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

pub fn build_provider_child_executor(
    session_id: &str,
    model: &str,
    anthropic_auth: Option<AuthSource>,
    web_evidence_collector: WebEvidenceCollector,
) -> Result<ProviderChildExecutor, String> {
    let client = ProviderClient::from_model_with_anthropic_auth(model, anthropic_auth)
        .map_err(|error| format_provider_child_init_reason(model, &error))?
        .with_prompt_cache(PromptCache::new(&format!("{session_id}-corpus-answer")));
    ProviderChildExecutor::new(client, model, web_evidence_collector)
}

pub enum ProviderChildBackend {
    Provider(ProviderChildExecutor),
    Unavailable { model: String, reason: String },
}

impl ProviderChildBackend {
    #[must_use]
    pub fn build(
        session_id: &str,
        model: &str,
        anthropic_auth: Option<AuthSource>,
        web_evidence_collector: WebEvidenceCollector,
    ) -> Self {
        match build_provider_child_executor(
            session_id,
            model,
            anthropic_auth,
            web_evidence_collector,
        ) {
            Ok(executor) => Self::Provider(executor),
            Err(reason) => Self::Unavailable {
                model: model.to_string(),
                reason,
            },
        }
    }

    #[must_use]
    pub fn build_with_resolver(
        session_id: &str,
        model: &str,
        auth_resolver: ProviderChildAuthResolver,
        web_evidence_collector: WebEvidenceCollector,
    ) -> Self {
        let anthropic_auth = match auth_resolver() {
            Ok(auth) => auth,
            Err(reason) => {
                return Self::Unavailable {
                    model: model.to_string(),
                    reason,
                };
            }
        };
        Self::build(session_id, model, anthropic_auth, web_evidence_collector)
    }

    #[must_use]
    pub fn model(&self) -> &str {
        match self {
            Self::Provider(executor) => executor.model(),
            Self::Unavailable { model, .. } => model,
        }
    }

    #[must_use]
    pub fn unavailable_reason(&self) -> Option<&str> {
        match self {
            Self::Provider(_) => None,
            Self::Unavailable { reason, .. } => Some(reason.as_str()),
        }
    }
}

impl ChildSubqueryExecutor for ProviderChildBackend {
    fn execute(
        &self,
        request: &ChildSubqueryRequest,
    ) -> Result<ChildSubqueryOutput, RecursiveRuntimeError> {
        match self {
            Self::Provider(executor) => executor.execute(request),
            Self::Unavailable { reason, model } => {
                let message = if reason.contains("provider child executor unavailable") {
                    reason.clone()
                } else {
                    format!("provider child executor unavailable for model={model}: {reason}")
                };
                Err(RecursiveRuntimeError::ChildExecution(message))
            }
        }
    }
}

#[must_use]
pub fn build_provider_backed_child_executor(
    session_id: &str,
    model: &str,
    auth_resolver: ProviderChildAuthResolver,
    web_evidence_collector: WebEvidenceCollector,
    fallback_renderer: ProviderFallbackRenderer,
) -> ProviderBackedChildExecutor {
    let backend = ProviderChildBackend::build_with_resolver(
        session_id,
        model,
        auth_resolver,
        Arc::clone(&web_evidence_collector),
    );
    let model = backend.model().to_string();
    let unavailable_reason = backend.unavailable_reason().map(ToOwned::to_owned);
    runtime::FallbackChildSubqueryExecutor::new(
        backend,
        model.clone(),
        Arc::new(move || unavailable_reason.clone()),
        Arc::new(format_provider_execution_fallback_reason),
        Arc::new(move |request, reason| fallback_renderer(request, reason)),
    )
}

#[must_use]
pub fn build_provider_extractive_child_executor(
    session_id: &str,
    model: &str,
    auth_resolver: ProviderChildAuthResolver,
    web_evidence_collector: WebEvidenceCollector,
) -> ProviderBackedChildExecutor {
    let render_model = model.to_string();
    let render_web_evidence_collector = Arc::clone(&web_evidence_collector);
    build_provider_backed_child_executor(
        session_id,
        model,
        auth_resolver,
        web_evidence_collector,
        Arc::new(move |request, reason| {
            render_extractive_child_answer(
                request,
                Some(reason),
                &render_model,
                &*render_web_evidence_collector,
            )
        }),
    )
}

#[must_use]
pub fn resolve_provider_child_model(
    cwd: &Path,
    active_model: Option<&str>,
    default_model: &str,
) -> String {
    let configured = ConfigLoader::default_for(cwd)
        .load()
        .ok()
        .and_then(|config| {
            config
                .rlm()
                .subcall_model
                .clone()
                .or_else(|| config.model().map(ToOwned::to_owned))
        });
    crate::resolve_model_alias(
        configured
            .as_deref()
            .or(active_model)
            .unwrap_or(default_model),
    )
    .to_string()
}

pub fn build_configured_provider_extractive_child_executor(
    cwd: &Path,
    session_id: &str,
    active_model: Option<&str>,
    default_model: &str,
    auth_resolver: ProviderChildAuthResolver,
    web_fetcher: MinimalWebEvidenceFetcher,
) -> ProviderBackedChildExecutor {
    let model = resolve_provider_child_model(cwd, active_model, default_model);
    let web_evidence_collector: WebEvidenceCollector =
        Arc::new(move |request| collect_minimal_web_evidence(request, &*web_fetcher));
    build_provider_extractive_child_executor(
        session_id,
        &model,
        auth_resolver,
        web_evidence_collector,
    )
}

pub fn resolve_runtime_provider_child_auth(cwd: &Path) -> Result<AuthSource, ApiError> {
    resolve_startup_auth_source(|| {
        let config = ConfigLoader::default_for(cwd).load().map_err(|error| {
            ApiError::Auth(format!("failed to load runtime OAuth config: {error}"))
        })?;
        Ok(config.oauth().cloned())
    })
}

#[must_use]
pub fn runtime_provider_child_auth_resolver(cwd: &Path) -> ProviderChildAuthResolver {
    let cwd = cwd.to_path_buf();
    Arc::new(move || {
        resolve_runtime_provider_child_auth(&cwd)
            .map(Some)
            .map_err(|error| error.to_string())
    })
}

#[must_use]
pub fn runtime_minimal_web_evidence_fetcher() -> MinimalWebEvidenceFetcher {
    Arc::new(minimal_web_research)
}

#[must_use]
pub fn build_runtime_configured_provider_extractive_child_executor(
    cwd: &Path,
    session_id: &str,
    active_model: Option<&str>,
    default_model: &str,
) -> ProviderBackedChildExecutor {
    build_configured_provider_extractive_child_executor(
        cwd,
        session_id,
        active_model,
        default_model,
        runtime_provider_child_auth_resolver(cwd),
        runtime_minimal_web_evidence_fetcher(),
    )
}

pub fn collect_minimal_web_evidence(
    request: &ChildSubqueryRequest,
    web_fetcher: &dyn Fn(&str, usize) -> Result<Vec<MinimalWebEvidence>, String>,
) -> Result<Vec<EvidenceRecord>, RecursiveRuntimeError> {
    if !matches!(request.web_policy.mode, WebAccessMode::On) {
        return Ok(Vec::new());
    }
    let Some(query) = request.web_research_query.as_deref() else {
        return Ok(Vec::new());
    };
    let fetch_limit = usize::try_from(request.web_policy.max_fetches.unwrap_or(1)).unwrap_or(1);
    let evidence = web_fetcher(query, fetch_limit).map_err(|error| {
        RecursiveRuntimeError::ChildExecution(format!(
            "minimal web research failed for subquery {}: {error}",
            request.subquery_id
        ))
    })?;
    Ok(evidence
        .into_iter()
        .enumerate()
        .map(|(index, item)| {
            let title = if item.fetched {
                item.title
            } else {
                format!("{} (search result only)", item.title)
            };
            EvidenceRecord::from_web_input(WebEvidenceInput {
                id: format!("{}-web-{}", request.subquery_id, index + 1),
                title,
                url: item.url,
                snippet: item.snippet,
                fetched_at_ms: None,
                fetched: Some(item.fetched),
            })
        })
        .collect())
}

#[must_use]
pub fn format_provider_child_init_reason(model: &str, error: &ApiError) -> String {
    match error {
        ApiError::MissingCredentials { provider, env_vars } => format!(
            "provider child executor unavailable for model={model}: missing {provider} credentials (set {})",
            env_vars.join(" or ")
        ),
        ApiError::ExpiredOAuthToken => format!(
            "provider child executor unavailable for model={model}: saved OAuth token is expired; re-authenticate before retrying"
        ),
        ApiError::Auth(message) => {
            format!("provider child executor unavailable for model={model}: auth error: {message}")
        }
        other => format!("provider child executor unavailable for model={model}: {other}"),
    }
}

impl ChildSubqueryExecutor for ProviderChildExecutor {
    fn execute(
        &self,
        request: &ChildSubqueryRequest,
    ) -> Result<ChildSubqueryOutput, RecursiveRuntimeError> {
        let web_context = collect_web_context(request, &*self.web_evidence_collector);
        let response = self
            .runtime
            .block_on(async {
                self.client
                    .send_message(&MessageRequest {
                        model: self.model.clone(),
                        max_tokens: max_tokens_for_model(&self.model),
                        messages: vec![InputMessage::user_text(build_provider_child_user_prompt(
                            request,
                            &web_context,
                        ))],
                        system: Some(build_corpus_subquery_system_prompt(request, &web_context)),
                        tools: None,
                        tool_choice: None,
                        stream: false,
                    })
                    .await
            })
            .map_err(map_provider_api_error_to_recursive_error)?;

        build_provider_child_output(&self.model, request, response, web_context)
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

fn collect_web_context(
    request: &ChildSubqueryRequest,
    web_evidence_collector: &dyn Fn(
        &ChildSubqueryRequest,
    ) -> Result<Vec<EvidenceRecord>, RecursiveRuntimeError>,
) -> CollectedWebContext {
    match request.web_policy.mode {
        WebAccessMode::Off => CollectedWebContext {
            evidence: Vec::new(),
            outcome: WebExecutionOutcome::skipped(
                "web research disabled for this subquery; execution stayed local-only",
            ),
            note: None,
        },
        WebAccessMode::Ask => {
            let Some(query) = request.web_research_query.as_deref() else {
                return CollectedWebContext::default();
            };
            let note =
                "web research requires explicit approval for this subquery before any external fetch";
            CollectedWebContext {
                evidence: Vec::new(),
                outcome: WebExecutionOutcome::approval_required(query, note),
                note: Some(note.to_string()),
            }
        }
        WebAccessMode::On => {
            let Some(query) = request.web_research_query.as_deref() else {
                return CollectedWebContext {
                    evidence: Vec::new(),
                    outcome: WebExecutionOutcome::skipped(
                        "web access enabled in policy, but this subquery was not flagged for external fetches",
                    ),
                    note: None,
                };
            };

            match web_evidence_collector(request) {
                Ok(evidence) if evidence.is_empty() => {
                    let note =
                        "approved web collection completed without attaching external evidence";
                    CollectedWebContext {
                        evidence,
                        outcome: WebExecutionOutcome::no_evidence(query, note),
                        note: Some(note.to_string()),
                    }
                }
                Ok(evidence) => {
                    let fetched_count = evidence
                        .iter()
                        .filter(|record| record.is_fetched_web_evidence())
                        .count();
                    let degraded = fetched_count < evidence.len();
                    let note = if degraded {
                        if fetched_count == 0 {
                            "attached bounded external evidence, but only search-result snippets were available; no page fetch completed"
                                .to_string()
                        } else {
                            format!(
                                "attached bounded external evidence; {fetched_count}/{} page fetches completed and the rest degraded to search-result snippets",
                                evidence.len()
                            )
                        }
                    } else {
                        "attached bounded external evidence".to_string()
                    };
                    CollectedWebContext {
                        outcome: WebExecutionOutcome {
                            status: runtime::WebExecutionStatus::Succeeded,
                            approved: true,
                            query: Some(query.to_string()),
                            evidence_count: u32::try_from(evidence.len()).unwrap_or(u32::MAX),
                            degraded,
                            note: Some(note.clone()),
                        },
                        note: Some(note),
                        evidence,
                    }
                }
                Err(error) => {
                    let note =
                        format!("approved web collection failed before model execution: {error}");
                    CollectedWebContext {
                        evidence: Vec::new(),
                        outcome: WebExecutionOutcome::failed(query, note.clone()),
                        note: Some(note),
                    }
                }
            }
        }
    }
}

fn build_provider_child_user_prompt(
    request: &ChildSubqueryRequest,
    web_context: &CollectedWebContext,
) -> String {
    let mut prompt = request.prompt.clone();
    if !web_context.evidence.is_empty() {
        prompt.push_str("\n\nAttached web evidence:\n");
        for (index, record) in web_context.evidence.iter().enumerate() {
            let fetch_state = if record.is_fetched_web_evidence() {
                "fetched"
            } else {
                "search-result snippet only"
            };
            prompt.push_str(&format!(
                "- [W{}] {} — {} ({})\n  {}\n",
                index + 1,
                record.title,
                record.locator,
                fetch_state,
                record.snippet
            ));
        }
    }
    if let Some(summary) = render_web_provenance_summary(web_context) {
        prompt.push_str("\n\n");
        prompt.push_str(&summary);
    }
    if let Some(note) = web_context.note.as_deref() {
        prompt.push_str("\n\nWeb execution note: ");
        prompt.push_str(note);
    }
    prompt
}

fn build_corpus_subquery_system_prompt(
    request: &ChildSubqueryRequest,
    web_context: &CollectedWebContext,
) -> String {
    let mut prompt = "You are a grounded corpus subquery worker. Answer the task using only the provided corpus slices unless attached web evidence is present. If web evidence is attached, you may use it only as external confirmation or freshness context. Be explicit about what comes from local corpus slices versus fetched web evidence. If the slices are insufficient, say what is missing briefly. Do not invent sources. Keep the answer concise and directly useful.".to_string();
    match request.web_policy.mode {
        WebAccessMode::Off => {
            prompt.push_str(" Web research is disabled for this subquery, so keep the answer strictly local to the provided slices and do not imply any external verification.");
        }
        WebAccessMode::Ask => {
            prompt.push_str(" External web research would require explicit approval for this subquery. Stay grounded in the provided slices; if fresh or external evidence is needed, say that approval is required before using the web.");
        }
        WebAccessMode::On => {
            if !web_context.evidence.is_empty() {
                prompt.push_str(" Approved web evidence is attached with the user message for this subquery. Use it carefully, cite it as external evidence, and avoid overstating confidence if the fetched material is thin.");
            } else if request.web_research_query.is_some() {
                prompt.push_str(" Web escalation was approved for this subquery, but no fetched web evidence is attached. Stay honest about that and avoid implying successful external verification.");
            } else {
                prompt.push_str(" Web access is enabled in principle, but this subquery was not flagged for external fetches. Stay grounded in the provided slices and do not imply web verification.");
            }
        }
    }
    if let Some(note) = web_context.note.as_deref() {
        prompt.push_str(" Runtime note: ");
        prompt.push_str(note);
    }
    prompt
}

fn build_provider_child_output(
    model: &str,
    request: &ChildSubqueryRequest,
    response: MessageResponse,
    web_context: CollectedWebContext,
) -> Result<ChildSubqueryOutput, RecursiveRuntimeError> {
    let mut answer = extract_provider_answer_text(&response);
    if answer.trim().is_empty() {
        return Err(RecursiveRuntimeError::ChildExecution(format!(
            "provider returned an empty child answer (request_id={})",
            response.request_id.as_deref().unwrap_or("unknown")
        )));
    }

    if let Some(note) = web_context.note.as_deref() {
        answer.push_str("\n\nWeb execution note: ");
        answer.push_str(note);
    }

    let web_execution = Some(web_context.outcome.clone());
    let web_execution_note = web_context.note.clone();

    Ok(ChildSubqueryOutput {
        subquery_id: request.subquery_id.clone(),
        answer,
        citations: request
            .slices
            .iter()
            .map(|slice| format!("{} ({})", slice.path, slice.chunk_id))
            .collect(),
        web_evidence: web_context.evidence,
        web_execution,
        web_execution_note,
        prompt_tokens: response.usage.input_tokens,
        completion_tokens: response.usage.output_tokens,
        cost_usd: response.usage.estimated_cost_usd(model).total_cost_usd(),
    })
}

pub fn render_extractive_child_answer(
    request: &ChildSubqueryRequest,
    reason: Option<&str>,
    model: &str,
    web_evidence_collector: &dyn Fn(
        &ChildSubqueryRequest,
    ) -> Result<Vec<EvidenceRecord>, RecursiveRuntimeError>,
) -> ChildSubqueryOutput {
    let web_context = collect_web_context(request, web_evidence_collector);
    let mut answer = request
        .slices
        .iter()
        .map(|slice| {
            let grounded_text = slice
                .metadata
                .get("text")
                .and_then(|value| value.as_str())
                .filter(|text: &&str| !text.trim().is_empty())
                .unwrap_or(slice.preview.trim());
            format!("{}: {}", slice.path, grounded_text.trim())
        })
        .collect::<Vec<_>>()
        .join("\n");
    let web_policy_note = match request.web_policy.mode {
        WebAccessMode::Off => Some(
            "Web research disabled for this subquery; response is grounded only in local slices.",
        ),
        WebAccessMode::Ask => Some(
            "Web research would require approval for this subquery; response remains local-only until approved.",
        ),
        WebAccessMode::On => {
            if web_context.evidence.is_empty() {
                Some(
                    "Web escalation was permitted for this subquery, but no external evidence was successfully attached; external facts remain unverified.",
                )
            } else {
                Some(
                    "Web escalation was permitted for this subquery; any attached web evidence is presented separately from local corpus slices.",
                )
            }
        }
    };
    if let Some(note) = web_policy_note {
        answer = format!("{note}\n{answer}");
    }
    if let Some(rendered_web) = render_web_context(&web_context) {
        answer = format!("{answer}\n\n{rendered_web}");
    }
    if let Some(reason) = reason {
        answer = format!(
            "Fallback: using an extractive local-only subquery answer because provider-backed execution is unavailable ({reason}; model={model}).\n{answer}"
        );
    }
    let web_execution_note = web_context
        .note
        .clone()
        .or_else(|| web_policy_note.map(str::to_string));
    let web_execution = Some(web_context.outcome.clone());
    ChildSubqueryOutput {
        subquery_id: request.subquery_id.clone(),
        answer,
        citations: request
            .slices
            .iter()
            .map(|slice| slice.chunk_id.clone())
            .collect(),
        web_evidence: web_context.evidence,
        web_execution,
        web_execution_note,
        prompt_tokens: u32::try_from(request.prompt.len()).unwrap_or(u32::MAX),
        completion_tokens: 0,
        cost_usd: 0.0,
    }
}

fn render_web_provenance_summary(web_context: &CollectedWebContext) -> Option<String> {
    let query = web_context.outcome.query.as_deref()?;
    let approval = if web_context.outcome.approved {
        "approved"
    } else {
        "not approved"
    };
    let degraded = if web_context.outcome.degraded {
        ", degraded"
    } else {
        ""
    };
    Some(format!(
        "Web provenance: status={}{}; approval={approval}; query=\"{query}\"; evidence={}",
        web_context.outcome.status.as_str(),
        degraded,
        web_context.outcome.evidence_count
    ))
}

fn render_web_context(web_context: &CollectedWebContext) -> Option<String> {
    let mut sections = Vec::new();
    if let Some(summary) = render_web_provenance_summary(web_context) {
        sections.push(summary);
    }
    if !web_context.evidence.is_empty() {
        sections.push(format!(
            "Web evidence:\n{}",
            web_context
                .evidence
                .iter()
                .enumerate()
                .map(|(index, item)| {
                    let fetch_state = if item.is_fetched_web_evidence() {
                        "fetched"
                    } else {
                        "search-result snippet only"
                    };
                    format!(
                        "- [W{}] {} — {} ({})\n  {}",
                        index + 1,
                        item.title,
                        item.locator,
                        fetch_state,
                        item.snippet
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }
    if let Some(note) = web_context.note.as_deref() {
        sections.push(format!("Web execution note: {note}"));
    }
    (!sections.is_empty()).then(|| sections.join("\n"))
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
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};

    use runtime::{
        ChildSubqueryRequest, RecursiveContextSlice, RecursiveRuntimeError, RuntimeBudget,
        WebAccessMode, WebPolicy,
    };

    use super::*;
    use crate::{AuthSource, MessageResponse, Usage};

    fn temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("api-child-{label}-{nanos}"))
    }

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
        LOCK.get_or_init(|| std::sync::Mutex::new(()))
            .lock()
            .expect("env lock")
    }

    fn sample_request(
        mode: WebAccessMode,
        web_research_query: Option<&str>,
    ) -> ChildSubqueryRequest {
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
    fn resolve_provider_child_model_prefers_rlm_subcall_model() {
        let cwd = temp_dir("model-resolution");
        fs::create_dir_all(cwd.join(".claw")).expect("config dir");
        fs::write(
            cwd.join(".claw").join("settings.json"),
            r#"{"model":"claude-opus-4-6","rlm":{"subcallModel":"haiku"}}"#,
        )
        .expect("write settings");

        let resolved =
            resolve_provider_child_model(&cwd, Some("claude-sonnet-4-6"), "claude-opus-4-6");
        assert_eq!(resolved, "claude-haiku-4-5-20251213");

        let _ = fs::remove_dir_all(cwd);
    }

    #[test]
    fn resolve_runtime_provider_child_auth_loads_oauth_config_for_refreshable_tokens() {
        let _guard = env_lock();
        std::env::remove_var("ANTHROPIC_API_KEY");
        std::env::remove_var("ANTHROPIC_AUTH_TOKEN");

        let cwd = temp_dir("runtime-auth");
        fs::create_dir_all(cwd.join(".claw")).expect("config dir");
        fs::write(
            cwd.join(".claw").join("settings.json"),
            r#"{"oauth":{"clientId":"runtime-client","authorizeUrl":"https://console.test/oauth/authorize","tokenUrl":"https://console.test/oauth/token","callbackPort":54545}}"#,
        )
        .expect("write settings");
        runtime::save_oauth_credentials(&runtime::OAuthTokenSet {
            access_token: "saved-access-token".to_string(),
            refresh_token: Some("saved-refresh-token".to_string()),
            expires_at: Some(u64::MAX),
            scopes: vec![],
        })
        .expect("save oauth credentials");

        let auth = resolve_runtime_provider_child_auth(&cwd).expect("runtime auth");
        assert_eq!(auth.bearer_token(), Some("saved-access-token"));

        runtime::clear_oauth_credentials().expect("clear oauth credentials");
        let _ = fs::remove_dir_all(cwd);
    }

    #[test]
    fn collect_minimal_web_evidence_shapes_search_only_hits() {
        let request = sample_request(WebAccessMode::On, Some("latest release"));
        let evidence = collect_minimal_web_evidence(&request, &|query, fetch_limit| {
            assert_eq!(query, "latest release");
            assert_eq!(fetch_limit, 1);
            Ok(vec![MinimalWebEvidence {
                title: "Example result".to_string(),
                url: "https://example.test/release".to_string(),
                snippet: "Search hit only".to_string(),
                fetched: false,
            }])
        })
        .expect("web evidence should collect");

        assert_eq!(evidence.len(), 1);
        assert_eq!(evidence[0].title, "Example result (search result only)");
        assert_eq!(evidence[0].locator, "https://example.test/release");
        assert_eq!(evidence[0].snippet, "Search hit only");
    }

    #[test]
    fn system_prompt_reflects_web_policy() {
        let off = build_corpus_subquery_system_prompt(
            &sample_request(WebAccessMode::Off, None),
            &CollectedWebContext::default(),
        );
        assert!(off.contains("Web research is disabled"));

        let ask = build_corpus_subquery_system_prompt(
            &sample_request(WebAccessMode::Ask, None),
            &CollectedWebContext::default(),
        );
        assert!(ask.contains("would require explicit approval"));

        let on = build_corpus_subquery_system_prompt(
            &sample_request(WebAccessMode::On, Some("latest release")),
            &CollectedWebContext {
                evidence: vec![EvidenceRecord::from_web_input(runtime::WebEvidenceInput {
                    id: "web-1".to_string(),
                    title: "Example release note".to_string(),
                    url: "https://example.test/release".to_string(),
                    snippet: "release snippet".to_string(),
                    fetched_at_ms: None,
                    fetched: None,
                })],
                outcome: WebExecutionOutcome::succeeded("latest release", 1, None),
                note: None,
            },
        );
        assert!(on.contains("Approved web evidence is attached"));
    }

    #[test]
    fn provider_child_output_maps_answer_usage_and_citations() {
        let request = sample_request(WebAccessMode::Off, None);
        let output = build_provider_child_output(
            "claude-sonnet-4-6",
            &request,
            response_with_text("Grounded answer"),
            CollectedWebContext::default(),
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
            CollectedWebContext::default(),
        )
        .expect_err("empty provider answers should fail");

        assert!(
            matches!(error, RecursiveRuntimeError::ChildExecution(message) if message.contains("empty child answer") && message.contains("req_123"))
        );
    }

    #[test]
    fn collect_web_context_degrades_failed_web_collection_into_note() {
        let request = sample_request(WebAccessMode::On, Some("freshness"));
        let context = collect_web_context(&request, &|_| {
            Err(RecursiveRuntimeError::ChildExecution(
                "collector failed".to_string(),
            ))
        });

        assert!(context.evidence.is_empty());
        assert!(context
            .note
            .as_deref()
            .is_some_and(|note| note.contains("collector failed")));
    }

    #[test]
    fn provider_child_output_surfaces_web_collection_note_without_failing() {
        let request = sample_request(WebAccessMode::On, Some("freshness"));
        let output = build_provider_child_output(
            "claude-sonnet-4-6",
            &request,
            response_with_text("Grounded answer"),
            CollectedWebContext {
                evidence: Vec::new(),
                outcome: WebExecutionOutcome::failed(
                    "freshness",
                    "approved web collection failed before model execution: collector failed",
                ),
                note: Some(
                    "approved web collection failed before model execution: collector failed"
                        .to_string(),
                ),
            },
        )
        .expect("degraded web context should still build output");

        assert!(output.answer.contains("Grounded answer"));
        assert!(output
            .answer
            .contains("approved web collection failed before model execution"));
    }

    #[test]
    fn shared_extractive_fallback_preserves_degraded_web_note() {
        let request = sample_request(WebAccessMode::On, Some("freshness"));
        let output = render_extractive_child_answer(
            &request,
            Some("missing provider auth"),
            "claude-sonnet-4-6",
            &|_| {
                Err(RecursiveRuntimeError::ChildExecution(
                    "collector failed".to_string(),
                ))
            },
        );

        assert!(output
            .answer
            .contains("Fallback: using an extractive local-only subquery answer"));
        assert!(output
            .answer
            .contains("Web escalation was permitted for this subquery"));
        assert!(output.answer.contains("Web execution note: approved web collection failed before model execution: collector failed"));
        assert_eq!(output.web_evidence, Vec::<EvidenceRecord>::new());
    }

    #[test]
    fn provider_child_user_prompt_includes_attached_web_evidence() {
        let request = sample_request(WebAccessMode::On, Some("latest release"));
        let prompt = build_provider_child_user_prompt(
            &request,
            &CollectedWebContext {
                evidence: vec![EvidenceRecord::from_web_input(runtime::WebEvidenceInput {
                    id: "web-1".to_string(),
                    title: "Example release note".to_string(),
                    url: "https://example.test/release".to_string(),
                    snippet: "release snippet".to_string(),
                    fetched_at_ms: None,
                    fetched: Some(true),
                })],
                outcome: WebExecutionOutcome::succeeded(
                    "latest release",
                    1,
                    Some("fetched from minimal web adapter".to_string()),
                ),
                note: Some("fetched from minimal web adapter".to_string()),
            },
        );

        assert!(prompt.contains("Summarize the relevant slice"));
        assert!(prompt.contains("Attached web evidence"));
        assert!(prompt.contains("[W1] Example release note"));
        assert!(prompt.contains("(fetched)"));
        assert!(prompt.contains("Web provenance: status=succeeded; approval=approved; query=\"latest release\"; evidence=1"));
        assert!(prompt.contains("Web execution note: fetched from minimal web adapter"));
    }

    #[test]
    fn collect_web_context_marks_search_only_evidence_as_degraded_success() {
        let request = sample_request(WebAccessMode::On, Some("latest release"));
        let context = collect_web_context(&request, &|_| {
            Ok(vec![EvidenceRecord::from_web_input(
                runtime::WebEvidenceInput {
                    id: "web-1".to_string(),
                    title: "Example release note (search result only)".to_string(),
                    url: "https://example.test/release".to_string(),
                    snippet: "Search result located, but page fetch failed: timeout".to_string(),
                    fetched_at_ms: None,
                    fetched: Some(false),
                },
            )])
        });

        assert_eq!(context.outcome.status.as_str(), "succeeded");
        assert!(context.outcome.degraded);
        assert_eq!(context.outcome.evidence_count, 1);
        assert!(context
            .note
            .as_deref()
            .is_some_and(|note| note.contains("only search-result snippets were available")));
        let rendered = render_web_context(&context).expect("web context should render");
        assert!(rendered.contains("Web provenance: status=succeeded, degraded; approval=approved; query=\"latest release\"; evidence=1"));
        assert!(rendered.contains("search-result snippet only"));
    }

    #[test]
    fn fallback_reason_wraps_child_errors() {
        let reason = format_provider_execution_fallback_reason(
            &RecursiveRuntimeError::ChildExecution("network down".to_string()),
        );
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

    #[test]
    fn child_init_reason_surfaces_missing_credentials_cleanly() {
        let reason = format_provider_child_init_reason(
            "claude-sonnet-4-6",
            &ApiError::missing_credentials("Anthropic", &["ANTHROPIC_API_KEY"]),
        );

        assert!(reason.contains("provider child executor unavailable"));
        assert!(reason.contains("model=claude-sonnet-4-6"));
        assert!(reason.contains("ANTHROPIC_API_KEY"));
    }

    #[test]
    fn shared_builder_constructs_executor_with_prompt_cache_ready_client() {
        let callback: WebEvidenceCollector = Arc::new(|_| Ok(Vec::new()));
        let executor = build_provider_child_executor(
            "session-1",
            "claude-sonnet-4-6",
            Some(AuthSource::ApiKey("test".to_string())),
            callback,
        )
        .expect("shared builder should build executor");

        assert_eq!(executor.model(), "claude-sonnet-4-6");
    }

    #[test]
    fn shared_backend_factory_builds_provider_variant_when_auth_is_available() {
        let callback: WebEvidenceCollector = Arc::new(|_| Ok(Vec::new()));
        let backend = ProviderChildBackend::build_with_resolver(
            "session-1",
            "claude-sonnet-4-6",
            Arc::new(|| Ok(Some(AuthSource::ApiKey("test".to_string())))),
            callback,
        );

        assert_eq!(backend.model(), "claude-sonnet-4-6");
        assert!(backend.unavailable_reason().is_none());
        assert!(matches!(backend, ProviderChildBackend::Provider(_)));
    }

    #[test]
    fn shared_backend_factory_preserves_resolver_failure_as_unavailable_reason() {
        let callback: WebEvidenceCollector = Arc::new(|_| Ok(Vec::new()));
        let backend = ProviderChildBackend::build_with_resolver(
            "session-1",
            "claude-sonnet-4-6",
            Arc::new(|| Err("oauth bootstrap failed".to_string())),
            callback,
        );

        assert_eq!(backend.model(), "claude-sonnet-4-6");
        assert_eq!(backend.unavailable_reason(), Some("oauth bootstrap failed"));
        assert!(matches!(backend, ProviderChildBackend::Unavailable { .. }));
    }

    #[test]
    fn provider_backed_executor_prefers_unavailable_reason_for_fallbacks() {
        let callback: WebEvidenceCollector = Arc::new(|_| Ok(Vec::new()));
        let executor = build_provider_backed_child_executor(
            "session-1",
            "claude-sonnet-4-6",
            Arc::new(|| Err("oauth bootstrap failed".to_string())),
            callback,
            Arc::new(|request, reason| ChildSubqueryOutput {
                subquery_id: request.subquery_id.clone(),
                answer: reason.to_string(),
                citations: Vec::new(),
                web_evidence: Vec::new(),
                web_execution: Some(WebExecutionOutcome::not_requested()),
                web_execution_note: None,
                prompt_tokens: 0,
                completion_tokens: 0,
                cost_usd: 0.0,
            }),
        );
        let output = executor
            .execute(&ChildSubqueryRequest {
                subquery_id: "subq-1".to_string(),
                prompt: "summarize".to_string(),
                slices: Vec::new(),
                budget: runtime::RuntimeBudget::default(),
                web_policy: runtime::WebPolicy {
                    mode: runtime::WebAccessMode::Off,
                    max_fetches: Some(0),
                },
                web_research_query: None,
            })
            .expect("fallback should render output");

        assert_eq!(output.answer, "oauth bootstrap failed");
    }

    #[test]
    fn provider_backed_executor_formats_runtime_failures_when_available_backend_errors() {
        let callback: WebEvidenceCollector = Arc::new(|_| Ok(Vec::new()));
        let executor = build_provider_backed_child_executor(
            "session-1",
            "claude-sonnet-4-6",
            Arc::new(|| Ok(None)),
            callback,
            Arc::new(|request, reason| ChildSubqueryOutput {
                subquery_id: request.subquery_id.clone(),
                answer: reason.to_string(),
                citations: Vec::new(),
                web_evidence: Vec::new(),
                web_execution: Some(WebExecutionOutcome::not_requested()),
                web_execution_note: None,
                prompt_tokens: 0,
                completion_tokens: 0,
                cost_usd: 0.0,
            }),
        );
        let output = executor
            .execute(&ChildSubqueryRequest {
                subquery_id: "subq-2".to_string(),
                prompt: "summarize".to_string(),
                slices: Vec::new(),
                budget: runtime::RuntimeBudget::default(),
                web_policy: runtime::WebPolicy {
                    mode: runtime::WebAccessMode::Off,
                    max_fetches: Some(0),
                },
                web_research_query: None,
            })
            .expect("fallback should render output");

        assert!(output
            .answer
            .contains("provider child executor unavailable"));
    }

    #[test]
    fn extractive_builder_uses_shared_extractive_fallback_renderer() {
        let callback: WebEvidenceCollector = Arc::new(|_| Ok(Vec::new()));
        let executor = build_provider_extractive_child_executor(
            "session-1",
            "claude-sonnet-4-6",
            Arc::new(|| Err("oauth bootstrap failed".to_string())),
            callback,
        );
        let output = executor
            .execute(&ChildSubqueryRequest {
                subquery_id: "subq-3".to_string(),
                prompt: "summarize the grounded evidence".to_string(),
                slices: vec![runtime::RecursiveContextSlice {
                    chunk_id: "chunk-1".to_string(),
                    document_id: "doc-1".to_string(),
                    path: "docs/spec.md".to_string(),
                    ordinal: 0,
                    start_offset: 0,
                    end_offset: 42,
                    preview: "provider-backed subqueries should cite grounded slices".to_string(),
                    metadata: std::collections::BTreeMap::new(),
                }],
                budget: runtime::RuntimeBudget::default(),
                web_policy: runtime::WebPolicy {
                    mode: runtime::WebAccessMode::Off,
                    max_fetches: Some(0),
                },
                web_research_query: None,
            })
            .expect("extractive fallback should render output");

        assert!(output
            .answer
            .contains("Fallback: using an extractive local-only subquery answer"));
        assert!(output.answer.contains("oauth bootstrap failed"));
        assert_eq!(output.citations, vec!["chunk-1".to_string()]);
    }
}
