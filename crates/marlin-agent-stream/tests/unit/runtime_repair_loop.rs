use std::{
    env, fs,
    path::PathBuf,
    process::Command,
    sync::{Arc, Mutex, OnceLock},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use marlin_agent_harness_types::{
    RuntimeRepairCaseId, RuntimeRepairContentSummary, RuntimeRepairCount,
    RuntimeRepairDenialReason, RuntimeRepairDurationMillis, RuntimeRepairHandoffStatus,
    RuntimeRepairLiveCaseReceipt, RuntimeRepairLiveCaseReceiptRequest, RuntimeRepairLiveGateStatus,
    RuntimeRepairModelCompletionId, RuntimeRepairModelId, RuntimeRepairNoLiveCaseReceipt,
    RuntimeRepairNoLiveCaseReceiptRequest, RuntimeRepairProfileRef,
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
    LoopProgramActionKind, LoopProgramEventKind, ModelEndpoint, ModelGateway,
    ModelGatewayCompletionOptions, ModelGatewayCompletionResponse, ModelGatewayMessageRole,
    ModelGatewayRequest, system_gateway_message, user_gateway_message,
};
use marlin_agent_runtime::{RuntimeEdgeModelGateway, RuntimeEdgePolicy, TokioAgentRuntime};
use marlin_agent_stream::LiteLlmStreamGateway;
use marlin_gerbil_scheme::{
    GerbilLoopCaseDriverProjectedLoopProgram, GerbilLoopCaseDriverProjectedLoopProgramRequest,
    load_gerbil_loop_case_driver_projected_loop_program,
};

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
const RUNTIME_LIVE_REPAIR_CASE_ID: &str = "real-repair-001";
const BASELINE_TEST_SCRIPT: &str = "printf 'SOURCE_BEGIN\\n'; cat \"$1\"; printf '\\nSOURCE_END\\n'; rustc --test \"$1\" -o \"$2\" && \"$2\"";
const APPLY_PATCH_AND_TEST_SCRIPT: &str = r#"cat > "$1" <<'EOF'
fn answer() -> i32 { 41 }

#[test]
fn answer_is_41() {
    assert_eq!(answer(), 41);
}
EOF
rustc --test "$1" -o "$2" && "$2""#;
const SCHEME_REAL_REPAIR_CASE_ID: &str = "real-repair-001";
const SCHEME_REAL_REPAIR_PROGRAM_ID: &str = "real-repair-001-scripted-loop";
const SCHEME_REAL_REPAIR_PROFILE_REF: &str = "real-repair-001/reactive-tool-loop";

#[cfg(unix)]
#[test]
#[ignore = "requires MARLIN_LIVE_LLM_GATE=1 and live LiteLLM provider credentials"]
fn live_runtime_repair_single_file_bug_fix_runs_llm_tool_and_verifier_loop() {
    let gate_receipt = runtime_live_repair_gate_receipt();
    if gate_receipt.status == RuntimeLiveRepairGateStatus::Disabled {
        eprintln!("skipping live repair loop: {gate_receipt:?}");
        return;
    }
    assert_eq!(
        gate_receipt.status,
        RuntimeLiveRepairGateStatus::Enabled,
        "live repair gate configured but not ready: {gate_receipt:?}"
    );

    let provider = gate_receipt
        .provider
        .clone()
        .expect("enabled gate receipt must include provider");
    let model = gate_receipt
        .model
        .clone()
        .expect("enabled gate receipt must include model");
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
    let driver = LoopProgramExecutionDriver::new(RuntimeRepairHybridHandoffExecutor::new())
        .with_event_mapper(mapper);

    let scheme_case = scheme_projected_real_repair_loop_case();
    let scheme_receipt = scheme_case.receipt();
    let loop_program = scheme_case.loop_program().clone();
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
            LoopProgramActionKind::InvokeModel,
            LoopProgramActionKind::DispatchTools,
            LoopProgramActionKind::Continue,
            LoopProgramActionKind::RewriteGraph,
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
    assert_eq!(tool_steps.len(), 1);
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
    assert_eq!(tool_receipts.len(), 1);
    assert_eq!(tool_receipts[0].phase, RuntimeRepairToolPhase::ApplyPatch);
    assert!(tool_receipts[0].success);
    let verification_receipts = verification_receipts
        .lock()
        .expect("verification receipts")
        .clone();
    assert_eq!(verification_receipts.len(), 1);
    assert!(verification_receipts[0].success);
    assert_eq!(verification_receipts[0].repaired_source, FIXED_FIXTURE);

    let live_receipt = RuntimeRepairLiveCaseReceipt::new(RuntimeRepairLiveCaseReceiptRequest {
        case_id: RuntimeRepairCaseId::new(scheme_receipt.case_id().as_str()),
        profile_ref: RuntimeRepairProfileRef::new(scheme_receipt.profile_ref().as_str()),
        program_id: execution_receipt.program_id.clone(),
        model_completion_id: RuntimeRepairModelCompletionId::new(model_completion.id),
        model: RuntimeRepairModelId::new(model_completion.model),
        elapsed_ms: RuntimeRepairDurationMillis::new(started_at.elapsed().as_millis() as u64),
        action_count: RuntimeRepairCount::new(execution_receipt.steps.len()),
        tool_projection_count: RuntimeRepairCount::new(
            tool_steps
                .iter()
                .map(|step| {
                    step.runtime_handoff_execution
                        .tool_process_projections
                        .len()
                })
                .sum(),
        ),
        patch_tool_success: tool_receipts[0].success,
        graph_rewrite_projected: execution_receipt
            .steps
            .iter()
            .any(|step| step.machine_receipt.action == LoopProgramActionKind::RewriteGraph),
        verification_success: verification_receipts[0].success,
        repaired_content: RuntimeRepairContentSummary::from_text(&repaired_content),
    });

    assert_eq!(live_receipt.case_id.as_str(), SCHEME_REAL_REPAIR_CASE_ID);
    assert_eq!(
        live_receipt.profile_ref.as_str(),
        SCHEME_REAL_REPAIR_PROFILE_REF
    );
    assert_eq!(
        live_receipt.program_id.as_str(),
        SCHEME_REAL_REPAIR_PROGRAM_ID
    );
    assert!(!live_receipt.model_completion_id.as_str().trim().is_empty());
    assert!(!live_receipt.model.as_str().trim().is_empty());
    assert_eq!(live_receipt.action_count.get(), 6);
    assert_eq!(live_receipt.tool_projection_count.get(), 1);
    assert!(live_receipt.patch_tool_success);
    assert!(live_receipt.graph_rewrite_projected);
    assert!(live_receipt.verification_success);
    assert_eq!(
        live_receipt.repaired_content,
        RuntimeRepairContentSummary::from_text(FIXED_FIXTURE)
    );
    eprintln!(
        "live runtime repair receipt: case={} profile={} model={} elapsed_ms={} actions={} tool_projections={} patch_success={} rewrite_projected={} verify_success={}",
        live_receipt.case_id,
        live_receipt.profile_ref,
        live_receipt.model,
        live_receipt.elapsed_ms.get(),
        live_receipt.action_count.get(),
        live_receipt.tool_projection_count.get(),
        live_receipt.patch_tool_success,
        live_receipt.graph_rewrite_projected,
        live_receipt.verification_success
    );
    fs::remove_dir_all(&repair_workspace).expect("remove repair workspace");
}

#[test]
fn live_runtime_repair_gate_receipt_reports_disabled_without_gate() {
    let receipt = runtime_live_repair_gate_receipt_from_lookup(|name| match name {
        LIVE_LLM_PROVIDER_ENV => Some("anthropic".to_owned()),
        LIVE_LLM_MODEL_ENV => Some("claude-sonnet-4".to_owned()),
        "ANTHROPIC_API_KEY" => Some("redacted".to_owned()),
        _ => None,
    });

    assert_eq!(receipt.case_id, RUNTIME_LIVE_REPAIR_CASE_ID);
    assert_eq!(receipt.status, RuntimeLiveRepairGateStatus::Disabled);
    assert_eq!(receipt.gate_env, LIVE_LLM_GATE_ENV);
    assert_eq!(
        receipt.denial_reason.as_deref(),
        Some("live LLM gate is disabled")
    );
    assert!(receipt.required_provider_key_envs.is_empty());
    assert!(!receipt.provider_api_key_present);
}

#[test]
fn live_runtime_repair_gate_receipt_reports_missing_provider_and_model() {
    let missing_provider = runtime_live_repair_gate_receipt_from_lookup(|name| match name {
        LIVE_LLM_GATE_ENV => Some("1".to_owned()),
        LIVE_LLM_MODEL_ENV => Some("claude-sonnet-4".to_owned()),
        _ => None,
    });
    assert_eq!(
        missing_provider.status,
        RuntimeLiveRepairGateStatus::MissingProvider
    );
    assert_eq!(
        missing_provider.denial_reason.as_deref(),
        Some("MARLIN_LIVE_LLM_PROVIDER is required when live LLM gate is enabled")
    );

    let missing_model = runtime_live_repair_gate_receipt_from_lookup(|name| match name {
        LIVE_LLM_GATE_ENV => Some("1".to_owned()),
        LIVE_LLM_PROVIDER_ENV => Some("anthropic".to_owned()),
        _ => None,
    });
    assert_eq!(
        missing_model.status,
        RuntimeLiveRepairGateStatus::MissingModel
    );
    assert_eq!(
        missing_model.denial_reason.as_deref(),
        Some("MARLIN_LIVE_LLM_MODEL is required when live LLM gate is enabled")
    );
}

#[test]
fn live_runtime_repair_gate_receipt_reports_missing_provider_key() {
    let receipt = runtime_live_repair_gate_receipt_from_lookup(|name| match name {
        LIVE_LLM_GATE_ENV => Some("1".to_owned()),
        LIVE_LLM_PROVIDER_ENV => Some("anthropic".to_owned()),
        LIVE_LLM_MODEL_ENV => Some("claude-sonnet-4".to_owned()),
        _ => None,
    });

    assert_eq!(
        receipt.status,
        RuntimeLiveRepairGateStatus::MissingProviderKey
    );
    assert_eq!(
        receipt.required_provider_key_envs,
        vec!["ANTHROPIC_API_KEY".to_owned()]
    );
    assert!(!receipt.provider_api_key_present);
    assert_eq!(
        receipt.denial_reason.as_deref(),
        Some("live LLM provider credentials are missing")
    );
}

#[test]
fn live_runtime_repair_gate_receipt_reports_enabled_with_override_key() {
    let receipt = runtime_live_repair_gate_receipt_from_lookup(|name| match name {
        LIVE_LLM_GATE_ENV => Some("yes".to_owned()),
        LIVE_LLM_PROVIDER_ENV => Some("deepseek".to_owned()),
        LIVE_LLM_MODEL_ENV => Some("deepseek-chat".to_owned()),
        LIVE_LLM_PROVIDER_API_KEY_ENV => Some("MARLIN_TEST_DEEPSEEK_KEY".to_owned()),
        "MARLIN_TEST_DEEPSEEK_KEY" => Some("redacted".to_owned()),
        _ => None,
    });

    assert_eq!(receipt.status, RuntimeLiveRepairGateStatus::Enabled);
    assert_eq!(receipt.provider.as_deref(), Some("deepseek"));
    assert_eq!(receipt.model.as_deref(), Some("deepseek-chat"));
    assert_eq!(
        receipt.provider_api_key_env_override.as_deref(),
        Some("MARLIN_TEST_DEEPSEEK_KEY")
    );
    assert_eq!(
        receipt.required_provider_key_envs,
        vec!["MARLIN_TEST_DEEPSEEK_KEY".to_owned()]
    );
    assert!(receipt.provider_api_key_present);
    assert!(receipt.denial_reason.is_none());
}

#[test]
fn runtime_live_repair_no_live_gate_denial_runs_typed_loop_receipt() {
    let gate_receipt = runtime_live_repair_gate_receipt_from_lookup(|name| match name {
        LIVE_LLM_PROVIDER_ENV => Some("openai".to_owned()),
        LIVE_LLM_MODEL_ENV => Some("gpt-repair-policy".to_owned()),
        "OPENAI_API_KEY" => Some("redacted".to_owned()),
        _ => None,
    });
    assert_eq!(gate_receipt.status, RuntimeLiveRepairGateStatus::Disabled);
    let denial_reason = gate_receipt
        .denial_reason
        .clone()
        .expect("disabled no-live gate denial reason");

    let driver = LoopProgramExecutionDriver::new(RuntimeRepairNoLiveHandoffExecutor::new())
        .with_event_mapper(NoLiveRepairDecisionMapper);
    let scheme_case = scheme_projected_real_repair_loop_case();
    let scheme_receipt = scheme_case.receipt();
    let execution_receipt = driver.run(LoopProgramExecutionRequest::new(
        scheme_case.loop_program().clone(),
        vec![LoopProgramEventKind::Start],
    ));

    assert_eq!(
        execution_receipt
            .steps
            .iter()
            .map(|step| step.machine_receipt.action.clone())
            .collect::<Vec<_>>(),
        vec![LoopProgramActionKind::InvokeModel]
    );

    let model_step = execution_receipt.steps.first().expect("model denial step");
    assert_eq!(
        model_step.machine_receipt.action,
        LoopProgramActionKind::InvokeModel
    );
    assert_eq!(
        model_step.runtime_handoff_execution.status,
        LoopProgramRuntimeHandoffExecutionReportStatus::Denied
    );
    assert!(
        execution_receipt.error.is_some(),
        "Scheme-projected repair loop should not synthesize a Rust stop branch for denied LLM: {execution_receipt:?}"
    );

    let no_live_receipt =
        RuntimeRepairNoLiveCaseReceipt::new(RuntimeRepairNoLiveCaseReceiptRequest {
            case_id: RuntimeRepairCaseId::new(scheme_receipt.case_id().as_str()),
            profile_ref: RuntimeRepairProfileRef::new(scheme_receipt.profile_ref().as_str()),
            program_id: execution_receipt.program_id.clone(),
            gate_status: runtime_repair_gate_status(gate_receipt.status),
            denial_reason: RuntimeRepairDenialReason::new(denial_reason),
            live_llm_allowed: false,
            action_count: RuntimeRepairCount::new(execution_receipt.steps.len()),
            model_handoff_status: runtime_repair_handoff_status(
                &model_step.runtime_handoff_execution.status,
            ),
        });

    assert_eq!(no_live_receipt.case_id.as_str(), SCHEME_REAL_REPAIR_CASE_ID);
    assert_eq!(
        no_live_receipt.profile_ref.as_str(),
        SCHEME_REAL_REPAIR_PROFILE_REF
    );
    assert_eq!(
        no_live_receipt.program_id.as_str(),
        SCHEME_REAL_REPAIR_PROGRAM_ID
    );
    assert_eq!(
        no_live_receipt.gate_status,
        RuntimeRepairLiveGateStatus::Disabled
    );
    assert_eq!(
        no_live_receipt.denial_reason.as_str(),
        "live LLM gate is disabled"
    );
    assert!(!no_live_receipt.live_llm_allowed);
    assert_eq!(no_live_receipt.action_count.get(), 1);
    assert_eq!(
        no_live_receipt.model_handoff_status,
        RuntimeRepairHandoffStatus::Denied
    );
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RuntimeLiveRepairGateReceipt {
    case_id: &'static str,
    gate_env: &'static str,
    gate_value: Option<String>,
    provider_env: &'static str,
    provider: Option<String>,
    model_env: &'static str,
    model: Option<String>,
    provider_api_key_env: &'static str,
    provider_api_key_env_override: Option<String>,
    required_provider_key_envs: Vec<String>,
    provider_api_key_present: bool,
    status: RuntimeLiveRepairGateStatus,
    denial_reason: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RuntimeLiveRepairGateStatus {
    Disabled,
    MissingProvider,
    MissingModel,
    MissingProviderKey,
    Enabled,
}

fn runtime_live_repair_gate_receipt() -> RuntimeLiveRepairGateReceipt {
    runtime_live_repair_gate_receipt_from_lookup(|name| env::var(name).ok())
}

fn runtime_live_repair_gate_receipt_from_lookup(
    mut get_env: impl FnMut(&str) -> Option<String>,
) -> RuntimeLiveRepairGateReceipt {
    let gate_value = clean_env_value(get_env(LIVE_LLM_GATE_ENV));
    let provider = clean_env_value(get_env(LIVE_LLM_PROVIDER_ENV));
    let model = clean_env_value(get_env(LIVE_LLM_MODEL_ENV));
    let provider_api_key_env_override = clean_env_value(get_env(LIVE_LLM_PROVIDER_API_KEY_ENV));
    let mut required_provider_key_envs = Vec::new();
    let mut provider_api_key_present = false;
    let mut status = RuntimeLiveRepairGateStatus::Enabled;
    let mut denial_reason = None;

    if !live_llm_gate_value_enabled(gate_value.as_deref()) {
        status = RuntimeLiveRepairGateStatus::Disabled;
        denial_reason = Some("live LLM gate is disabled".to_owned());
    } else if provider.is_none() {
        status = RuntimeLiveRepairGateStatus::MissingProvider;
        denial_reason = Some(format!(
            "{LIVE_LLM_PROVIDER_ENV} is required when live LLM gate is enabled"
        ));
    } else if model.is_none() {
        status = RuntimeLiveRepairGateStatus::MissingModel;
        denial_reason = Some(format!(
            "{LIVE_LLM_MODEL_ENV} is required when live LLM gate is enabled"
        ));
    } else if let Some(override_env) = provider_api_key_env_override.as_deref() {
        required_provider_key_envs.push(override_env.to_owned());
        provider_api_key_present = clean_env_value(get_env(override_env)).is_some();
    } else {
        required_provider_key_envs.extend(
            default_provider_key_envs(provider.as_deref().expect("provider checked"))
                .iter()
                .map(|name| (*name).to_owned()),
        );
        provider_api_key_present = required_provider_key_envs
            .iter()
            .any(|name| clean_env_value(get_env(name)).is_some());
    }

    if status == RuntimeLiveRepairGateStatus::Enabled
        && !required_provider_key_envs.is_empty()
        && !provider_api_key_present
    {
        status = RuntimeLiveRepairGateStatus::MissingProviderKey;
        denial_reason = Some("live LLM provider credentials are missing".to_owned());
    }

    RuntimeLiveRepairGateReceipt {
        case_id: RUNTIME_LIVE_REPAIR_CASE_ID,
        gate_env: LIVE_LLM_GATE_ENV,
        gate_value,
        provider_env: LIVE_LLM_PROVIDER_ENV,
        provider,
        model_env: LIVE_LLM_MODEL_ENV,
        model,
        provider_api_key_env: LIVE_LLM_PROVIDER_API_KEY_ENV,
        provider_api_key_env_override,
        required_provider_key_envs,
        provider_api_key_present,
        status,
        denial_reason,
    }
}

fn clean_env_value(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_owned())
    })
}

fn live_llm_gate_value_enabled(value: Option<&str>) -> bool {
    matches!(
        value,
        Some("1" | "true" | "TRUE" | "yes" | "YES" | "on" | "ON")
    )
}

fn default_provider_key_envs(provider: &str) -> &'static [&'static str] {
    match provider {
        "anthropic" => &["ANTHROPIC_API_KEY"],
        "deepseek" => &["DEEPSEEK_API_KEY"],
        "openai" => &["OPENAI_API_KEY"],
        "openrouter" => &["OPENROUTER_API_KEY"],
        _ => &[],
    }
}

fn runtime_repair_gate_status(status: RuntimeLiveRepairGateStatus) -> RuntimeRepairLiveGateStatus {
    match status {
        RuntimeLiveRepairGateStatus::Disabled => RuntimeRepairLiveGateStatus::Disabled,
        RuntimeLiveRepairGateStatus::MissingProvider => {
            RuntimeRepairLiveGateStatus::MissingProvider
        }
        RuntimeLiveRepairGateStatus::MissingModel => RuntimeRepairLiveGateStatus::MissingModel,
        RuntimeLiveRepairGateStatus::MissingProviderKey => {
            RuntimeRepairLiveGateStatus::MissingProviderKey
        }
        RuntimeLiveRepairGateStatus::Enabled => RuntimeRepairLiveGateStatus::Enabled,
    }
}

fn runtime_repair_handoff_status(
    status: &LoopProgramRuntimeHandoffExecutionReportStatus,
) -> RuntimeRepairHandoffStatus {
    match status {
        LoopProgramRuntimeHandoffExecutionReportStatus::Empty => RuntimeRepairHandoffStatus::Empty,
        LoopProgramRuntimeHandoffExecutionReportStatus::Completed => {
            RuntimeRepairHandoffStatus::Completed
        }
        LoopProgramRuntimeHandoffExecutionReportStatus::Deferred => {
            RuntimeRepairHandoffStatus::Deferred
        }
        LoopProgramRuntimeHandoffExecutionReportStatus::Denied => {
            RuntimeRepairHandoffStatus::Denied
        }
    }
}

#[derive(Clone)]
struct LiveRepairDecisionMapper {
    gateway: Arc<dyn ModelGateway>,
    endpoint: ModelEndpoint,
    bug_file: PathBuf,
    test_binary: PathBuf,
    completion_receipts: Arc<Mutex<Vec<ModelGatewayCompletionResponse>>>,
    tool_receipts: Arc<Mutex<Vec<RuntimeRepairToolReceipt>>>,
    verification_receipts: Arc<Mutex<Vec<RuntimeRepairVerificationReceipt>>>,
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

    fn tool_receipts(&self) -> Arc<Mutex<Vec<RuntimeRepairToolReceipt>>> {
        Arc::clone(&self.tool_receipts)
    }

    fn verification_receipts(&self) -> Arc<Mutex<Vec<RuntimeRepairVerificationReceipt>>> {
        Arc::clone(&self.verification_receipts)
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
        let phase = if self
            .completion_receipts
            .lock()
            .expect("completion receipts")
            .is_empty()
        {
            RuntimeRepairToolPhase::BaselineTest
        } else {
            RuntimeRepairToolPhase::ApplyPatch
        };
        if phase == RuntimeRepairToolPhase::ApplyPatch && !self.model_requested_patch() {
            return Some(LoopProgramEventKind::Error);
        }

        let script = match phase {
            RuntimeRepairToolPhase::BaselineTest => BASELINE_TEST_SCRIPT,
            RuntimeRepairToolPhase::ApplyPatch => APPLY_PATCH_AND_TEST_SCRIPT,
        };
        let spawn_receipt = spawn_runtime_shell(
            projection,
            vec![
                "-c".to_owned(),
                script.to_owned(),
                "runtime-live-repair-live".to_owned(),
                self.bug_file.to_string_lossy().into_owned(),
                self.test_binary.to_string_lossy().into_owned(),
            ],
        );
        let receipt = RuntimeRepairToolReceipt {
            phase,
            success: spawn_receipt.output.status.success(),
            stdout: String::from_utf8_lossy(&spawn_receipt.output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&spawn_receipt.output.stderr).into_owned(),
            observed_source: fs::read_to_string(&self.bug_file)
                .expect("read repair source after tool receipt"),
        };
        let event = match receipt.phase {
            RuntimeRepairToolPhase::BaselineTest => (!receipt.success
                && receipt.stdout.contains("SOURCE_BEGIN")
                && receipt.stdout.contains(PATCH_FIND))
            .then_some(LoopProgramEventKind::ToolReceipt)
            .or(Some(LoopProgramEventKind::Error)),
            RuntimeRepairToolPhase::ApplyPatch => (receipt.success
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
                "runtime-live-repair-verify",
                self.bug_file.to_string_lossy().as_ref(),
                self.test_binary.to_string_lossy().as_ref(),
            ])
            .output()
            .expect("run live repair verifier");
        let receipt = RuntimeRepairVerificationReceipt {
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
                let repair_source =
                    fs::read_to_string(&self.bug_file).expect("read repair source for model");
                let request = ModelGatewayRequest::new(
                    self.endpoint.clone(),
                    vec![
                        system_gateway_message(
                            "You are the runtime live repair policy planner. Use only the supplied repair target. Reply with a typed patch receipt and no prose.",
                        ),
                        user_gateway_message(format!(
                            "The repair target contains a failing Rust unit test.\n\nsource:\n{}\n\nEmit these exact lines for one allowed file repair:\n{}\nFIND:{}\nREPLACE:{}",
                            repair_source,
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
                .then_some(LoopProgramEventKind::ToolRequest)
                .or(Some(LoopProgramEventKind::Error))
            }
            LoopProgramActionKind::Continue => self
                .tool_receipts
                .lock()
                .expect("tool receipts")
                .last()
                .filter(|receipt| {
                    receipt.phase == RuntimeRepairToolPhase::ApplyPatch && receipt.success
                })
                .map(|_| LoopProgramEventKind::ModelEvent)
                .or(Some(LoopProgramEventKind::Error)),
            LoopProgramActionKind::RewriteGraph => Some(LoopProgramEventKind::RuntimeReceipt),
            LoopProgramActionKind::Verify => self.run_verification_receipt(),
            _ => None,
        }
    }
}

#[derive(Clone, Copy)]
struct NoLiveRepairDecisionMapper;

impl LoopProgramEventMapper for NoLiveRepairDecisionMapper {
    fn next_event(
        &self,
        machine_receipt: &GenericLoopMachineReceipt,
        runtime_handoff_execution: &LoopProgramRuntimeHandoffExecutionReceipt,
    ) -> Option<LoopProgramEventKind> {
        match (
            &machine_receipt.action,
            runtime_handoff_execution.status.clone(),
        ) {
            (
                LoopProgramActionKind::DispatchTools,
                LoopProgramRuntimeHandoffExecutionReportStatus::Completed,
            ) => Some(LoopProgramEventKind::ToolReceipt),
            (
                LoopProgramActionKind::InvokeModel,
                LoopProgramRuntimeHandoffExecutionReportStatus::Denied,
            ) => Some(LoopProgramEventKind::Error),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RuntimeRepairToolPhase {
    BaselineTest,
    ApplyPatch,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RuntimeRepairToolReceipt {
    phase: RuntimeRepairToolPhase,
    success: bool,
    stdout: String,
    stderr: String,
    observed_source: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RuntimeRepairVerificationReceipt {
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
struct RuntimeRepairHybridHandoffExecutor {
    router: LoopProgramRuntimeHandoffRouter,
    agent_flow: AgentFlowLoopProgramRuntimeHandoffExecutor,
}

impl RuntimeRepairHybridHandoffExecutor {
    fn new() -> Self {
        let handlers = LoopProgramRuntimeHandoffRouterHandlers {
            model_handler: Arc::new(StaticLoopProgramRuntimeHandoffHandler::handled(
                LoopProgramRuntimeOwner::new("runtime.model.gateway.repair-planner"),
            )),
            verification_handler: Arc::new(StaticLoopProgramRuntimeHandoffHandler::handled(
                LoopProgramRuntimeOwner::new("runtime.verification.single-file"),
            )),
            graph_handler: Arc::new(StaticLoopProgramRuntimeHandoffHandler::handled(
                LoopProgramRuntimeOwner::new("runtime.graph.rewrite"),
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

impl LoopProgramRuntimeHandoffExecutor for RuntimeRepairHybridHandoffExecutor {
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

#[derive(Clone)]
struct RuntimeRepairNoLiveHandoffExecutor {
    router: LoopProgramRuntimeHandoffRouter,
    agent_flow: AgentFlowLoopProgramRuntimeHandoffExecutor,
}

impl RuntimeRepairNoLiveHandoffExecutor {
    fn new() -> Self {
        let handlers = LoopProgramRuntimeHandoffRouterHandlers {
            model_handler: Arc::new(StaticLoopProgramRuntimeHandoffHandler::denied(
                LoopProgramRuntimeOwner::new("runtime.model.gateway.no-live-llm"),
            )),
            control_handler: Arc::new(StaticLoopProgramRuntimeHandoffHandler::handled(
                LoopProgramRuntimeOwner::new("runtime.control"),
            )),
            ..LoopProgramRuntimeHandoffRouterHandlers::default()
        };
        Self {
            router: LoopProgramRuntimeHandoffRouter::new(handlers),
            agent_flow: AgentFlowLoopProgramRuntimeHandoffExecutor::new(
                LoopProgramRuntimeOwner::new("runtime.agent-flow.repair-tool.no-live"),
            ),
        }
    }
}

impl LoopProgramRuntimeHandoffExecutor for RuntimeRepairNoLiveHandoffExecutor {
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

fn scheme_projected_real_repair_loop_case() -> GerbilLoopCaseDriverProjectedLoopProgram {
    static PROJECTED_CASE: OnceLock<GerbilLoopCaseDriverProjectedLoopProgram> = OnceLock::new();

    PROJECTED_CASE
        .get_or_init(|| {
            let request = GerbilLoopCaseDriverProjectedLoopProgramRequest::new(
                SCHEME_REAL_REPAIR_CASE_ID,
                SCHEME_REAL_REPAIR_PROGRAM_ID,
            )
            .with_expected_vertical_trace_count(7)
            .with_profile_ref(SCHEME_REAL_REPAIR_PROFILE_REF)
            .with_live_llm_required(true)
            .with_required_capability("+tool-repair")
            .with_required_capability("+verification");

            load_gerbil_loop_case_driver_projected_loop_program(&request)
                .expect("real-repair-001 should load as a Scheme-projected LoopProgram")
        })
        .clone()
}

fn unique_temp_repair_workspace() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock after epoch")
        .as_nanos();
    env::temp_dir().join(format!(
        "marlin-runtime-live-repair-{}-{nanos}",
        std::process::id()
    ))
}

fn live_llm_timeout() -> Duration {
    env::var(LIVE_LLM_TIMEOUT_MS_ENV)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|millis| *millis > 0)
        .map(Duration::from_millis)
        .unwrap_or(Duration::from_millis(DEFAULT_LIVE_LLM_TIMEOUT_MS))
}
