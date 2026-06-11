use std::sync::Arc;

use marlin_agent_protocol::{RuntimeHome, RuntimeSandboxPolicy};
use marlin_agent_runtime::{
    CancellationToken, HookRuntime, RuntimeContext, RuntimeEnvironment, RuntimeEvent,
    RuntimeFuture, SubAgentRuntime, TokioAgentRuntime,
};
use tokio_stream::StreamExt;

#[tokio::test]
async fn runtime_emits_protocol_owned_events() {
    let (runtime, mut events) = TokioAgentRuntime::new(4);

    runtime
        .context()
        .emit(RuntimeEvent::new("runtime.test", "observed"))
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
