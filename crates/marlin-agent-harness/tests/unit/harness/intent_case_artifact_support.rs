use std::{
    fs,
    sync::{Arc, OnceLock},
};

use marlin_agent_harness::{
    IntentCaseArtifactBundleMaterializationReceipt, IntentCaseArtifactKind,
    IntentCaseObservedSpanSource, IntentCaseSpanName, TraceRecorder,
};
use marlin_agent_kernel::{
    AgentFlowLoopProgramRuntimeHandoffExecutor, GenericLoopMachineReceipt,
    HybridLoopProgramRuntimeHandoffExecutor, LoopProgramEventMapper, LoopProgramExecutionDriver,
    LoopProgramExecutionReceipt, LoopProgramExecutionRequest,
    LoopProgramRuntimeHandoffExecutionReceipt, LoopProgramRuntimeHandoffExecutionReportStatus,
    LoopProgramRuntimeHandoffHandler, LoopProgramRuntimeHandoffRouter,
    LoopProgramRuntimeHandoffRouterHandlers, LoopProgramRuntimeOwner,
    LoopProgramToolProcessCommandTemplate, LoopProgramToolProcessProgram,
    StaticLoopProgramRuntimeHandoffHandler, StaticLoopProgramToolProcessResolver,
};
use marlin_agent_protocol::LoopProgramEventKind;
use marlin_gerbil_scheme::{
    GerbilLoopCaseDriverCapability, GerbilLoopCaseDriverVerticalTraceReceipt,
    project_gerbil_loop_case_driver_intent_case_artifact_manifest,
    project_gerbil_loop_case_driver_loop_event_kind, project_gerbil_loop_case_driver_loop_program,
    run_gerbil_config_interface_case_driver_smoke, verify_gerbil_loop_case_driver_vertical_trace,
};

pub(crate) fn config_interface_case_driver_stdout() -> String {
    static STDOUT: OnceLock<String> = OnceLock::new();

    STDOUT
        .get_or_init(|| {
            run_gerbil_config_interface_case_driver_smoke()
                .expect("gxi case-driver smoke should produce verified stdout")
        })
        .clone()
}

pub(crate) fn gerbil_vertical_receipts() -> Vec<GerbilLoopCaseDriverVerticalTraceReceipt> {
    verify_gerbil_loop_case_driver_vertical_trace(&config_interface_case_driver_stdout(), 7)
        .expect("vertical trace verifies")
}

pub(crate) fn observed_span_source_for_vertical_receipt(
    receipt: &GerbilLoopCaseDriverVerticalTraceReceipt,
) -> IntentCaseObservedSpanSource {
    let manifest = project_gerbil_loop_case_driver_intent_case_artifact_manifest(
        receipt,
        "expected-span-probe",
    );
    IntentCaseObservedSpanSource::new(manifest.expected_span_names())
}

pub(crate) fn observed_span_source_for_vertical_receipt_with_trace(
    receipt: &GerbilLoopCaseDriverVerticalTraceReceipt,
    trace_recorder: &TraceRecorder,
) -> IntentCaseObservedSpanSource {
    let manifest = project_gerbil_loop_case_driver_intent_case_artifact_manifest(
        receipt,
        "expected-span-probe",
    );
    IntentCaseObservedSpanSource::new(
        manifest.expected_span_names().into_iter().chain(
            trace_recorder
                .span_names()
                .into_iter()
                .map(|span_name| IntentCaseSpanName::new(span_name.as_str())),
        ),
    )
}

pub(crate) fn execute_vertical_receipt(
    receipt: &GerbilLoopCaseDriverVerticalTraceReceipt,
) -> LoopProgramExecutionReceipt {
    let loop_program = project_gerbil_loop_case_driver_loop_program(receipt)
        .expect("vertical trace projects into LoopProgram");
    LoopProgramExecutionDriver::new(scheme_projected_runtime_executor())
        .with_event_mapper(SchemeProjectedLoopProgramEventMapper::from_vertical_trace(
            receipt,
        ))
        .with_max_steps(receipt.transition_count() + 2)
        .run(LoopProgramExecutionRequest::new(
            loop_program,
            vec![LoopProgramEventKind::Start],
        ))
}

pub(crate) fn artifact_content(
    bundle: &IntentCaseArtifactBundleMaterializationReceipt,
    kind: IntentCaseArtifactKind,
) -> String {
    let artifact = bundle
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == kind)
        .expect("artifact kind materialized");
    fs::read_to_string(&artifact.path).expect("read materialized artifact")
}

pub(crate) fn cap(tag: impl Into<String>) -> GerbilLoopCaseDriverCapability {
    GerbilLoopCaseDriverCapability::new(tag)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SchemeProjectedLoopProgramEventMapper {
    events: Box<[LoopProgramEventKind]>,
}

impl SchemeProjectedLoopProgramEventMapper {
    pub(crate) fn from_vertical_trace(receipt: &GerbilLoopCaseDriverVerticalTraceReceipt) -> Self {
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

pub(crate) fn scheme_projected_runtime_executor() -> HybridLoopProgramRuntimeHandoffExecutor {
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

pub(crate) fn handled_by(owner: &'static str) -> Arc<dyn LoopProgramRuntimeHandoffHandler> {
    Arc::new(StaticLoopProgramRuntimeHandoffHandler::handled(
        LoopProgramRuntimeOwner::new(owner),
    ))
}

pub(crate) fn tool_shell_resolver(script: &'static str) -> StaticLoopProgramToolProcessResolver {
    StaticLoopProgramToolProcessResolver::new(
        vec![
            LoopProgramToolProcessCommandTemplate::new(
                "agent-flow.tool-intent",
                ["loop-program.dispatch-tools"],
                LoopProgramToolProcessProgram::new("sh"),
            )
            .with_args(["-c", script]),
        ]
        .into_boxed_slice(),
    )
}
