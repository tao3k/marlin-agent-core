use super::{
    ContinuationOp, GerbilPooLoopProgramCompilerBoundary, GerbilPooLoopProgramCompilerOwner,
    GerbilPooLoopProgramCompilerSerializationBoundary, LoopMechanismPolicyId,
    LoopProgramActionKind, LoopProgramEventKind, LoopProgramExecutionDriver,
    LoopProgramExecutionRequest, LoopProgramExecutionStatus,
    LoopProgramRuntimeHandoffExecutionReportStatus, LoopProgramRuntimeHandoffRouter,
    LoopProgramRuntimeHandoffRouterHandlers, ReceiptDrivenLoopProgramEventMapper,
    decode_gerbil_poo_loop_program_compiler_receipt,
    failure_retry_poo_loop_program_compiler_payload, failure_retry_script, handled_by,
    handled_by_with_event, policy_combination_matrix_poo_loop_program_compiler_payload,
    policy_combination_matrix_script, poo_loop_program_compiler_envelope,
    poo_loop_program_compiler_payload, poo_loop_program_compiler_registry, real_repair_script,
};

#[test]
fn poo_loop_program_compiler_receipt_runs_scripted_loop_through_kernel_driver() {
    let registry = poo_loop_program_compiler_registry();
    let envelope = poo_loop_program_compiler_envelope(poo_loop_program_compiler_payload([7; 32]));
    let compiler_receipt = decode_gerbil_poo_loop_program_compiler_receipt(&registry, &envelope)
        .expect("POO loop program compiler receipt decodes");

    let handlers = LoopProgramRuntimeHandoffRouterHandlers {
        control_handler: handled_by("runtime.control"),
        model_handler: handled_by("runtime.model"),
        tool_handler: handled_by("runtime.tool"),
        graph_handler: handled_by("runtime.graph"),
        verification_handler: handled_by("runtime.verification"),
        ..LoopProgramRuntimeHandoffRouterHandlers::default()
    };
    let driver = LoopProgramExecutionDriver::new(LoopProgramRuntimeHandoffRouter::new(handlers))
        .with_event_mapper(real_repair_script())
        .with_max_steps(16);

    let execution_receipt = driver.run(LoopProgramExecutionRequest::new(
        compiler_receipt.loop_program,
        vec![LoopProgramEventKind::Start],
    ));

    assert_eq!(
        execution_receipt.status,
        LoopProgramExecutionStatus::Stopped
    );
    assert!(execution_receipt.error.is_none());
    assert_eq!(execution_receipt.steps.len(), 6);
    assert_eq!(
        execution_receipt
            .steps
            .iter()
            .map(|step| step.machine_receipt.action.clone())
            .collect::<Vec<_>>(),
        vec![
            LoopProgramActionKind::InvokeModel,
            LoopProgramActionKind::DispatchTools,
            LoopProgramActionKind::Continue,
            LoopProgramActionKind::RewriteGraph,
            LoopProgramActionKind::Verify,
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
            Some(LoopProgramEventKind::ModelEvent),
            Some(LoopProgramEventKind::RuntimeReceipt),
            Some(LoopProgramEventKind::VerificationReceipt),
            None,
        ]
    );
    assert!(execution_receipt.steps.iter().all(|step| {
        step.runtime_handoff_plan.handoffs.len() == 1
            && step.runtime_handoff_execution.status
                == LoopProgramRuntimeHandoffExecutionReportStatus::Completed
    }));
    assert_eq!(
        execution_receipt
            .steps
            .iter()
            .map(|step| step.runtime_handoff_execution.executions[0].owner.as_str())
            .collect::<Vec<_>>(),
        vec![
            "runtime.model",
            "runtime.tool",
            "runtime.control",
            "runtime.graph",
            "runtime.verification",
            "runtime.control",
        ]
    );
}

#[test]
fn poo_loop_program_compiler_receipt_runs_failure_retry_profile_from_runtime_receipts() {
    let registry = poo_loop_program_compiler_registry();
    let envelope =
        poo_loop_program_compiler_envelope(failure_retry_poo_loop_program_compiler_payload());
    let compiler_receipt = decode_gerbil_poo_loop_program_compiler_receipt(&registry, &envelope)
        .expect("failure-retry POO compiler receipt decodes");

    let handlers = LoopProgramRuntimeHandoffRouterHandlers {
        model_handler: handled_by_with_event(
            "runtime.model.failure-classifier",
            LoopProgramEventKind::ModelEvent,
        ),
        runtime_handler: handled_by_with_event(
            "runtime.continuation.retry",
            LoopProgramEventKind::RuntimeReceipt,
        ),
        tool_handler: handled_by_with_event(
            "runtime.tool.retry",
            LoopProgramEventKind::ToolReceipt,
        ),
        verification_handler: handled_by_with_event(
            "runtime.verification.failure-retry",
            LoopProgramEventKind::VerificationReceipt,
        ),
        control_handler: handled_by("runtime.control"),
        ..LoopProgramRuntimeHandoffRouterHandlers::default()
    };
    let driver = LoopProgramExecutionDriver::new(LoopProgramRuntimeHandoffRouter::new(handlers))
        .with_event_mapper(ReceiptDrivenLoopProgramEventMapper)
        .with_max_steps(16);

    let execution_receipt = driver.run(LoopProgramExecutionRequest::new(
        compiler_receipt.loop_program,
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
            LoopProgramActionKind::InvokeModel,
            LoopProgramActionKind::RuntimeHandoff,
            LoopProgramActionKind::DispatchTools,
            LoopProgramActionKind::Verify,
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
            Some(LoopProgramEventKind::ModelEvent),
            Some(LoopProgramEventKind::RuntimeReceipt),
            Some(LoopProgramEventKind::ToolReceipt),
            Some(LoopProgramEventKind::VerificationReceipt),
            None,
        ]
    );
}

#[test]
fn poo_loop_program_compiler_receipt_runs_real_repair_from_runtime_receipts() {
    let registry = poo_loop_program_compiler_registry();
    let envelope = poo_loop_program_compiler_envelope(poo_loop_program_compiler_payload([7; 32]));
    let compiler_receipt = decode_gerbil_poo_loop_program_compiler_receipt(&registry, &envelope)
        .expect("POO loop program compiler receipt decodes");

    let handlers = LoopProgramRuntimeHandoffRouterHandlers {
        control_handler: handled_by_with_event("runtime.control", LoopProgramEventKind::ModelEvent),
        model_handler: handled_by_with_event("runtime.model", LoopProgramEventKind::ToolRequest),
        tool_handler: handled_by_with_event("runtime.tool", LoopProgramEventKind::ToolReceipt),
        graph_handler: handled_by_with_event("runtime.graph", LoopProgramEventKind::RuntimeReceipt),
        verification_handler: handled_by_with_event(
            "runtime.verification",
            LoopProgramEventKind::VerificationReceipt,
        ),
        ..LoopProgramRuntimeHandoffRouterHandlers::default()
    };
    let driver = LoopProgramExecutionDriver::new(LoopProgramRuntimeHandoffRouter::new(handlers))
        .with_event_mapper(ReceiptDrivenLoopProgramEventMapper)
        .with_max_steps(16);

    let execution_receipt = driver.run(LoopProgramExecutionRequest::new(
        compiler_receipt.loop_program,
        vec![LoopProgramEventKind::Start],
    ));

    assert_eq!(
        execution_receipt.status,
        LoopProgramExecutionStatus::Stopped
    );
    assert!(execution_receipt.error.is_none());
    assert_eq!(execution_receipt.steps.len(), 6);
    assert_eq!(
        execution_receipt
            .steps
            .iter()
            .map(|step| step.machine_receipt.action.clone())
            .collect::<Vec<_>>(),
        vec![
            LoopProgramActionKind::InvokeModel,
            LoopProgramActionKind::DispatchTools,
            LoopProgramActionKind::Continue,
            LoopProgramActionKind::RewriteGraph,
            LoopProgramActionKind::Verify,
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
            Some(LoopProgramEventKind::ModelEvent),
            Some(LoopProgramEventKind::RuntimeReceipt),
            Some(LoopProgramEventKind::VerificationReceipt),
            None,
        ]
    );
    assert_eq!(
        execution_receipt
            .steps
            .iter()
            .filter_map(|step| step.runtime_handoff_execution.executions[0]
                .next_event
                .clone())
            .collect::<Vec<_>>(),
        vec![
            LoopProgramEventKind::ToolRequest,
            LoopProgramEventKind::ToolReceipt,
            LoopProgramEventKind::ModelEvent,
            LoopProgramEventKind::RuntimeReceipt,
            LoopProgramEventKind::VerificationReceipt,
            LoopProgramEventKind::ModelEvent,
        ]
    );
}

#[test]
fn poo_loop_program_compiler_receipt_runs_policy_combination_matrix_through_kernel_driver() {
    let registry = poo_loop_program_compiler_registry();
    let envelope = poo_loop_program_compiler_envelope(
        policy_combination_matrix_poo_loop_program_compiler_payload(),
    );
    let compiler_receipt = decode_gerbil_poo_loop_program_compiler_receipt(&registry, &envelope)
        .expect("policy-combination POO compiler receipt decodes");

    assert_eq!(
        compiler_receipt.profile_id.as_str(),
        "policy-combination/memory-rewrite-checker"
    );
    assert_eq!(
        compiler_receipt.compiler_owner,
        GerbilPooLoopProgramCompilerOwner::GerbilPooFlow
    );
    assert_eq!(
        compiler_receipt.scheme_boundary,
        GerbilPooLoopProgramCompilerBoundary::SchemeTypesToRustTypes
    );
    assert_eq!(
        compiler_receipt.serialization_boundary,
        GerbilPooLoopProgramCompilerSerializationBoundary::RustOwnedCliTraceCrossProcess
    );
    assert_eq!(
        compiler_receipt.loop_program.program_id.as_str(),
        "policy-combination-memory-rewrite-checker"
    );
    assert_eq!(compiler_receipt.loop_program.mechanism_policies.len(), 3);
    assert_eq!(compiler_receipt.loop_program.transitions.len(), 6);
    assert_eq!(
        compiler_receipt.loop_program.policy_epoch,
        compiler_receipt.resolved_policy_pack.policy_epoch
    );
    assert_eq!(
        compiler_receipt.loop_program.policy_digest,
        compiler_receipt.resolved_policy_pack.policy_digest
    );

    let handlers = LoopProgramRuntimeHandoffRouterHandlers {
        memory_handler: handled_by("runtime.memory.recall"),
        control_handler: handled_by("runtime.control"),
        model_handler: handled_by("runtime.model.maker"),
        tool_handler: handled_by("runtime.tool.repair"),
        graph_handler: handled_by("runtime.graph.dynamic-rewrite"),
        verification_handler: handled_by("runtime.verification.checker"),
        ..LoopProgramRuntimeHandoffRouterHandlers::default()
    };
    let driver = LoopProgramExecutionDriver::new(LoopProgramRuntimeHandoffRouter::new(handlers))
        .with_event_mapper(policy_combination_matrix_script())
        .with_max_steps(16);

    let execution_receipt = driver.run(LoopProgramExecutionRequest::new(
        compiler_receipt.loop_program,
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
            LoopProgramActionKind::InvokeModel,
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
            "runtime.memory.recall",
            "runtime.model.maker",
            "runtime.graph.dynamic-rewrite",
            "runtime.tool.repair",
            "runtime.verification.checker",
            "runtime.control",
        ]
    );
}

#[test]
fn poo_loop_program_compiler_receipt_runs_policy_combination_matrix_from_runtime_receipts() {
    let registry = poo_loop_program_compiler_registry();
    let envelope = poo_loop_program_compiler_envelope(
        policy_combination_matrix_poo_loop_program_compiler_payload(),
    );
    let compiler_receipt = decode_gerbil_poo_loop_program_compiler_receipt(&registry, &envelope)
        .expect("policy-combination POO compiler receipt decodes");

    let handlers = LoopProgramRuntimeHandoffRouterHandlers {
        memory_handler: handled_by_with_event(
            "runtime.memory.recall",
            LoopProgramEventKind::RuntimeReceipt,
        ),
        control_handler: handled_by("runtime.control"),
        model_handler: handled_by_with_event(
            "runtime.model.maker",
            LoopProgramEventKind::ModelEvent,
        ),
        tool_handler: handled_by_with_event(
            "runtime.tool.repair",
            LoopProgramEventKind::ToolReceipt,
        ),
        graph_handler: handled_by_with_event(
            "runtime.graph.dynamic-rewrite",
            LoopProgramEventKind::RuntimeReceipt,
        ),
        verification_handler: handled_by_with_event(
            "runtime.verification.checker",
            LoopProgramEventKind::VerificationReceipt,
        ),
        ..LoopProgramRuntimeHandoffRouterHandlers::default()
    };
    let driver = LoopProgramExecutionDriver::new(LoopProgramRuntimeHandoffRouter::new(handlers))
        .with_event_mapper(ReceiptDrivenLoopProgramEventMapper)
        .with_max_steps(16);

    let execution_receipt = driver.run(LoopProgramExecutionRequest::new(
        compiler_receipt.loop_program,
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
            LoopProgramActionKind::InvokeModel,
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
            .map(|step| step.generated_event.clone())
            .collect::<Vec<_>>(),
        vec![
            Some(LoopProgramEventKind::RuntimeReceipt),
            Some(LoopProgramEventKind::ModelEvent),
            Some(LoopProgramEventKind::RuntimeReceipt),
            Some(LoopProgramEventKind::ToolReceipt),
            Some(LoopProgramEventKind::VerificationReceipt),
            None,
        ]
    );
}

#[test]
fn poo_loop_program_compiler_receipt_runs_failure_retry_profile_through_kernel_driver() {
    let registry = poo_loop_program_compiler_registry();
    let envelope =
        poo_loop_program_compiler_envelope(failure_retry_poo_loop_program_compiler_payload());
    let compiler_receipt = decode_gerbil_poo_loop_program_compiler_receipt(&registry, &envelope)
        .expect("failure-retry POO compiler receipt decodes");

    assert_eq!(
        compiler_receipt.profile_id.as_str(),
        "marlin-failure-retry-profile/typed-recovery"
    );
    assert_eq!(
        compiler_receipt.loop_program.program_id.as_str(),
        "failure-retry-typed-recovery"
    );
    assert!(
        compiler_receipt
            .loop_program
            .uses_policy(&LoopMechanismPolicyId::new("failure-retry-budget"))
    );
    assert_eq!(
        compiler_receipt
            .resolved_policy_pack
            .hot
            .budget_caps
            .max_attempts,
        3
    );
    match &compiler_receipt.resolved_policy_pack.hot.continuation_table[0] {
        ContinuationOp::Retry {
            graph_template,
            max_attempts,
        } => {
            assert_eq!(graph_template.get(), 1);
            assert_eq!(*max_attempts, 3);
        }
        other => panic!("expected retry continuation op, got {other:?}"),
    }

    let handlers = LoopProgramRuntimeHandoffRouterHandlers {
        model_handler: handled_by("runtime.model.failure-classifier"),
        runtime_handler: handled_by("runtime.continuation.retry"),
        tool_handler: handled_by("runtime.tool.retry"),
        verification_handler: handled_by("runtime.verification.failure-retry"),
        control_handler: handled_by("runtime.control"),
        ..LoopProgramRuntimeHandoffRouterHandlers::default()
    };
    let driver = LoopProgramExecutionDriver::new(LoopProgramRuntimeHandoffRouter::new(handlers))
        .with_event_mapper(failure_retry_script())
        .with_max_steps(16);

    let execution_receipt = driver.run(LoopProgramExecutionRequest::new(
        compiler_receipt.loop_program,
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
            LoopProgramActionKind::InvokeModel,
            LoopProgramActionKind::RuntimeHandoff,
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
            "runtime.model.failure-classifier",
            "runtime.continuation.retry",
            "runtime.tool.retry",
            "runtime.verification.failure-retry",
            "runtime.control",
        ]
    );
}
