use super::support::{assert_workspace_schema_artifact, loop_graph_artifact};
use marlin_agent_runtime::{RuntimeContext, TokioAgentRuntime};
use marlin_gerbil_scheme::{
    GerbilArtifactKind, GerbilCommandCompiler, GerbilCommandSpec, GerbilCompileRequest,
    GerbilSource,
};
use std::{env, time::Duration};
use tokio::time::sleep;

#[tokio::test]
async fn command_compiler_with_runtime_reads_typed_artifact_from_stdout() {
    let command = GerbilCommandSpec::new("/bin/sh").arg("-c").arg(
        "cat >/dev/null; printf '%s\n' '{\"artifact\":{\"LoopGraph\":{\"graph_id\":\"from-runtime-command\",\"nodes\":[],\"edges\":[]}}}'",
    );
    let compiler = GerbilCommandCompiler::new(command);
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let context = runtime.context();

    let artifact = compiler
        .compile_with_runtime(
            &context,
            GerbilSource::new("audit/control-plane", "(module audit/control-plane)"),
            GerbilArtifactKind::LoopGraph,
        )
        .await
        .expect("runtime command output should decode to requested artifact kind");

    assert_eq!(artifact, loop_graph_artifact("from-runtime-command"));
    assert_runtime_process_registry_empty(&context);
}

#[tokio::test]
async fn command_compiler_with_runtime_reads_batched_artifacts_from_stdout_lines() {
    let command = GerbilCommandSpec::new("/bin/sh").arg("-c").arg(
        "cat >/dev/null; printf '%s\n' '{\"artifact\":{\"LoopGraph\":{\"graph_id\":\"runtime-batch-graph\",\"nodes\":[],\"edges\":[]}}}' '{\"artifact\":{\"WorkspaceSchema\":{\"schema_id\":\"workspace-record\",\"required_properties\":[\"ID\",\"TITLE\"],\"todo_states\":[\"TODO\",\"DONE\"]}}}'",
    );
    let compiler = GerbilCommandCompiler::new(command);
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let context = runtime.context();

    let artifacts = compiler
        .compile_requests_with_runtime(
            &context,
            vec![
                GerbilCompileRequest::new(
                    GerbilSource::new("audit/control-plane", "(module audit/control-plane)"),
                    GerbilArtifactKind::LoopGraph,
                ),
                GerbilCompileRequest::new(
                    GerbilSource::new(
                        "audit/workspace-schema",
                        "(workspace-schema workspace-record)",
                    ),
                    GerbilArtifactKind::WorkspaceSchema,
                ),
            ],
        )
        .await
        .expect("runtime command output should decode newline-delimited artifacts");

    assert_eq!(artifacts.len(), 2);
    assert_eq!(artifacts[0], loop_graph_artifact("runtime-batch-graph"));
    assert_workspace_schema_artifact(artifacts[1].clone());
    assert_runtime_process_registry_empty(&context);
}

#[tokio::test]
async fn command_compiler_with_runtime_reports_stdout_and_stderr_diagnostics_when_command_fails() {
    let command = GerbilCommandSpec::new("/bin/sh").arg("-c").arg(
        "cat >/dev/null; printf '%s\n' 'adapter stdout expected LoopGraph'; printf '%s\n' 'adapter stderr expected LoopGraph' >&2; exit 70",
    );
    let compiler = GerbilCommandCompiler::new(command);
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let context = runtime.context();

    let error = compiler
        .compile_with_runtime(
            &context,
            GerbilSource::new("audit/control-plane", "(module audit/control-plane)"),
            GerbilArtifactKind::LoopGraph,
        )
        .await
        .unwrap_err();

    assert!(error.contains("gerbil compiler command failed"));
    assert!(error.contains("exit status: 70"));
    assert!(error.contains("stderr: adapter stderr expected LoopGraph"));
    assert!(error.contains("stdout: adapter stdout expected LoopGraph"));
    assert_runtime_process_registry_empty(&context);
}

#[cfg(unix)]
#[tokio::test]
async fn command_compiler_with_runtime_exposes_command_lifecycle_metadata() {
    let current_dir = env::temp_dir();
    let command = GerbilCommandSpec::new("/bin/sh")
        .arg("-c")
        .arg(
            "cat >/dev/null; sleep 1; printf '%s\n' \
             '{\"artifact\":{\"LoopGraph\":{\"graph_id\":\"runtime-lifecycle\",\"nodes\":[],\"edges\":[]}}}'",
        )
        .current_dir(current_dir.clone());
    let compiler = GerbilCommandCompiler::new(command);
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let context = runtime.context();
    let compile = compiler.compile_with_runtime(
        &context,
        GerbilSource::new("audit/control-plane", "(module audit/control-plane)"),
        GerbilArtifactKind::LoopGraph,
    );
    tokio::pin!(compile);

    tokio::select! {
        result = &mut compile => panic!("runtime command completed before metadata inspection: {result:?}"),
        _ = sleep(Duration::from_millis(100)) => {}
    }

    let active_processes = context.process_registry().active_processes();
    assert_eq!(active_processes.len(), 1);
    let command = active_processes[0]
        .command
        .as_ref()
        .expect("runtime process should carry command metadata");
    assert_eq!(command.command_kind.as_str(), "gerbil-compiler-command");
    assert_eq!(command.argv[0], "/bin/sh");
    assert_eq!(command.argv[1], "-c");
    assert_eq!(
        command.cwd.as_deref(),
        Some(current_dir.display().to_string().as_str())
    );
    assert_eq!(command.sub_agent_role, None);

    let artifact = compile
        .await
        .expect("runtime command output should decode to requested artifact kind");
    assert_eq!(artifact, loop_graph_artifact("runtime-lifecycle"));
    assert_runtime_process_registry_empty(&context);
}

fn assert_runtime_process_registry_empty(context: &RuntimeContext) {
    let registry = context.process_registry();
    assert!(registry.active_processes().is_empty());
    assert!(registry.cleanup_candidates().is_empty());
}
