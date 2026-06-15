use marlin_gerbil_scheme::{
    GerbilCommandProfile, GerbilResidentRuntimeHealthStatus, GerbilResidentRuntimePlan,
    GerbilResidentRuntimeShutdownStatus,
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

    let shutdown = process
        .shutdown()
        .expect("resident already-exited shutdown");
    assert_eq!(
        shutdown.status,
        GerbilResidentRuntimeShutdownStatus::AlreadyExited
    );
    assert!(shutdown.exit_success);
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
