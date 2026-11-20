// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Tokio-backed execution substrate for agent providers, tools, and sub-agents.

mod cleanup;
mod context;
mod events;
mod fanout_item;
mod handle;
mod receipt;
mod task;
mod traits;

use context::context_visibility_from_sub_agent_policy;
pub use context::{RuntimeContext, RuntimeExecutionIdentity};
pub use events::{
    EventStream, MAX_RUNTIME_EVENT_HEAP_BYTES, RuntimeEventSendError, RuntimeEventSink,
    RuntimeEventSizeRejection, RuntimeEventStream,
};
pub use fanout_item::WorkingCopySubAgentFanoutItem;
pub use handle::{RuntimeTaskScope, TokioAgentRuntime, TokioAgentRuntimeBuildRequest};
pub use marlin_agent_protocol::{
    AgentEvent as RuntimeEvent, GraphId, RunId, RuntimeEnvironment, SubAgentConfigSurface,
    SubAgentContextPolicy, SubAgentContextVisibility, SubAgentPerformanceBudget,
    SubAgentPermissionSet, SubAgentSpawnConfig, SubAgentSpawnPolicy, SubAgentSpawnProfile,
    SubAgentSpawnStrategy,
};
pub use marlin_agent_sessions::{
    AgentSessionContext, ContextExpansionPolicy, ContextNamespace, ContextVisibility,
    RuntimeBlockingBridgePolicy, RuntimeBlockingBridgeStrategy, RuntimeFanoutJoinPolicy,
    RuntimeTaskTrackerPolicy, SessionId, SessionIdError, SessionIdentity, SessionIsolationPolicy,
    SessionIsolationReceipt, SessionKind, SessionRuntimeSnapshot, TokioRuntimeDiagnosticsPolicy,
    TokioRuntimeFlavor, TokioRuntimePolicy, TokioRuntimePolicyReceipt,
};
pub use receipt::{
    RuntimeFanoutOutput, RuntimeFanoutReceipt, RuntimeFanoutResult, RuntimeFanoutTaskReceipt,
    RuntimeFanoutTaskStatus, RuntimeTaskShutdownReceipt, RuntimeTaskShutdownReceiptInput,
    RuntimeTaskShutdownRequest, RuntimeTaskShutdownStatus, RuntimeTaskTrackerShutdownState,
    SubAgentSpawnReceipt,
};
pub(crate) use task::RuntimeTaskGuard;
pub use task::{
    RuntimeAdmissionError, RuntimeOperationId, RuntimeProcessBudget, RuntimeResourceGeneration,
    RuntimeResourceId, RuntimeTask, RuntimeTaskJoinError, RuntimeTaskOutcome, RuntimeTaskTracker,
};
pub use tokio_util::sync::CancellationToken;
pub use traits::{HookRuntime, ProviderRuntime, RuntimeFuture, SubAgentRuntime, ToolRuntime};
