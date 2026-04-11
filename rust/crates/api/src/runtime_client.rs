use std::collections::BTreeMap;

use runtime::{
    ApiClient, ApiRequest, AssistantEvent, ContentBlock, ConversationMessage, ConversationRuntime,
    MessageRole, PermissionPolicy, PromptCacheEvent, RuntimeError, RuntimeFeatureConfig, Session,
    ToolExecutor,
};

use crate::{
    max_tokens_for_model, resolve_model_alias, AuthSource, ContentBlockDelta, InputContentBlock,
    InputMessage, MessageRequest, MessageResponse, OutputContentBlock, PromptCache,
    PromptCacheRecord, ProviderClient, ToolChoice, ToolDefinition, ToolResultContentBlock,
};

pub trait ProviderRuntimeObserver {
    fn on_model_invoked(&mut self) {}

    fn on_text_delta(&mut self, _text: &str) -> Result<(), RuntimeError> {
        Ok(())
    }

    fn on_tool_use_ready(&mut self, _name: &str, _input: &str) -> Result<(), RuntimeError> {
        Ok(())
    }

    fn on_message_stop(&mut self) -> Result<(), RuntimeError> {
        Ok(())
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct NoopProviderRuntimeObserver;

impl ProviderRuntimeObserver for NoopProviderRuntimeObserver {}

pub struct ProviderRuntimeApiClient<O = NoopProviderRuntimeObserver> {
    runtime: tokio::runtime::Runtime,
    client: ProviderClient,
    model: String,
    tools: Vec<ToolDefinition>,
    observer: O,
}

struct ProviderRuntimeBuildContext {
    client: ProviderClient,
    model: String,
    tools: Vec<ToolDefinition>,
}

impl ProviderRuntimeBuildContext {
    fn new(
        model: String,
        tools: Vec<ToolDefinition>,
        anthropic_auth: Option<AuthSource>,
    ) -> Result<Self, String> {
        let resolved_model = resolve_model_alias(&model).to_string();
        let client =
            ProviderClient::from_model_with_anthropic_auth(&resolved_model, anthropic_auth)
                .map_err(|error| error.to_string())?;
        Ok(Self {
            client,
            model: resolved_model,
            tools,
        })
    }

    fn into_api_client<O>(
        self,
        prompt_cache_namespace: Option<&str>,
        observer: O,
    ) -> Result<ProviderRuntimeApiClient<O>, String>
    where
        O: ProviderRuntimeObserver,
    {
        let client = match prompt_cache_namespace {
            Some(namespace) => self.client.with_prompt_cache(PromptCache::new(namespace)),
            None => self.client,
        };
        Ok(ProviderRuntimeApiClient {
            runtime: tokio::runtime::Runtime::new().map_err(|error| error.to_string())?,
            client,
            model: self.model,
            tools: self.tools,
            observer,
        })
    }
}

impl ProviderRuntimeApiClient<NoopProviderRuntimeObserver> {
    #[allow(clippy::needless_pass_by_value)]
    pub fn new(model: String, tools: Vec<ToolDefinition>) -> Result<Self, String> {
        Self::new_with_auth(model, tools, None)
    }

    #[allow(clippy::needless_pass_by_value)]
    pub fn new_with_auth(
        model: String,
        tools: Vec<ToolDefinition>,
        anthropic_auth: Option<AuthSource>,
    ) -> Result<Self, String> {
        ProviderRuntimeBuildContext::new(model, tools, anthropic_auth)?
            .into_api_client(None, NoopProviderRuntimeObserver)
    }
}

impl<O> ProviderRuntimeApiClient<O>
where
    O: ProviderRuntimeObserver,
{
    #[must_use]
    pub fn with_prompt_cache(mut self, namespace: &str) -> Self {
        self.client = self.client.with_prompt_cache(PromptCache::new(namespace));
        self
    }

    #[must_use]
    pub fn with_observer<NO>(self, observer: NO) -> ProviderRuntimeApiClient<NO>
    where
        NO: ProviderRuntimeObserver,
    {
        ProviderRuntimeApiClient {
            runtime: self.runtime,
            client: self.client,
            model: self.model,
            tools: self.tools,
            observer,
        }
    }

    #[must_use]
    pub fn model(&self) -> &str {
        &self.model
    }
}

fn build_provider_runtime_api_client_internal<O>(
    model: String,
    tools: Vec<ToolDefinition>,
    prompt_cache_namespace: Option<&str>,
    anthropic_auth: Option<AuthSource>,
    observer: O,
) -> Result<ProviderRuntimeApiClient<O>, String>
where
    O: ProviderRuntimeObserver,
{
    ProviderRuntimeBuildContext::new(model, tools, anthropic_auth)?
        .into_api_client(prompt_cache_namespace, observer)
}

#[allow(clippy::too_many_arguments)]
fn build_provider_conversation_runtime_internal<T, O>(
    session: Session,
    model: String,
    tools: Vec<ToolDefinition>,
    prompt_cache_namespace: &str,
    anthropic_auth: Option<AuthSource>,
    tool_executor: T,
    permission_policy: PermissionPolicy,
    system_prompt: Vec<String>,
    feature_config: Option<&RuntimeFeatureConfig>,
    observer: O,
) -> Result<ConversationRuntime<ProviderRuntimeApiClient<O>, T>, String>
where
    T: ToolExecutor,
    O: ProviderRuntimeObserver,
{
    let api_client = build_provider_runtime_api_client_internal(
        model,
        tools,
        Some(prompt_cache_namespace),
        anthropic_auth,
        observer,
    )?;

    Ok(match feature_config {
        Some(config) => ConversationRuntime::new_with_features(
            session,
            api_client,
            tool_executor,
            permission_policy,
            system_prompt,
            config,
        ),
        None => ConversationRuntime::new(
            session,
            api_client,
            tool_executor,
            permission_policy,
            system_prompt,
        ),
    })
}

#[allow(clippy::needless_pass_by_value)]
pub fn build_provider_runtime_api_client(
    model: String,
    tools: Vec<ToolDefinition>,
    prompt_cache_namespace: &str,
) -> Result<ProviderRuntimeApiClient, String> {
    build_provider_runtime_api_client_with_auth(model, tools, prompt_cache_namespace, None)
}

#[allow(clippy::needless_pass_by_value)]
pub fn build_provider_runtime_api_client_with_auth(
    model: String,
    tools: Vec<ToolDefinition>,
    prompt_cache_namespace: &str,
    anthropic_auth: Option<AuthSource>,
) -> Result<ProviderRuntimeApiClient, String> {
    build_provider_runtime_api_client_internal(
        model,
        tools,
        Some(prompt_cache_namespace),
        anthropic_auth,
        NoopProviderRuntimeObserver,
    )
}

#[allow(clippy::needless_pass_by_value)]
pub fn build_provider_runtime_api_client_with_auth_and_observer<O>(
    model: String,
    tools: Vec<ToolDefinition>,
    prompt_cache_namespace: &str,
    anthropic_auth: Option<AuthSource>,
    observer: O,
) -> Result<ProviderRuntimeApiClient<O>, String>
where
    O: ProviderRuntimeObserver,
{
    build_provider_runtime_api_client_internal(
        model,
        tools,
        Some(prompt_cache_namespace),
        anthropic_auth,
        observer,
    )
}

#[allow(clippy::needless_pass_by_value)]
pub fn build_provider_conversation_runtime<T>(
    model: String,
    tools: Vec<ToolDefinition>,
    prompt_cache_namespace: &str,
    tool_executor: T,
    permission_policy: PermissionPolicy,
    system_prompt: Vec<String>,
) -> Result<ConversationRuntime<ProviderRuntimeApiClient, T>, String>
where
    T: ToolExecutor,
{
    build_provider_conversation_runtime_for_session(
        Session::new(),
        model,
        tools,
        prompt_cache_namespace,
        tool_executor,
        permission_policy,
        system_prompt,
    )
}

#[allow(clippy::needless_pass_by_value)]
pub fn build_provider_conversation_runtime_for_session<T>(
    session: Session,
    model: String,
    tools: Vec<ToolDefinition>,
    prompt_cache_namespace: &str,
    tool_executor: T,
    permission_policy: PermissionPolicy,
    system_prompt: Vec<String>,
) -> Result<ConversationRuntime<ProviderRuntimeApiClient, T>, String>
where
    T: ToolExecutor,
{
    build_provider_conversation_runtime_with_auth_for_session(
        session,
        model,
        tools,
        prompt_cache_namespace,
        None,
        tool_executor,
        permission_policy,
        system_prompt,
    )
}

#[allow(clippy::needless_pass_by_value)]
pub fn build_provider_conversation_runtime_with_auth_for_session<T>(
    session: Session,
    model: String,
    tools: Vec<ToolDefinition>,
    prompt_cache_namespace: &str,
    anthropic_auth: Option<AuthSource>,
    tool_executor: T,
    permission_policy: PermissionPolicy,
    system_prompt: Vec<String>,
) -> Result<ConversationRuntime<ProviderRuntimeApiClient, T>, String>
where
    T: ToolExecutor,
{
    build_provider_conversation_runtime_internal(
        session,
        model,
        tools,
        prompt_cache_namespace,
        anthropic_auth,
        tool_executor,
        permission_policy,
        system_prompt,
        None,
        NoopProviderRuntimeObserver,
    )
}

#[allow(clippy::needless_pass_by_value)]
#[allow(clippy::too_many_arguments)]
pub fn build_provider_conversation_runtime_with_features_and_observer_for_session<T, O>(
    session: Session,
    model: String,
    tools: Vec<ToolDefinition>,
    prompt_cache_namespace: &str,
    anthropic_auth: Option<AuthSource>,
    tool_executor: T,
    permission_policy: PermissionPolicy,
    system_prompt: Vec<String>,
    feature_config: &RuntimeFeatureConfig,
    observer: O,
) -> Result<ConversationRuntime<ProviderRuntimeApiClient<O>, T>, String>
where
    T: ToolExecutor,
    O: ProviderRuntimeObserver,
{
    build_provider_conversation_runtime_internal(
        session,
        model,
        tools,
        prompt_cache_namespace,
        anthropic_auth,
        tool_executor,
        permission_policy,
        system_prompt,
        Some(feature_config),
        observer,
    )
}

impl<O> ApiClient for ProviderRuntimeApiClient<O>
where
    O: ProviderRuntimeObserver,
{
    #[allow(clippy::too_many_lines)]
    fn stream(&mut self, request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError> {
        self.observer.on_model_invoked();
        let message_request = MessageRequest {
            model: self.model.clone(),
            max_tokens: max_tokens_for_model(&self.model),
            messages: convert_messages(&request.messages),
            system: (!request.system_prompt.is_empty()).then(|| request.system_prompt.join("\n\n")),
            tools: (!self.tools.is_empty()).then(|| self.tools.clone()),
            tool_choice: (!self.tools.is_empty()).then_some(ToolChoice::Auto),
            stream: true,
        };

        self.runtime.block_on(async {
            let mut stream = self
                .client
                .stream_message(&message_request)
                .await
                .map_err(|error| RuntimeError::new(error.to_string()))?;
            let mut events = Vec::new();
            let mut pending_tools: BTreeMap<u32, (String, String, String)> = BTreeMap::new();
            let mut saw_stop = false;

            while let Some(event) = stream
                .next_event()
                .await
                .map_err(|error| RuntimeError::new(error.to_string()))?
            {
                match event {
                    crate::StreamEvent::MessageStart(start) => {
                        for block in start.message.content {
                            push_output_block(
                                block,
                                0,
                                &mut events,
                                &mut pending_tools,
                                true,
                                &mut self.observer,
                            )?;
                        }
                    }
                    crate::StreamEvent::ContentBlockStart(start) => {
                        push_output_block(
                            start.content_block,
                            start.index,
                            &mut events,
                            &mut pending_tools,
                            true,
                            &mut self.observer,
                        )?;
                    }
                    crate::StreamEvent::ContentBlockDelta(delta) => match delta.delta {
                        ContentBlockDelta::TextDelta { text } => {
                            if !text.is_empty() {
                                self.observer.on_text_delta(&text)?;
                                events.push(AssistantEvent::TextDelta(text));
                            }
                        }
                        ContentBlockDelta::InputJsonDelta { partial_json } => {
                            if let Some((_, _, input)) = pending_tools.get_mut(&delta.index) {
                                input.push_str(&partial_json);
                            }
                        }
                        ContentBlockDelta::ThinkingDelta { .. }
                        | ContentBlockDelta::SignatureDelta { .. } => {}
                    },
                    crate::StreamEvent::ContentBlockStop(stop) => {
                        if let Some((id, name, input)) = pending_tools.remove(&stop.index) {
                            self.observer.on_tool_use_ready(&name, &input)?;
                            events.push(AssistantEvent::ToolUse { id, name, input });
                        }
                    }
                    crate::StreamEvent::MessageDelta(delta) => {
                        events.push(AssistantEvent::Usage(delta.usage.token_usage()));
                    }
                    crate::StreamEvent::MessageStop(_) => {
                        saw_stop = true;
                        self.observer.on_message_stop()?;
                        events.push(AssistantEvent::MessageStop);
                    }
                }
            }

            push_prompt_cache_record(&self.client, &mut events);

            if !saw_stop
                && events.iter().any(|event| {
                    matches!(event, AssistantEvent::TextDelta(text) if !text.is_empty())
                        || matches!(event, AssistantEvent::ToolUse { .. })
                })
            {
                events.push(AssistantEvent::MessageStop);
            }

            if events
                .iter()
                .any(|event| matches!(event, AssistantEvent::MessageStop))
            {
                return Ok(events);
            }

            let response = self
                .client
                .send_message(&MessageRequest {
                    stream: false,
                    ..message_request.clone()
                })
                .await
                .map_err(|error| RuntimeError::new(error.to_string()))?;
            let mut events = response_to_events(response, &mut self.observer)?;
            push_prompt_cache_record(&self.client, &mut events);
            Ok(events)
        })
    }
}

fn convert_messages(messages: &[ConversationMessage]) -> Vec<InputMessage> {
    messages
        .iter()
        .filter_map(|message| {
            let role = match message.role {
                MessageRole::System | MessageRole::User | MessageRole::Tool => "user",
                MessageRole::Assistant => "assistant",
            };
            let content = message
                .blocks
                .iter()
                .map(|block| match block {
                    ContentBlock::Text { text } => InputContentBlock::Text { text: text.clone() },
                    ContentBlock::ToolUse { id, name, input } => InputContentBlock::ToolUse {
                        id: id.clone(),
                        name: name.clone(),
                        input: serde_json::from_str(input)
                            .unwrap_or_else(|_| serde_json::json!({ "raw": input })),
                    },
                    ContentBlock::ToolResult {
                        tool_use_id,
                        output,
                        is_error,
                        ..
                    } => InputContentBlock::ToolResult {
                        tool_use_id: tool_use_id.clone(),
                        content: vec![ToolResultContentBlock::Text {
                            text: output.clone(),
                        }],
                        is_error: *is_error,
                    },
                })
                .collect::<Vec<_>>();
            if content.is_empty() {
                None
            } else {
                Some(InputMessage {
                    role: role.to_string(),
                    content,
                })
            }
        })
        .collect()
}

fn push_output_block<O>(
    block: OutputContentBlock,
    index: u32,
    events: &mut Vec<AssistantEvent>,
    pending_tools: &mut BTreeMap<u32, (String, String, String)>,
    streaming: bool,
    observer: &mut O,
) -> Result<(), RuntimeError>
where
    O: ProviderRuntimeObserver,
{
    match block {
        OutputContentBlock::Text { text } => {
            if !text.is_empty() {
                observer.on_text_delta(&text)?;
                events.push(AssistantEvent::TextDelta(text));
            }
        }
        OutputContentBlock::ToolUse { id, name, input } => {
            let input = if input == serde_json::json!({}) {
                String::new()
            } else {
                input.to_string()
            };
            if streaming {
                pending_tools.insert(index, (id, name, input));
            } else {
                observer.on_tool_use_ready(&name, &input)?;
                events.push(AssistantEvent::ToolUse { id, name, input });
            }
        }
        OutputContentBlock::Thinking { .. } | OutputContentBlock::RedactedThinking { .. } => {}
    }
    Ok(())
}

fn response_to_events<O>(
    response: MessageResponse,
    observer: &mut O,
) -> Result<Vec<AssistantEvent>, RuntimeError>
where
    O: ProviderRuntimeObserver,
{
    let mut events = Vec::new();
    let mut pending_tools = BTreeMap::new();
    for (index, block) in response.content.into_iter().enumerate() {
        push_output_block(
            block,
            index as u32,
            &mut events,
            &mut pending_tools,
            false,
            observer,
        )?;
    }
    observer.on_message_stop()?;
    events.push(AssistantEvent::Usage(response.usage.token_usage()));
    events.push(AssistantEvent::MessageStop);
    Ok(events)
}

fn push_prompt_cache_record(client: &ProviderClient, events: &mut Vec<AssistantEvent>) {
    if let Some(record) = client.take_last_prompt_cache_record() {
        if let Some(event) = prompt_cache_record_to_runtime_event(record) {
            events.push(AssistantEvent::PromptCache(event));
        }
    }
}

fn prompt_cache_record_to_runtime_event(record: PromptCacheRecord) -> Option<PromptCacheEvent> {
    let cache_break = record.cache_break?;
    Some(PromptCacheEvent {
        unexpected: cache_break.unexpected,
        reason: cache_break.reason,
        previous_cache_read_input_tokens: cache_break.previous_cache_read_input_tokens,
        current_cache_read_input_tokens: cache_break.current_cache_read_input_tokens,
        token_drop: cache_break.token_drop,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        build_provider_conversation_runtime_with_features_and_observer_for_session,
        build_provider_runtime_api_client_with_auth_and_observer,
        prompt_cache_record_to_runtime_event, ProviderRuntimeObserver,
    };
    use crate::{AuthSource, CacheBreakEvent, PromptCacheRecord, ToolDefinition};
    use runtime::{
        ExecutionProfile, PermissionMode, PermissionPolicy, RuntimeError, RuntimeFeatureConfig,
        Session, ToolError, ToolExecutor,
    };

    #[derive(Default)]
    struct RecordingObserver {
        model_invocations: usize,
        text_deltas: Vec<String>,
        tools: Vec<(String, String)>,
    }

    impl ProviderRuntimeObserver for RecordingObserver {
        fn on_model_invoked(&mut self) {
            self.model_invocations += 1;
        }

        fn on_text_delta(&mut self, text: &str) -> Result<(), RuntimeError> {
            self.text_deltas.push(text.to_string());
            Ok(())
        }

        fn on_tool_use_ready(&mut self, name: &str, input: &str) -> Result<(), RuntimeError> {
            self.tools.push((name.to_string(), input.to_string()));
            Ok(())
        }
    }

    #[derive(Default)]
    struct NoopToolExecutor;

    impl ToolExecutor for NoopToolExecutor {
        fn execute(&mut self, _tool_name: &str, _input: &str) -> Result<String, ToolError> {
            Ok("ok".to_string())
        }
    }

    #[test]
    fn prompt_cache_records_convert_to_runtime_events() {
        let event = prompt_cache_record_to_runtime_event(PromptCacheRecord {
            cache_break: Some(CacheBreakEvent {
                unexpected: true,
                reason: "drop".to_string(),
                previous_cache_read_input_tokens: 500,
                current_cache_read_input_tokens: 100,
                token_drop: 400,
            }),
            stats: crate::PromptCacheStats::default(),
        })
        .expect("prompt cache break should map to runtime event");

        assert!(event.unexpected);
        assert_eq!(event.token_drop, 400);
    }

    #[test]
    fn build_provider_runtime_api_client_supports_observer_configuration() {
        let client = build_provider_runtime_api_client_with_auth_and_observer(
            "claude-opus-4-6".to_string(),
            vec![ToolDefinition {
                name: "read_file".to_string(),
                description: Some("Read a file".to_string()),
                input_schema: serde_json::json!({"type": "object"}),
            }],
            "runtime-client-test",
            Some(AuthSource::ApiKey("test-key".to_string())),
            RecordingObserver::default(),
        )
        .expect("provider runtime api client should build");

        assert_eq!(client.model(), "claude-opus-4-6");
    }

    #[test]
    fn build_provider_runtime_with_features_and_observer_uses_shared_constructor() {
        let feature_config =
            RuntimeFeatureConfig::default().with_execution_profile(ExecutionProfile::Balanced);
        let runtime = build_provider_conversation_runtime_with_features_and_observer_for_session(
            Session::new(),
            "claude-opus-4-6".to_string(),
            Vec::new(),
            "runtime-client-test",
            Some(AuthSource::ApiKey("test-key".to_string())),
            NoopToolExecutor,
            PermissionPolicy::new(PermissionMode::WorkspaceWrite),
            vec!["system prompt".to_string()],
            &feature_config,
            RecordingObserver::default(),
        )
        .expect("conversation runtime should build");

        assert_eq!(runtime.session().messages.len(), 0);
    }

    #[test]
    fn observer_callbacks_can_record_runtime_activity() {
        let mut observer = RecordingObserver::default();
        observer.on_model_invoked();
        observer.on_text_delta("hello").expect("text callback");
        observer
            .on_tool_use_ready("read_file", "{}")
            .expect("tool callback");

        assert_eq!(observer.model_invocations, 1);
        assert_eq!(observer.text_deltas, vec!["hello"]);
        assert_eq!(
            observer.tools,
            vec![("read_file".to_string(), "{}".to_string())]
        );
    }
}
