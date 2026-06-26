use marlin_gerbil_scheme::{
    GERBIL_ADAPTER_MODULE, GERBIL_LOADPATH_ENV, GerbilCommandProfile, GerbilResidentRuntimePlan,
    GerbilResidentRuntimeProcessStatus, GerbilResidentRuntimeShutdownStatus,
    GerbilResidentStrategyEventKind, GerbilResidentStrategyExecutionPerformanceScope,
    GerbilResidentStrategyExecutionRequest, GerbilResidentStrategyExecutionResponse,
    GerbilResidentStrategyExecutionStatus, GerbilResidentStrategyRequest, GerbilSchemeValue,
};
use tempfile::Builder;

#[test]
fn resident_runtime_process_plan_uses_prepared_batch_launcher() {
    let root = Builder::new()
        .prefix("marlin-resident-runtime-process-")
        .tempdir()
        .expect("resident runtime tempdir");
    let handle = GerbilResidentRuntimePlan::forked_context(root.path(), "forked-session")
        .with_command_profile(
            GerbilCommandProfile::new("/opt/gerbil/bin/gxi")
                .current_dir("/tmp")
                .env("CUSTOM_GERBIL_FLAG", "enabled")
                .arg("ignored-module-entrypoint"),
        )
        .prepare()
        .expect("prepare resident runtime");

    let process = handle.process_plan();
    let command_profile = process
        .command_profile
        .as_ref()
        .expect("resident process command profile");

    assert_eq!(
        process.status,
        GerbilResidentRuntimeProcessStatus::ReadyToSpawn
    );
    assert!(process.process_reuse_required);
    assert!(!process.state_isolated);
    assert_eq!(command_profile.program, "/opt/gerbil/bin/gxi");
    assert_eq!(command_profile.current_dir.as_deref(), Some("/tmp"));
    assert_eq!(
        command_profile
            .env
            .get("CUSTOM_GERBIL_FLAG")
            .map(String::as_str),
        Some("enabled")
    );
    assert!(command_profile.env.contains_key(GERBIL_LOADPATH_ENV));
    assert_eq!(command_profile.args.len(), 1);
    assert_eq!(command_profile.args[0], GERBIL_ADAPTER_MODULE);
    assert!(root.path().join("src/marlin/adapter.ss").exists());

    let receipt = handle.process_receipt();
    assert_eq!(
        receipt.status,
        GerbilResidentRuntimeProcessStatus::ReadyToSpawn
    );
    assert_eq!(receipt.written_asset_count, handle.written_assets().len());
}

#[test]
fn disabled_resident_runtime_process_plan_never_spawns() {
    let root = Builder::new()
        .prefix("marlin-resident-runtime-disabled-")
        .tempdir()
        .expect("resident runtime tempdir");
    let handle = GerbilResidentRuntimePlan::disabled(root.path())
        .prepare()
        .expect("prepare disabled resident runtime");
    let process = handle.process_plan();

    assert_eq!(process.status, GerbilResidentRuntimeProcessStatus::Disabled);
    assert!(!process.process_reuse_required);
    assert!(process.command_profile.is_none());
    assert_eq!(
        handle
            .spawn_process()
            .expect_err("disabled resident runtime should not spawn")
            .kind(),
        std::io::ErrorKind::InvalidInput
    );
}

#[test]
fn resident_runtime_process_owner_spawns_configured_command() {
    let root = Builder::new()
        .prefix("marlin-resident-runtime-spawn-")
        .tempdir()
        .expect("resident runtime tempdir");
    let handle = GerbilResidentRuntimePlan::shared_context(root.path(), "spawn-session")
        .with_command_profile(GerbilCommandProfile::new("true"))
        .prepare()
        .expect("prepare resident runtime");

    let mut process = handle
        .spawn_process()
        .expect("spawn resident runtime process");
    let child_id = process.child_id();
    let status = process.wait().expect("wait resident runtime process");

    assert!(child_id > 0);
    assert_eq!(
        process.plan().status,
        GerbilResidentRuntimeProcessStatus::ReadyToSpawn
    );
    assert!(status.success());
}

#[test]
fn resident_runtime_process_execution_receipt_preserves_typed_scheme_payload() {
    let root = Builder::new()
        .prefix("marlin-resident-runtime-typed-execution-")
        .tempdir()
        .expect("resident runtime tempdir");
    let handle = GerbilResidentRuntimePlan::shared_context(root.path(), "typed-execution-session")
        .with_command_profile(GerbilCommandProfile::new("yes"))
        .prepare()
        .expect("prepare resident runtime");

    let mut process = handle
        .spawn_process()
        .expect("spawn typed resident runtime process");
    let request = GerbilResidentStrategyExecutionRequest {
        strategy_request: GerbilResidentStrategyRequest::new(
            "policy-receipt",
            GerbilResidentStrategyEventKind::PolicyChange,
        ),
        payload: GerbilSchemeValue::text("typed-policy-projection"),
    };
    let receipt = process
        .strategy_execution_receipt(request, |request| {
            GerbilResidentStrategyExecutionResponse::executed(request.payload.clone())
        })
        .expect("typed resident execution receipt");
    let shutdown = process.shutdown().expect("shutdown typed resident runtime");

    assert_eq!(
        receipt.status,
        GerbilResidentStrategyExecutionStatus::Executed
    );
    assert_eq!(
        receipt.performance.scope,
        GerbilResidentStrategyExecutionPerformanceScope::AdmissionAndExecution
    );
    assert!(receipt.performance.executor_invoked);
    assert!(receipt.performance.process_reuse_required);
    assert!(receipt.performance.process_reuse_observed);
    assert_eq!(
        receipt.performance.child_id,
        receipt.request_receipt.child_id
    );
    assert_eq!(
        receipt.response_payload,
        Some(GerbilSchemeValue::text("typed-policy-projection"))
    );
    assert_eq!(
        shutdown.status,
        GerbilResidentRuntimeShutdownStatus::Terminated
    );
}
