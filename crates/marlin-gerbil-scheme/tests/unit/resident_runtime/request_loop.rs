use marlin_gerbil_scheme::{
    GERBIL_LOADPATH_ENV, GerbilCommandProfile, GerbilResidentRuntimeHealthStatus,
    GerbilResidentRuntimePlan, GerbilResidentRuntimeShutdownStatus,
    GerbilResidentStrategyEventKind, GerbilResidentStrategyExecutionPerformanceScope,
    GerbilResidentStrategyExecutionRequest, GerbilResidentStrategyExecutionResponse,
    GerbilResidentStrategyExecutionStatus, GerbilResidentStrategyGxiSmokeBridge,
    GerbilResidentStrategyRequest, GerbilResidentStrategyRequestStatus, GerbilSchemeValue,
    gerbil_runtime_loadpath, write_gerbil_runtime_assets,
};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use tempfile::Builder;

#[test]
fn resident_runtime_process_reports_health_and_terminating_shutdown() {
    let root = Builder::new()
        .prefix("marlin-resident-runtime-health-")
        .tempdir()
        .expect("resident runtime tempdir");
    let adapter = resident_sleeping_adapter(root.path(), "health-adapter");
    let handle = GerbilResidentRuntimePlan::shared_context(root.path(), "health-session")
        .with_command_profile(GerbilCommandProfile::new(adapter.to_string_lossy()))
        .prepare()
        .expect("prepare resident runtime");

    let mut process = handle
        .spawn_process()
        .expect("spawn resident runtime process");
    let health = process.health_receipt().expect("resident health receipt");

    assert_eq!(health.status, GerbilResidentRuntimeHealthStatus::Running);
    assert_eq!(
        health
            .session_id
            .as_ref()
            .map(|session_id| session_id.as_str()),
        Some("health-session")
    );
    assert!(health.process_reuse_required);
    assert!(health.exit_code.is_none());

    let request_receipt = process
        .strategy_request_receipt(
            GerbilResidentStrategyRequest::new(
                "health-request",
                GerbilResidentStrategyEventKind::DynamicReplan,
            )
            .with_session_id("health-session")
            .with_policy_epoch(13),
        )
        .expect("resident strategy request receipt");
    assert_eq!(
        request_receipt.status,
        GerbilResidentStrategyRequestStatus::Accepted
    );
    assert_eq!(request_receipt.lane_id.as_str(), "dynamic-replan");
    assert_eq!(request_receipt.policy_epoch, Some(13));
    assert_eq!(request_receipt.child_id, Some(health.child_id));
    assert_eq!(
        request_receipt.process_health,
        Some(GerbilResidentRuntimeHealthStatus::Running)
    );

    let response_payload =
        GerbilSchemeValue::record([("handled", GerbilSchemeValue::boolean(true))]);
    let execution_receipt = process
        .strategy_execution_receipt(
            GerbilResidentStrategyExecutionRequest::new(
                "execution-request",
                GerbilResidentStrategyEventKind::DynamicReplan,
                GerbilSchemeValue::record([("policy_epoch", GerbilSchemeValue::integer(13))]),
            )
            .with_session_id("health-session")
            .with_policy_epoch(13),
            |_| {
                GerbilResidentStrategyExecutionResponse::executed(response_payload.clone())
                    .with_derived_session_id("derived-health-session")
            },
        )
        .expect("resident strategy execution receipt");
    assert_eq!(
        execution_receipt.request_receipt.status,
        GerbilResidentStrategyRequestStatus::Accepted
    );
    assert_eq!(
        execution_receipt.status,
        GerbilResidentStrategyExecutionStatus::Executed,
        "{execution_receipt:#?}"
    );
    assert_eq!(
        execution_receipt.performance.scope,
        GerbilResidentStrategyExecutionPerformanceScope::AdmissionAndExecution
    );
    assert!(execution_receipt.performance.executor_invoked);
    assert!(execution_receipt.performance.process_reuse_required);
    assert!(execution_receipt.performance.process_reuse_observed);
    assert_eq!(
        execution_receipt.performance.child_id,
        execution_receipt.request_receipt.child_id
    );
    assert_eq!(execution_receipt.response_payload, Some(response_payload));
    assert_eq!(
        execution_receipt
            .derived_session_id
            .as_ref()
            .map(|session_id| session_id.as_str()),
        Some("derived-health-session")
    );

    let shutdown = process.shutdown().expect("resident terminating shutdown");
    assert_eq!(
        shutdown.status,
        GerbilResidentRuntimeShutdownStatus::Terminated
    );
    assert!(!shutdown.exit_success);
}

#[test]
fn resident_runtime_process_shutdown_reports_already_exited_child() {
    let root = Builder::new()
        .prefix("marlin-resident-runtime-exited-")
        .tempdir()
        .expect("resident runtime tempdir");
    let handle = GerbilResidentRuntimePlan::shared_context(root.path(), "exited-session")
        .with_command_profile(GerbilCommandProfile::new("true"))
        .prepare()
        .expect("prepare resident runtime");

    let mut process = handle
        .spawn_process()
        .expect("spawn resident runtime process");
    let status = process.wait().expect("wait for true command");
    assert!(status.success());

    let request_receipt = process
        .strategy_request_receipt(GerbilResidentStrategyRequest::new(
            "exited-request",
            GerbilResidentStrategyEventKind::PolicyChange,
        ))
        .expect("resident strategy request receipt for exited process");
    assert_eq!(
        request_receipt.status,
        GerbilResidentStrategyRequestStatus::ProcessNotRunning
    );
    assert_eq!(request_receipt.lane_id.as_str(), "policy-change");
    assert_eq!(
        request_receipt.process_health,
        Some(GerbilResidentRuntimeHealthStatus::Exited)
    );
    assert_eq!(request_receipt.child_id, Some(process.child_id()));

    let mut executor_called = false;
    let execution_receipt = process
        .strategy_execution_receipt(
            GerbilResidentStrategyExecutionRequest::new(
                "exited-execution-request",
                GerbilResidentStrategyEventKind::PolicyChange,
                GerbilSchemeValue::empty_record(),
            ),
            |_| {
                executor_called = true;
                GerbilResidentStrategyExecutionResponse::runtime_error(
                    "executor must not run after process exit",
                )
            },
        )
        .expect("resident strategy execution receipt for exited process");
    assert!(!executor_called);
    assert_eq!(
        execution_receipt.request_receipt.status,
        GerbilResidentStrategyRequestStatus::ProcessNotRunning
    );
    assert_eq!(
        execution_receipt.status,
        GerbilResidentStrategyExecutionStatus::AdmissionRejected
    );
    assert_eq!(
        execution_receipt.performance.scope,
        GerbilResidentStrategyExecutionPerformanceScope::AdmissionAndExecution
    );
    assert!(!execution_receipt.performance.executor_invoked);
    assert!(execution_receipt.performance.process_reuse_required);
    assert!(!execution_receipt.performance.process_reuse_observed);
    assert_eq!(
        execution_receipt.performance.child_id,
        execution_receipt.request_receipt.child_id
    );
    assert!(execution_receipt.response_payload.is_none());

    let shutdown = process
        .shutdown()
        .expect("resident already-exited shutdown");
    assert_eq!(
        shutdown.status,
        GerbilResidentRuntimeShutdownStatus::AlreadyExited
    );
    assert!(shutdown.exit_success);
}

#[test]
fn resident_runtime_execution_performance_receipts_prove_process_reuse() {
    let root = Builder::new()
        .prefix("marlin-resident-runtime-reuse-")
        .tempdir()
        .expect("resident runtime tempdir");
    let adapter = resident_sleeping_adapter(root.path(), "reuse-adapter");
    let handle = GerbilResidentRuntimePlan::forked_context(root.path(), "reuse-session")
        .with_command_profile(GerbilCommandProfile::new(adapter.to_string_lossy()))
        .prepare()
        .expect("prepare resident runtime");

    let mut process = handle
        .spawn_process()
        .expect("spawn reusable resident runtime process");
    let first = process
        .strategy_execution_receipt(
            GerbilResidentStrategyExecutionRequest::new(
                "first-reuse-request",
                GerbilResidentStrategyEventKind::PolicyChange,
                GerbilSchemeValue::text("first"),
            ),
            |request| {
                std::thread::sleep(std::time::Duration::from_millis(1));
                GerbilResidentStrategyExecutionResponse::executed(request.payload.clone())
            },
        )
        .expect("first resident reuse execution receipt");
    let second = process
        .strategy_execution_receipt(
            GerbilResidentStrategyExecutionRequest::new(
                "second-reuse-request",
                GerbilResidentStrategyEventKind::DynamicReplan,
                GerbilSchemeValue::text("second"),
            ),
            |request| {
                std::thread::sleep(std::time::Duration::from_millis(1));
                GerbilResidentStrategyExecutionResponse::executed(request.payload.clone())
            },
        )
        .expect("second resident reuse execution receipt");

    assert_eq!(
        first.status,
        GerbilResidentStrategyExecutionStatus::Executed
    );
    assert_eq!(
        second.status,
        GerbilResidentStrategyExecutionStatus::Executed
    );
    assert!(first.performance.executor_invoked);
    assert!(second.performance.executor_invoked);
    assert!(first.performance.process_reuse_observed);
    assert!(second.performance.process_reuse_observed);
    assert_eq!(first.performance.child_id, Some(process.child_id()));
    assert_eq!(second.performance.child_id, Some(process.child_id()));
    assert_eq!(first.performance.child_id, second.performance.child_id);
    assert!(first.performance.elapsed_micros > 0);
    assert!(second.performance.elapsed_micros > 0);

    let shutdown = process.shutdown().expect("resident reuse shutdown");
    assert_eq!(
        shutdown.status,
        GerbilResidentRuntimeShutdownStatus::Terminated
    );
}

#[test]
#[ignore]
fn resident_runtime_real_gxi_smoke_bridge_executes_typed_strategy() {
    let Some(gxi) = crate::command::support::local_gxi() else {
        return;
    };
    let root = Builder::new()
        .prefix("marlin-resident-runtime-real-gxi-")
        .tempdir()
        .expect("resident runtime real gxi tempdir");
    write_gerbil_runtime_assets(root.path()).expect("write Gerbil runtime assets");

    let adapter = resident_sleeping_adapter(root.path(), "real-gxi-resident-adapter");
    let handle = GerbilResidentRuntimePlan::shared_context(root.path(), "real-gxi-session")
        .with_command_profile(GerbilCommandProfile::new(adapter.to_string_lossy()))
        .prepare()
        .expect("prepare resident runtime for real gxi bridge");
    let mut process = handle
        .spawn_process()
        .expect("spawn resident runtime for real gxi bridge");

    let bridge_profile = GerbilCommandProfile::new(gxi.to_string_lossy().into_owned()).env(
        GERBIL_LOADPATH_ENV,
        gerbil_runtime_loadpath(root.path())
            .to_string_lossy()
            .into_owned(),
    );
    let mut bridge = GerbilResidentStrategyGxiSmokeBridge::marlin_deck_runtime(bridge_profile);
    let request_payload = GerbilSchemeValue::record([
        (
            "command",
            GerbilSchemeValue::text("resident strategy smoke"),
        ),
        ("hook", GerbilSchemeValue::text("dynamic-replan")),
        ("attempt", GerbilSchemeValue::integer(1)),
    ]);
    let execution_receipt = process
        .strategy_execution_receipt_with_executor(
            GerbilResidentStrategyExecutionRequest::new(
                "real-gxi-strategy-request",
                GerbilResidentStrategyEventKind::DynamicReplan,
                request_payload.clone(),
            )
            .with_session_id("real-gxi-session")
            .with_policy_epoch(21),
            &mut bridge,
        )
        .expect("real gxi resident strategy execution receipt");
    if execution_receipt.status != GerbilResidentStrategyExecutionStatus::Executed {
        eprintln!("real gxi resident strategy receipt: {execution_receipt:#?}");
    }

    assert_eq!(
        execution_receipt.status,
        GerbilResidentStrategyExecutionStatus::Executed
    );
    assert!(execution_receipt.performance.executor_invoked);
    assert!(execution_receipt.performance.process_reuse_observed);
    assert_eq!(
        execution_receipt
            .derived_session_id
            .as_ref()
            .map(|session_id| session_id.as_str()),
        Some("real-gxi-session:gerbil-resident")
    );

    let response_payload = execution_receipt
        .response_payload
        .as_ref()
        .expect("real gxi bridge returned typed payload");
    assert_eq!(
        response_payload
            .get("kind")
            .and_then(GerbilSchemeValue::as_text),
        Some("marlin.resident.strategy.procedure-response.v1")
    );
    assert_eq!(
        response_payload
            .get("request_id")
            .and_then(GerbilSchemeValue::as_text),
        Some("real-gxi-strategy-request")
    );
    assert_eq!(
        response_payload
            .get("policy_epoch")
            .and_then(GerbilSchemeValue::as_integer),
        Some(21)
    );
    assert_eq!(
        response_payload
            .get("handled")
            .and_then(GerbilSchemeValue::as_bool),
        Some(true)
    );
    assert_eq!(response_payload.get("payload"), Some(&request_payload));

    let shutdown = process
        .shutdown()
        .expect("resident real gxi bridge shutdown");
    assert_eq!(
        shutdown.status,
        GerbilResidentRuntimeShutdownStatus::Terminated
    );
}

fn resident_sleeping_adapter(root: &std::path::Path, name: &str) -> std::path::PathBuf {
    let path = root.join(name);
    std::fs::write(
        &path,
        r#"#!/bin/sh
sleep 30
"#,
    )
    .expect("write resident sleeping adapter");
    make_executable(&path);
    path
}

#[cfg(unix)]
fn make_executable(path: &std::path::Path) {
    let mut permissions = std::fs::metadata(path)
        .expect("resident sleeping adapter metadata")
        .permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions).expect("resident sleeping adapter executable");
}

#[cfg(not(unix))]
fn make_executable(_path: &std::path::Path) {}
