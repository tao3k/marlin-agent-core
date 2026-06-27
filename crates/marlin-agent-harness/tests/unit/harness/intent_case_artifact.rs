use std::sync::{Arc, OnceLock};

use marlin_agent_harness::{
    GerbilScriptedIntentCaseArtifactBundleRequest, IntentCaseArtifactKind,
    materialize_gerbil_scripted_intent_case_artifact_bundle,
};
use marlin_agent_kernel::{
    AgentFlowLoopProgramRuntimeHandoffExecutor, GenericLoopMachineReceipt,
    HybridLoopProgramRuntimeHandoffExecutor, LoopProgramEventMapper, LoopProgramExecutionDriver,
    LoopProgramExecutionRequest, LoopProgramRuntimeHandoffExecutionReceipt,
    LoopProgramRuntimeHandoffExecutionReportStatus, LoopProgramRuntimeHandoffHandler,
    LoopProgramRuntimeHandoffRouter, LoopProgramRuntimeHandoffRouterHandlers,
    LoopProgramRuntimeOwner, StaticLoopProgramRuntimeHandoffHandler,
};
use marlin_agent_protocol::LoopProgramEventKind;
use marlin_gerbil_scheme::{
    GerbilLoopCaseDriverVerticalTraceReceipt, project_gerbil_loop_case_driver_loop_event_kind,
    project_gerbil_loop_case_driver_loop_program, run_gerbil_config_interface_case_driver_smoke,
    verify_gerbil_loop_case_driver_vertical_trace,
};

#[test]
fn harness_materializes_scripted_intent_case_bundles_for_all_gerbil_vertical_cases() {
    let stdout = config_interface_case_driver_stdout();
    let vertical_receipts =
        verify_gerbil_loop_case_driver_vertical_trace(&stdout, 7).expect("vertical trace verifies");
    let output_root = tempfile::tempdir().expect("create intent-case artifact tempdir");

    for receipt in &vertical_receipts {
        let loop_program = project_gerbil_loop_case_driver_loop_program(receipt)
            .expect("vertical trace projects into LoopProgram");
        let driver = LoopProgramExecutionDriver::new(scheme_projected_runtime_executor())
            .with_event_mapper(SchemeProjectedLoopProgramEventMapper::from_vertical_trace(
                receipt,
            ))
            .with_max_steps(receipt.transition_count() + 2);
        let execution_receipt = driver.run(LoopProgramExecutionRequest::new(
            loop_program,
            vec![LoopProgramEventKind::Start],
        ));
        let bundle = materialize_gerbil_scripted_intent_case_artifact_bundle(
            GerbilScriptedIntentCaseArtifactBundleRequest {
                output_root: output_root.path().to_owned(),
                run_id: format!("scripted-bundle-{}", receipt.case_id().as_str()).into(),
                vertical_trace: receipt.clone(),
                execution_receipt,
            },
        )
        .expect("scripted intent-case bundle materializes");

        assert!(bundle.bundle_root.is_dir());
        assert!(bundle.manifest_path.is_file());
        assert!(bundle.manifest.has_core_artifact_bundle());
        assert!(bundle.has_artifact_kind(IntentCaseArtifactKind::Intent));
        assert!(bundle.has_artifact_kind(IntentCaseArtifactKind::PolicyPack));
        assert!(bundle.has_artifact_kind(IntentCaseArtifactKind::LoopProgram));
        assert!(bundle.has_artifact_kind(IntentCaseArtifactKind::VerticalTrace));
        assert!(bundle.has_artifact_kind(IntentCaseArtifactKind::ExecutionTrace));
        assert!(bundle.has_artifact_kind(IntentCaseArtifactKind::ReplayScript));
        assert_eq!(
            bundle.artifacts.len(),
            bundle
                .manifest
                .artifacts
                .iter()
                .filter(|artifact| artifact.present)
                .count()
        );

        for artifact in &bundle.artifacts {
            assert!(artifact.path.is_file(), "missing artifact {artifact:?}");
            assert_ne!(artifact.bytes_written, 0, "empty artifact {artifact:?}");
            let path = artifact.path.to_string_lossy();
            assert!(
                !path.ends_with(".json") && !path.ends_with(".jsonl"),
                "internal scripted bundle should not use JSON artifact paths: {path}"
            );
        }

        if receipt.tool_intent_count() > 0 {
            assert!(bundle.has_artifact_kind(IntentCaseArtifactKind::ToolCalls));
        }
        if receipt.memory_intent_count() > 0 {
            assert!(bundle.has_artifact_kind(IntentCaseArtifactKind::MemoryReceipts));
        }
        if receipt.live_llm_required() {
            assert!(bundle.has_artifact_kind(IntentCaseArtifactKind::ModelEvents));
            assert!(bundle.has_artifact_kind(IntentCaseArtifactKind::DiffPatch));
            assert!(bundle.has_artifact_kind(IntentCaseArtifactKind::TestBefore));
            assert!(bundle.has_artifact_kind(IntentCaseArtifactKind::TestAfter));
        }
    }
}

fn config_interface_case_driver_stdout() -> String {
    static STDOUT: OnceLock<String> = OnceLock::new();

    STDOUT
        .get_or_init(|| {
            run_gerbil_config_interface_case_driver_smoke()
                .expect("gxi case-driver smoke should produce verified stdout")
        })
        .clone()
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SchemeProjectedLoopProgramEventMapper {
    events: Box<[LoopProgramEventKind]>,
}

impl SchemeProjectedLoopProgramEventMapper {
    fn from_vertical_trace(receipt: &GerbilLoopCaseDriverVerticalTraceReceipt) -> Self {
        Self {
            events: receipt
                .transition_events()
                .map(|event| {
                    project_gerbil_loop_case_driver_loop_event_kind(event)
                        .expect("Scheme vertical event should project")
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        }
    }
}

impl LoopProgramEventMapper for SchemeProjectedLoopProgramEventMapper {
    fn next_event(
        &self,
        machine_receipt: &GenericLoopMachineReceipt,
        runtime_handoff_execution: &LoopProgramRuntimeHandoffExecutionReceipt,
    ) -> Option<LoopProgramEventKind> {
        if runtime_handoff_execution.status
            == LoopProgramRuntimeHandoffExecutionReportStatus::Denied
        {
            return Some(LoopProgramEventKind::Error);
        }
        if runtime_handoff_execution.status
            != LoopProgramRuntimeHandoffExecutionReportStatus::Completed
        {
            return None;
        }

        self.events
            .get(machine_receipt.step_index.get() as usize)
            .cloned()
    }
}

fn scheme_projected_runtime_executor() -> HybridLoopProgramRuntimeHandoffExecutor {
    let handlers = LoopProgramRuntimeHandoffRouterHandlers {
        model_handler: handled_by("scheme.projected.model"),
        graph_handler: handled_by("scheme.projected.graph"),
        verification_handler: handled_by("scheme.projected.verification"),
        control_handler: handled_by("scheme.projected.control"),
        ..LoopProgramRuntimeHandoffRouterHandlers::default()
    };

    HybridLoopProgramRuntimeHandoffExecutor::new(
        LoopProgramRuntimeHandoffRouter::new(handlers),
        AgentFlowLoopProgramRuntimeHandoffExecutor::new(LoopProgramRuntimeOwner::new(
            "scheme.projected.agent-flow",
        )),
    )
}

fn handled_by(owner: &'static str) -> Arc<dyn LoopProgramRuntimeHandoffHandler> {
    Arc::new(StaticLoopProgramRuntimeHandoffHandler::handled(
        LoopProgramRuntimeOwner::new(owner),
    ))
}
