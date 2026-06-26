use marlin_agent_kernel::{GenericLoopMachine, GenericLoopMachineError};
use marlin_agent_protocol::{
    LoopMechanismPolicyId, LoopPolicyDigest, LoopPolicyEpoch, LoopProgram, LoopProgramActionKind,
    LoopProgramEventKind, LoopProgramId, LoopProgramInput, LoopProgramStateId,
    LoopProgramTransition, LoopProgramTransitionId,
};

#[test]
fn loop_machine_emits_runtime_actions_from_compiled_loop_program() {
    let mut machine = GenericLoopMachine::new(sample_loop_program());

    assert!(machine.uses_policy(&LoopMechanismPolicyId::new("reactive-tool-loop-base")));
    assert_eq!(machine.current_state().as_str(), "start");

    let model_step = machine
        .apply_event(LoopProgramEventKind::Start)
        .expect("start should invoke model");
    assert_eq!(model_step.action, LoopProgramActionKind::InvokeModel);
    assert_eq!(model_step.receipt.step_index.get(), 1);
    assert_eq!(model_step.receipt.transition_id.as_str(), "start-model");
    assert_eq!(model_step.receipt.from.as_str(), "start");
    assert_eq!(model_step.receipt.to.as_str(), "await-model");
    assert!(!model_step.receipt.stopped);

    let tool_step = machine
        .apply_event(LoopProgramEventKind::ToolRequest)
        .expect("tool request should dispatch tools");
    assert_eq!(tool_step.action, LoopProgramActionKind::DispatchTools);
    assert_eq!(tool_step.receipt.transition_id.as_str(), "model-tools");
    assert_eq!(machine.current_state().as_str(), "await-tools");

    let continue_step = machine
        .apply_event(LoopProgramEventKind::ToolReceipt)
        .expect("tool receipt should continue");
    assert_eq!(continue_step.action, LoopProgramActionKind::Continue);
    assert_eq!(continue_step.receipt.to.as_str(), "await-model");
}

#[test]
fn loop_machine_keeps_dynamic_rewrite_as_policy_action_not_provider_driver() {
    let mut machine = GenericLoopMachine::new(sample_loop_program());

    machine
        .apply_event(LoopProgramEventKind::Start)
        .expect("start should invoke model");
    let rewrite_step = machine
        .apply_event(LoopProgramEventKind::ModelEvent)
        .expect("model event should rewrite graph");

    assert_eq!(rewrite_step.action, LoopProgramActionKind::RewriteGraph);
    assert_eq!(
        rewrite_step.receipt.transition_id.as_str(),
        "dynamic-rewrite"
    );
    assert_eq!(rewrite_step.receipt.to.as_str(), "rewritten");
    assert!(machine.uses_policy(&LoopMechanismPolicyId::new(
        "claude-style-dynamic-graph-rewrite"
    )));
}

#[test]
fn loop_machine_stops_on_terminal_action_and_rejects_more_events() {
    let mut machine = GenericLoopMachine::new(sample_loop_program());

    machine
        .apply_event(LoopProgramEventKind::Start)
        .expect("start should invoke model");
    machine
        .apply_event(LoopProgramEventKind::ModelEvent)
        .expect("model event should rewrite graph");
    machine
        .apply_event(LoopProgramEventKind::RuntimeReceipt)
        .expect("runtime receipt should verify");
    let stop_step = machine
        .apply_event(LoopProgramEventKind::VerificationReceipt)
        .expect("verification receipt should stop");

    assert_eq!(stop_step.action, LoopProgramActionKind::Stop);
    assert!(stop_step.receipt.stopped);
    assert!(machine.is_stopped());

    let error = machine
        .apply_event(LoopProgramEventKind::StopSignal)
        .expect_err("stopped machine should reject later events");
    assert_eq!(
        error,
        GenericLoopMachineError::AlreadyStopped {
            program_id: LoopProgramId::new("repo-build-reactive-turn"),
            state: LoopProgramStateId::new("stopped"),
        }
    );
}

#[test]
fn loop_machine_reports_missing_transition_without_side_effects() {
    let mut machine = GenericLoopMachine::new(sample_loop_program());

    let error = machine
        .apply_event(LoopProgramEventKind::ToolReceipt)
        .expect_err("tool receipt is invalid from start state");

    assert_eq!(
        error,
        GenericLoopMachineError::NoTransition {
            program_id: LoopProgramId::new("repo-build-reactive-turn"),
            state: LoopProgramStateId::new("start"),
            event: LoopProgramEventKind::ToolReceipt,
        }
    );
    assert_eq!(machine.current_state().as_str(), "start");
    assert_eq!(machine.step_index().get(), 0);
}

fn sample_loop_program() -> LoopProgram {
    LoopProgram::new(LoopProgramInput {
        program_id: LoopProgramId::new("repo-build-reactive-turn"),
        policy_epoch: LoopPolicyEpoch::new(42),
        policy_digest: LoopPolicyDigest::from_bytes([9_u8; 32]),
        mechanism_policies: vec![
            LoopMechanismPolicyId::new("reactive-tool-loop-base"),
            LoopMechanismPolicyId::new("codex-style-pending-input-drain"),
            LoopMechanismPolicyId::new("openrath-style-resource-key-dispatch"),
            LoopMechanismPolicyId::new("claude-style-dynamic-graph-rewrite"),
        ]
        .into_boxed_slice(),
        initial_state: LoopProgramStateId::new("start"),
        transitions: vec![
            LoopProgramTransition {
                transition_id: LoopProgramTransitionId::new("start-model"),
                from: LoopProgramStateId::new("start"),
                event: LoopProgramEventKind::Start,
                action: LoopProgramActionKind::InvokeModel,
                to: LoopProgramStateId::new("await-model"),
            },
            LoopProgramTransition {
                transition_id: LoopProgramTransitionId::new("model-tools"),
                from: LoopProgramStateId::new("await-model"),
                event: LoopProgramEventKind::ToolRequest,
                action: LoopProgramActionKind::DispatchTools,
                to: LoopProgramStateId::new("await-tools"),
            },
            LoopProgramTransition {
                transition_id: LoopProgramTransitionId::new("tools-continue"),
                from: LoopProgramStateId::new("await-tools"),
                event: LoopProgramEventKind::ToolReceipt,
                action: LoopProgramActionKind::Continue,
                to: LoopProgramStateId::new("await-model"),
            },
            LoopProgramTransition {
                transition_id: LoopProgramTransitionId::new("dynamic-rewrite"),
                from: LoopProgramStateId::new("await-model"),
                event: LoopProgramEventKind::ModelEvent,
                action: LoopProgramActionKind::RewriteGraph,
                to: LoopProgramStateId::new("rewritten"),
            },
            LoopProgramTransition {
                transition_id: LoopProgramTransitionId::new("verify-rewrite"),
                from: LoopProgramStateId::new("rewritten"),
                event: LoopProgramEventKind::RuntimeReceipt,
                action: LoopProgramActionKind::Verify,
                to: LoopProgramStateId::new("verifying"),
            },
            LoopProgramTransition {
                transition_id: LoopProgramTransitionId::new("verification-stop"),
                from: LoopProgramStateId::new("verifying"),
                event: LoopProgramEventKind::VerificationReceipt,
                action: LoopProgramActionKind::Stop,
                to: LoopProgramStateId::new("stopped"),
            },
        ]
        .into_boxed_slice(),
    })
}
