use std::{fs, path::PathBuf};

use marlin_agent_kernel::{
    AgentFlowLoopProgramRuntimeHandoffExecutor, LoopProgramDerivedSessionPolicyStatus,
    LoopProgramExecutionDriver, LoopProgramExecutionRequest, LoopProgramExecutionStatus,
    LoopProgramFileSandbox, LoopProgramFileWriteSideEffectStatus, LoopProgramFileWriteTemplate,
    LoopProgramRuntimeHandoffExecutionReportStatus, LoopProgramRuntimeOwner,
    LoopProgramRuntimeSideEffectExecutor, LoopProgramRuntimeSideEffectStatus,
    StaticLoopProgramFileWriteResolver, StaticLoopProgramToolProcessResolver,
};
use marlin_agent_protocol::{
    AgentFlowMemoryOperation, LoopProgramActionKind, LoopProgramEventKind,
};
use marlin_agent_runtime::TokioAgentRuntime;
use marlin_gerbil_scheme::verify_gerbil_loop_case_driver_vertical_trace;

use super::support::{
    SchemeProjectedMemoryRecallDecisionMapper, cap, run_config_interface_case_driver_smoke,
    scheme_projected_dispatch_tools_shell_resolver, scheme_projected_event_mapper,
    scheme_projected_loop_program, scheme_projected_runtime_executor, tool_projection_count,
    unique_scheme_projected_runtime_workspace,
};

#[test]
fn config_interface_vertical_trace_routes_projected_maker_checker_lanes_apart() {
    let stdout = run_config_interface_case_driver_smoke();
    let vertical_receipts =
        verify_gerbil_loop_case_driver_vertical_trace(&stdout, 7).expect("vertical trace verifies");
    let receipt = vertical_receipts
        .iter()
        .find(|receipt| {
            receipt.has_capability(&cap("+maker"))
                && receipt.has_capability(&cap("+checker"))
                && receipt.transition_actions().collect::<Vec<_>>()
                    == vec!["invoke_model", "verify", "stop"]
        })
        .expect("Scheme vertical trace should include a maker/checker loop shape");
    let loop_program = scheme_projected_loop_program(receipt);
    let driver = LoopProgramExecutionDriver::new(scheme_projected_runtime_executor())
        .with_event_mapper(scheme_projected_event_mapper(receipt))
        .with_max_steps(receipt.transition_count() + 2);

    let execution_receipt = driver.run(LoopProgramExecutionRequest::new(
        loop_program,
        vec![LoopProgramEventKind::Start],
    ));

    assert_eq!(
        execution_receipt.status,
        LoopProgramExecutionStatus::Stopped
    );
    assert!(execution_receipt.error.is_none());
    assert_eq!(
        execution_receipt
            .steps
            .iter()
            .map(|step| step.runtime_handoff_execution.executions[0].owner.as_str())
            .collect::<Vec<_>>(),
        vec![
            "scheme.projected.model",
            "scheme.projected.verification",
            "scheme.projected.control",
        ]
    );
    assert_eq!(
        execution_receipt
            .steps
            .iter()
            .map(|step| step.machine_receipt.action.clone())
            .collect::<Vec<_>>(),
        vec![
            LoopProgramActionKind::InvokeModel,
            LoopProgramActionKind::Verify,
            LoopProgramActionKind::Stop,
        ]
    );
}

#[cfg(unix)]
#[tokio::test]
async fn config_interface_vertical_trace_drives_projected_dynamic_rewrite_before_repair() {
    let stdout = run_config_interface_case_driver_smoke();
    let vertical_receipts =
        verify_gerbil_loop_case_driver_vertical_trace(&stdout, 7).expect("vertical trace verifies");
    let receipt = vertical_receipts
        .iter()
        .find(|receipt| {
            receipt.has_capability(&cap("+dynamic-rewrite"))
                && receipt.transition_actions().collect::<Vec<_>>()
                    == vec!["rewrite_graph", "dispatch_tools", "verify", "stop"]
        })
        .expect("Scheme vertical trace should include a dynamic rewrite repair loop shape");
    let loop_program = scheme_projected_loop_program(receipt);
    let driver = LoopProgramExecutionDriver::new(scheme_projected_runtime_executor())
        .with_event_mapper(scheme_projected_event_mapper(receipt))
        .with_max_steps(receipt.transition_count() + 2);

    let execution_receipt = driver.run(LoopProgramExecutionRequest::new(
        loop_program,
        vec![LoopProgramEventKind::Start],
    ));

    assert_eq!(
        execution_receipt.status,
        LoopProgramExecutionStatus::Stopped
    );
    assert!(execution_receipt.error.is_none());
    assert_eq!(
        execution_receipt
            .steps
            .iter()
            .map(|step| step.machine_receipt.action.clone())
            .collect::<Vec<_>>(),
        vec![
            LoopProgramActionKind::RewriteGraph,
            LoopProgramActionKind::DispatchTools,
            LoopProgramActionKind::Verify,
            LoopProgramActionKind::Stop,
        ]
    );
    assert_eq!(
        execution_receipt
            .steps
            .iter()
            .map(|step| step.runtime_handoff_execution.executions[0].owner.as_str())
            .collect::<Vec<_>>(),
        vec![
            "scheme.projected.graph",
            "scheme.projected.agent-flow",
            "scheme.projected.verification",
            "scheme.projected.control",
        ]
    );
    assert_eq!(tool_projection_count(&execution_receipt), 1);

    let (runtime, _events) = TokioAgentRuntime::new(4);
    let replay_bundle =
        LoopProgramRuntimeSideEffectExecutor::new(scheme_projected_dispatch_tools_shell_resolver(
            "printf scheme-projected-dynamic-rewrite-tool-spawn",
        ))
        .with_started_at_ms(950)
        .with_observed_at_ms(975)
        .execute_loop_execution(&runtime.context(), &execution_receipt)
        .await;

    assert_eq!(
        replay_bundle.policy_status,
        LoopProgramDerivedSessionPolicyStatus::Ready
    );
    assert!(replay_bundle.allows_replay());
    let tool_processes = replay_bundle
        .step_replay_bundles
        .iter()
        .flat_map(|bundle| bundle.side_effects.tool_processes.iter())
        .collect::<Vec<_>>();
    assert_eq!(tool_processes.len(), 1);
    let spawn_receipt = tool_processes[0]
        .spawn_receipt
        .as_ref()
        .expect("Scheme-projected dynamic rewrite tool should spawn");
    assert!(spawn_receipt.output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&spawn_receipt.output.stdout),
        "scheme-projected-dynamic-rewrite-tool-spawn"
    );
    assert!(spawn_receipt.output.stderr.is_empty());
}

#[cfg(unix)]
#[tokio::test]
async fn config_interface_vertical_trace_uses_projected_memory_recall_to_select_tool_path() {
    let stdout = run_config_interface_case_driver_smoke();
    let vertical_receipts =
        verify_gerbil_loop_case_driver_vertical_trace(&stdout, 7).expect("vertical trace verifies");
    let receipt = vertical_receipts
        .iter()
        .find(|receipt| {
            receipt.has_capability(&cap("+memory-recall"))
                && receipt.transition_actions().collect::<Vec<_>>()
                    == vec!["read_memory", "dispatch_tools", "stop"]
        })
        .expect("Scheme vertical trace should include a memory recall loop shape");
    let loop_program = scheme_projected_loop_program(receipt);
    let driver = LoopProgramExecutionDriver::new(AgentFlowLoopProgramRuntimeHandoffExecutor::new(
        LoopProgramRuntimeOwner::new("scheme.projected.agent-flow.memory-policy"),
    ))
    .with_event_mapper(SchemeProjectedMemoryRecallDecisionMapper)
    .with_max_steps(receipt.transition_count() + 2);

    let execution_receipt = driver.run(LoopProgramExecutionRequest::new(
        loop_program,
        vec![LoopProgramEventKind::Start],
    ));

    assert_eq!(
        execution_receipt.status,
        LoopProgramExecutionStatus::Stopped
    );
    assert!(execution_receipt.error.is_none());
    assert_eq!(
        execution_receipt
            .steps
            .iter()
            .map(|step| step.machine_receipt.action.clone())
            .collect::<Vec<_>>(),
        vec![
            LoopProgramActionKind::ReadMemory,
            LoopProgramActionKind::DispatchTools,
            LoopProgramActionKind::Stop,
        ]
    );
    assert_eq!(
        execution_receipt
            .steps
            .iter()
            .map(|step| step.generated_event.clone())
            .collect::<Vec<_>>(),
        vec![
            Some(LoopProgramEventKind::ToolRequest),
            Some(LoopProgramEventKind::ToolReceipt),
            None,
        ]
    );
    assert_eq!(
        execution_receipt.steps[0]
            .runtime_handoff_execution
            .memory_projections[0]
            .intent
            .operation,
        AgentFlowMemoryOperation::Recall
    );
    assert_eq!(tool_projection_count(&execution_receipt), 1);

    let (runtime, _events) = TokioAgentRuntime::new(4);
    let replay_bundle =
        LoopProgramRuntimeSideEffectExecutor::new(scheme_projected_dispatch_tools_shell_resolver(
            "printf scheme-projected-memory-recall-tool-spawn",
        ))
        .with_started_at_ms(1000)
        .with_observed_at_ms(1025)
        .execute_loop_execution(&runtime.context(), &execution_receipt)
        .await;

    assert_eq!(
        replay_bundle.policy_status,
        LoopProgramDerivedSessionPolicyStatus::Ready
    );
    assert!(replay_bundle.allows_replay());
    assert!(
        replay_bundle
            .step_replay_bundles
            .iter()
            .any(
                |bundle| bundle.side_effects.status == LoopProgramRuntimeSideEffectStatus::Empty
                    && !bundle.handoff_execution.memory_projections.is_empty()
            )
    );
    let tool_processes = replay_bundle
        .step_replay_bundles
        .iter()
        .flat_map(|bundle| bundle.side_effects.tool_processes.iter())
        .collect::<Vec<_>>();
    assert_eq!(tool_processes.len(), 1);
    let spawn_receipt = tool_processes[0]
        .spawn_receipt
        .as_ref()
        .expect("Scheme-projected memory-selected tool should spawn");
    assert!(spawn_receipt.output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&spawn_receipt.output.stdout),
        "scheme-projected-memory-recall-tool-spawn"
    );
    assert!(spawn_receipt.output.stderr.is_empty());
}

#[cfg(unix)]
#[tokio::test]
async fn config_interface_vertical_trace_drives_projected_repair_file_write_and_verifier() {
    let stdout = run_config_interface_case_driver_smoke();
    let vertical_receipts =
        verify_gerbil_loop_case_driver_vertical_trace(&stdout, 7).expect("vertical trace verifies");
    let receipt = vertical_receipts
        .iter()
        .find(|receipt| {
            receipt.live_llm_required()
                && receipt.has_capability(&cap("+tool-repair"))
                && receipt.has_capability(&cap("+verification"))
                && receipt
                    .transition_actions()
                    .any(|action| action == "dispatch_tools")
        })
        .expect("Scheme vertical trace should include a repair file-write path");
    let loop_program = scheme_projected_loop_program(receipt);
    let driver = LoopProgramExecutionDriver::new(scheme_projected_runtime_executor())
        .with_event_mapper(scheme_projected_event_mapper(receipt))
        .with_max_steps(receipt.transition_count() + 2);

    let execution_receipt = driver.run(LoopProgramExecutionRequest::new(
        loop_program,
        vec![LoopProgramEventKind::Start],
    ));

    assert_eq!(
        execution_receipt.status,
        LoopProgramExecutionStatus::Stopped
    );
    assert!(
        execution_receipt
            .steps
            .iter()
            .any(
                |step| step.machine_receipt.action == LoopProgramActionKind::Verify
                    && step.runtime_handoff_execution.status
                        == LoopProgramRuntimeHandoffExecutionReportStatus::Completed
            ),
        "Scheme-projected repair loop should complete a verifier handoff: {execution_receipt:?}"
    );
    assert!(
        tool_projection_count(&execution_receipt) > 0,
        "Scheme-projected repair loop should emit a typed tool projection: {execution_receipt:?}"
    );

    let workspace = unique_scheme_projected_runtime_workspace();
    let bug_relative_path = PathBuf::from("src/lib.rs");
    let bug_file = workspace.join(&bug_relative_path);
    fs::create_dir_all(bug_file.parent().expect("bug fixture parent"))
        .expect("create Scheme-projected repair fixture dir");
    fs::write(&bug_file, "fn answer() -> i32 { 40 }\n")
        .expect("write Scheme-projected repair fixture");

    let (runtime, _events) = TokioAgentRuntime::new(4);
    let replay_bundle =
        LoopProgramRuntimeSideEffectExecutor::new(StaticLoopProgramToolProcessResolver::default())
            .with_file_write_resolver(StaticLoopProgramFileWriteResolver::new(
                vec![LoopProgramFileWriteTemplate::new(
                    "agent-flow.tool-intent",
                    ["loop-program.dispatch-tools"],
                    bug_relative_path.clone(),
                    b"fn answer() -> i32 { 41 }\n".to_vec(),
                )]
                .into_boxed_slice(),
            ))
            .with_file_sandbox(
                LoopProgramFileSandbox::new(workspace.clone())
                    .with_allowed_relative_paths([bug_relative_path.clone()]),
            )
            .execute_loop_execution(&runtime.context(), &execution_receipt)
            .await;

    assert_eq!(
        replay_bundle.policy_status,
        LoopProgramDerivedSessionPolicyStatus::Ready
    );
    assert!(replay_bundle.allows_replay());
    let file_writes = replay_bundle
        .step_replay_bundles
        .iter()
        .flat_map(|bundle| bundle.side_effects.file_writes.iter())
        .collect::<Vec<_>>();
    assert_eq!(file_writes.len(), 1);
    let file_write = file_writes[0];
    assert_eq!(
        file_write.status,
        LoopProgramFileWriteSideEffectStatus::Completed
    );
    let write_receipt = file_write
        .write_receipt
        .as_ref()
        .expect("Scheme-projected repair should emit a file-write receipt");
    assert_eq!(write_receipt.relative_path, bug_relative_path);
    assert_eq!(write_receipt.path, bug_file);
    assert_eq!(
        write_receipt.bytes_written,
        "fn answer() -> i32 { 41 }\n".len()
    );
    assert_ne!(
        write_receipt.before_hash.as_ref().expect("before hash"),
        &write_receipt.after_hash
    );
    assert_eq!(
        fs::read_to_string(&bug_file).expect("read Scheme-projected repaired fixture"),
        "fn answer() -> i32 { 41 }\n"
    );
    fs::remove_dir_all(&workspace).expect("remove Scheme-projected repair workspace");
}
