//! Runtime context and execution identity ownership.

use crate::observability;
use tokio::sync::mpsc;

use super::{
    AgentSessionContext, CancellationToken, ContextNamespace, ContextVisibility, GraphId, RunId,
    RuntimeEnvironment, RuntimeEvent, RuntimeEventSink, SessionId, SessionIsolationReceipt,
    SessionKind, SubAgentContextPolicy, SubAgentContextVisibility, WorkingCopyIsolationReceipt,
};

/// Per-call runtime context passed into provider, tool, and sub-agent work.
#[derive(Clone, Debug)]
pub struct RuntimeContext {
    pub(super) cancellation: CancellationToken,
    pub(super) events: RuntimeEventSink,
    pub(super) environment: RuntimeEnvironment,
    pub(super) execution: Option<RuntimeExecutionIdentity>,
    pub(super) session: AgentSessionContext,
    pub(super) working_copy_receipts: Vec<WorkingCopyIsolationReceipt>,
    pub(super) process_registry: observability::RuntimeProcessRegistryHandle,
    pub(super) process_cleanup_policy: observability::RuntimeProcessCleanupPolicy,
}

impl RuntimeContext {
    pub fn cancellation_token(&self) -> CancellationToken {
        self.cancellation.clone()
    }

    pub fn event_sink(&self) -> RuntimeEventSink {
        self.events.clone()
    }

    pub fn environment(&self) -> &RuntimeEnvironment {
        &self.environment
    }

    pub fn session(&self) -> &AgentSessionContext {
        &self.session
    }

    pub fn working_copy_receipts(&self) -> &[WorkingCopyIsolationReceipt] {
        &self.working_copy_receipts
    }

    pub fn execution_identity(&self) -> Option<&RuntimeExecutionIdentity> {
        self.execution.as_ref()
    }

    pub fn process_registry(&self) -> observability::RuntimeProcessRegistryHandle {
        self.process_registry.clone()
    }

    pub fn process_cleanup_policy(&self) -> &observability::RuntimeProcessCleanupPolicy {
        &self.process_cleanup_policy
    }

    pub fn with_execution_identity(mut self, execution: RuntimeExecutionIdentity) -> Self {
        self.execution = Some(execution);
        self
    }

    pub fn with_session_context(mut self, session: AgentSessionContext) -> Self {
        self.session = session;
        self
    }

    pub fn with_working_copy_receipt(mut self, receipt: WorkingCopyIsolationReceipt) -> Self {
        self.working_copy_receipts.push(receipt);
        self
    }

    pub fn child_context(&self) -> Self {
        Self {
            cancellation: self.cancellation.child_token(),
            events: self.events.clone(),
            environment: self.environment.clone(),
            execution: self.execution.clone(),
            session: self.session.clone(),
            working_copy_receipts: self.working_copy_receipts.clone(),
            process_registry: self.process_registry.clone(),
            process_cleanup_policy: self.process_cleanup_policy.clone(),
        }
    }

    pub fn child_context_with_environment(&self, environment: RuntimeEnvironment) -> Self {
        Self {
            cancellation: self.cancellation.child_token(),
            events: self.events.clone(),
            environment,
            execution: self.execution.clone(),
            session: self.session.clone(),
            working_copy_receipts: self.working_copy_receipts.clone(),
            process_registry: self.process_registry.clone(),
            process_cleanup_policy: self.process_cleanup_policy.clone(),
        }
    }

    pub fn child_context_for_session(
        &self,
        kind: SessionKind,
        child_session_id: impl Into<SessionId>,
        requested_visibility: ContextVisibility,
    ) -> (Self, SessionIsolationReceipt) {
        let (session, receipt) =
            self.session
                .child_session(kind, child_session_id, requested_visibility);
        (
            Self {
                cancellation: self.cancellation.child_token(),
                events: self.events.clone(),
                environment: self.environment.clone(),
                execution: self.execution.clone(),
                session,
                working_copy_receipts: self.working_copy_receipts.clone(),
                process_registry: self.process_registry.clone(),
                process_cleanup_policy: self.process_cleanup_policy.clone(),
            },
            receipt,
        )
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancellation.is_cancelled()
    }

    pub async fn emit(
        &self,
        event: RuntimeEvent,
    ) -> Result<(), mpsc::error::SendError<RuntimeEvent>> {
        self.events.emit(event).await
    }
}

/// Graph-loop execution identity propagated through runtime child contexts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeExecutionIdentity {
    run_id: RunId,
    graph_id: GraphId,
}

impl RuntimeExecutionIdentity {
    pub fn new(run_id: impl Into<String>, graph_id: impl Into<String>) -> Self {
        Self {
            run_id: RunId::new(run_id),
            graph_id: GraphId::new(graph_id),
        }
    }

    pub fn from_parts(run_id: RunId, graph_id: GraphId) -> Self {
        Self { run_id, graph_id }
    }

    pub fn run_id(&self) -> &str {
        self.run_id.as_str()
    }

    pub fn graph_id(&self) -> &str {
        self.graph_id.as_str()
    }
}

pub(super) fn context_visibility_from_sub_agent_policy(
    policy: &SubAgentContextPolicy,
) -> ContextVisibility {
    ContextVisibility::from_namespaces(
        policy
            .visibility
            .iter()
            .map(runtime_context_namespace_from_protocol),
    )
    .with_max_history_items(policy.max_history_items)
}

fn runtime_context_namespace_from_protocol(
    visibility: &SubAgentContextVisibility,
) -> ContextNamespace {
    match visibility {
        SubAgentContextVisibility::System => ContextNamespace::System,
        SubAgentContextVisibility::User => ContextNamespace::User,
        SubAgentContextVisibility::Workspace => ContextNamespace::Workspace,
        SubAgentContextVisibility::Memory => ContextNamespace::Memory,
    }
}
