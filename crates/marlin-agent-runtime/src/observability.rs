//! Stable tracing names for agent-core runtime observability.

use std::fmt;

use marlin_agent_protocol::{AgentEvent, AgentEventTopic, AgentSpanName, ExecutorName, NodeId};

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
/// Tracing span name for kernel provider node execution.
pub const SPAN_AGENT_PROVIDER: &str = "agent.provider";
/// Tracing span name for kernel tool node execution.
pub const SPAN_AGENT_TOOL: &str = "agent.tool";
/// Tracing span name for kernel sub-agent node execution.
pub const SPAN_AGENT_SUB_AGENT: &str = "agent.sub_agent";
/// Tracing span name for hook dispatch batches.
pub const SPAN_HOOK_DISPATCH: &str = "hook.dispatch";
/// Tracing span name for one hook handler run.
pub const SPAN_HOOK_RUN: &str = "hook.run";
/// Tracing span name for one harness-managed scenario execution.
pub const SPAN_HARNESS_EXECUTION: &str = "harness.execution";
/// Tracing span name for one completed harness execution report.
pub const SPAN_HARNESS_RESULT: &str = "harness.result";

/// Runtime event topic for graph-loop execution lifecycle messages.
pub const TOPIC_KERNEL_EXECUTION: &str = "kernel.execution";
/// Runtime event topic for graph-loop node lifecycle messages.
pub const TOPIC_KERNEL_NODE: &str = "kernel.node";
/// Runtime event topic for hook dispatch receipts emitted by the kernel.
pub const TOPIC_KERNEL_HOOK: &str = "kernel.hook";
/// Runtime event topic for sub-agent lifecycle activity emitted by the kernel.
pub const TOPIC_KERNEL_SUB_AGENT: &str = "kernel.sub_agent";

/// Tracing field key for runtime task kind.
pub const FIELD_RUNTIME_KIND: &str = "runtime_kind";
/// Tracing field key for graph node kind.
pub const FIELD_NODE_KIND: &str = "node_kind";
/// Tracing field key for graph node identifier.
pub const FIELD_NODE_ID: &str = "node_id";
/// Tracing field key for graph node executor name.
pub const FIELD_EXECUTOR: &str = "executor";
/// Tracing field key for hook event names.
pub const FIELD_HOOK_EVENT: &str = "hook_event";
/// Tracing field key for hook batch size.
pub const FIELD_HOOK_COUNT: &str = "hook_count";
/// Tracing field key for hook registration identifiers.
pub const FIELD_HOOK_ID: &str = "hook_id";
/// Tracing field key for hook execution mode.
pub const FIELD_HOOK_MODE: &str = "hook_mode";
/// Tracing field key for hook handler type.
pub const FIELD_HOOK_HANDLER: &str = "hook_handler";
/// Tracing field key for harness scenario identifiers.
pub const FIELD_SCENARIO_ID: &str = "scenario_id";
/// Tracing field key for graph-loop run identifiers.
pub const FIELD_RUN_ID: &str = "run_id";
/// Tracing field key for a parent graph-loop run identifier.
pub const FIELD_PARENT_RUN_ID: &str = "parent_run_id";
/// Tracing field key for a child graph-loop run identifier.
pub const FIELD_CHILD_RUN_ID: &str = "child_run_id";
/// Tracing field key for graph identifiers.
pub const FIELD_GRAPH_ID: &str = "graph_id";
/// Tracing field key for a sub-agent source classifier.
pub const FIELD_SUB_AGENT_SOURCE: &str = "sub_agent_source";
/// Tracing field key for a sub-agent reference.
pub const FIELD_AGENT_REFERENCE: &str = "agent_reference";
/// Tracing field key for execution status.
pub const FIELD_STATUS: &str = "status";
/// Tracing field key for elapsed execution milliseconds.
pub const FIELD_DURATION_MS: &str = "duration_ms";
/// Tracing field key for diagnostic count.
pub const FIELD_DIAGNOSTIC_COUNT: &str = "diagnostic_count";
/// Tracing field key for runtime event count.
pub const FIELD_EVENT_COUNT: &str = "event_count";

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
/// Graph node kind for provider-backed nodes.
pub const NODE_KIND_PROVIDER: &str = "provider";
/// Graph node kind for tool-backed nodes.
pub const NODE_KIND_TOOL: &str = "tool";
/// Graph node kind for sub-agent-backed nodes.
pub const NODE_KIND_SUB_AGENT: &str = "sub_agent";
/// Sub-agent source value for kernel-owned sub-agent graph nodes.
pub const SUB_AGENT_SOURCE_KERNEL_NODE: &str = "kernel.sub-agent-node";
/// Sub-agent source value used when no narrower source is available.
pub const SUB_AGENT_SOURCE_UNSPECIFIED: &str = "unspecified";

/// Runtime-owned identifier for a registered hook handler.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HookRegistrationId<'a> {
    value: &'a str,
}

impl<'a> HookRegistrationId<'a> {
    /// Borrows a hook registration identifier for tracing.
    pub fn borrowed(value: &'a str) -> Self {
        Self { value }
    }

    /// Returns the hook registration identifier as text.
    pub fn as_str(self) -> &'a str {
        self.value
    }
}

/// Create a typed `runtime.task` span name.
pub fn runtime_task_span_name() -> AgentSpanName {
    AgentSpanName::new(SPAN_RUNTIME_TASK)
}

/// Create a typed `runtime.provider` span name.
pub fn runtime_provider_span_name() -> AgentSpanName {
    AgentSpanName::new(SPAN_RUNTIME_PROVIDER)
}

/// Create a typed `runtime.tool` span name.
pub fn runtime_tool_span_name() -> AgentSpanName {
    AgentSpanName::new(SPAN_RUNTIME_TOOL)
}

/// Create a typed `runtime.sub_agent` span name.
pub fn runtime_sub_agent_span_name() -> AgentSpanName {
    AgentSpanName::new(SPAN_RUNTIME_SUB_AGENT)
}

/// Create a typed `runtime.hook` span name.
pub fn runtime_hook_span_name() -> AgentSpanName {
    AgentSpanName::new(SPAN_RUNTIME_HOOK)
}

/// Create a typed `agent.provider` span name.
pub fn agent_provider_span_name() -> AgentSpanName {
    AgentSpanName::new(SPAN_AGENT_PROVIDER)
}

/// Create a typed `agent.tool` span name.
pub fn agent_tool_span_name() -> AgentSpanName {
    AgentSpanName::new(SPAN_AGENT_TOOL)
}

/// Create a typed `agent.sub_agent` span name.
pub fn agent_sub_agent_span_name() -> AgentSpanName {
    AgentSpanName::new(SPAN_AGENT_SUB_AGENT)
}

/// Create a typed `hook.dispatch` span name.
pub fn hook_dispatch_span_name() -> AgentSpanName {
    AgentSpanName::new(SPAN_HOOK_DISPATCH)
}

/// Create a typed `hook.run` span name.
pub fn hook_run_span_name() -> AgentSpanName {
    AgentSpanName::new(SPAN_HOOK_RUN)
}

/// Create a typed `harness.execution` span name.
pub fn harness_execution_span_name() -> AgentSpanName {
    AgentSpanName::new(SPAN_HARNESS_EXECUTION)
}

/// Create a typed `harness.result` span name.
pub fn harness_result_span_name() -> AgentSpanName {
    AgentSpanName::new(SPAN_HARNESS_RESULT)
}

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

/// Create an `agent.provider` tracing span for a kernel provider node.
pub fn agent_provider_span(node_id: &NodeId, executor: &ExecutorName) -> tracing::Span {
    tracing::info_span!(
        "agent.provider",
        node_kind = NODE_KIND_PROVIDER,
        node_id = node_id.as_str(),
        executor = executor.as_str()
    )
}

/// Create an `agent.tool` tracing span for a kernel tool node.
pub fn agent_tool_span(node_id: &NodeId, executor: &ExecutorName) -> tracing::Span {
    tracing::info_span!(
        "agent.tool",
        node_kind = NODE_KIND_TOOL,
        node_id = node_id.as_str(),
        executor = executor.as_str()
    )
}

/// Create an `agent.sub_agent` tracing span for a kernel sub-agent node.
pub fn agent_sub_agent_span(node_id: &NodeId, executor: &ExecutorName) -> tracing::Span {
    agent_sub_agent_span_with_source(
        node_id,
        executor,
        SUB_AGENT_SOURCE_UNSPECIFIED,
        executor.as_str(),
    )
}

/// Create an `agent.sub_agent` tracing span with sub-agent correlation fields.
pub fn agent_sub_agent_span_with_source(
    node_id: &NodeId,
    executor: &ExecutorName,
    sub_agent_source: &str,
    agent_reference: &str,
) -> tracing::Span {
    tracing::info_span!(
        "agent.sub_agent",
        node_kind = NODE_KIND_SUB_AGENT,
        node_id = node_id.as_str(),
        executor = executor.as_str(),
        sub_agent_source,
        agent_reference
    )
}

/// Create a `hook.dispatch` tracing span for a batch of hook handlers.
pub fn hook_dispatch_span(hook_event: impl fmt::Debug, hook_count: usize) -> tracing::Span {
    tracing::info_span!("hook.dispatch", hook_event = ?hook_event, hook_count)
}

/// Create a `hook.run` tracing span for one registered hook handler.
pub fn hook_run_span(
    hook_id: HookRegistrationId<'_>,
    hook_event: impl fmt::Debug,
    hook_mode: impl fmt::Debug,
    hook_handler: impl fmt::Debug,
) -> tracing::Span {
    tracing::info_span!(
        "hook.run",
        hook_id = hook_id.as_str(),
        hook_event = ?hook_event,
        hook_mode = ?hook_mode,
        hook_handler = ?hook_handler
    )
}

/// Create a typed `kernel.execution` runtime event topic.
pub fn kernel_execution_topic() -> AgentEventTopic {
    AgentEventTopic::new(TOPIC_KERNEL_EXECUTION)
}

/// Create a typed `kernel.node` runtime event topic.
pub fn kernel_node_topic() -> AgentEventTopic {
    AgentEventTopic::new(TOPIC_KERNEL_NODE)
}

/// Create a typed `kernel.hook` runtime event topic.
pub fn kernel_hook_topic() -> AgentEventTopic {
    AgentEventTopic::new(TOPIC_KERNEL_HOOK)
}

/// Create a typed `kernel.sub_agent` runtime event topic.
pub fn kernel_sub_agent_topic() -> AgentEventTopic {
    AgentEventTopic::new(TOPIC_KERNEL_SUB_AGENT)
}

/// Create a runtime event using a stable observability topic.
pub fn runtime_event(topic: AgentEventTopic, message: impl Into<String>) -> AgentEvent {
    AgentEvent::new(topic, message)
}

/// Create a `kernel.execution` runtime event.
pub fn kernel_execution_event(message: impl Into<String>) -> AgentEvent {
    runtime_event(kernel_execution_topic(), message)
}

/// Create a `kernel.node` runtime event.
pub fn kernel_node_event(message: impl Into<String>) -> AgentEvent {
    runtime_event(kernel_node_topic(), message)
}

/// Create a `kernel.hook` runtime event.
pub fn kernel_hook_event(message: impl Into<String>) -> AgentEvent {
    runtime_event(kernel_hook_topic(), message)
}

/// Create a `kernel.sub_agent` runtime event.
pub fn kernel_sub_agent_event(message: impl Into<String>) -> AgentEvent {
    runtime_event(kernel_sub_agent_topic(), message)
}
