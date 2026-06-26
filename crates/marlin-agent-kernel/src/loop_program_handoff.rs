//! Runtime handoff projection for `LoopProgramActionKind` receipts.

use marlin_agent_protocol::{
    AgentFlowIntent, AgentFlowIntentId, AgentFlowMemoryIntent, AgentFlowMemoryOperation,
    AgentFlowMemoryTarget, AgentFlowPlacementIntent, AgentFlowPlacementOperation,
    AgentFlowPlacementTarget, AgentFlowToolIntent, AgentFlowToolName, LoopProgramActionKind,
    LoopProgramId,
};

use crate::GenericLoopMachineReceipt;

const DISPATCH_TOOLS_TOOL: &str = "loop-program.dispatch-tools";
const MEMORY_TARGET: &str = "loop-program.memory";
const PLACEMENT_TARGET: &str = "loop-program.placement";

/// Runtime handoff plan derived from `GenericLoopMachine` action receipts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoopProgramRuntimeHandoffPlan {
    pub program_id: LoopProgramId,
    pub handoffs: Box<[LoopProgramRuntimeHandoff]>,
}

impl LoopProgramRuntimeHandoffPlan {
    pub fn from_receipts(
        program_id: LoopProgramId,
        receipts: &[GenericLoopMachineReceipt],
    ) -> Self {
        Self {
            program_id,
            handoffs: receipts
                .iter()
                .map(LoopProgramRuntimeHandoff::from_receipt)
                .collect(),
        }
    }

    pub fn agent_flow_intents(&self) -> Vec<AgentFlowIntent> {
        self.handoffs
            .iter()
            .filter_map(|handoff| handoff.agent_flow_intent.clone())
            .collect()
    }
}

/// One runtime handoff projection for a loop-program action receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoopProgramRuntimeHandoff {
    pub receipt: GenericLoopMachineReceipt,
    pub kind: LoopProgramRuntimeHandoffKind,
    pub agent_flow_intent: Option<AgentFlowIntent>,
}

impl LoopProgramRuntimeHandoff {
    pub fn from_receipt(receipt: &GenericLoopMachineReceipt) -> Self {
        let kind = LoopProgramRuntimeHandoffKind::from_action(&receipt.action);
        let agent_flow_intent = agent_flow_intent_for_receipt(receipt);
        Self {
            receipt: receipt.clone(),
            kind,
            agent_flow_intent,
        }
    }
}

/// Concrete runtime lane selected for a `LoopProgramActionKind`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LoopProgramRuntimeHandoffKind {
    Continue,
    ModelInvocation,
    ToolDispatch,
    MemoryRecall,
    MemoryStore,
    PlacementRequest,
    RuntimeHandoff,
    GraphRewrite,
    SessionFork,
    AgentDelegation,
    Verification,
    HumanGate,
    ContextCompaction,
    ReceiptEmission,
    Stop,
}

impl LoopProgramRuntimeHandoffKind {
    pub fn from_action(action: &LoopProgramActionKind) -> Self {
        match action {
            LoopProgramActionKind::Continue => Self::Continue,
            LoopProgramActionKind::InvokeModel => Self::ModelInvocation,
            LoopProgramActionKind::DispatchTools => Self::ToolDispatch,
            LoopProgramActionKind::ReadMemory => Self::MemoryRecall,
            LoopProgramActionKind::WriteMemory => Self::MemoryStore,
            LoopProgramActionKind::RequestPlacement => Self::PlacementRequest,
            LoopProgramActionKind::RuntimeHandoff => Self::RuntimeHandoff,
            LoopProgramActionKind::RewriteGraph => Self::GraphRewrite,
            LoopProgramActionKind::ForkSession => Self::SessionFork,
            LoopProgramActionKind::DelegateAgent => Self::AgentDelegation,
            LoopProgramActionKind::Verify => Self::Verification,
            LoopProgramActionKind::HumanGate => Self::HumanGate,
            LoopProgramActionKind::CompactContext => Self::ContextCompaction,
            LoopProgramActionKind::EmitReceipt => Self::ReceiptEmission,
            LoopProgramActionKind::Stop => Self::Stop,
        }
    }
}

fn agent_flow_intent_for_receipt(receipt: &GenericLoopMachineReceipt) -> Option<AgentFlowIntent> {
    let intent_id = AgentFlowIntentId::new(format!(
        "loop-program:{}:{}",
        receipt.program_id.as_str(),
        receipt.step_index.get()
    ));
    match receipt.action {
        LoopProgramActionKind::DispatchTools => Some(AgentFlowIntent::Tool(AgentFlowToolIntent {
            intent_id,
            tool_name: AgentFlowToolName::new(DISPATCH_TOOLS_TOOL),
        })),
        LoopProgramActionKind::ReadMemory => Some(AgentFlowIntent::Memory(AgentFlowMemoryIntent {
            intent_id,
            target: AgentFlowMemoryTarget::new(MEMORY_TARGET),
            operation: AgentFlowMemoryOperation::Recall,
        })),
        LoopProgramActionKind::WriteMemory => {
            Some(AgentFlowIntent::Memory(AgentFlowMemoryIntent {
                intent_id,
                target: AgentFlowMemoryTarget::new(MEMORY_TARGET),
                operation: AgentFlowMemoryOperation::Store,
            }))
        }
        LoopProgramActionKind::CompactContext => {
            Some(AgentFlowIntent::Memory(AgentFlowMemoryIntent {
                intent_id,
                target: AgentFlowMemoryTarget::new(MEMORY_TARGET),
                operation: AgentFlowMemoryOperation::Compact,
            }))
        }
        LoopProgramActionKind::RequestPlacement => {
            Some(AgentFlowIntent::Placement(AgentFlowPlacementIntent {
                intent_id,
                target: AgentFlowPlacementTarget::new(PLACEMENT_TARGET),
                operation: AgentFlowPlacementOperation::BindWorkspace,
            }))
        }
        LoopProgramActionKind::ForkSession => {
            Some(AgentFlowIntent::Placement(AgentFlowPlacementIntent {
                intent_id,
                target: AgentFlowPlacementTarget::new(PLACEMENT_TARGET),
                operation: AgentFlowPlacementOperation::ForkSession,
            }))
        }
        LoopProgramActionKind::DelegateAgent => {
            Some(AgentFlowIntent::Placement(AgentFlowPlacementIntent {
                intent_id,
                target: AgentFlowPlacementTarget::new(PLACEMENT_TARGET),
                operation: AgentFlowPlacementOperation::Delegate,
            }))
        }
        _ => None,
    }
}
