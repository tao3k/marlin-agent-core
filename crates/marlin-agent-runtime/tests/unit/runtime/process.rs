use std::process::Command;

use marlin_agent_runtime::{
    ManagedChildProcess, ManagedChildProcessSpec, TokioAgentRuntime, observability,
};

#[cfg(unix)]
#[test]
fn managed_child_process_removes_successful_process_from_active_tracking() {
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let context = runtime.context();
    let mut command = Command::new("sh");
    command.arg("-c").arg("exit 0");

    let child = ManagedChildProcess::spawn_with_spec(
        &context,
        &mut command,
        ManagedChildProcessSpec::new(observability::RuntimeProcessKind::Tool, "tool:echo")
            .with_started_at_ms(10),
    )
    .expect("managed child process should spawn");
    let pid = child.pid();
    assert_eq!(
        context
            .process_registry()
            .lock()
            .expect("process registry lock should be available")
            .get(pid)
            .map(|process| &process.status),
        Some(&observability::RuntimeProcessStatus::Running)
    );

    let status = child
        .wait_observed_at(20)
        .expect("managed child process should wait");

    assert!(status.success());
    assert!(
        context
            .process_registry()
            .lock()
            .expect("process registry lock should be available")
            .get(pid)
            .is_none()
    );
}

#[cfg(unix)]
#[test]
fn dropped_managed_child_process_becomes_cleanup_candidate() {
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let context = runtime.context();
    let mut command = Command::new("sh");
    command.arg("-c").arg("exit 0");
    let child = ManagedChildProcess::spawn_with_spec(
        &context,
        &mut command,
        ManagedChildProcessSpec::new(observability::RuntimeProcessKind::Tool, "tool:unawaited")
            .with_started_at_ms(10),
    )
    .expect("managed child process should spawn");
    let pid = child.pid();

    drop(child);

    let registry = context.process_registry();
    let registry = registry
        .lock()
        .expect("process registry lock should be available");
    assert_eq!(
        registry.get(pid).map(|process| &process.status),
        Some(&observability::RuntimeProcessStatus::CleanupRequested)
    );
    assert_eq!(registry.cleanup_candidates().len(), 1);
}

#[cfg(unix)]
#[test]
fn managed_child_process_kill_removes_process_from_active_tracking() {
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let context = runtime.context();
    let mut command = Command::new("sh");
    command.arg("-c").arg("sleep 10");
    let child = ManagedChildProcess::spawn_with_spec(
        &context,
        &mut command,
        ManagedChildProcessSpec::new(
            observability::RuntimeProcessKind::SubAgent,
            "sub-agent:long-task",
        )
        .with_started_at_ms(10),
    )
    .expect("managed child process should spawn");
    let pid = child.pid();

    let _status = child
        .kill_observed_at(20)
        .expect("managed child process should be killable");

    assert!(
        context
            .process_registry()
            .lock()
            .expect("process registry lock should be available")
            .get(pid)
            .is_none()
    );
}
