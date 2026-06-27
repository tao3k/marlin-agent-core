//! Runtime handoff execution receipts, routers, and typed handlers.

use std::sync::Arc;

use marlin_agent_protocol::{
    AgentFlowIntent, AgentFlowMemoryIntent, AgentFlowReceipt, AgentFlowReceiptId,
    AgentFlowRuntimeHandoffId, AgentFlowSession, AgentFlowSessionTransform, AgentFlowTransformId,
    LoopProgramEventKind, LoopProgramId,
};
use marlin_agent_runtime::{
    AgentFlowLoopStepRequest, observability::RuntimeCommandObservation,
    project_agent_flow_loop_step,
};

use crate::{
    GenericLoopMachineStepIndex, LoopProgramRuntimeHandoff, LoopProgramRuntimeHandoffKind,
    LoopProgramRuntimeHandoffPlan,
};

use super::tool_process::LoopProgramToolProcessProjectionReceipt;

/// Stable owner identifier for a runtime handoff handler.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LoopProgramRuntimeOwner(String);

impl LoopProgramRuntimeOwner {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Execution status for one runtime handoff projection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LoopProgramRuntimeHandoffExecutionStatus {
    Handled,
    Deferred,
    Denied,
}

/// One typed runtime execution receipt for a loop-program handoff.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoopProgramRuntimeHandoffExecution {
    pub program_id: LoopProgramId,
    pub step_index: GenericLoopMachineStepIndex,
    pub kind: LoopProgramRuntimeHandoffKind,
    pub owner: LoopProgramRuntimeOwner,
    pub status: LoopProgramRuntimeHandoffExecutionStatus,
    pub agent_flow_intent: Option<AgentFlowIntent>,
    pub next_event: Option<LoopProgramEventKind>,
}

impl LoopProgramRuntimeHandoffExecution {
    pub fn handled(owner: LoopProgramRuntimeOwner, handoff: &LoopProgramRuntimeHandoff) -> Self {
        Self::from_handoff(
            owner,
            LoopProgramRuntimeHandoffExecutionStatus::Handled,
            handoff,
        )
    }

    pub fn deferred(owner: LoopProgramRuntimeOwner, handoff: &LoopProgramRuntimeHandoff) -> Self {
        Self::from_handoff(
            owner,
            LoopProgramRuntimeHandoffExecutionStatus::Deferred,
            handoff,
        )
    }

    pub fn denied(owner: LoopProgramRuntimeOwner, handoff: &LoopProgramRuntimeHandoff) -> Self {
        Self::from_handoff(
            owner,
            LoopProgramRuntimeHandoffExecutionStatus::Denied,
            handoff,
        )
    }

    fn from_handoff(
        owner: LoopProgramRuntimeOwner,
        status: LoopProgramRuntimeHandoffExecutionStatus,
        handoff: &LoopProgramRuntimeHandoff,
    ) -> Self {
        Self {
            program_id: handoff.receipt.program_id.clone(),
            step_index: handoff.receipt.step_index,
            kind: handoff.kind.clone(),
            owner,
            status,
            agent_flow_intent: handoff.agent_flow_intent.clone(),
            next_event: None,
        }
    }

    pub fn with_next_event(mut self, next_event: LoopProgramEventKind) -> Self {
        self.next_event = Some(next_event);
        self
    }
}

/// Memory projection derived from a handled Agent-Flow memory intent.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoopProgramMemoryProjectionReceipt {
    pub program_id: LoopProgramId,
    pub step_index: GenericLoopMachineStepIndex,
    pub owner: LoopProgramRuntimeOwner,
    pub intent: AgentFlowMemoryIntent,
}

impl LoopProgramMemoryProjectionReceipt {
    pub fn new(
        owner: LoopProgramRuntimeOwner,
        handoff: &LoopProgramRuntimeHandoff,
        intent: AgentFlowMemoryIntent,
    ) -> Self {
        Self {
            program_id: handoff.receipt.program_id.clone(),
            step_index: handoff.receipt.step_index,
            owner,
            intent,
        }
    }
}

/// Aggregate status for a runtime handoff execution receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LoopProgramRuntimeHandoffExecutionReportStatus {
    Empty,
    Completed,
    Deferred,
    Denied,
}

/// Typed execution receipt emitted after a handoff plan reaches runtime handlers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoopProgramRuntimeHandoffExecutionReceipt {
    pub program_id: LoopProgramId,
    pub status: LoopProgramRuntimeHandoffExecutionReportStatus,
    pub executions: Box<[LoopProgramRuntimeHandoffExecution]>,
    pub agent_flow_receipt: Option<AgentFlowReceipt>,
    pub tool_process_projections: Box<[LoopProgramToolProcessProjectionReceipt]>,
    pub memory_projections: Box<[LoopProgramMemoryProjectionReceipt]>,
}

impl LoopProgramRuntimeHandoffExecutionReceipt {
    pub fn new(
        program_id: LoopProgramId,
        executions: impl Into<Box<[LoopProgramRuntimeHandoffExecution]>>,
    ) -> Self {
        let executions = executions.into();
        let status = execution_report_status(&executions);
        Self {
            program_id,
            status,
            executions,
            agent_flow_receipt: None,
            tool_process_projections: Box::new([]),
            memory_projections: Box::new([]),
        }
    }

    pub fn with_agent_flow_receipt(mut self, receipt: AgentFlowReceipt) -> Self {
        self.agent_flow_receipt = Some(receipt);
        self
    }

    pub fn with_tool_process_projections(
        mut self,
        projections: impl Into<Box<[LoopProgramToolProcessProjectionReceipt]>>,
    ) -> Self {
        self.tool_process_projections = projections.into();
        self
    }

    pub fn with_memory_projections(
        mut self,
        projections: impl Into<Box<[LoopProgramMemoryProjectionReceipt]>>,
    ) -> Self {
        self.memory_projections = projections.into();
        self
    }
}

/// Runtime lane handler for one concrete handoff.
pub trait LoopProgramRuntimeHandoffHandler: Send + Sync + 'static {
    fn handle(&self, handoff: &LoopProgramRuntimeHandoff) -> LoopProgramRuntimeHandoffExecution;
}

/// Runtime executor for a full handoff plan.
pub trait LoopProgramRuntimeHandoffExecutor: Send + Sync + 'static {
    fn execute_plan(
        &self,
        plan: &LoopProgramRuntimeHandoffPlan,
    ) -> LoopProgramRuntimeHandoffExecutionReceipt;
}

/// Handler that records a typed defer receipt without taking ownership of runtime work.
#[derive(Clone, Debug)]
pub struct DeferredLoopProgramRuntimeHandoffHandler {
    owner: LoopProgramRuntimeOwner,
}

impl DeferredLoopProgramRuntimeHandoffHandler {
    pub fn new(owner: LoopProgramRuntimeOwner) -> Self {
        Self { owner }
    }
}

impl Default for DeferredLoopProgramRuntimeHandoffHandler {
    fn default() -> Self {
        Self::new(LoopProgramRuntimeOwner::new("kernel.loop-program.deferred"))
    }
}

impl LoopProgramRuntimeHandoffHandler for DeferredLoopProgramRuntimeHandoffHandler {
    fn handle(&self, handoff: &LoopProgramRuntimeHandoff) -> LoopProgramRuntimeHandoffExecution {
        LoopProgramRuntimeHandoffExecution::deferred(self.owner.clone(), handoff)
    }
}

/// Static receipt handler for adapters that already know their handoff outcome.
#[derive(Clone, Debug)]
pub struct StaticLoopProgramRuntimeHandoffHandler {
    owner: LoopProgramRuntimeOwner,
    status: LoopProgramRuntimeHandoffExecutionStatus,
    next_event: Option<LoopProgramEventKind>,
}

impl StaticLoopProgramRuntimeHandoffHandler {
    pub fn handled(owner: LoopProgramRuntimeOwner) -> Self {
        Self::new(owner, LoopProgramRuntimeHandoffExecutionStatus::Handled)
    }

    pub fn handled_with_next_event(
        owner: LoopProgramRuntimeOwner,
        next_event: LoopProgramEventKind,
    ) -> Self {
        Self::handled(owner).with_next_event(next_event)
    }

    pub fn deferred(owner: LoopProgramRuntimeOwner) -> Self {
        Self::new(owner, LoopProgramRuntimeHandoffExecutionStatus::Deferred)
    }

    pub fn denied(owner: LoopProgramRuntimeOwner) -> Self {
        Self::new(owner, LoopProgramRuntimeHandoffExecutionStatus::Denied)
    }

    fn new(
        owner: LoopProgramRuntimeOwner,
        status: LoopProgramRuntimeHandoffExecutionStatus,
    ) -> Self {
        Self {
            owner,
            status,
            next_event: None,
        }
    }

    pub fn with_next_event(mut self, next_event: LoopProgramEventKind) -> Self {
        self.next_event = Some(next_event);
        self
    }
}

impl LoopProgramRuntimeHandoffHandler for StaticLoopProgramRuntimeHandoffHandler {
    fn handle(&self, handoff: &LoopProgramRuntimeHandoff) -> LoopProgramRuntimeHandoffExecution {
        let execution = LoopProgramRuntimeHandoffExecution::from_handoff(
            self.owner.clone(),
            self.status.clone(),
            handoff,
        );
        match self.next_event.clone() {
            Some(next_event) => execution.with_next_event(next_event),
            None => execution,
        }
    }
}

/// Typed sandbox/policy handler that denies selected tool dispatch intents.
#[derive(Clone, Debug)]
pub struct DenylistedLoopProgramToolDispatchHandler {
    owner: LoopProgramRuntimeOwner,
    denylisted_tool_names: Box<[String]>,
}

impl DenylistedLoopProgramToolDispatchHandler {
    pub fn new<I, S>(owner: LoopProgramRuntimeOwner, denylisted_tool_names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            owner,
            denylisted_tool_names: denylisted_tool_names.into_iter().map(Into::into).collect(),
        }
    }

    fn denies(&self, handoff: &LoopProgramRuntimeHandoff) -> bool {
        let Some(AgentFlowIntent::Tool(tool_intent)) = &handoff.agent_flow_intent else {
            return false;
        };
        self.denylisted_tool_names
            .iter()
            .any(|tool_name| tool_name == tool_intent.tool_name.as_str())
    }
}

impl LoopProgramRuntimeHandoffHandler for DenylistedLoopProgramToolDispatchHandler {
    fn handle(&self, handoff: &LoopProgramRuntimeHandoff) -> LoopProgramRuntimeHandoffExecution {
        if self.denies(handoff) {
            LoopProgramRuntimeHandoffExecution::denied(self.owner.clone(), handoff)
        } else {
            LoopProgramRuntimeHandoffExecution::handled(self.owner.clone(), handoff)
        }
    }
}

/// Named handler slots for fine-grained runtime handoff policy injection.
#[derive(Clone)]
pub struct LoopProgramRuntimeHandoffRouterHandlers {
    pub control_handler: Arc<dyn LoopProgramRuntimeHandoffHandler>,
    pub model_handler: Arc<dyn LoopProgramRuntimeHandoffHandler>,
    pub tool_handler: Arc<dyn LoopProgramRuntimeHandoffHandler>,
    pub memory_handler: Arc<dyn LoopProgramRuntimeHandoffHandler>,
    pub placement_handler: Arc<dyn LoopProgramRuntimeHandoffHandler>,
    pub runtime_handler: Arc<dyn LoopProgramRuntimeHandoffHandler>,
    pub graph_handler: Arc<dyn LoopProgramRuntimeHandoffHandler>,
    pub session_handler: Arc<dyn LoopProgramRuntimeHandoffHandler>,
    pub agent_handler: Arc<dyn LoopProgramRuntimeHandoffHandler>,
    pub verification_handler: Arc<dyn LoopProgramRuntimeHandoffHandler>,
    pub human_gate_handler: Arc<dyn LoopProgramRuntimeHandoffHandler>,
    pub receipt_handler: Arc<dyn LoopProgramRuntimeHandoffHandler>,
}

impl Default for LoopProgramRuntimeHandoffRouterHandlers {
    fn default() -> Self {
        let deferred: Arc<dyn LoopProgramRuntimeHandoffHandler> =
            Arc::new(DeferredLoopProgramRuntimeHandoffHandler::default());
        Self {
            control_handler: Arc::clone(&deferred),
            model_handler: Arc::clone(&deferred),
            tool_handler: Arc::clone(&deferred),
            memory_handler: Arc::clone(&deferred),
            placement_handler: Arc::clone(&deferred),
            runtime_handler: Arc::clone(&deferred),
            graph_handler: Arc::clone(&deferred),
            session_handler: Arc::clone(&deferred),
            agent_handler: Arc::clone(&deferred),
            verification_handler: Arc::clone(&deferred),
            human_gate_handler: Arc::clone(&deferred),
            receipt_handler: Arc::clone(&deferred),
        }
    }
}

/// Fine-grained router from handoff kind to runtime handler.
#[derive(Clone, Default)]
pub struct LoopProgramRuntimeHandoffRouter {
    handlers: LoopProgramRuntimeHandoffRouterHandlers,
}

impl LoopProgramRuntimeHandoffRouter {
    pub fn new(handlers: LoopProgramRuntimeHandoffRouterHandlers) -> Self {
        Self { handlers }
    }

    fn handler_for_kind(
        &self,
        kind: &LoopProgramRuntimeHandoffKind,
    ) -> &dyn LoopProgramRuntimeHandoffHandler {
        match kind {
            LoopProgramRuntimeHandoffKind::Continue | LoopProgramRuntimeHandoffKind::Stop => {
                self.handlers.control_handler.as_ref()
            }
            LoopProgramRuntimeHandoffKind::ModelInvocation => self.handlers.model_handler.as_ref(),
            LoopProgramRuntimeHandoffKind::ToolDispatch => self.handlers.tool_handler.as_ref(),
            LoopProgramRuntimeHandoffKind::MemoryRecall
            | LoopProgramRuntimeHandoffKind::MemoryStore
            | LoopProgramRuntimeHandoffKind::ContextCompaction => {
                self.handlers.memory_handler.as_ref()
            }
            LoopProgramRuntimeHandoffKind::PlacementRequest => {
                self.handlers.placement_handler.as_ref()
            }
            LoopProgramRuntimeHandoffKind::RuntimeHandoff => self.handlers.runtime_handler.as_ref(),
            LoopProgramRuntimeHandoffKind::GraphRewrite => self.handlers.graph_handler.as_ref(),
            LoopProgramRuntimeHandoffKind::SessionFork => self.handlers.session_handler.as_ref(),
            LoopProgramRuntimeHandoffKind::AgentDelegation => self.handlers.agent_handler.as_ref(),
            LoopProgramRuntimeHandoffKind::Verification => {
                self.handlers.verification_handler.as_ref()
            }
            LoopProgramRuntimeHandoffKind::HumanGate => self.handlers.human_gate_handler.as_ref(),
            LoopProgramRuntimeHandoffKind::ReceiptEmission => {
                self.handlers.receipt_handler.as_ref()
            }
        }
    }
}

impl LoopProgramRuntimeHandoffExecutor for LoopProgramRuntimeHandoffRouter {
    fn execute_plan(
        &self,
        plan: &LoopProgramRuntimeHandoffPlan,
    ) -> LoopProgramRuntimeHandoffExecutionReceipt {
        LoopProgramRuntimeHandoffExecutionReceipt::new(
            plan.program_id.clone(),
            plan.handoffs
                .iter()
                .map(|handoff| self.handler_for_kind(&handoff.kind).handle(handoff))
                .collect::<Vec<_>>(),
        )
    }
}

/// Runtime executor that projects Agent-Flow handoff intents through the runtime substrate.
#[derive(Clone, Debug)]
pub struct AgentFlowLoopProgramRuntimeHandoffExecutor {
    owner: LoopProgramRuntimeOwner,
    deferred_owner: LoopProgramRuntimeOwner,
    admitted_at_ms: u64,
}

impl AgentFlowLoopProgramRuntimeHandoffExecutor {
    pub fn new(owner: LoopProgramRuntimeOwner) -> Self {
        Self {
            owner,
            deferred_owner: LoopProgramRuntimeOwner::new("kernel.loop-program.deferred"),
            admitted_at_ms: 0,
        }
    }

    pub fn with_admitted_at_ms(mut self, admitted_at_ms: u64) -> Self {
        self.admitted_at_ms = admitted_at_ms;
        self
    }

    pub fn execute_agent_flow_request(
        &self,
        request: LoopProgramAgentFlowRuntimeHandoffRequest,
    ) -> LoopProgramRuntimeHandoffExecutionReceipt {
        self.execute_agent_flow_plan(
            &request.plan,
            request.session,
            request.transform_id,
            request.handoff_id,
            request.receipt_id,
        )
    }

    fn execute_agent_flow_plan(
        &self,
        plan: &LoopProgramRuntimeHandoffPlan,
        session: AgentFlowSession,
        transform_id: AgentFlowTransformId,
        handoff_id: AgentFlowRuntimeHandoffId,
        receipt_id: AgentFlowReceiptId,
    ) -> LoopProgramRuntimeHandoffExecutionReceipt {
        let agent_flow_intents = plan.agent_flow_intents();
        if agent_flow_intents.is_empty() {
            return LoopProgramRuntimeHandoffRouter::default().execute_plan(plan);
        }

        let tool_process_projections = plan
            .handoffs
            .iter()
            .filter_map(|handoff| {
                let Some(AgentFlowIntent::Tool(tool_intent)) = &handoff.agent_flow_intent else {
                    return None;
                };
                let command = RuntimeCommandObservation::new("agent-flow.tool-intent")
                    .with_argv([tool_intent.tool_name.as_str()]);
                Some(LoopProgramToolProcessProjectionReceipt::new(
                    self.owner.clone(),
                    handoff,
                    command,
                ))
            })
            .collect::<Vec<_>>();
        let memory_projections = plan
            .handoffs
            .iter()
            .filter_map(|handoff| {
                let Some(AgentFlowIntent::Memory(memory_intent)) = &handoff.agent_flow_intent
                else {
                    return None;
                };
                Some(LoopProgramMemoryProjectionReceipt::new(
                    self.owner.clone(),
                    handoff,
                    memory_intent.clone(),
                ))
            })
            .collect::<Vec<_>>();

        let transform = AgentFlowSessionTransform::new(
            transform_id,
            session.session_id.clone(),
            agent_flow_intents,
        );
        let agent_flow_receipt = match project_agent_flow_loop_step(AgentFlowLoopStepRequest {
            session,
            transform,
            handoff_id,
            receipt_id,
            admitted_at_ms: self.admitted_at_ms,
        }) {
            Ok(receipt) => receipt,
            Err(_) => {
                return LoopProgramRuntimeHandoffExecutionReceipt::new(
                    plan.program_id.clone(),
                    plan.handoffs
                        .iter()
                        .map(|handoff| {
                            if handoff.agent_flow_intent.is_some() {
                                LoopProgramRuntimeHandoffExecution::denied(
                                    self.owner.clone(),
                                    handoff,
                                )
                            } else {
                                LoopProgramRuntimeHandoffExecution::deferred(
                                    self.deferred_owner.clone(),
                                    handoff,
                                )
                            }
                        })
                        .collect::<Vec<_>>(),
                );
            }
        };

        LoopProgramRuntimeHandoffExecutionReceipt::new(
            plan.program_id.clone(),
            plan.handoffs
                .iter()
                .map(|handoff| {
                    if handoff.agent_flow_intent.is_some() {
                        let execution = LoopProgramRuntimeHandoffExecution::handled(
                            self.owner.clone(),
                            handoff,
                        );
                        match agent_flow_next_event_for_handoff(handoff) {
                            Some(next_event) => execution.with_next_event(next_event),
                            None => execution,
                        }
                    } else {
                        LoopProgramRuntimeHandoffExecution::deferred(
                            self.deferred_owner.clone(),
                            handoff,
                        )
                    }
                })
                .collect::<Vec<_>>(),
        )
        .with_agent_flow_receipt(agent_flow_receipt)
        .with_tool_process_projections(tool_process_projections)
        .with_memory_projections(memory_projections)
    }
}

fn agent_flow_next_event_for_handoff(
    handoff: &LoopProgramRuntimeHandoff,
) -> Option<LoopProgramEventKind> {
    match handoff.kind {
        LoopProgramRuntimeHandoffKind::ToolDispatch => Some(LoopProgramEventKind::ToolReceipt),
        LoopProgramRuntimeHandoffKind::MemoryRecall
        | LoopProgramRuntimeHandoffKind::MemoryStore
        | LoopProgramRuntimeHandoffKind::PlacementRequest
        | LoopProgramRuntimeHandoffKind::SessionFork
        | LoopProgramRuntimeHandoffKind::AgentDelegation
        | LoopProgramRuntimeHandoffKind::ContextCompaction => {
            Some(LoopProgramEventKind::RuntimeReceipt)
        }
        _ => None,
    }
}

/// Explicit Agent-Flow request for projecting loop-program handoffs from a real source session.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoopProgramAgentFlowRuntimeHandoffRequest {
    pub plan: LoopProgramRuntimeHandoffPlan,
    pub session: AgentFlowSession,
    pub transform_id: AgentFlowTransformId,
    pub handoff_id: AgentFlowRuntimeHandoffId,
    pub receipt_id: AgentFlowReceiptId,
}

impl LoopProgramAgentFlowRuntimeHandoffRequest {
    pub fn for_session(plan: LoopProgramRuntimeHandoffPlan, session: AgentFlowSession) -> Self {
        let program_id = plan.program_id.as_str().to_owned();
        Self {
            plan,
            session,
            transform_id: AgentFlowTransformId::new(format!(
                "loop-program:{program_id}:agent-flow-transform"
            )),
            handoff_id: AgentFlowRuntimeHandoffId::new(format!(
                "loop-program:{program_id}:agent-flow-handoff"
            )),
            receipt_id: AgentFlowReceiptId::new(format!(
                "loop-program:{program_id}:agent-flow-receipt"
            )),
        }
    }

    pub fn with_transform_id(mut self, transform_id: impl Into<AgentFlowTransformId>) -> Self {
        self.transform_id = transform_id.into();
        self
    }

    pub fn with_handoff_id(mut self, handoff_id: impl Into<AgentFlowRuntimeHandoffId>) -> Self {
        self.handoff_id = handoff_id.into();
        self
    }

    pub fn with_receipt_id(mut self, receipt_id: impl Into<AgentFlowReceiptId>) -> Self {
        self.receipt_id = receipt_id.into();
        self
    }
}

impl LoopProgramRuntimeHandoffExecutor for AgentFlowLoopProgramRuntimeHandoffExecutor {
    fn execute_plan(
        &self,
        plan: &LoopProgramRuntimeHandoffPlan,
    ) -> LoopProgramRuntimeHandoffExecutionReceipt {
        let session = AgentFlowSession::root(format!(
            "loop-program:{}:agent-flow-session",
            plan.program_id.as_str()
        ));
        let request = LoopProgramAgentFlowRuntimeHandoffRequest::for_session(plan.clone(), session)
            .with_transform_id(AgentFlowTransformId::new(format!(
                "loop-program:{}:agent-flow-transform",
                plan.program_id.as_str()
            )))
            .with_handoff_id(AgentFlowRuntimeHandoffId::new(format!(
                "loop-program:{}:agent-flow-handoff",
                plan.program_id.as_str()
            )))
            .with_receipt_id(AgentFlowReceiptId::new(format!(
                "loop-program:{}:agent-flow-receipt",
                plan.program_id.as_str()
            )));
        self.execute_agent_flow_request(request)
    }
}

fn execution_report_status(
    executions: &[LoopProgramRuntimeHandoffExecution],
) -> LoopProgramRuntimeHandoffExecutionReportStatus {
    if executions.is_empty() {
        return LoopProgramRuntimeHandoffExecutionReportStatus::Empty;
    }
    if executions
        .iter()
        .any(|execution| execution.status == LoopProgramRuntimeHandoffExecutionStatus::Denied)
    {
        return LoopProgramRuntimeHandoffExecutionReportStatus::Denied;
    }
    if executions
        .iter()
        .any(|execution| execution.status == LoopProgramRuntimeHandoffExecutionStatus::Deferred)
    {
        return LoopProgramRuntimeHandoffExecutionReportStatus::Deferred;
    }
    LoopProgramRuntimeHandoffExecutionReportStatus::Completed
}
