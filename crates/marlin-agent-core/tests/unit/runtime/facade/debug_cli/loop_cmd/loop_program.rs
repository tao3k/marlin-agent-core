use std::{fs, path::PathBuf};

use marlin_agent_core::{
    GraphLoopExecutionStatus, LoopProgramDerivedSessionPolicyStatusReceipt, LoopProgramRunReceipt,
    LoopProgramRunStatusReceipt, LoopProgramRuntimeHandoffStatusReceipt,
    LoopProgramRuntimeSideEffectStatusReceipt, LoopRunReceipt,
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

#[test]
fn debug_cli_loop_program_run_consumes_config_interface_runtime_handoff_fixture() {
    let input = config_interface_runtime_handoff_fixture();
    let run = run_marlin_cli_from_args([
        "loop",
        "program",
        "run",
        "--input",
        input.to_str().expect("utf8 fixture path"),
    ]);

    assert_eq!(run.status, 0, "{}", run.stderr);
    let receipt: LoopProgramRunReceipt =
        serde_json::from_str(&run.stdout).expect("loop program fixture receipt");
    assert_eq!(receipt.program_id, "marlin-runtime-handoff-real-llm");
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
        Some("loop-program:marlin-runtime-handoff-real-llm:agent-flow-handoff")
    );
    assert_eq!(
        receipt
            .runtime_replay_bundle
            .agent_flow_derived_session_id
            .as_deref(),
        Some("loop-program:marlin-runtime-handoff-real-llm:agent-flow-session")
    );
}

#[test]
fn debug_cli_loop_run_consumes_config_interface_loop_case_fixtures() {
    for case in config_interface_loop_case_fixtures() {
        let input = config_interface_loop_case_fixture(case.file);
        let run = run_marlin_cli_from_args([
            "loop",
            "run",
            "--input",
            input.to_str().expect("utf8 loop fixture path"),
            "--catalog",
            config_interface_real_llm_catalog()
                .to_str()
                .expect("utf8 catalog path"),
            "--max-iterations",
            "1",
            "--no-store",
        ]);

        assert_eq!(run.status, 0, "{}\ncase={}", run.stderr, case.graph_id);
        let receipt: LoopRunReceipt =
            serde_json::from_str(&run.stdout).expect("loop run fixture receipt");
        assert_eq!(receipt.run_id.as_str(), "marlin-loop-run");
        assert_eq!(receipt.iteration_count, 1, "case={}", case.graph_id);
        assert_eq!(receipt.report_path, None, "case={}", case.graph_id);
        assert_eq!(
            receipt.terminal_status,
            Some(GraphLoopExecutionStatus::Completed),
            "case={}",
            case.graph_id
        );
        assert_eq!(receipt.reports.len(), 1, "case={}", case.graph_id);
        assert!(receipt.replayable, "case={}", case.graph_id);
        assert_eq!(receipt.missing_trace_count, 0, "case={}", case.graph_id);

        let observation = receipt
            .runtime_observation
            .as_ref()
            .expect("runtime observation");
        assert_eq!(observation.graph_id.as_str(), case.graph_id);
        assert_eq!(observation.run_id.as_str(), "marlin-loop-run");
        assert_eq!(
            observation.status,
            GraphLoopExecutionStatus::Completed.into()
        );
        assert_eq!(
            observation.terminal_status,
            Some(GraphLoopExecutionStatus::Completed)
        );
        assert!(
            receipt.governance_receipt.is_none(),
            "case={} should not synthesize governance without --governance",
            case.graph_id
        );
    }
}

fn config_interface_runtime_handoff_fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
        "../marlin-gerbil-scheme/gerbil/t/fixtures/config-interface/loop-cases/runtime-handoff-llm.loop-program-run.json",
    )
}

struct ConfigInterfaceLoopCaseFixture {
    file: &'static str,
    graph_id: &'static str,
}

fn config_interface_loop_case_fixtures() -> [ConfigInterfaceLoopCaseFixture; 3] {
    [
        ConfigInterfaceLoopCaseFixture {
            file: "policy-receipt-gate-llm.loop.json",
            graph_id: "marlin-policy-receipt-gate-real-llm",
        },
        ConfigInterfaceLoopCaseFixture {
            file: "loop-contract-llm.loop.json",
            graph_id: "marlin-loop-contract-real-llm",
        },
        ConfigInterfaceLoopCaseFixture {
            file: "failure-retry-llm.loop.json",
            graph_id: "marlin-failure-retry-real-llm",
        },
    ]
}

fn config_interface_loop_case_fixture(file: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../marlin-gerbil-scheme/gerbil/t/fixtures/config-interface/loop-cases")
        .join(file)
}

fn config_interface_real_llm_catalog() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
        "../marlin-gerbil-scheme/gerbil/t/fixtures/config-interface/loop-cases/real-llm-catalog.toml",
    )
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
