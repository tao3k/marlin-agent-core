use std::{
    env, fs,
    path::PathBuf,
    process::Command,
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use marlin_agent_kernel::{
    AgentFlowLoopProgramRuntimeHandoffExecutor, GenericLoopMachineReceipt, LoopProgramEventMapper,
    LoopProgramExecutionDriver, LoopProgramExecutionRequest, LoopProgramExecutionStatus,
    LoopProgramRuntimeHandoffExecutionReceipt, LoopProgramRuntimeHandoffExecutionReportStatus,
    LoopProgramRuntimeHandoffExecutor, LoopProgramRuntimeHandoffPlan,
    LoopProgramRuntimeHandoffRouter, LoopProgramRuntimeHandoffRouterHandlers,
    LoopProgramRuntimeOwner, LoopProgramToolProcessProgram, LoopProgramToolProcessSpawnRequest,
    StaticLoopProgramRuntimeHandoffHandler, spawn_loop_program_tool_process,
};
use marlin_agent_protocol::{
    LoopMechanismPolicyId, LoopPolicyDigest, LoopPolicyEpoch, LoopProgram, LoopProgramActionKind,
    LoopProgramEventKind, LoopProgramId, LoopProgramInput, LoopProgramStateId,
    LoopProgramTransition, LoopProgramTransitionId, ModelEndpoint, ModelGateway,
    ModelGatewayCompletionOptions, ModelGatewayCompletionResponse, ModelGatewayMessageRole,
    ModelGatewayRequest, system_gateway_message, user_gateway_message,
};
use marlin_agent_runtime::{RuntimeEdgeModelGateway, RuntimeEdgePolicy, TokioAgentRuntime};
use marlin_agent_stream::LiteLlmStreamGateway;

const LIVE_LLM_GATE_ENV: &str = "MARLIN_LIVE_LLM_GATE";
const LIVE_LLM_PROVIDER_ENV: &str = "MARLIN_LIVE_LLM_PROVIDER";
const LIVE_LLM_MODEL_ENV: &str = "MARLIN_LIVE_LLM_MODEL";
const LIVE_LLM_PROVIDER_API_KEY_ENV: &str = "MARLIN_LIVE_LLM_PROVIDER_API_KEY_ENV";
const LIVE_LLM_TIMEOUT_MS_ENV: &str = "MARLIN_LIVE_LLM_TIMEOUT_MS";
const DEFAULT_LIVE_LLM_TIMEOUT_MS: u64 = 60_000;
const PATCH_INTENT: &str = "PATCH_INTENT:single-file-replace";
const PATCH_FIND: &str = "fn answer() -> i32 { 40 }";
const PATCH_REPLACE: &str = "fn answer() -> i32 { 41 }";
const BUGGY_FIXTURE: &str =
    "fn answer() -> i32 { 40 }\n\n#[test]\nfn answer_is_41() {\n    assert_eq!(answer(), 41);\n}\n";
const FIXED_FIXTURE: &str =
    "fn answer() -> i32 { 41 }\n\n#[test]\nfn answer_is_41() {\n    assert_eq!(answer(), 41);\n}\n";
const BASELINE_TEST_SCRIPT: &str = "printf 'SOURCE_BEGIN\\n'; cat \"$1\"; printf '\\nSOURCE_END\\n'; rustc --test \"$1\" -o \"$2\" && \"$2\"";
const APPLY_PATCH_AND_TEST_SCRIPT: &str = r#"cat > "$1" <<'EOF'
fn answer() -> i32 { 41 }

#[test]
fn answer_is_41() {
    assert_eq!(answer(), 41);
}
EOF
rustc --test "$1" -o "$2" && "$2""#;

#[cfg(unix)]
#[test]
#[ignore = "requires MARLIN_LIVE_LLM_GATE=1 and live LiteLLM provider credentials"]
fn live_real_repair_001_single_file_bug_fix_runs_llm_tool_and_verifier_loop() {
    if !live_llm_gate_enabled() {
        eprintln!("skipping live repair loop: set {LIVE_LLM_GATE_ENV}=1 to enable");
        return;
    }

    let provider = required_live_llm_env(LIVE_LLM_PROVIDER_ENV);
    let model = required_live_llm_env(LIVE_LLM_MODEL_ENV);
    require_live_provider_key(&provider);
    let repair_workspace = unique_temp_repair_workspace();
    fs::create_dir_all(&repair_workspace).expect("create repair workspace");
    let bug_file = repair_workspace.join("lib.rs");
    let test_binary = repair_workspace.join("lib-tests");
    fs::write(&bug_file, BUGGY_FIXTURE).expect("write bug fixture");
    assert_eq!(
        fs::read_to_string(&bug_file).expect("read bug fixture"),
        BUGGY_FIXTURE
    );

    let edge_gateway = RuntimeEdgeModelGateway::new(
        LiteLlmStreamGateway::new(),
        RuntimeEdgePolicy::new()
            .with_concurrency_limit(1)
            .with_timeout_ms(live_llm_timeout().as_millis() as u64),
    )
    .expect("runtime edge model gateway");
    let mapper = LiveRepairDecisionMapper::new(
        Arc::new(edge_gateway),
        ModelEndpoint::new(provider, model),
        bug_file.clone(),
        test_binary.clone(),
    )
    .expect("live repair decision mapper");
    let completion_receipts = mapper.completion_receipts();
    let tool_receipts = mapper.tool_receipts();
    let verification_receipts = mapper.verification_receipts();
    let driver = LoopProgramExecutionDriver::new(RealRepairHybridHandoffExecutor::new())
        .with_event_mapper(mapper);

    let loop_program = real_repair_001_single_file_loop_program();
    let started_at = Instant::now();
    let execution_receipt = driver.run(LoopProgramExecutionRequest::new(
        loop_program,
        vec![LoopProgramEventKind::Start],
    ));

    assert_eq!(
        execution_receipt.status,
        LoopProgramExecutionStatus::Stopped
    );
    assert_eq!(
        execution_receipt
            .steps
            .iter()
            .map(|step| step.machine_receipt.action.clone())
            .collect::<Vec<_>>(),
        vec![
            LoopProgramActionKind::DispatchTools,
            LoopProgramActionKind::InvokeModel,
            LoopProgramActionKind::DispatchTools,
            LoopProgramActionKind::Verify,
            LoopProgramActionKind::Stop,
        ]
    );
    let model_completion = completion_receipts
        .lock()
        .expect("completion receipts")
        .first()
        .cloned()
        .expect("live model completion receipt");
    assert_eq!(
        model_completion.choices[0].message.role,
        ModelGatewayMessageRole::Assistant
    );
    assert!(
        model_completion.choices[0]
            .message
            .content
            .to_ascii_lowercase()
            .contains(&PATCH_INTENT.to_ascii_lowercase()),
        "live repair model did not return the typed patch intent: {:?}",
        model_completion.choices[0].message.content
    );
    assert!(
        model_completion.choices[0]
            .message
            .content
            .contains(PATCH_REPLACE),
        "live repair model did not identify the concrete replacement: {:?}",
        model_completion.choices[0].message.content
    );

    let tool_steps = execution_receipt
        .steps
        .iter()
        .filter(|step| step.machine_receipt.action == LoopProgramActionKind::DispatchTools)
        .collect::<Vec<_>>();
    assert_eq!(tool_steps.len(), 2);
    assert!(tool_steps.iter().all(|step| {
        step.runtime_handoff_execution.status
            == LoopProgramRuntimeHandoffExecutionReportStatus::Completed
    }));
    assert!(tool_steps.iter().all(|step| {
        !step
            .runtime_handoff_execution
            .tool_process_projections
            .is_empty()
    }));

    let repaired_content = fs::read_to_string(&bug_file).expect("read repaired fixture");
    let tool_receipts = tool_receipts.lock().expect("tool receipts").clone();
    assert_eq!(tool_receipts.len(), 2);
    assert_eq!(tool_receipts[0].phase, RealRepairToolPhase::BaselineTest);
    assert!(!tool_receipts[0].success);
    assert!(tool_receipts[0].stdout.contains("SOURCE_BEGIN"));
    assert!(tool_receipts[0].stdout.contains(PATCH_FIND));
    assert_eq!(tool_receipts[1].phase, RealRepairToolPhase::ApplyPatch);
    assert!(tool_receipts[1].success);
    let verification_receipts = verification_receipts
        .lock()
        .expect("verification receipts")
        .clone();
    assert_eq!(verification_receipts.len(), 1);
    assert!(verification_receipts[0].success);
    assert_eq!(verification_receipts[0].repaired_source, FIXED_FIXTURE);

    let live_receipt = RealRepair001LiveCaseReceipt {
        case_id: "real-repair-001",
        program_id: execution_receipt.program_id.as_str().to_owned(),
        model_completion_id: model_completion.id,
        model: model_completion.model,
        elapsed_ms: started_at.elapsed().as_millis() as u64,
        action_count: execution_receipt.steps.len(),
        tool_projection_count: tool_steps
            .iter()
            .map(|step| {
                step.runtime_handoff_execution
                    .tool_process_projections
                    .len()
            })
            .sum(),
        baseline_test_success: tool_receipts[0].success,
        patch_tool_success: tool_receipts[1].success,
        verification_success: verification_receipts[0].success,
        repaired_content,
    };

    assert_eq!(live_receipt.case_id, "real-repair-001");
    assert_eq!(live_receipt.program_id, "real-repair-001-live-single-file");
    assert!(!live_receipt.model_completion_id.trim().is_empty());
    assert!(!live_receipt.model.trim().is_empty());
    assert_eq!(live_receipt.action_count, 5);
    assert_eq!(live_receipt.tool_projection_count, 2);
    assert!(!live_receipt.baseline_test_success);
    assert!(live_receipt.patch_tool_success);
    assert!(live_receipt.verification_success);
    assert_eq!(live_receipt.repaired_content, FIXED_FIXTURE);
    eprintln!(
        "live real-repair-001 receipt: model={} elapsed_ms={} actions={} tool_projections={} baseline_success={} patch_success={} verify_success={}",
        live_receipt.model,
        live_receipt.elapsed_ms,
        live_receipt.action_count,
        live_receipt.tool_projection_count,
        live_receipt.baseline_test_success,
        live_receipt.patch_tool_success,
        live_receipt.verification_success
    );
    fs::remove_dir_all(&repair_workspace).expect("remove repair workspace");
}

#[derive(Debug)]
struct RealRepair001LiveCaseReceipt {
    case_id: &'static str,
    program_id: String,
    model_completion_id: String,
    model: String,
    elapsed_ms: u64,
    action_count: usize,
    tool_projection_count: usize,
    baseline_test_success: bool,
    patch_tool_success: bool,
    verification_success: bool,
    repaired_content: String,
}

#[derive(Clone)]
struct LiveRepairDecisionMapper {
    gateway: Arc<dyn ModelGateway>,
    endpoint: ModelEndpoint,
    bug_file: PathBuf,
    test_binary: PathBuf,
    completion_receipts: Arc<Mutex<Vec<ModelGatewayCompletionResponse>>>,
    tool_receipts: Arc<Mutex<Vec<RealRepairToolReceipt>>>,
    verification_receipts: Arc<Mutex<Vec<RealRepairVerificationReceipt>>>,
}

impl LiveRepairDecisionMapper {
    fn new(
        gateway: Arc<dyn ModelGateway>,
        endpoint: ModelEndpoint,
        bug_file: PathBuf,
        test_binary: PathBuf,
    ) -> Result<Self, String> {
        if !bug_file.is_absolute() {
            return Err("live repair bug file must be absolute".to_owned());
        }
        if !test_binary.is_absolute() {
            return Err("live repair test binary must be absolute".to_owned());
        }

        Ok(Self {
            gateway,
            endpoint,
            bug_file,
            test_binary,
            completion_receipts: Arc::new(Mutex::new(Vec::new())),
            tool_receipts: Arc::new(Mutex::new(Vec::new())),
            verification_receipts: Arc::new(Mutex::new(Vec::new())),
        })
    }

    fn completion_receipts(&self) -> Arc<Mutex<Vec<ModelGatewayCompletionResponse>>> {
        Arc::clone(&self.completion_receipts)
    }

    fn tool_receipts(&self) -> Arc<Mutex<Vec<RealRepairToolReceipt>>> {
        Arc::clone(&self.tool_receipts)
    }

    fn verification_receipts(&self) -> Arc<Mutex<Vec<RealRepairVerificationReceipt>>> {
        Arc::clone(&self.verification_receipts)
    }

    fn baseline_receipt(&self) -> RealRepairToolReceipt {
        self.tool_receipts
            .lock()
            .expect("tool receipts")
            .iter()
            .find(|receipt| receipt.phase == RealRepairToolPhase::BaselineTest)
            .cloned()
            .expect("baseline tool receipt must exist before model invocation")
    }

    fn model_requested_patch(&self) -> bool {
        self.completion_receipts
            .lock()
            .expect("completion receipts")
            .last()
            .map(|response| {
                let content = response
                    .choices
                    .first()
                    .map(|choice| choice.message.content.as_str())
                    .unwrap_or_default();
                content.contains(PATCH_INTENT)
                    && content.contains(PATCH_FIND)
                    && content.contains(PATCH_REPLACE)
            })
            .unwrap_or(false)
    }

    fn run_next_tool_receipt(
        &self,
        runtime_handoff_execution: &LoopProgramRuntimeHandoffExecutionReceipt,
    ) -> Option<LoopProgramEventKind> {
        let projection = runtime_handoff_execution
            .tool_process_projections
            .first()
            .cloned()
            .expect("dispatch tools action must project a tool process");
        let phase = if self.tool_receipts.lock().expect("tool receipts").is_empty() {
            RealRepairToolPhase::BaselineTest
        } else {
            RealRepairToolPhase::ApplyPatch
        };
        if phase == RealRepairToolPhase::ApplyPatch && !self.model_requested_patch() {
            return Some(LoopProgramEventKind::Error);
        }

        let script = match phase {
            RealRepairToolPhase::BaselineTest => BASELINE_TEST_SCRIPT,
            RealRepairToolPhase::ApplyPatch => APPLY_PATCH_AND_TEST_SCRIPT,
        };
        let spawn_receipt = spawn_runtime_shell(
            projection,
            vec![
                "-c".to_owned(),
                script.to_owned(),
                "real-repair-001-live".to_owned(),
                self.bug_file.to_string_lossy().into_owned(),
                self.test_binary.to_string_lossy().into_owned(),
            ],
        );
        let receipt = RealRepairToolReceipt {
            phase,
            success: spawn_receipt.output.status.success(),
            stdout: String::from_utf8_lossy(&spawn_receipt.output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&spawn_receipt.output.stderr).into_owned(),
            observed_source: fs::read_to_string(&self.bug_file)
                .expect("read repair source after tool receipt"),
        };
        let event = match receipt.phase {
            RealRepairToolPhase::BaselineTest => (!receipt.success
                && receipt.stdout.contains("SOURCE_BEGIN")
                && receipt.stdout.contains(PATCH_FIND))
            .then_some(LoopProgramEventKind::ToolReceipt)
            .or(Some(LoopProgramEventKind::Error)),
            RealRepairToolPhase::ApplyPatch => (receipt.success
                && receipt.observed_source == FIXED_FIXTURE)
                .then_some(LoopProgramEventKind::ToolReceipt)
                .or(Some(LoopProgramEventKind::Error)),
        };
        self.tool_receipts
            .lock()
            .expect("tool receipts")
            .push(receipt);
        event
    }

    fn run_verification_receipt(&self) -> Option<LoopProgramEventKind> {
        let output = Command::new("sh")
            .args([
                "-c",
                "rustc --test \"$1\" -o \"$2\" && \"$2\"",
                "real-repair-001-verify",
                self.bug_file.to_string_lossy().as_ref(),
                self.test_binary.to_string_lossy().as_ref(),
            ])
            .output()
            .expect("run live repair verifier");
        let receipt = RealRepairVerificationReceipt {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            repaired_source: fs::read_to_string(&self.bug_file)
                .expect("read repair source after verification"),
        };
        let event = (receipt.success && receipt.repaired_source == FIXED_FIXTURE)
            .then_some(LoopProgramEventKind::VerificationReceipt)
            .or(Some(LoopProgramEventKind::Error));
        self.verification_receipts
            .lock()
            .expect("verification receipts")
            .push(receipt);
        event
    }
}

impl LoopProgramEventMapper for LiveRepairDecisionMapper {
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
            LoopProgramActionKind::DispatchTools => {
                self.run_next_tool_receipt(runtime_handoff_execution)
            }
            LoopProgramActionKind::InvokeModel => {
                let baseline = self.baseline_receipt();
                let request = ModelGatewayRequest::new(
                    self.endpoint.clone(),
                    vec![
                        system_gateway_message(
                            "You are the real-repair-001 policy planner. Use only the tool receipt evidence. Reply with a typed patch receipt and no prose.",
                        ),
                        user_gateway_message(format!(
                            "The baseline tool receipt observed a failing Rust test.\n\nstdout:\n{}\n\nstderr:\n{}\n\nEmit these exact lines for one allowed file repair:\n{}\nFIND:{}\nREPLACE:{}",
                            baseline.stdout,
                            baseline.stderr,
                            PATCH_INTENT,
                            PATCH_FIND,
                            PATCH_REPLACE
                        )),
                    ],
                )
                .with_options(ModelGatewayCompletionOptions {
                    max_tokens: Some(96),
                    temperature: Some(0.0),
                    ..Default::default()
                });
                let response = complete_gateway_synchronously(self.gateway.as_ref(), request)
                    .expect("live repair gateway completion");
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
                (repair_text.contains(PATCH_INTENT)
                    && repair_text.contains(PATCH_FIND)
                    && repair_text.contains(PATCH_REPLACE))
                .then_some(LoopProgramEventKind::ModelEvent)
                .or(Some(LoopProgramEventKind::Error))
            }
            LoopProgramActionKind::Verify => self.run_verification_receipt(),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RealRepairToolPhase {
    BaselineTest,
    ApplyPatch,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RealRepairToolReceipt {
    phase: RealRepairToolPhase,
    success: bool,
    stdout: String,
    stderr: String,
    observed_source: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RealRepairVerificationReceipt {
    success: bool,
    stdout: String,
    stderr: String,
    repaired_source: String,
}

fn spawn_runtime_shell(
    projection: marlin_agent_kernel::LoopProgramToolProcessProjectionReceipt,
    args: Vec<String>,
) -> marlin_agent_kernel::LoopProgramToolProcessSpawnReceipt {
    let (runtime, _events) = TokioAgentRuntime::new(4);
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tool process runtime")
        .block_on(spawn_loop_program_tool_process(
            &runtime.context(),
            LoopProgramToolProcessSpawnRequest::new(
                projection,
                LoopProgramToolProcessProgram::new("sh"),
            )
            .with_args(args.into_boxed_slice())
            .with_started_at_ms(300)
            .with_observed_at_ms(340),
        ))
        .expect("repair tool should spawn")
}

fn complete_gateway_synchronously(
    gateway: &dyn ModelGateway,
    request: ModelGatewayRequest,
) -> marlin_agent_protocol::ModelGatewayResult<ModelGatewayCompletionResponse> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("current-thread runtime")
        .block_on(gateway.complete(request))
}

#[derive(Clone)]
struct RealRepairHybridHandoffExecutor {
    router: LoopProgramRuntimeHandoffRouter,
    agent_flow: AgentFlowLoopProgramRuntimeHandoffExecutor,
}

impl RealRepairHybridHandoffExecutor {
    fn new() -> Self {
        let handlers = LoopProgramRuntimeHandoffRouterHandlers {
            model_handler: Arc::new(StaticLoopProgramRuntimeHandoffHandler::handled(
                LoopProgramRuntimeOwner::new("runtime.model.gateway.repair-planner"),
            )),
            verification_handler: Arc::new(StaticLoopProgramRuntimeHandoffHandler::handled(
                LoopProgramRuntimeOwner::new("runtime.verification.single-file"),
            )),
            control_handler: Arc::new(StaticLoopProgramRuntimeHandoffHandler::handled(
                LoopProgramRuntimeOwner::new("runtime.control"),
            )),
            ..LoopProgramRuntimeHandoffRouterHandlers::default()
        };
        Self {
            router: LoopProgramRuntimeHandoffRouter::new(handlers),
            agent_flow: AgentFlowLoopProgramRuntimeHandoffExecutor::new(
                LoopProgramRuntimeOwner::new("runtime.agent-flow.repair-tool"),
            ),
        }
    }
}

impl LoopProgramRuntimeHandoffExecutor for RealRepairHybridHandoffExecutor {
    fn execute_plan(
        &self,
        plan: &LoopProgramRuntimeHandoffPlan,
    ) -> LoopProgramRuntimeHandoffExecutionReceipt {
        if plan
            .handoffs
            .iter()
            .any(|handoff| handoff.agent_flow_intent.is_some())
        {
            self.agent_flow.execute_plan(plan)
        } else {
            self.router.execute_plan(plan)
        }
    }
}

fn real_repair_001_single_file_loop_program() -> LoopProgram {
    LoopProgram::new(LoopProgramInput {
        program_id: LoopProgramId::new("real-repair-001-live-single-file"),
        policy_epoch: LoopPolicyEpoch::new(18),
        policy_digest: LoopPolicyDigest::from_bytes([18_u8; 32]),
        mechanism_policies: vec![
            LoopMechanismPolicyId::new("real-repair-001"),
            LoopMechanismPolicyId::new("live-llm-repair"),
            LoopMechanismPolicyId::new("tool-sandbox"),
            LoopMechanismPolicyId::new("verification-gate"),
        ]
        .into_boxed_slice(),
        initial_state: LoopProgramStateId::new("start"),
        transitions: vec![
            transition(
                "start-baseline-observation",
                "start",
                LoopProgramEventKind::Start,
                LoopProgramActionKind::DispatchTools,
                "baseline-observed",
            ),
            transition(
                "baseline-observation-plan",
                "baseline-observed",
                LoopProgramEventKind::ToolReceipt,
                LoopProgramActionKind::InvokeModel,
                "llm-planned",
            ),
            transition(
                "live-llm-plan-patch-tool",
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

fn unique_temp_repair_workspace() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock after epoch")
        .as_nanos();
    env::temp_dir().join(format!(
        "marlin-live-real-repair-001-{}-{nanos}",
        std::process::id()
    ))
}

fn live_llm_gate_enabled() -> bool {
    matches!(
        env::var(LIVE_LLM_GATE_ENV).as_deref(),
        Ok("1" | "true" | "TRUE" | "yes" | "YES" | "on" | "ON")
    )
}

fn required_live_llm_env(name: &str) -> String {
    env::var(name).unwrap_or_else(|_| panic!("{name} must be set when {LIVE_LLM_GATE_ENV}=1"))
}

fn require_live_provider_key(provider: &str) {
    if let Ok(env_name) = env::var(LIVE_LLM_PROVIDER_API_KEY_ENV) {
        if env::var(&env_name).is_ok_and(|value| !value.trim().is_empty()) {
            return;
        }
        panic!(
            "{env_name} must be set when {LIVE_LLM_PROVIDER_API_KEY_ENV}={env_name} and {LIVE_LLM_GATE_ENV}=1"
        );
    }

    let expected_env_names: &[&str] = match provider {
        "anthropic" => &["ANTHROPIC_API_KEY"],
        "deepseek" => &["DEEPSEEK_API_KEY"],
        "openai" => &["OPENAI_API_KEY"],
        "openrouter" => &["OPENROUTER_API_KEY"],
        _ => return,
    };

    if expected_env_names
        .iter()
        .any(|name| env::var(name).is_ok_and(|value| !value.trim().is_empty()))
    {
        return;
    }

    panic!(
        "{provider} live LLM gate requires one of {:?}, or set {LIVE_LLM_PROVIDER_API_KEY_ENV} to a provider-specific key env name",
        expected_env_names
    );
}

fn live_llm_timeout() -> Duration {
    env::var(LIVE_LLM_TIMEOUT_MS_ENV)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|millis| *millis > 0)
        .map(Duration::from_millis)
        .unwrap_or(Duration::from_millis(DEFAULT_LIVE_LLM_TIMEOUT_MS))
}
