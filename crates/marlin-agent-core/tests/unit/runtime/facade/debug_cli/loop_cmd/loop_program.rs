use std::fs;

use marlin_agent_core::{
    LoopProgramDerivedSessionPolicyStatusReceipt, LoopProgramRunReceipt,
    LoopProgramRunStatusReceipt, LoopProgramRuntimeHandoffStatusReceipt,
    LoopProgramRuntimeSideEffectStatusReceipt,
    protocol::{
        LoopMechanismPolicyId, LoopPolicyDigest, LoopPolicyEpoch, LoopProgram,
        LoopProgramActionKind, LoopProgramEventKind, LoopProgramId, LoopProgramInput,
        LoopProgramStateId, LoopProgramTransition, LoopProgramTransitionId,
    },
    run_marlin_cli_from_args,
};
use tempfile::tempdir;

#[test]
fn debug_cli_loop_program_run_projects_runtime_replay_bundle() {
    let dir = tempdir().expect("tempdir");
    let input = dir.path().join("loop-program-run.json");
    fs::write(
        &input,
        serde_json::to_string(&serde_json::json!({
            "program": sample_loop_program(),
            "events": [
                "start",
                "tool_request",
                "tool_receipt",
                "model_event",
                "runtime_receipt",
                "verification_receipt"
            ]
        }))
        .expect("loop program JSON"),
    )
    .expect("write loop program");

    let run = run_marlin_cli_from_args([
        "loop",
        "program",
        "run",
        "--input",
        input.to_str().expect("utf8 path"),
    ]);

    assert_eq!(run.status, 0, "{}", run.stderr);
    let receipt: LoopProgramRunReceipt =
        serde_json::from_str(&run.stdout).expect("loop program run receipt");
    assert_eq!(receipt.program_id, "debug-cli-loop-program");
    assert_eq!(receipt.status, LoopProgramRunStatusReceipt::Stopped);
    assert_eq!(receipt.action_receipt_count, 6);
    assert_eq!(receipt.last_action.as_deref(), Some("stop"));
    assert!(receipt.error.is_none());
    assert_eq!(
        receipt.runtime_handoff.status,
        LoopProgramRuntimeHandoffStatusReceipt::Deferred
    );
    assert_eq!(receipt.runtime_handoff.execution_count, 6);
    assert!(receipt.runtime_handoff.agent_flow_receipt_present);
    assert_eq!(receipt.runtime_handoff.tool_process_projection_count, 1);
    assert_eq!(receipt.runtime_handoff.memory_projection_count, 0);
    assert_eq!(
        receipt.runtime_replay_bundle.policy_status,
        LoopProgramDerivedSessionPolicyStatusReceipt::Deferred
    );
    assert!(!receipt.runtime_replay_bundle.allows_replay);
    assert!(receipt.runtime_replay_bundle.requires_follow_up);
    assert!(!receipt.runtime_replay_bundle.blocks_replay);
    assert_eq!(
        receipt.runtime_replay_bundle.side_effect_status,
        LoopProgramRuntimeSideEffectStatusReceipt::Skipped
    );
    assert_eq!(
        receipt
            .runtime_replay_bundle
            .agent_flow_handoff_id
            .as_deref(),
        Some("loop-program:debug-cli-loop-program:agent-flow-handoff")
    );
    assert_eq!(
        receipt
            .runtime_replay_bundle
            .agent_flow_derived_session_id
            .as_deref(),
        Some("loop-program:debug-cli-loop-program:agent-flow-session")
    );
}

fn sample_loop_program() -> LoopProgram {
    LoopProgram::new(LoopProgramInput {
        program_id: LoopProgramId::new("debug-cli-loop-program"),
        policy_epoch: LoopPolicyEpoch::new(7),
        policy_digest: LoopPolicyDigest::from_bytes([5_u8; 32]),
        mechanism_policies: vec![
            LoopMechanismPolicyId::new("reactive-tool-loop-base"),
            LoopMechanismPolicyId::new("dynamic-agent-flow-replay"),
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
