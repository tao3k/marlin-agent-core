use std::{
    fmt::{Debug, Write as _},
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use marlin_agent_kernel::{
    AgentFlowLoopProgramRuntimeHandoffExecutor, DenylistedLoopProgramToolDispatchHandler,
    GenericLoopMachineReceipt, HybridLoopProgramRuntimeHandoffExecutor,
    LoopProgramDerivedSessionPolicyStatus, LoopProgramEventMapper, LoopProgramExecutionDriver,
    LoopProgramExecutionReceipt, LoopProgramExecutionRequest, LoopProgramExecutionStatus,
    LoopProgramFileSandbox, LoopProgramFileWriteSideEffectStatus, LoopProgramFileWriteTemplate,
    LoopProgramRuntimeHandoff, LoopProgramRuntimeHandoffExecution,
    LoopProgramRuntimeHandoffExecutionReceipt, LoopProgramRuntimeHandoffExecutionReportStatus,
    LoopProgramRuntimeHandoffExecutionStatus, LoopProgramRuntimeHandoffExecutor,
    LoopProgramRuntimeHandoffHandler, LoopProgramRuntimeHandoffPlan,
    LoopProgramRuntimeHandoffRouter, LoopProgramRuntimeHandoffRouterHandlers,
    LoopProgramRuntimeOwner, LoopProgramRuntimeSideEffectExecutor,
    LoopProgramRuntimeSideEffectStatus, LoopProgramToolProcessCommandTemplate,
    LoopProgramToolProcessProgram, LoopProgramToolProcessSideEffectStatus,
    LoopProgramToolProcessSpawnRequest, PolicyGatedAgentFlowLoopProgramRuntimeHandoffExecutor,
    ReceiptDrivenLoopProgramEventMapper, ScriptedLoopProgramEventMapper,
    StaticLoopProgramFileWriteResolver, StaticLoopProgramRuntimeHandoffHandler,
    StaticLoopProgramToolProcessResolver, spawn_loop_program_tool_process,
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
                "real-repair-001-completion",
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
                        system_gateway_message("real-repair-001 no-write repair planner"),
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

#[derive(Debug)]
struct RetryBudgetToolHandler {
    owner: LoopProgramRuntimeOwner,
    denied_attempts_before_success: usize,
    attempts: AtomicUsize,
}

impl RetryBudgetToolHandler {
    fn new(owner: LoopProgramRuntimeOwner, denied_attempts_before_success: usize) -> Self {
        Self {
            owner,
            denied_attempts_before_success,
            attempts: AtomicUsize::new(0),
        }
    }
}

impl LoopProgramRuntimeHandoffHandler for RetryBudgetToolHandler {
    fn handle(&self, handoff: &LoopProgramRuntimeHandoff) -> LoopProgramRuntimeHandoffExecution {
        let attempt = self.attempts.fetch_add(1, Ordering::SeqCst);
        if attempt < self.denied_attempts_before_success {
            LoopProgramRuntimeHandoffExecution::denied(self.owner.clone(), handoff)
        } else {
            LoopProgramRuntimeHandoffExecution::handled(self.owner.clone(), handoff)
        }
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

#[derive(Clone, Debug, Eq, PartialEq)]
struct RealPolicyImprovementRecommendation {
    priority: &'static str,
    target: &'static str,
    evidence: &'static str,
    action: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RealPolicyExperimentReceipt {
    case_id: &'static str,
    program_id: String,
    policy_ids: Box<[String]>,
    policy_digest: String,
    loop_program_digest: String,
    runtime_behavior_digest: String,
    receipt_digest: String,
    status: LoopProgramExecutionStatus,
    action_path: Box<[LoopProgramActionKind]>,
    owner_path: Box<[String]>,
    generated_events: Box<[Option<LoopProgramEventKind>]>,
    denied_handoff_count: usize,
    handled_handoff_count: usize,
    agent_flow_intent_count: usize,
    tool_projection_count: usize,
    memory_projection_count: usize,
    improvement_recommendations: Box<[RealPolicyImprovementRecommendation]>,
}

fn real_policy_experiment_receipt(
    case_id: &'static str,
    loop_program: &LoopProgram,
    execution_receipt: &LoopProgramExecutionReceipt,
) -> RealPolicyExperimentReceipt {
    let mut receipt = RealPolicyExperimentReceipt {
        case_id,
        program_id: execution_receipt.program_id.as_str().to_owned(),
        policy_ids: loop_program
            .mechanism_policies
            .iter()
            .map(|policy| policy.as_str().to_owned())
            .collect::<Vec<_>>()
            .into_boxed_slice(),
        policy_digest: policy_digest_hex(&loop_program.policy_digest),
        loop_program_digest: loop_program_digest(loop_program),
        runtime_behavior_digest: runtime_behavior_digest(execution_receipt),
        receipt_digest: String::new(),
        status: execution_receipt.status.clone(),
        action_path: execution_receipt
            .steps
            .iter()
            .map(|step| step.machine_receipt.action.clone())
            .collect::<Vec<_>>()
            .into_boxed_slice(),
        owner_path: execution_receipt
            .steps
            .iter()
            .flat_map(|step| {
                step.runtime_handoff_execution
                    .executions
                    .iter()
                    .map(|execution| execution.owner.as_str().to_owned())
            })
            .collect::<Vec<_>>()
            .into_boxed_slice(),
        generated_events: execution_receipt
            .steps
            .iter()
            .map(|step| step.generated_event.clone())
            .collect::<Vec<_>>()
            .into_boxed_slice(),
        denied_handoff_count: execution_receipt
            .steps
            .iter()
            .flat_map(|step| step.runtime_handoff_execution.executions.iter())
            .filter(|execution| {
                execution.status == LoopProgramRuntimeHandoffExecutionStatus::Denied
            })
            .count(),
        handled_handoff_count: execution_receipt
            .steps
            .iter()
            .flat_map(|step| step.runtime_handoff_execution.executions.iter())
            .filter(|execution| {
                execution.status == LoopProgramRuntimeHandoffExecutionStatus::Handled
            })
            .count(),
        agent_flow_intent_count: execution_receipt
            .steps
            .iter()
            .flat_map(|step| step.runtime_handoff_execution.executions.iter())
            .filter(|execution| execution.agent_flow_intent.is_some())
            .count(),
        tool_projection_count: execution_receipt
            .steps
            .iter()
            .map(|step| {
                step.runtime_handoff_execution
                    .tool_process_projections
                    .len()
            })
            .sum(),
        memory_projection_count: execution_receipt
            .steps
            .iter()
            .map(|step| step.runtime_handoff_execution.memory_projections.len())
            .sum(),
        improvement_recommendations: Box::new([]),
    };
    receipt.improvement_recommendations =
        real_policy_improvement_recommendations(&receipt).into_boxed_slice();
    receipt.receipt_digest = real_policy_receipt_digest(&receipt);
    receipt
}

fn policy_digest_hex(policy_digest: &LoopPolicyDigest) -> String {
    hex_bytes(policy_digest.as_bytes())
}

fn loop_program_digest(loop_program: &LoopProgram) -> String {
    let mut digest = StableReceiptDigest::new("loop-program.v1");
    digest.write_u32(loop_program.schema_version);
    digest.write_str(loop_program.program_id.as_str());
    digest.write_debug(&loop_program.policy_epoch);
    digest.write_bytes(loop_program.policy_digest.as_bytes());
    digest.write_str(loop_program.initial_state.as_str());
    for policy in loop_program.mechanism_policies.iter() {
        digest.write_str(policy.as_str());
    }
    for transition in loop_program.transitions.iter() {
        digest.write_str(transition.transition_id.as_str());
        digest.write_str(transition.from.as_str());
        digest.write_debug(&transition.event);
        digest.write_debug(&transition.action);
        digest.write_str(transition.to.as_str());
    }
    digest.finish()
}

fn runtime_behavior_digest(execution_receipt: &LoopProgramExecutionReceipt) -> String {
    let mut digest = StableReceiptDigest::new("runtime-behavior.v1");
    digest.write_str(execution_receipt.program_id.as_str());
    digest.write_debug(&execution_receipt.status);
    for step in execution_receipt.steps.iter() {
        digest.write_debug(&step.machine_receipt.action);
        digest.write_debug(&step.generated_event);
        digest.write_debug(&step.runtime_handoff_execution.status);
        digest.write_usize(
            step.runtime_handoff_execution
                .tool_process_projections
                .len(),
        );
        digest.write_usize(step.runtime_handoff_execution.memory_projections.len());
        for execution in step.runtime_handoff_execution.executions.iter() {
            digest.write_str(execution.owner.as_str());
            digest.write_debug(&execution.status);
            digest.write_debug(&execution.kind);
            digest.write_debug(&execution.next_event);
            digest.write_bool(execution.agent_flow_intent.is_some());
        }
    }
    digest.finish()
}

fn real_policy_receipt_digest(receipt: &RealPolicyExperimentReceipt) -> String {
    let mut digest = StableReceiptDigest::new("real-policy-experiment.v1");
    digest.write_str(receipt.case_id);
    digest.write_str(&receipt.program_id);
    digest.write_str(&receipt.policy_digest);
    digest.write_str(&receipt.loop_program_digest);
    digest.write_str(&receipt.runtime_behavior_digest);
    digest.write_debug(&receipt.status);
    for policy in receipt.policy_ids.iter() {
        digest.write_str(policy);
    }
    for action in receipt.action_path.iter() {
        digest.write_debug(action);
    }
    for owner in receipt.owner_path.iter() {
        digest.write_str(owner);
    }
    for event in receipt.generated_events.iter() {
        digest.write_debug(event);
    }
    digest.write_usize(receipt.denied_handoff_count);
    digest.write_usize(receipt.handled_handoff_count);
    digest.write_usize(receipt.agent_flow_intent_count);
    digest.write_usize(receipt.tool_projection_count);
    digest.write_usize(receipt.memory_projection_count);
    for recommendation in receipt.improvement_recommendations.iter() {
        digest.write_str(recommendation.priority);
        digest.write_str(recommendation.target);
        digest.write_str(recommendation.evidence);
        digest.write_str(recommendation.action);
    }
    digest.finish()
}

struct StableReceiptDigest {
    value: u64,
}

impl StableReceiptDigest {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x00000100000001b3;

    fn new(scope: &'static str) -> Self {
        let mut digest = Self {
            value: Self::FNV_OFFSET,
        };
        digest.write_str(scope);
        digest
    }

    fn write_bool(&mut self, value: bool) {
        self.write_str(if value { "true" } else { "false" });
    }

    fn write_u32(&mut self, value: u32) {
        self.write_str(&value.to_string());
    }

    fn write_usize(&mut self, value: usize) {
        self.write_str(&value.to_string());
    }

    fn write_debug(&mut self, value: &impl Debug) {
        self.write_str(&format!("{value:?}"));
    }

    fn write_str(&mut self, value: &str) {
        self.write_bytes(value.as_bytes());
        self.write_bytes(&[0]);
    }

    fn write_bytes(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.value ^= u64::from(*byte);
            self.value = self.value.wrapping_mul(Self::FNV_PRIME);
        }
    }

    fn finish(self) -> String {
        format!("fnv1a64:{:016x}", self.value)
    }
}

fn hex_bytes(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut output, "{byte:02x}").expect("writing to String should not fail");
    }
    output
}

fn real_policy_improvement_recommendations(
    receipt: &RealPolicyExperimentReceipt,
) -> Vec<RealPolicyImprovementRecommendation> {
    let mut recommendations = Vec::new();

    if receipt.denied_handoff_count > 0 {
        recommendations.push(RealPolicyImprovementRecommendation {
            priority: "P0",
            target: "runtime.sandbox.denylist",
            evidence: "denied handoff receipt crossed the event pump",
            action: "preserve deny receipt details when promoting this case into policy reflection",
        });
    }

    if receipt
        .policy_ids
        .iter()
        .any(|policy| policy == "real-policy-005-memory-recall")
        && receipt.memory_projection_count == 0
    {
        recommendations.push(RealPolicyImprovementRecommendation {
            priority: "P1",
            target: "runtime.agent-flow.memory-projection",
            evidence: "memory recall survived as a typed intent but did not emit a memory projection receipt",
            action: "route the memory leg through the Agent-Flow executor before treating recall as closed",
        });
    }

    if receipt
        .action_path
        .iter()
        .any(|action| action == &LoopProgramActionKind::DispatchTools)
        && receipt.tool_projection_count == 0
        && receipt.denied_handoff_count == 0
    {
        recommendations.push(RealPolicyImprovementRecommendation {
            priority: "P1",
            target: "runtime.tool-sandbox.spawn",
            evidence: "tool dispatch was handled without a tool-process projection receipt",
            action: "promote the case to a real tool+sandbox spawn before using it as full-loop evidence",
        });
    }

    if receipt.policy_ids.len() > 1
        && !receipt
            .policy_ids
            .iter()
            .any(|policy| policy.contains("poo"))
    {
        recommendations.push(RealPolicyImprovementRecommendation {
            priority: "P2",
            target: "gerbil.config-interface.policy-pack",
            evidence: "policy combination is still assembled as a Rust test fixture",
            action: "derive the same LoopProgram from a Gerbil POO profile and resolved policy pack",
        });
    }

    recommendations
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

fn real_repair_001_no_write_llm_loop_program() -> LoopProgram {
    LoopProgram::new(LoopProgramInput {
        program_id: LoopProgramId::new("real-repair-001-no-write-llm"),
        policy_epoch: LoopPolicyEpoch::new(16),
        policy_digest: LoopPolicyDigest::from_bytes([16_u8; 32]),
        mechanism_policies: vec![
            LoopMechanismPolicyId::new("real-repair-001"),
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

fn real_repair_001_single_file_loop_program() -> LoopProgram {
    LoopProgram::new(LoopProgramInput {
        program_id: LoopProgramId::new("real-repair-001-single-file"),
        policy_epoch: LoopPolicyEpoch::new(17),
        policy_digest: LoopPolicyDigest::from_bytes([17_u8; 32]),
        mechanism_policies: vec![
            LoopMechanismPolicyId::new("real-repair-001"),
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

fn unique_temp_repair_file() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "marlin-real-repair-001-{}-{nanos}.rs",
        std::process::id()
    ))
}

fn unique_temp_repair_workspace() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "marlin-real-repair-001-workspace-{}-{nanos}",
        std::process::id()
    ))
}

fn real_policy_001_sandbox_denylist_loop_program() -> LoopProgram {
    LoopProgram::new(LoopProgramInput {
        program_id: LoopProgramId::new("real-policy-001-sandbox-denylist"),
        policy_epoch: LoopPolicyEpoch::new(10),
        policy_digest: LoopPolicyDigest::from_bytes([10_u8; 32]),
        mechanism_policies: vec![
            LoopMechanismPolicyId::new("real-policy-001-sandbox-denylist"),
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

fn real_tool_sandbox_loop_program() -> LoopProgram {
    LoopProgram::new(LoopProgramInput {
        program_id: LoopProgramId::new("real-tool-sandbox-loop"),
        policy_epoch: LoopPolicyEpoch::new(10),
        policy_digest: LoopPolicyDigest::from_bytes([10_u8; 32]),
        mechanism_policies: vec![
            LoopMechanismPolicyId::new("real-policy-001-tool-sandbox"),
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

fn real_policy_002_retry_budget_loop_program() -> LoopProgram {
    LoopProgram::new(LoopProgramInput {
        program_id: LoopProgramId::new("real-policy-002-retry-budget"),
        policy_epoch: LoopPolicyEpoch::new(11),
        policy_digest: LoopPolicyDigest::from_bytes([11_u8; 32]),
        mechanism_policies: vec![
            LoopMechanismPolicyId::new("real-policy-002-retry-budget"),
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

fn real_policy_003_maker_checker_loop_program() -> LoopProgram {
    LoopProgram::new(LoopProgramInput {
        program_id: LoopProgramId::new("real-policy-003-maker-checker"),
        policy_epoch: LoopPolicyEpoch::new(12),
        policy_digest: LoopPolicyDigest::from_bytes([12_u8; 32]),
        mechanism_policies: vec![LoopMechanismPolicyId::new("real-policy-003-maker-checker")]
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

fn real_policy_004_dynamic_rewrite_loop_program() -> LoopProgram {
    LoopProgram::new(LoopProgramInput {
        program_id: LoopProgramId::new("real-policy-004-dynamic-rewrite"),
        policy_epoch: LoopPolicyEpoch::new(13),
        policy_digest: LoopPolicyDigest::from_bytes([13_u8; 32]),
        mechanism_policies: vec![
            LoopMechanismPolicyId::new("real-policy-004-dynamic-rewrite"),
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

fn real_policy_005_memory_recall_loop_program() -> LoopProgram {
    LoopProgram::new(LoopProgramInput {
        program_id: LoopProgramId::new("real-policy-005-memory-recall"),
        policy_epoch: LoopPolicyEpoch::new(14),
        policy_digest: LoopPolicyDigest::from_bytes([14_u8; 32]),
        mechanism_policies: vec![
            LoopMechanismPolicyId::new("real-policy-005-memory-recall"),
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

fn policy_combination_matrix_loop_program() -> LoopProgram {
    LoopProgram::new(LoopProgramInput {
        program_id: LoopProgramId::new("policy-combination-memory-rewrite-checker"),
        policy_epoch: LoopPolicyEpoch::new(15),
        policy_digest: LoopPolicyDigest::from_bytes([15_u8; 32]),
        mechanism_policies: vec![
            LoopMechanismPolicyId::new("real-policy-003-maker-checker"),
            LoopMechanismPolicyId::new("real-policy-004-dynamic-rewrite"),
            LoopMechanismPolicyId::new("real-policy-005-memory-recall"),
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
#[path = "real_policy.rs"]
mod real_policy;
#[path = "real_repair.rs"]
mod real_repair;
