use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use marlin_agent_kernel::{
    AgentFlowLoopProgramRuntimeHandoffExecutor, DenylistedLoopProgramToolDispatchHandler,
    GenericLoopMachineReceipt, HybridLoopProgramRuntimeHandoffExecutor,
    LoopProgramDerivedSessionPolicyStatus, LoopProgramEventMapper, LoopProgramExecutionDriver,
    LoopProgramExecutionReceipt, LoopProgramExecutionRequest, LoopProgramExecutionStatus,
    LoopProgramFileSandbox, LoopProgramFileWriteSideEffectStatus, LoopProgramFileWriteTemplate,
    LoopProgramRuntimeHandoffExecutionReceipt, LoopProgramRuntimeHandoffExecutionReportStatus,
    LoopProgramRuntimeHandoffExecutionStatus, LoopProgramRuntimeHandoffExecutor,
    LoopProgramRuntimeHandoffHandler, LoopProgramRuntimeHandoffPlan,
    LoopProgramRuntimeHandoffRouter, LoopProgramRuntimeHandoffRouterHandlers,
    LoopProgramRuntimeOwner, LoopProgramRuntimeSideEffectExecutor,
    LoopProgramRuntimeSideEffectStatus, LoopProgramToolProcessCommandTemplate,
    LoopProgramToolProcessProgram, LoopProgramToolProcessSideEffectStatus,
    PolicyGatedAgentFlowLoopProgramRuntimeHandoffExecutor, ReceiptDrivenLoopProgramEventMapper,
    RetryBudgetToolHandler, ScriptedLoopProgramEventMapper, StaticLoopProgramFileWriteResolver,
    StaticLoopProgramRuntimeHandoffHandler, StaticLoopProgramToolProcessResolver,
};
use marlin_agent_protocol::{
    AgentFlowIntent, AgentFlowMemoryOperation, LoopMechanismPolicyId, LoopPolicyDigest,
    LoopPolicyEpoch, LoopProgram, LoopProgramActionKind, LoopProgramEventKind, LoopProgramId,
    LoopProgramInput, LoopProgramStateId, LoopProgramTransition, LoopProgramTransitionId,
    ModelEndpoint, ModelGateway, ModelGatewayCompletionChoice, ModelGatewayCompletionResponse,
    ModelGatewayError, ModelGatewayFuture, ModelGatewayMessageRole, ModelGatewayRequest,
    ModelGatewayResult, assistant_gateway_message, system_gateway_message, user_gateway_message,
};
use marlin_agent_runtime::{RuntimeEdgeModelGateway, RuntimeEdgePolicy, TokioAgentRuntime};

fn loop_script() -> impl LoopProgramEventMapper {
    ScriptedLoopProgramEventMapper::new(
        vec![
            (
                LoopProgramActionKind::InvokeModel,
                LoopProgramEventKind::ToolRequest,
            ),
            (
                LoopProgramActionKind::DispatchTools,
                LoopProgramEventKind::ToolReceipt,
            ),
            (
                LoopProgramActionKind::Continue,
                LoopProgramEventKind::ModelEvent,
            ),
            (
                LoopProgramActionKind::RewriteGraph,
                LoopProgramEventKind::RuntimeReceipt,
            ),
            (
                LoopProgramActionKind::Verify,
                LoopProgramEventKind::VerificationReceipt,
            ),
        ]
        .into_boxed_slice(),
    )
}

fn handled_by(owner: &'static str) -> Arc<dyn LoopProgramRuntimeHandoffHandler> {
    Arc::new(StaticLoopProgramRuntimeHandoffHandler::handled(
        LoopProgramRuntimeOwner::new(owner),
    ))
}

fn handled_by_with_event(
    owner: &'static str,
    next_event: LoopProgramEventKind,
) -> Arc<dyn LoopProgramRuntimeHandoffHandler> {
    Arc::new(
        StaticLoopProgramRuntimeHandoffHandler::handled_with_next_event(
            LoopProgramRuntimeOwner::new(owner),
            next_event,
        ),
    )
}

fn denied_by(owner: &'static str) -> Arc<dyn LoopProgramRuntimeHandoffHandler> {
    Arc::new(StaticLoopProgramRuntimeHandoffHandler::denied(
        LoopProgramRuntimeOwner::new(owner),
    ))
}

#[derive(Clone, Debug)]
struct StaticRepairGateway {
    repair_intent: String,
    requests: Arc<Mutex<Vec<ModelGatewayRequest>>>,
}

impl StaticRepairGateway {
    fn new(repair_intent: impl Into<String>) -> Self {
        Self {
            repair_intent: repair_intent.into(),
            requests: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn requests(&self) -> Vec<ModelGatewayRequest> {
        self.requests.lock().expect("gateway requests").clone()
    }
}

impl ModelGateway for StaticRepairGateway {
    fn complete(
        &self,
        request: ModelGatewayRequest,
    ) -> ModelGatewayFuture<ModelGatewayResult<ModelGatewayCompletionResponse>> {
        let repair_intent = self.repair_intent.clone();
        let requests = Arc::clone(&self.requests);
        Box::pin(async move {
            request
                .endpoint()
                .validate_contract()
                .map_err(ModelGatewayError::EndpointContract)?;
            let model = request.endpoint().litellm_model_id().as_str().to_owned();
            requests.lock().expect("gateway requests").push(request);
            Ok(ModelGatewayCompletionResponse::new(
                "runtime-repair-completion",
                model,
                vec![ModelGatewayCompletionChoice::new(
                    0,
                    assistant_gateway_message(repair_intent),
                    Some("stop".to_owned()),
                )],
            ))
        })
    }
}

#[derive(Clone)]
struct GatewayRepairDecisionMapper {
    gateway: Arc<dyn ModelGateway>,
    endpoint: ModelEndpoint,
    completion_receipts: Arc<Mutex<Vec<ModelGatewayCompletionResponse>>>,
    tool_event: Option<LoopProgramEventKind>,
    verification_event: Option<LoopProgramEventKind>,
}

impl GatewayRepairDecisionMapper {
    fn new(gateway: Arc<dyn ModelGateway>) -> Self {
        Self {
            gateway,
            endpoint: ModelEndpoint::new("openai", "gpt-5-mini"),
            completion_receipts: Arc::new(Mutex::new(Vec::new())),
            tool_event: None,
            verification_event: None,
        }
    }

    fn with_tool_event(mut self, event: LoopProgramEventKind) -> Self {
        self.tool_event = Some(event);
        self
    }

    fn with_verification_event(mut self, event: LoopProgramEventKind) -> Self {
        self.verification_event = Some(event);
        self
    }

    fn completion_receipts(&self) -> Arc<Mutex<Vec<ModelGatewayCompletionResponse>>> {
        Arc::clone(&self.completion_receipts)
    }
}

impl LoopProgramEventMapper for GatewayRepairDecisionMapper {
    fn next_event(
        &self,
        machine_receipt: &GenericLoopMachineReceipt,
        runtime_handoff_execution: &LoopProgramRuntimeHandoffExecutionReceipt,
    ) -> Option<LoopProgramEventKind> {
        if runtime_handoff_execution.status
            != LoopProgramRuntimeHandoffExecutionReportStatus::Completed
        {
            return None;
        }

        match machine_receipt.action {
            LoopProgramActionKind::InvokeModel => {
                let request = ModelGatewayRequest::new(
                    self.endpoint.clone(),
                    vec![
                        system_gateway_message("runtime repair no-write repair planner"),
                        user_gateway_message(
                            "Fix a single-file bug by selecting a typed patch intent.",
                        ),
                    ],
                );
                let response = complete_gateway_synchronously(self.gateway.as_ref(), request)
                    .expect("repair gateway completion");
                let repair_text = response
                    .choices
                    .first()
                    .map(|choice| choice.message.content.as_str())
                    .unwrap_or_default()
                    .to_owned();
                self.completion_receipts
                    .lock()
                    .expect("completion receipts")
                    .push(response);
                repair_text
                    .contains("PATCH_INTENT:single-file-add-one")
                    .then_some(LoopProgramEventKind::ModelEvent)
                    .or(Some(LoopProgramEventKind::Error))
            }
            LoopProgramActionKind::DispatchTools => self.tool_event.clone(),
            LoopProgramActionKind::Verify => self.verification_event.clone(),
            _ => None,
        }
    }
}

fn complete_gateway_synchronously(
    gateway: &dyn ModelGateway,
    request: ModelGatewayRequest,
) -> ModelGatewayResult<ModelGatewayCompletionResponse> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("current-thread runtime")
        .block_on(gateway.complete(request))
}

#[derive(Clone)]
struct RealRepairPolicyGatedHandoffExecutor {
    inner: PolicyGatedAgentFlowLoopProgramRuntimeHandoffExecutor,
}

impl RealRepairPolicyGatedHandoffExecutor {
    fn new() -> Self {
        let handlers = LoopProgramRuntimeHandoffRouterHandlers {
            model_handler: handled_by("runtime.model.gateway.repair-planner"),
            tool_handler: handled_by("runtime.policy.repair-tool-admission"),
            graph_handler: handled_by("runtime.graph.dynamic-rewrite"),
            verification_handler: handled_by("runtime.verification.single-file"),
            control_handler: handled_by("runtime.control"),
            ..LoopProgramRuntimeHandoffRouterHandlers::default()
        };
        Self {
            inner: PolicyGatedAgentFlowLoopProgramRuntimeHandoffExecutor::new(
                LoopProgramRuntimeHandoffRouter::new(handlers),
                AgentFlowLoopProgramRuntimeHandoffExecutor::new(LoopProgramRuntimeOwner::new(
                    "runtime.agent-flow.repair-tool",
                )),
            ),
        }
    }
}

impl LoopProgramRuntimeHandoffExecutor for RealRepairPolicyGatedHandoffExecutor {
    fn execute_plan(
        &self,
        plan: &LoopProgramRuntimeHandoffPlan,
    ) -> LoopProgramRuntimeHandoffExecutionReceipt {
        self.inner.execute_plan(plan)
    }
}

#[derive(Clone, Debug)]
struct MemoryRecallDecisionMapper;

impl LoopProgramEventMapper for MemoryRecallDecisionMapper {
    fn next_event(
        &self,
        machine_receipt: &GenericLoopMachineReceipt,
        runtime_handoff_execution: &LoopProgramRuntimeHandoffExecutionReceipt,
    ) -> Option<LoopProgramEventKind> {
        if runtime_handoff_execution.status
            != LoopProgramRuntimeHandoffExecutionReportStatus::Completed
        {
            return None;
        }

        match machine_receipt.action {
            LoopProgramActionKind::ReadMemory
                if runtime_handoff_execution
                    .memory_projections
                    .iter()
                    .any(|projection| {
                        projection.intent.operation == AgentFlowMemoryOperation::Recall
                    }) =>
            {
                Some(LoopProgramEventKind::ToolRequest)
            }
            LoopProgramActionKind::DispatchTools => Some(LoopProgramEventKind::ToolReceipt),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
struct PolicyCombinationDecisionMapper;

impl LoopProgramEventMapper for PolicyCombinationDecisionMapper {
    fn next_event(
        &self,
        machine_receipt: &GenericLoopMachineReceipt,
        runtime_handoff_execution: &LoopProgramRuntimeHandoffExecutionReceipt,
    ) -> Option<LoopProgramEventKind> {
        if runtime_handoff_execution.status
            != LoopProgramRuntimeHandoffExecutionReportStatus::Completed
        {
            return None;
        }

        match machine_receipt.action {
            LoopProgramActionKind::ReadMemory
                if has_memory_recall_intent(runtime_handoff_execution) =>
            {
                Some(LoopProgramEventKind::RuntimeReceipt)
            }
            LoopProgramActionKind::InvokeModel => Some(LoopProgramEventKind::ModelEvent),
            LoopProgramActionKind::RewriteGraph => Some(LoopProgramEventKind::RuntimeReceipt),
            LoopProgramActionKind::DispatchTools => Some(LoopProgramEventKind::ToolReceipt),
            LoopProgramActionKind::Verify => Some(LoopProgramEventKind::VerificationReceipt),
            _ => None,
        }
    }
}

fn has_memory_recall_intent(
    runtime_handoff_execution: &LoopProgramRuntimeHandoffExecutionReceipt,
) -> bool {
    runtime_handoff_execution
        .executions
        .iter()
        .filter_map(|execution| execution.agent_flow_intent.as_ref())
        .any(|intent| {
            matches!(
                intent,
                AgentFlowIntent::Memory(memory_intent)
                    if memory_intent.operation == AgentFlowMemoryOperation::Recall
            )
        })
}

fn sample_loop_program() -> LoopProgram {
    LoopProgram::new(LoopProgramInput {
        program_id: LoopProgramId::new("execution-driver-loop"),
        policy_epoch: LoopPolicyEpoch::new(8),
        policy_digest: LoopPolicyDigest::from_bytes([7_u8; 32]),
        mechanism_policies: vec![LoopMechanismPolicyId::new("reactive-tool-loop-base")]
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

fn runtime_repair_no_write_llm_loop_program() -> LoopProgram {
    LoopProgram::new(LoopProgramInput {
        program_id: LoopProgramId::new("runtime-repair-no-write-llm"),
        policy_epoch: LoopPolicyEpoch::new(16),
        policy_digest: LoopPolicyDigest::from_bytes([16_u8; 32]),
        mechanism_policies: vec![
            LoopMechanismPolicyId::new("runtime-repair"),
            LoopMechanismPolicyId::new("llm-repair-no-write"),
        ]
        .into_boxed_slice(),
        initial_state: LoopProgramStateId::new("start"),
        transitions: vec![
            transition(
                "start-llm-repair",
                "start",
                LoopProgramEventKind::Start,
                LoopProgramActionKind::InvokeModel,
                "llm-planned",
            ),
            transition(
                "llm-plan-stop",
                "llm-planned",
                LoopProgramEventKind::ModelEvent,
                LoopProgramActionKind::Stop,
                "stopped",
            ),
        ]
        .into_boxed_slice(),
    })
}

fn runtime_repair_single_file_loop_program() -> LoopProgram {
    LoopProgram::new(LoopProgramInput {
        program_id: LoopProgramId::new("runtime-repair-single-file"),
        policy_epoch: LoopPolicyEpoch::new(17),
        policy_digest: LoopPolicyDigest::from_bytes([17_u8; 32]),
        mechanism_policies: vec![
            LoopMechanismPolicyId::new("runtime-repair"),
            LoopMechanismPolicyId::new("llm-repair"),
            LoopMechanismPolicyId::new("tool-sandbox"),
            LoopMechanismPolicyId::new("verification-gate"),
        ]
        .into_boxed_slice(),
        initial_state: LoopProgramStateId::new("start"),
        transitions: vec![
            transition(
                "start-llm-repair",
                "start",
                LoopProgramEventKind::Start,
                LoopProgramActionKind::InvokeModel,
                "llm-planned",
            ),
            transition(
                "llm-plan-tool",
                "llm-planned",
                LoopProgramEventKind::ModelEvent,
                LoopProgramActionKind::DispatchTools,
                "await-tool",
            ),
            transition(
                "tool-verify",
                "await-tool",
                LoopProgramEventKind::ToolReceipt,
                LoopProgramActionKind::Verify,
                "await-verification",
            ),
            transition(
                "verify-stop",
                "await-verification",
                LoopProgramEventKind::VerificationReceipt,
                LoopProgramActionKind::Stop,
                "stopped",
            ),
        ]
        .into_boxed_slice(),
    })
}

fn unique_temp_repair_workspace() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "marlin-runtime-repair-workspace-{}-{nanos}",
        std::process::id()
    ))
}

fn single_tool_dispatch_error_stop_loop_program() -> LoopProgram {
    LoopProgram::new(LoopProgramInput {
        program_id: LoopProgramId::new("kernel-fixture-tool-dispatch-error-stop"),
        policy_epoch: LoopPolicyEpoch::new(10),
        policy_digest: LoopPolicyDigest::from_bytes([10_u8; 32]),
        mechanism_policies: vec![
            LoopMechanismPolicyId::new("kernel-fixture-tool-dispatch-error"),
            LoopMechanismPolicyId::new("agent-flow-tool-projection"),
        ]
        .into_boxed_slice(),
        initial_state: LoopProgramStateId::new("start"),
        transitions: vec![
            transition(
                "start-tool",
                "start",
                LoopProgramEventKind::Start,
                LoopProgramActionKind::DispatchTools,
                "await-tool",
            ),
            transition(
                "tool-denied-stop",
                "await-tool",
                LoopProgramEventKind::Error,
                LoopProgramActionKind::Stop,
                "stopped",
            ),
        ]
        .into_boxed_slice(),
    })
}

fn single_tool_dispatch_receipt_stop_loop_program() -> LoopProgram {
    LoopProgram::new(LoopProgramInput {
        program_id: LoopProgramId::new("kernel-fixture-tool-dispatch-receipt-stop"),
        policy_epoch: LoopPolicyEpoch::new(10),
        policy_digest: LoopPolicyDigest::from_bytes([10_u8; 32]),
        mechanism_policies: vec![
            LoopMechanismPolicyId::new("kernel-fixture-tool-dispatch-receipt"),
            LoopMechanismPolicyId::new("agent-flow-tool-projection"),
        ]
        .into_boxed_slice(),
        initial_state: LoopProgramStateId::new("start"),
        transitions: vec![
            transition(
                "start-tool",
                "start",
                LoopProgramEventKind::Start,
                LoopProgramActionKind::DispatchTools,
                "await-tool",
            ),
            transition(
                "tool-stop",
                "await-tool",
                LoopProgramEventKind::ToolReceipt,
                LoopProgramActionKind::Stop,
                "stopped",
            ),
        ]
        .into_boxed_slice(),
    })
}

fn two_attempt_tool_dispatch_loop_program() -> LoopProgram {
    LoopProgram::new(LoopProgramInput {
        program_id: LoopProgramId::new("kernel-fixture-two-attempt-tool-dispatch"),
        policy_epoch: LoopPolicyEpoch::new(11),
        policy_digest: LoopPolicyDigest::from_bytes([11_u8; 32]),
        mechanism_policies: vec![
            LoopMechanismPolicyId::new("kernel-fixture-two-attempt-dispatch"),
            LoopMechanismPolicyId::new("agent-flow-tool-projection"),
        ]
        .into_boxed_slice(),
        initial_state: LoopProgramStateId::new("start"),
        transitions: vec![
            transition(
                "start-tool",
                "start",
                LoopProgramEventKind::Start,
                LoopProgramActionKind::DispatchTools,
                "await-tool",
            ),
            transition(
                "tool-error-retry",
                "await-tool",
                LoopProgramEventKind::Error,
                LoopProgramActionKind::DispatchTools,
                "await-tool-retry",
            ),
            transition(
                "retry-tool-stop",
                "await-tool-retry",
                LoopProgramEventKind::ToolReceipt,
                LoopProgramActionKind::Stop,
                "stopped",
            ),
        ]
        .into_boxed_slice(),
    })
}

fn model_then_verify_loop_program() -> LoopProgram {
    LoopProgram::new(LoopProgramInput {
        program_id: LoopProgramId::new("kernel-fixture-model-then-verify"),
        policy_epoch: LoopPolicyEpoch::new(12),
        policy_digest: LoopPolicyDigest::from_bytes([12_u8; 32]),
        mechanism_policies: vec![LoopMechanismPolicyId::new("kernel-fixture-model-verify")]
            .into_boxed_slice(),
        initial_state: LoopProgramStateId::new("start"),
        transitions: vec![
            transition(
                "start-maker",
                "start",
                LoopProgramEventKind::Start,
                LoopProgramActionKind::InvokeModel,
                "await-maker",
            ),
            transition(
                "maker-checker",
                "await-maker",
                LoopProgramEventKind::ModelEvent,
                LoopProgramActionKind::Verify,
                "await-checker",
            ),
            transition(
                "checker-stop",
                "await-checker",
                LoopProgramEventKind::VerificationReceipt,
                LoopProgramActionKind::Stop,
                "stopped",
            ),
        ]
        .into_boxed_slice(),
    })
}

fn rewrite_tool_verify_loop_program() -> LoopProgram {
    LoopProgram::new(LoopProgramInput {
        program_id: LoopProgramId::new("kernel-fixture-rewrite-tool-verify"),
        policy_epoch: LoopPolicyEpoch::new(13),
        policy_digest: LoopPolicyDigest::from_bytes([13_u8; 32]),
        mechanism_policies: vec![
            LoopMechanismPolicyId::new("kernel-fixture-rewrite-tool"),
            LoopMechanismPolicyId::new("verification-gate"),
        ]
        .into_boxed_slice(),
        initial_state: LoopProgramStateId::new("start"),
        transitions: vec![
            transition(
                "start-rewrite",
                "start",
                LoopProgramEventKind::Start,
                LoopProgramActionKind::RewriteGraph,
                "rewritten",
            ),
            transition(
                "rewrite-tool",
                "rewritten",
                LoopProgramEventKind::RuntimeReceipt,
                LoopProgramActionKind::DispatchTools,
                "await-tool",
            ),
            transition(
                "tool-verify",
                "await-tool",
                LoopProgramEventKind::ToolReceipt,
                LoopProgramActionKind::Verify,
                "await-verification",
            ),
            transition(
                "verify-stop",
                "await-verification",
                LoopProgramEventKind::VerificationReceipt,
                LoopProgramActionKind::Stop,
                "stopped",
            ),
        ]
        .into_boxed_slice(),
    })
}

fn memory_recall_then_tool_loop_program() -> LoopProgram {
    LoopProgram::new(LoopProgramInput {
        program_id: LoopProgramId::new("kernel-fixture-memory-then-tool"),
        policy_epoch: LoopPolicyEpoch::new(14),
        policy_digest: LoopPolicyDigest::from_bytes([14_u8; 32]),
        mechanism_policies: vec![
            LoopMechanismPolicyId::new("kernel-fixture-memory-recall"),
            LoopMechanismPolicyId::new("agent-flow-memory-projection"),
        ]
        .into_boxed_slice(),
        initial_state: LoopProgramStateId::new("start"),
        transitions: vec![
            transition(
                "start-memory",
                "start",
                LoopProgramEventKind::Start,
                LoopProgramActionKind::ReadMemory,
                "memory-ready",
            ),
            transition(
                "memory-tool",
                "memory-ready",
                LoopProgramEventKind::ToolRequest,
                LoopProgramActionKind::DispatchTools,
                "await-tool",
            ),
            transition(
                "tool-stop",
                "await-tool",
                LoopProgramEventKind::ToolReceipt,
                LoopProgramActionKind::Stop,
                "stopped",
            ),
        ]
        .into_boxed_slice(),
    })
}

fn memory_model_rewrite_tool_verify_loop_program() -> LoopProgram {
    LoopProgram::new(LoopProgramInput {
        program_id: LoopProgramId::new("kernel-fixture-memory-model-rewrite-tool-verify"),
        policy_epoch: LoopPolicyEpoch::new(15),
        policy_digest: LoopPolicyDigest::from_bytes([15_u8; 32]),
        mechanism_policies: vec![
            LoopMechanismPolicyId::new("kernel-fixture-model-verify"),
            LoopMechanismPolicyId::new("kernel-fixture-rewrite-tool"),
            LoopMechanismPolicyId::new("kernel-fixture-memory-recall"),
        ]
        .into_boxed_slice(),
        initial_state: LoopProgramStateId::new("start"),
        transitions: vec![
            transition(
                "start-memory",
                "start",
                LoopProgramEventKind::Start,
                LoopProgramActionKind::ReadMemory,
                "memory-ready",
            ),
            transition(
                "memory-maker",
                "memory-ready",
                LoopProgramEventKind::RuntimeReceipt,
                LoopProgramActionKind::InvokeModel,
                "await-maker",
            ),
            transition(
                "maker-rewrite",
                "await-maker",
                LoopProgramEventKind::ModelEvent,
                LoopProgramActionKind::RewriteGraph,
                "rewritten",
            ),
            transition(
                "rewrite-tool",
                "rewritten",
                LoopProgramEventKind::RuntimeReceipt,
                LoopProgramActionKind::DispatchTools,
                "await-tool",
            ),
            transition(
                "tool-checker",
                "await-tool",
                LoopProgramEventKind::ToolReceipt,
                LoopProgramActionKind::Verify,
                "await-checker",
            ),
            transition(
                "checker-stop",
                "await-checker",
                LoopProgramEventKind::VerificationReceipt,
                LoopProgramActionKind::Stop,
                "stopped",
            ),
        ]
        .into_boxed_slice(),
    })
}

fn transition(
    transition_id: &'static str,
    from: &'static str,
    event: LoopProgramEventKind,
    action: LoopProgramActionKind,
    to: &'static str,
) -> LoopProgramTransition {
    LoopProgramTransition {
        transition_id: LoopProgramTransitionId::new(transition_id),
        from: LoopProgramStateId::new(from),
        event,
        action,
        to: LoopProgramStateId::new(to),
    }
}

fn tool_error_loop_program() -> LoopProgram {
    LoopProgram::new(LoopProgramInput {
        program_id: LoopProgramId::new("execution-driver-tool-error-loop"),
        policy_epoch: LoopPolicyEpoch::new(9),
        policy_digest: LoopPolicyDigest::from_bytes([8_u8; 32]),
        mechanism_policies: vec![LoopMechanismPolicyId::new("tool-error-route")].into_boxed_slice(),
        initial_state: LoopProgramStateId::new("start"),
        transitions: vec![
            LoopProgramTransition {
                transition_id: LoopProgramTransitionId::new("start-tool"),
                from: LoopProgramStateId::new("start"),
                event: LoopProgramEventKind::Start,
                action: LoopProgramActionKind::DispatchTools,
                to: LoopProgramStateId::new("await-tool"),
            },
            LoopProgramTransition {
                transition_id: LoopProgramTransitionId::new("tool-error-stop"),
                from: LoopProgramStateId::new("await-tool"),
                event: LoopProgramEventKind::Error,
                action: LoopProgramActionKind::Stop,
                to: LoopProgramStateId::new("stopped"),
            },
        ]
        .into_boxed_slice(),
    })
}

#[path = "driver.rs"]
mod driver;
#[path = "runtime_policy.rs"]
mod runtime_policy;
#[path = "runtime_repair.rs"]
mod runtime_repair;
