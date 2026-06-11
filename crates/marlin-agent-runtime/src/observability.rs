//! Stable tracing names for agent-core runtime observability.

/// Tracing span name for runtime-owned task execution.
pub const SPAN_RUNTIME_TASK: &str = "runtime.task";
/// Tracing span name for provider runtime calls.
pub const SPAN_RUNTIME_PROVIDER: &str = "runtime.provider";
/// Tracing span name for tool runtime calls.
pub const SPAN_RUNTIME_TOOL: &str = "runtime.tool";
/// Tracing span name for sub-agent runtime calls.
pub const SPAN_RUNTIME_SUB_AGENT: &str = "runtime.sub_agent";
/// Tracing span name for hook runtime calls.
pub const SPAN_RUNTIME_HOOK: &str = "runtime.hook";

/// Runtime task kind used when no narrower task category is available.
pub const RUNTIME_KIND_GENERIC: &str = "generic";
/// Runtime task kind for cancellable task execution.
pub const RUNTIME_KIND_CANCELLABLE: &str = "cancellable";
/// Runtime task kind for blocking task execution.
pub const RUNTIME_KIND_BLOCKING: &str = "blocking";
/// Runtime task kind for provider execution.
pub const RUNTIME_KIND_PROVIDER: &str = "provider";
/// Runtime task kind for tool execution.
pub const RUNTIME_KIND_TOOL: &str = "tool";
/// Runtime task kind for sub-agent execution.
pub const RUNTIME_KIND_SUB_AGENT: &str = "sub_agent";
/// Runtime task kind for hook execution.
pub const RUNTIME_KIND_HOOK: &str = "hook";

/// Create a `runtime.task` tracing span with the supplied runtime kind.
pub fn runtime_task_span(runtime_kind: &'static str) -> tracing::Span {
    tracing::info_span!("runtime.task", runtime_kind)
}

/// Create a `runtime.provider` tracing span.
pub fn runtime_provider_span() -> tracing::Span {
    tracing::info_span!("runtime.provider", runtime_kind = RUNTIME_KIND_PROVIDER)
}

/// Create a `runtime.tool` tracing span.
pub fn runtime_tool_span() -> tracing::Span {
    tracing::info_span!("runtime.tool", runtime_kind = RUNTIME_KIND_TOOL)
}

/// Create a `runtime.sub_agent` tracing span.
pub fn runtime_sub_agent_span() -> tracing::Span {
    tracing::info_span!("runtime.sub_agent", runtime_kind = RUNTIME_KIND_SUB_AGENT)
}

/// Create a `runtime.hook` tracing span.
pub fn runtime_hook_span() -> tracing::Span {
    tracing::info_span!("runtime.hook", runtime_kind = RUNTIME_KIND_HOOK)
}
