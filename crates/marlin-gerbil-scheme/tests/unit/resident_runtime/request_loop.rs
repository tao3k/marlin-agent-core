use marlin_gerbil_scheme::{
    GerbilArtifactKind, GerbilCommandProfile, GerbilCompileRequest, GerbilCompiledArtifact,
    GerbilResidentRuntimeHealthStatus, GerbilResidentRuntimePlan,
    GerbilResidentRuntimeShutdownStatus, GerbilSource,
};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use tempfile::Builder;

#[test]
fn resident_runtime_process_reports_health_and_graceful_shutdown() {
    let root = Builder::new()
        .prefix("marlin-resident-runtime-health-")
        .tempdir()
        .expect("resident runtime tempdir");
    let adapter = resident_fake_batch_adapter(root.path(), "health-adapter");
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

    let shutdown = process.shutdown().expect("resident graceful shutdown");
    assert_eq!(
        shutdown.status,
        GerbilResidentRuntimeShutdownStatus::GracefulExit
    );
    assert!(shutdown.exit_success);
}

#[test]
fn resident_runtime_process_compiles_multiple_requests_over_one_loop() {
    let root = Builder::new()
        .prefix("marlin-resident-runtime-loop-")
        .tempdir()
        .expect("resident runtime tempdir");
    let adapter = resident_fake_batch_adapter(root.path(), "loop-adapter");
    let handle = GerbilResidentRuntimePlan::shared_context(root.path(), "loop-session")
        .with_command_profile(GerbilCommandProfile::new(adapter.to_string_lossy()))
        .prepare()
        .expect("prepare resident runtime");
    let mut process = handle
        .spawn_process()
        .expect("spawn resident runtime process");

    let artifacts = process
        .compile_requests(vec![
            GerbilCompileRequest::new(
                GerbilSource::new("audit/control-plane", "(module audit/control-plane)"),
                GerbilArtifactKind::LoopGraph,
            ),
            GerbilCompileRequest::new(
                GerbilSource::new("audit/control-plane", "(module audit/control-plane)"),
                GerbilArtifactKind::LoopGraph,
            ),
        ])
        .expect("resident request loop should compile both requests");

    assert_eq!(artifacts.len(), 2);
    assert_loop_graph_id(&artifacts[0], "resident-loop-graph");
    assert_loop_graph_id(&artifacts[1], "resident-loop-graph");

    let shutdown = process.shutdown().expect("resident graceful shutdown");
    assert_eq!(
        shutdown.status,
        GerbilResidentRuntimeShutdownStatus::GracefulExit
    );
}

#[test]
fn resident_runtime_process_preserves_per_request_errors() {
    let root = Builder::new()
        .prefix("marlin-resident-runtime-loop-error-")
        .tempdir()
        .expect("resident runtime tempdir");
    let adapter = resident_fake_batch_adapter(root.path(), "loop-error-adapter");
    let handle = GerbilResidentRuntimePlan::shared_context(root.path(), "loop-error-session")
        .with_command_profile(GerbilCommandProfile::new(adapter.to_string_lossy()))
        .prepare()
        .expect("prepare resident runtime");
    let mut process = handle
        .spawn_process()
        .expect("spawn resident runtime process");

    let results = process
        .compile_request_results(vec![
            GerbilCompileRequest {
                source: GerbilSource::new("audit/control-plane", "(module audit/control-plane)"),
                expected: GerbilArtifactKind::LoopGraph,
                contract_facts: None,
            },
            GerbilCompileRequest {
                source: GerbilSource::new("audit/control-plane", "(invalid resident request)"),
                expected: GerbilArtifactKind::LoopGraph,
                contract_facts: None,
            },
        ])
        .expect("resident request loop should preserve result envelopes");

    assert_loop_graph_id(
        results[0]
            .as_ref()
            .expect("first resident request succeeds"),
        "resident-loop-graph",
    );
    let error = results[1]
        .as_ref()
        .expect_err("second resident request fails");
    assert!(error.contains("gerbil compiler command failed for request 1"));
    assert!(error.contains("resident fake adapter rejected invalid request"));

    let shutdown = process.shutdown().expect("resident graceful shutdown");
    assert_eq!(
        shutdown.status,
        GerbilResidentRuntimeShutdownStatus::GracefulExit
    );
}

fn assert_loop_graph_id(artifact: &GerbilCompiledArtifact, expected_id: &str) {
    let GerbilCompiledArtifact::LoopGraph(graph) = artifact else {
        panic!("expected loop graph artifact");
    };
    assert_eq!(graph.graph_id, expected_id);
}

fn resident_fake_batch_adapter(root: &std::path::Path, name: &str) -> std::path::PathBuf {
    let path = root.join(name);
    std::fs::write(
        &path,
        r#"#!/bin/sh
while IFS= read -r line; do
  case "$line" in
    *invalid*)
      printf '%s\n' '{"error":{"message":"resident fake adapter rejected invalid request"}}'
      ;;
    *)
      printf '%s\n' '{"artifact":{"LoopGraph":{"graph_id":"resident-loop-graph","nodes":[],"edges":[]}}}'
      ;;
  esac
done
"#,
    )
    .expect("write resident fake adapter");
    make_executable(&path);
    path
}

#[cfg(unix)]
fn make_executable(path: &std::path::Path) {
    let mut permissions = std::fs::metadata(path)
        .expect("resident fake adapter metadata")
        .permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions).expect("resident fake adapter executable");
}

#[cfg(not(unix))]
fn make_executable(_path: &std::path::Path) {}
