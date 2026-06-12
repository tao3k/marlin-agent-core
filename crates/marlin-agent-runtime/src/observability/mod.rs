//! Runtime observability interface.

pub mod process;
mod spans;

pub use process::{
    AsyncManagedChildProcess, ManagedChildProcessSpec, RuntimeCommandCleanupEntry,
    RuntimeCommandCleanupOutcome, RuntimeCommandCleanupReceipt, RuntimeCommandKind,
    RuntimeCommandObservation, RuntimeProcessCleanupController, RuntimeProcessCleanupFailure,
    RuntimeProcessCleanupPolicy, RuntimeProcessCleanupSweep, RuntimeProcessExitStatus,
    RuntimeProcessHandle, RuntimeProcessKind, RuntimeProcessKindCounts, RuntimeProcessLiveness,
    RuntimeProcessObservation, RuntimeProcessObservationTimestampMs, RuntimeProcessOutput,
    RuntimeProcessRegistrationError, RuntimeProcessRegistry, RuntimeProcessRegistryHandle,
    RuntimeProcessRegistrySnapshot, RuntimeProcessStatus, RuntimeProcessStatusCounts,
    RuntimeProcessTerminator, SysinfoRuntimeProcessController,
};
pub use spans::{
    FIELD_AGENT_REFERENCE, FIELD_CHILD_RUN_ID, FIELD_CLEANUP_ATTEMPTS,
    FIELD_CLEANUP_CANDIDATE_COUNT, FIELD_CLEANUP_OUTCOME, FIELD_DIAGNOSTIC_COUNT,
    FIELD_DURATION_MS, FIELD_EVENT_COUNT, FIELD_EXECUTOR, FIELD_FORCE_CLEANUP_RECOMMENDED,
    FIELD_GRAPH_ID, FIELD_HOOK_COUNT, FIELD_HOOK_EVENT, FIELD_HOOK_HANDLER, FIELD_HOOK_ID,
    FIELD_HOOK_MODE, FIELD_LAST_CLEANUP_ATTEMPT_AT_MS, FIELD_NODE_ID, FIELD_NODE_KIND,
    FIELD_OBSERVED_AT_MS, FIELD_OWNER_REFERENCE, FIELD_PARENT_RUN_ID, FIELD_PROCESS_EVENT,
    FIELD_PROCESS_HANDLE, FIELD_PROCESS_ID, FIELD_PROCESS_KIND, FIELD_PROCESS_STATUS,
    FIELD_REMOVED_STALE_COUNT, FIELD_REQUIRES_FOLLOW_UP, FIELD_RETAINED_IN_REGISTRY, FIELD_RUN_ID,
    FIELD_RUNTIME_KIND, FIELD_SCENARIO_ID, FIELD_STATUS, FIELD_SUB_AGENT_SOURCE,
    FIELD_TERMINATION_FAILED_COUNT, FIELD_TERMINATION_REQUESTED_COUNT, HookRegistrationId,
    NODE_KIND_PROVIDER, NODE_KIND_SUB_AGENT, NODE_KIND_TOOL, PROCESS_EVENT_CLEANUP_REQUESTED,
    PROCESS_EVENT_CLEANUP_SWEEP, PROCESS_EVENT_FAILED, PROCESS_EVENT_FINISHED,
    PROCESS_EVENT_ORPHANED, PROCESS_EVENT_REMOVED_STALE, PROCESS_EVENT_TERMINATION_FAILED,
    PROCESS_EVENT_TERMINATION_REQUESTED, PROCESS_EVENT_TRACKED, RUNTIME_KIND_BLOCKING,
    RUNTIME_KIND_CANCELLABLE, RUNTIME_KIND_GENERIC, RUNTIME_KIND_HOOK, RUNTIME_KIND_PROVIDER,
    RUNTIME_KIND_SUB_AGENT, RUNTIME_KIND_TOOL, SPAN_AGENT_PROVIDER, SPAN_AGENT_SUB_AGENT,
    SPAN_AGENT_TOOL, SPAN_HARNESS_EXECUTION, SPAN_HARNESS_RESULT, SPAN_HOOK_DISPATCH,
    SPAN_HOOK_RUN, SPAN_RUNTIME_HOOK, SPAN_RUNTIME_PROVIDER, SPAN_RUNTIME_SUB_AGENT,
    SPAN_RUNTIME_TASK, SPAN_RUNTIME_TOOL, SUB_AGENT_SOURCE_KERNEL_NODE,
    SUB_AGENT_SOURCE_UNSPECIFIED, TARGET_RUNTIME_PROCESS, TOPIC_KERNEL_EXECUTION,
    TOPIC_KERNEL_HOOK, TOPIC_KERNEL_NODE, TOPIC_KERNEL_SUB_AGENT, agent_provider_span,
    agent_provider_span_name, agent_sub_agent_span, agent_sub_agent_span_name,
    agent_sub_agent_span_with_source, agent_tool_span, agent_tool_span_name,
    harness_execution_span_name, harness_result_span_name, hook_dispatch_span,
    hook_dispatch_span_name, hook_run_span, hook_run_span_name, kernel_execution_event,
    kernel_execution_topic, kernel_hook_event, kernel_hook_topic, kernel_node_event,
    kernel_node_topic, kernel_sub_agent_event, kernel_sub_agent_topic, runtime_event,
    runtime_hook_span, runtime_hook_span_name, runtime_provider_span, runtime_provider_span_name,
    runtime_sub_agent_span, runtime_sub_agent_span_name, runtime_task_span, runtime_task_span_name,
    runtime_tool_span, runtime_tool_span_name,
};
