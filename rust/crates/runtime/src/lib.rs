mod bash;
mod bootstrap;
mod budget;
mod compact;
mod config;
mod conversation;
mod corpus;
mod file_ops;
mod hooks;
mod hybrid;
mod json;
mod mcp;
mod mcp_client;
mod mcp_stdio;
mod oauth;
mod permissions;
mod prompt;
mod remote;
mod rlm;
pub mod sandbox;
mod session;
mod sse;
mod trace;
mod usage;
mod ux;

pub use bash::{execute_bash, BashCommandInput, BashCommandOutput};
pub use bootstrap::{BootstrapPhase, BootstrapPlan};
pub use budget::{BudgetSliceRequest, BudgetStopReason, RuntimeBudget, RuntimeBudgetUsage};
pub use compact::{
    compact_session, estimate_session_tokens, format_compact_summary,
    get_compact_continuation_message, should_compact, CompactionConfig, CompactionResult,
};
pub use config::{
    ConfigEntry, ConfigError, ConfigLoader, ConfigSource, McpConfigCollection,
    McpManagedProxyServerConfig, McpOAuthConfig, McpRemoteServerConfig, McpSdkServerConfig,
    McpServerConfig, McpStdioServerConfig, McpTransport, McpWebSocketServerConfig, OAuthConfig,
    ResolvedPermissionMode, RuntimeConfig, RuntimeFeatureConfig, RuntimeHookConfig,
    RuntimePermissionRuleConfig, RuntimePluginConfig, RuntimeRagConfig, RuntimeRlmConfig,
    RuntimeWebResearchConfig, RuntimeWebResearchMode, ScopedMcpServerConfig,
    CLAW_SETTINGS_SCHEMA_NAME,
};
pub use conversation::{
    auto_compaction_threshold_from_env, ApiClient, ApiRequest, AssistantEvent, AutoCompactionEvent,
    ConversationRuntime, PromptCacheEvent, RuntimeError, StaticToolExecutor, ToolError,
    ToolExecutor, TurnSummary,
};
pub use corpus::{
    attach_corpus, default_corpus_store_dir, inspect_corpus, list_corpora, load_corpus,
    search_corpus, search_corpus_manifest, slice_corpus, CorpusAttachOptions, CorpusBackend,
    CorpusChunk, CorpusDocument, CorpusDocumentSummary, CorpusError, CorpusInspectResult,
    CorpusKind, CorpusManifest, CorpusRootSummary, CorpusSkipSummary, CorpusSlice, RetrievalHit,
    RetrievalResult,
};
pub use file_ops::{
    edit_file, glob_search, grep_search, read_file, write_file, EditFileOutput, GlobSearchOutput,
    GrepSearchInput, GrepSearchOutput, ReadFileOutput, StructuredPatchHunk, TextFilePayload,
    WriteFileOutput,
};
pub use hooks::{
    HookAbortSignal, HookEvent, HookProgressEvent, HookProgressReporter, HookRunResult, HookRunner,
};
pub use hybrid::{
    evaluate_web_escalation, format_citations, is_local_evidence_weak, local_evidence_trace_event,
    normalize_local_evidence, summarize_local_evidence, web_evidence_trace_event,
    EscalationHeuristicInput, EscalationOutcome, EscalationReason, EvidenceKind, EvidenceRecord,
    HybridExecutionContext, LocalEvidenceSummary, WebAccessDecision, WebAccessMode,
    WebEvidenceInput, WebExecutionOutcome, WebExecutionStatus, WebPolicy,
};
pub use mcp::{
    mcp_server_signature, mcp_tool_name, mcp_tool_prefix, normalize_name_for_mcp,
    scoped_mcp_config_hash, unwrap_ccr_proxy_url,
};
pub use mcp_client::{
    McpClientAuth, McpClientBootstrap, McpClientTransport, McpManagedProxyTransport,
    McpRemoteTransport, McpSdkTransport, McpStdioTransport,
};
pub use mcp_stdio::{
    spawn_mcp_stdio_process, JsonRpcError, JsonRpcId, JsonRpcRequest, JsonRpcResponse,
    ManagedMcpTool, McpInitializeClientInfo, McpInitializeParams, McpInitializeResult,
    McpInitializeServerInfo, McpListResourcesParams, McpListResourcesResult, McpListToolsParams,
    McpListToolsResult, McpReadResourceParams, McpReadResourceResult, McpResource,
    McpResourceContents, McpServerManager, McpServerManagerError, McpStdioProcess, McpTool,
    McpToolCallContent, McpToolCallParams, McpToolCallResult, UnsupportedMcpServer,
};
pub use oauth::{
    clear_oauth_credentials, code_challenge_s256, credentials_path, generate_pkce_pair,
    generate_state, load_oauth_credentials, loopback_redirect_uri, parse_oauth_callback_query,
    parse_oauth_callback_request_target, save_oauth_credentials, OAuthAuthorizationRequest,
    OAuthCallbackParams, OAuthRefreshRequest, OAuthTokenExchangeRequest, OAuthTokenSet,
    PkceChallengeMethod, PkceCodePair,
};
pub use permissions::{
    PermissionContext, PermissionMode, PermissionOutcome, PermissionOverride, PermissionPolicy,
    PermissionPromptDecision, PermissionPrompter, PermissionRequest,
};
pub use prompt::{
    load_system_prompt, prepend_bullets, ContextFile, ProjectContext, PromptBuildError,
    SystemPromptBuilder, FRONTIER_MODEL_NAME, SYSTEM_PROMPT_DYNAMIC_BOUNDARY,
};
pub use remote::{
    inherited_upstream_proxy_env, no_proxy_list, read_token, upstream_proxy_ws_url,
    RemoteSessionContext, UpstreamProxyBootstrap, UpstreamProxyState, DEFAULT_REMOTE_BASE_URL,
    DEFAULT_SESSION_TOKEN_PATH, DEFAULT_SYSTEM_CA_BUNDLE, NO_PROXY_HOSTS, UPSTREAM_PROXY_ENV_KEYS,
};
pub use rlm::{
    export_trace, render_trace_summary, ChildExecutionFallbackFormatter,
    ChildExecutionFallbackRenderer, ChildOutputAggregator, ChildSubqueryExecutor,
    ChildSubqueryOutput, ChildSubqueryRequest, DefaultChildOutputAggregator,
    FallbackChildSubqueryExecutor, RecursiveContextSlice, RecursiveConversationRuntime,
    RecursiveCorpusPeekResult, RecursiveExecutionMode, RecursiveExecutionResult,
    RecursiveIterationState, RecursiveRuntimeError, RecursiveRuntimeState, RecursiveStopReason,
};
pub use sandbox::{
    build_linux_sandbox_command, detect_container_environment, detect_container_environment_from,
    resolve_sandbox_status, resolve_sandbox_status_for_request, ContainerEnvironment,
    FilesystemIsolationMode, LinuxSandboxCommand, SandboxConfig, SandboxDetectionInputs,
    SandboxRequest, SandboxStatus,
};
pub use session::{
    ContentBlock, ConversationMessage, MessageRole, Session, SessionCompaction, SessionError,
    SessionFork,
};
pub use sse::{IncrementalSseParser, SseEvent};
pub use trace::{
    TraceCounters, TraceError, TraceEvent, TraceEventType, TraceFinalStatus, TraceLedger,
};
pub use usage::{
    format_usd, pricing_for_model, ModelPricing, TokenUsage, UsageCostEstimate, UsageTracker,
};
pub use ux::{
    Citation, ConfidenceLevel, ConfidenceNote, EvidenceProvenance, ExecutionProfile,
    ExecutionProfileConfig, ExecutionProfileParseError, FinalAnswer,
};

#[cfg(test)]
pub(crate) fn test_env_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    LOCK.get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
