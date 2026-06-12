//! Tokio-backed execution substrate for agent providers, tools, and sub-agents.

mod cleanup;
mod context;
mod events;
mod handle;
mod receipt;
mod task;
mod traits;

pub use context::{RuntimeContext, RuntimeExecutionIdentity};
pub use events::{EventStream, RuntimeEventSink, RuntimeEventStream};
pub use handle::TokioAgentRuntime;
pub use marlin_agent_protocol::{
    AgentEvent as RuntimeEvent, GraphId, RunId, RuntimeEnvironment, SubAgentConfigSurface,
    SubAgentContextPolicy, SubAgentContextVisibility, SubAgentPerformanceBudget,
    SubAgentPermissionSet, SubAgentSpawnConfig, SubAgentSpawnPolicy, SubAgentSpawnProfile,
    SubAgentSpawnStrategy,
};
pub use marlin_agent_sessions::{
    AgentSessionContext, ContextExpansionPolicy, ContextNamespace, ContextVisibility, SessionId,
    SessionIdError, SessionIdentity, SessionIsolationPolicy, SessionIsolationReceipt, SessionKind,
};
pub use receipt::SubAgentSpawnReceipt;
pub use task::{RuntimeTask, RuntimeTaskOutcome};
pub use tokio_util::sync::CancellationToken;
pub use traits::{HookRuntime, ProviderRuntime, RuntimeFuture, SubAgentRuntime, ToolRuntime};
