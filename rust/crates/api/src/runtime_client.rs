use std::collections::BTreeMap;

use runtime::{
    ApiClient, ApiRequest, AssistantEvent, ContentBlock, ConversationMessage,
    ConversationRuntime, MessageRole, PermissionPolicy, RuntimeError, Session, ToolExecutor,
};

use crate::{
    max_tokens_for_model, resolve_model_alias, ContentBlockDelta, InputContentBlock, InputMessage,
    MessageRequest, MessageResponse, OutputContentBlock, PromptCache, ProviderClient, ToolChoice,
    ToolDefinition, ToolResultContentBlock,
};

pub struct ProviderRuntimeApiClient {
    runtime: tokio::runtime::Runtime,
    client: ProviderClient,
    model: String,
    tools: Vec<ToolDefinition>,
}

impl ProviderRuntimeApiClient {
    #[allow(clippy::needless_pass_by_value)]
    pub fn new(model: String, tools: Vec<ToolDefinition>) -> Result<Self, String> {
        let model = resolve_model_alias(&model).clone();
        let client = ProviderClient::from_model(&model).map_err(|error| error.to_string())?;
        Ok(Self {
            runtime: tokio::runtime::Runtime::new().map_err(|error| error.to_string())?,
            client,
            model,
            tools,
        })
    }

    #[must_use]
    pub fn with_prompt_cache(mut self, namespace: &str) -> Self {
        self.client = self.client.with_prompt_cache(PromptCache::new(namespace));
        self
    }

    #[must_use]
    pub fn model(&self) -> &str {
        &self.model
    }
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
    let api_client = ProviderRuntimeApiClient::new(model, tools)?
        .with_prompt_cache(prompt_cache_namespace);
    Ok(ConversationRuntime::new(
        Session::new(),
        api_client,
        tool_executor,
        permission_policy,
        system_prompt,
    ))
}

impl ApiClient for ProviderRuntimeApiClient {
    #[allow(clippy::too_many_lines)]
    fn stream(&mut self, request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError> {
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
                            push_output_block(block, 0, &mut events, &mut pending_tools, true);
                        }
                    }
                    crate::StreamEvent::ContentBlockStart(start) => {
                        push_output_block(
                            start.content_block,
                            start.index,
                            &mut events,
                            &mut pending_tools,
                            true,
                        );
                    }
                    crate::StreamEvent::ContentBlockDelta(delta) => match delta.delta {
                        ContentBlockDelta::TextDelta { text } => {
                            if !text.is_empty() {
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
                            events.push(AssistantEvent::ToolUse { id, name, input });
                        }
                    }
                    crate::StreamEvent::MessageDelta(delta) => {
                        events.push(AssistantEvent::Usage(delta.usage.token_usage()));
                    }
                    crate::StreamEvent::MessageStop(_) => {
                        saw_stop = true;
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
            let mut events = response_to_events(response);
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

fn push_output_block(
    block: OutputContentBlock,
    index: u32,
    events: &mut Vec<AssistantEvent>,
    pending_tools: &mut BTreeMap<u32, (String, String, String)>,
    streaming: bool,
) {
    match block {
        OutputContentBlock::Text { text } => {
            if !text.is_empty() {
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
                events.push(AssistantEvent::ToolUse { id, name, input });
            }
        }
        OutputContentBlock::Thinking { .. } | OutputContentBlock::RedactedThinking { .. } => {}
    }
}

fn response_to_events(response: MessageResponse) -> Vec<AssistantEvent> {
    let mut events = Vec::new();
    let mut pending_tools = BTreeMap::new();
    for (index, block) in response.content.into_iter().enumerate() {
        push_output_block(block, index as u32, &mut events, &mut pending_tools, false);
    }
    events.push(AssistantEvent::Usage(response.usage.token_usage()));
    events.push(AssistantEvent::MessageStop);
    events
}

fn push_prompt_cache_record(client: &ProviderClient, events: &mut Vec<AssistantEvent>) {
    if let Some(record) = client.take_last_prompt_cache_record() {
        if let Some(cache_break) = record.cache_break {
            events.push(AssistantEvent::PromptCache(runtime::PromptCacheEvent {
                unexpected: cache_break.unexpected,
                reason: cache_break.reason,
                previous_cache_read_input_tokens: cache_break.previous_cache_read_input_tokens,
                current_cache_read_input_tokens: cache_break.current_cache_read_input_tokens,
                token_drop: cache_break.token_drop,
            }));
        }
    }
}
