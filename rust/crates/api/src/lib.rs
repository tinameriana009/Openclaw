mod child;
mod client;
mod error;
mod prompt_cache;
mod providers;
mod runtime_client;
mod sse;
mod types;
mod web;

pub use child::{
    build_configured_provider_extractive_child_executor, build_provider_backed_child_executor,
    build_provider_child_executor, build_provider_child_executor_with_config,
    build_provider_extractive_child_executor,
    build_runtime_configured_provider_extractive_child_executor,
    build_runtime_configured_provider_recursive_runtime,
    build_runtime_configured_provider_recursive_task_runtime, collect_minimal_web_evidence,
    format_provider_child_init_reason, format_provider_execution_fallback_reason,
    prepare_runtime_configured_provider_recursive_task_config,
    prepare_runtime_configured_provider_recursive_task_run, render_extractive_child_answer,
    resolve_provider_child_model, resolve_runtime_provider_child_auth,
    run_runtime_configured_provider_recursive_query,
    run_runtime_configured_provider_recursive_task,
    run_runtime_configured_provider_recursive_task_config, runtime_minimal_web_evidence_fetcher,
    runtime_provider_child_auth_resolver, MinimalWebEvidence, MinimalWebEvidenceFetcher,
    ProviderBackedChildExecutor, ProviderChildAuthResolver, ProviderChildBackend,
    ProviderChildExecutor, ProviderChildExecutorConfig, ProviderFallbackRenderer,
    ProviderPreparedRecursiveTaskRun, ProviderRecursiveRunArtifacts,
    ProviderRecursiveRuntimeConfig, ProviderRecursiveTaskConfig, ProviderRecursiveTaskRequest,
    WebEvidenceCollector,
};
pub use client::{
    oauth_token_is_expired, read_base_url, read_xai_base_url, resolve_saved_oauth_token,
    resolve_startup_auth_source, MessageStream, OAuthTokenSet, ProviderClient,
};
pub use error::ApiError;
pub use prompt_cache::{
    CacheBreakEvent, PromptCache, PromptCacheConfig, PromptCachePaths, PromptCacheRecord,
    PromptCacheStats,
};
pub use providers::anthropic::{AnthropicClient, AnthropicClient as ApiClient, AuthSource};
pub use providers::openai_compat::{OpenAiCompatClient, OpenAiCompatConfig};
pub use providers::{
    detect_provider_kind, max_tokens_for_model, resolve_model_alias, ProviderKind,
};
pub use runtime_client::{
    build_provider_conversation_runtime, ProviderRuntimeApiClient,
};
pub use sse::{parse_frame, SseParser};
pub use types::{
    ContentBlockDelta, ContentBlockDeltaEvent, ContentBlockStartEvent, ContentBlockStopEvent,
    InputContentBlock, InputMessage, MessageDelta, MessageDeltaEvent, MessageRequest,
    MessageResponse, MessageStartEvent, MessageStopEvent, OutputContentBlock, StreamEvent,
    ToolChoice, ToolDefinition, ToolResultContentBlock, Usage,
};
pub use web::minimal_web_research;

pub use telemetry::{
    AnalyticsEvent, AnthropicRequestProfile, ClientIdentity, JsonlTelemetrySink,
    MemoryTelemetrySink, SessionTraceRecord, SessionTracer, TelemetryEvent, TelemetrySink,
    DEFAULT_ANTHROPIC_VERSION,
};
