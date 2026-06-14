use std::io::ErrorKind;
use std::process::Stdio;

use marlin_agent_runtime::{
    AsyncManagedChildProcess, ManagedChildProcessSpec, SubAgentSpawnProfile, TokioAgentRuntime,
    observability,
};
use tokio::io::AsyncWriteExt;
use tokio::process::Command as AsyncCommand;

#[cfg(unix)]
#[tokio::test]
async fn async_managed_child_process_removes_successful_process_from_active_tracking() {
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let context = runtime.context();
    let mut command = AsyncCommand::new("sh");
    command.arg("-c").arg("exit 0");

    let child = AsyncManagedChildProcess::spawn_with_spec(
        &context,
        command,
        ManagedChildProcessSpec::new(observability::RuntimeProcessKind::Tool, "tool:async-echo")
            .with_started_at_ms(10),
    )
    .await
    .expect("async managed child process should spawn");
    let pid = child.pid();
    assert_eq!(
        context
            .process_registry()
            .get(pid)
            .map(|process| process.status),
        Some(observability::RuntimeProcessStatus::Running)
    );

    let status = child
        .wait_observed_at(20)
        .await
        .expect("async managed child process should wait");

    assert!(status.success());
    assert!(context.process_registry().get(pid).is_none());
}

#[cfg(unix)]
#[tokio::test]
async fn async_managed_child_process_records_command_lifecycle_metadata() {
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let context = runtime.context();
    let mut command = AsyncCommand::new("sh");
    command.arg("-c").arg("sleep 1");
    let command_observation = observability::RuntimeCommandObservation::new("shell-command")
        .with_argv(["sh", "-c", "sleep 1"])
        .with_cwd("/tmp")
        .with_sub_agent_source("kernel-node")
        .with_sub_agent_profile(
            SubAgentSpawnProfile::new("researcher", "asp_explorer", "research")
                .with_nickname("Galileo"),
        );

    let child = AsyncManagedChildProcess::spawn_with_spec(
        &context,
        command,
        ManagedChildProcessSpec::new(
            observability::RuntimeProcessKind::Tool,
            "tool:async-command-lifecycle",
        )
        .with_command(command_observation.clone())
        .with_started_at_ms(10),
    )
    .await
    .expect("async managed child process should spawn");
    let pid = child.pid();
    let process = context
        .process_registry()
        .get(pid)
        .expect("managed child process should be active");

    assert_eq!(process.command, Some(command_observation.clone()));
    let profile = process
        .command
        .as_ref()
        .and_then(|command| command.sub_agent_profile.as_ref())
        .expect("managed child process should preserve sub-agent profile");
    assert_eq!(profile.profile_id.as_str(), "researcher");
    assert_eq!(profile.agent_type.as_str(), "asp_explorer");
    assert_eq!(profile.role, "research");
    assert_eq!(profile.nickname.as_deref(), Some("Galileo"));

    child
        .kill()
        .await
        .expect("managed child process should be killable");
    assert!(context.process_registry().active_processes().is_empty());
}

#[cfg(unix)]
#[tokio::test]
async fn dropped_async_managed_child_process_becomes_cleanup_candidate() {
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let context = runtime.context();
    let mut command = AsyncCommand::new("sh");
    command.arg("-c").arg("sleep 10");

    let child = AsyncManagedChildProcess::spawn_with_spec(
        &context,
        command,
        ManagedChildProcessSpec::new(observability::RuntimeProcessKind::Tool, "tool:async-drop")
            .with_started_at_ms(10),
    )
    .await
    .expect("async managed child process should spawn");
    let pid = child.pid();

    drop(child);

    let registry = context.process_registry();
    assert_eq!(
        registry.get(pid).map(|process| process.status),
        Some(observability::RuntimeProcessStatus::CleanupRequested)
    );
    assert_eq!(registry.cleanup_candidates().len(), 1);
}

#[cfg(unix)]
#[tokio::test]
async fn async_managed_child_process_rejects_duplicate_active_handle() {
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let context = runtime.context();
    let handle = observability::RuntimeProcessHandle::new("tool:async-shared-handle");
    let mut first_command = AsyncCommand::new("sh");
    first_command.arg("-c").arg("sleep 10");
    let first = AsyncManagedChildProcess::spawn_with_spec(
        &context,
        first_command,
        ManagedChildProcessSpec::new(observability::RuntimeProcessKind::Tool, "tool:async-first")
            .with_handle(handle.clone())
            .with_started_at_ms(10),
    )
    .await
    .expect("first async managed child process should spawn");
    let first_pid = first.pid();
    let mut second_command = AsyncCommand::new("sh");
    second_command.arg("-c").arg("sleep 10");

    let error = AsyncManagedChildProcess::spawn_with_spec(
        &context,
        second_command,
        ManagedChildProcessSpec::new(observability::RuntimeProcessKind::Tool, "tool:async-second")
            .with_handle(handle.clone())
            .with_started_at_ms(11),
    )
    .await
    .expect_err("duplicate active handle should be rejected");

    assert_eq!(error.kind(), ErrorKind::AlreadyExists);
    {
        let registry = context.process_registry();
        assert_eq!(registry.active_processes().len(), 1);
        assert_eq!(
            registry.get_by_handle(&handle).map(|process| process.pid),
            Some(first_pid)
        );
    }

    first
        .kill_observed_at(20)
        .await
        .expect("first async managed child process should be killable");
}

#[tokio::test]
async fn async_managed_child_process_spawn_failure_does_not_pollute_registry() {
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let context = runtime.context();
    let command = AsyncCommand::new("__marlin_agent_runtime_missing_command__");

    let error = AsyncManagedChildProcess::spawn_with_spec(
        &context,
        command,
        ManagedChildProcessSpec::new(
            observability::RuntimeProcessKind::Tool,
            "tool:async-missing",
        ),
    )
    .await
    .expect_err("missing command should not spawn");

    assert_eq!(error.kind(), ErrorKind::NotFound);
    assert!(context.process_registry().active_processes().is_empty());
}

#[cfg(unix)]
#[tokio::test]
async fn async_managed_child_process_captures_stdout_and_stderr() {
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let context = runtime.context();
    let mut command = AsyncCommand::new("sh");
    command
        .arg("-c")
        .arg("read line; printf 'stdout:%s\\n' \"$line\"; printf 'stderr:%s\\n' \"$line\" >&2")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = AsyncManagedChildProcess::spawn_with_spec(
        &context,
        command,
        ManagedChildProcessSpec::new(observability::RuntimeProcessKind::Tool, "tool:async-stdio")
            .with_started_at_ms(10),
    )
    .await
    .expect("async managed child process should spawn");
    let pid = child.pid();
    let mut stdin = child
        .take_stdin()
        .expect("async managed child process should expose stdin");
    stdin
        .write_all(b"gerbil\n")
        .await
        .expect("stdin write should succeed");
    drop(stdin);

    let output = child
        .wait_with_output_observed_at(20)
        .await
        .expect("async managed child process should collect output");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "stdout:gerbil\n");
    assert_eq!(String::from_utf8_lossy(&output.stderr), "stderr:gerbil\n");
    assert!(context.process_registry().get(pid).is_none());
}

#[cfg(unix)]
#[tokio::test]
async fn async_managed_child_process_captures_failed_output_and_removes_tracking() {
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let context = runtime.context();
    let mut command = AsyncCommand::new("sh");
    command
        .arg("-c")
        .arg("printf 'bad stdout'; printf 'bad stderr' >&2; exit 7")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let child = AsyncManagedChildProcess::spawn_with_spec(
        &context,
        command,
        ManagedChildProcessSpec::new(
            observability::RuntimeProcessKind::Tool,
            "tool:async-failed-output",
        )
        .with_started_at_ms(10),
    )
    .await
    .expect("async managed child process should spawn");
    let pid = child.pid();

    let output = child
        .wait_with_output_observed_at(20)
        .await
        .expect("async managed child process should collect failed output");

    assert!(!output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "bad stdout");
    assert_eq!(String::from_utf8_lossy(&output.stderr), "bad stderr");
    assert!(context.process_registry().get(pid).is_none());
}
