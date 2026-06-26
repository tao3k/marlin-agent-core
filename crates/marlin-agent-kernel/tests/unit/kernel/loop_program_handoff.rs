use marlin_agent_kernel::{
    GenericLoopMachineReceipt, GenericLoopMachineStepIndex, LoopProgramRuntimeHandoffKind,
    LoopProgramRuntimeHandoffPlan,
};
use marlin_agent_protocol::{
    AgentFlowIntent, AgentFlowMemoryOperation, AgentFlowPlacementOperation, LoopProgramActionKind,
    LoopProgramEventKind, LoopProgramId, LoopProgramStateId, LoopProgramTransitionId,
};

#[test]
fn handoff_plan_maps_memory_actions_to_agent_flow_memory_intents() {
    let plan = LoopProgramRuntimeHandoffPlan::from_receipts(
        LoopProgramId::new("memory-program"),
        &[
            receipt(1, LoopProgramActionKind::ReadMemory),
            receipt(2, LoopProgramActionKind::WriteMemory),
            receipt(3, LoopProgramActionKind::CompactContext),
        ],
    );

    assert_eq!(
        plan.handoffs[0].kind,
        LoopProgramRuntimeHandoffKind::MemoryRecall
    );
    assert_eq!(
        plan.handoffs[1].kind,
        LoopProgramRuntimeHandoffKind::MemoryStore
    );
    assert_eq!(
        plan.handoffs[2].kind,
        LoopProgramRuntimeHandoffKind::ContextCompaction
    );

    let intents = plan.agent_flow_intents();
    assert_eq!(intents.len(), 3);
    assert_memory_intent(&intents[0], AgentFlowMemoryOperation::Recall);
    assert_memory_intent(&intents[1], AgentFlowMemoryOperation::Store);
    assert_memory_intent(&intents[2], AgentFlowMemoryOperation::Compact);
}

#[test]
fn handoff_plan_maps_placement_actions_to_agent_flow_placement_intents() {
    let plan = LoopProgramRuntimeHandoffPlan::from_receipts(
        LoopProgramId::new("placement-program"),
        &[
            receipt(1, LoopProgramActionKind::RequestPlacement),
            receipt(2, LoopProgramActionKind::ForkSession),
            receipt(3, LoopProgramActionKind::DelegateAgent),
        ],
    );

    assert_eq!(
        plan.handoffs[0].kind,
        LoopProgramRuntimeHandoffKind::PlacementRequest
    );
    assert_eq!(
        plan.handoffs[1].kind,
        LoopProgramRuntimeHandoffKind::SessionFork
    );
    assert_eq!(
        plan.handoffs[2].kind,
        LoopProgramRuntimeHandoffKind::AgentDelegation
    );

    let intents = plan.agent_flow_intents();
    assert_eq!(intents.len(), 3);
    assert_placement_intent(&intents[0], AgentFlowPlacementOperation::BindWorkspace);
    assert_placement_intent(&intents[1], AgentFlowPlacementOperation::ForkSession);
    assert_placement_intent(&intents[2], AgentFlowPlacementOperation::Delegate);
}

#[test]
fn handoff_plan_keeps_model_and_terminal_actions_out_of_agent_flow_intents() {
    let plan = LoopProgramRuntimeHandoffPlan::from_receipts(
        LoopProgramId::new("runtime-only-program"),
        &[
            receipt(1, LoopProgramActionKind::InvokeModel),
            receipt(2, LoopProgramActionKind::RewriteGraph),
            receipt(3, LoopProgramActionKind::Verify),
            receipt(4, LoopProgramActionKind::Stop),
        ],
    );

    assert_eq!(
        plan.handoffs[0].kind,
        LoopProgramRuntimeHandoffKind::ModelInvocation
    );
    assert_eq!(
        plan.handoffs[1].kind,
        LoopProgramRuntimeHandoffKind::GraphRewrite
    );
    assert_eq!(
        plan.handoffs[2].kind,
        LoopProgramRuntimeHandoffKind::Verification
    );
    assert_eq!(plan.handoffs[3].kind, LoopProgramRuntimeHandoffKind::Stop);
    assert!(plan.agent_flow_intents().is_empty());
}

fn receipt(step: u64, action: LoopProgramActionKind) -> GenericLoopMachineReceipt {
    GenericLoopMachineReceipt {
        program_id: LoopProgramId::new("program"),
        step_index: GenericLoopMachineStepIndex::new(step),
        transition_id: LoopProgramTransitionId::new(format!("transition-{step}")),
        from: LoopProgramStateId::new("from"),
        event: LoopProgramEventKind::RuntimeReceipt,
        action,
        to: LoopProgramStateId::new("to"),
        stopped: false,
    }
}

fn assert_memory_intent(intent: &AgentFlowIntent, operation: AgentFlowMemoryOperation) {
    let AgentFlowIntent::Memory(memory_intent) = intent else {
        panic!("expected memory intent");
    };
    assert_eq!(memory_intent.target.as_str(), "loop-program.memory");
    assert_eq!(memory_intent.operation, operation);
}

fn assert_placement_intent(intent: &AgentFlowIntent, operation: AgentFlowPlacementOperation) {
    let AgentFlowIntent::Placement(placement_intent) = intent else {
        panic!("expected placement intent");
    };
    assert_eq!(placement_intent.target.as_str(), "loop-program.placement");
    assert_eq!(placement_intent.operation, operation);
}
