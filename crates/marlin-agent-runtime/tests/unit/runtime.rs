use std::sync::Arc;

use marlin_agent_environment::{RuntimeEnvironmentRequest, RuntimeEnvironmentResolver};
use marlin_agent_protocol::{
    RuntimeEnvironmentActivationPolicy, RuntimeHome, RuntimeSandboxPolicy, RuntimeWorkspaceProject,
};
use marlin_agent_runtime::{
    CancellationToken, HookRuntime, RuntimeContext, RuntimeEnvironment, RuntimeExecutionIdentity,
    RuntimeFuture, SubAgentRuntime, TokioAgentRuntime, observability,
};
use tokio_stream::StreamExt;

#[tokio::test]
async fn runtime_emits_protocol_owned_events() {
    let (runtime, mut events) = TokioAgentRuntime::new(4);

    runtime
        .context()
        .emit(observability::runtime_event(
            "runtime.test".into(),
            "observed",
        ))
        .await
        .expect("event sink should be open");

    let event = events.next().await.expect("event should be emitted");
    assert_eq!(event.topic, "runtime.test");
    assert_eq!(event.message, "observed");
}

#[tokio::test]
async fn runtime_context_exposes_custom_environment() {
    let environment = RuntimeEnvironment::default()
        .with_home(RuntimeHome::custom("/tmp/marlin-home").with_profile("runtime"))
        .with_cwd("/tmp/workspace")
        .with_sandbox(RuntimeSandboxPolicy {
            writable_roots: vec!["/tmp/workspace".into()],
            network_access: true,
            exclude_tmpdir_env_var: false,
            exclude_slash_tmp: true,
        });

    let (runtime, _events) =
        TokioAgentRuntime::with_environment(4, CancellationToken::new(), environment.clone());

    assert_eq!(runtime.environment(), &environment);
    assert_eq!(runtime.context().environment(), &environment);
}

#[test]
fn runtime_context_carries_execution_identity_to_children() {
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let child_environment = RuntimeEnvironment::default().with_cwd("/tmp/child");
    let context = runtime
        .context()
        .with_execution_identity(RuntimeExecutionIdentity::new(
            "run-identity",
            "graph-identity",
        ));

    assert!(runtime.context().execution_identity().is_none());
    assert_eq!(
        context
            .execution_identity()
            .expect("execution identity should be present")
            .run_id(),
        "run-identity"
    );
    assert_eq!(
        context
            .execution_identity()
            .expect("execution identity should be present")
            .graph_id(),
        "graph-identity"
    );

    let child_context = context.child_context();
    assert_eq!(
        child_context
            .execution_identity()
            .expect("child context should inherit execution identity")
            .run_id(),
        "run-identity"
    );

    let child_context_with_environment =
        context.child_context_with_environment(child_environment.clone());
    assert_eq!(
        child_context_with_environment
            .execution_identity()
            .expect("environment child context should inherit execution identity")
            .graph_id(),
        "graph-identity"
    );
    assert_eq!(
        child_context_with_environment.environment(),
        &child_environment
    );
}

#[test]
fn runtime_context_shares_process_registry_with_children() {
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let process_registry = runtime.process_registry();
    process_registry.track(
        observability::RuntimeProcessObservation::new(
            2100,
            observability::RuntimeProcessKind::Tool,
            "tool:apply",
        )
        .with_started_at_ms(1),
    );

    let child_context = runtime.context().child_context();
    let child_registry = child_context.process_registry();
    let finished = child_registry
        .finish(2100, 2)
        .expect("process should be visible from child context");

    assert_eq!(
        finished.status,
        observability::RuntimeProcessStatus::Finished
    );
    assert!(process_registry.active_processes().is_empty());
}

#[tokio::test]
async fn sub_agent_can_run_with_child_environment() {
    let parent_environment = RuntimeEnvironment::default().with_cwd("/tmp/parent");
    let child_environment = RuntimeEnvironment::default()
        .with_home(RuntimeHome::custom("/tmp/marlin-home/sub/reviewer"))
        .with_cwd("/tmp/child");
    let (runtime, _events) = TokioAgentRuntime::with_environment(
        4,
        CancellationToken::new(),
        parent_environment.clone(),
    );

    let output = runtime
        .spawn_sub_agent_with_environment(
            Arc::new(EnvironmentEchoSubAgent),
            (),
            child_environment.clone(),
        )
        .join()
        .await
        .expect("sub-agent task should finish");

    assert_eq!(runtime.environment(), &parent_environment);
    assert_eq!(output, child_environment);
}

#[tokio::test]
async fn runtime_runs_with_resolved_active_workspace_project_environment() {
    let activation = RuntimeEnvironmentActivationPolicy::direnv_project().with_direnv_reload();
    let project = RuntimeWorkspaceProject::trusted("main", "/tmp/project")
        .with_project_config("/tmp/project/.marlin")
        .with_activation(activation.clone())
        .with_sandbox(RuntimeSandboxPolicy {
            writable_roots: vec!["/tmp/project".into()],
            network_access: true,
            exclude_tmpdir_env_var: false,
            exclude_slash_tmp: true,
        });
    let resolution = RuntimeEnvironmentResolver::new().resolve_with_receipt(
        RuntimeEnvironmentRequest::default()
            .with_custom_home("/tmp/marlin-home")
            .with_workspace_project(project)
            .with_active_workspace_project("main"),
    );
    let environment = resolution.environment;
    let (runtime, _events) =
        TokioAgentRuntime::with_environment(4, CancellationToken::new(), environment.clone());

    let output = runtime
        .spawn_sub_agent(Arc::new(EnvironmentEchoSubAgent), ())
        .join()
        .await
        .expect("sub-agent task should finish");
    let project_root = std::path::PathBuf::from("/tmp/project");

    assert_eq!(runtime.environment().cwd.as_ref(), Some(&project_root));
    assert_eq!(runtime.environment().activation, activation);
    assert_eq!(
        runtime
            .environment()
            .active_workspace_project
            .as_ref()
            .map(|project_id| project_id.as_str()),
        Some("main")
    );
    assert_eq!(output, environment);
}

#[tokio::test]
async fn hook_runtime_executes_with_runtime_environment() {
    let environment = RuntimeEnvironment::default()
        .with_home(RuntimeHome::custom("/tmp/marlin-home"))
        .with_cwd("/tmp/workspace");
    let (runtime, _events) =
        TokioAgentRuntime::with_environment(4, CancellationToken::new(), environment.clone());

    let (request, output_environment) = runtime
        .spawn_hook(Arc::new(EnvironmentEchoHook), "pre-tool".to_owned())
        .join()
        .await
        .expect("hook task should finish");

    assert_eq!(request, "pre-tool");
    assert_eq!(output_environment, environment);
}

#[derive(Clone, Debug)]
struct EnvironmentEchoSubAgent;

impl SubAgentRuntime for EnvironmentEchoSubAgent {
    type Input = ();
    type Output = RuntimeEnvironment;

    fn run_sub_agent(
        &self,
        _input: Self::Input,
        context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output> {
        let environment = context.environment().clone();
        Box::pin(async move { environment })
    }
}

#[derive(Clone, Debug)]
struct EnvironmentEchoHook;

impl HookRuntime for EnvironmentEchoHook {
    type Request = String;
    type Output = (String, RuntimeEnvironment);

    fn run_hook(
        &self,
        request: Self::Request,
        context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output> {
        let environment = context.environment().clone();
        Box::pin(async move { (request, environment) })
    }
}
