use std::{future::pending, sync::Arc};

use marlin_agent_core::{
    AgentExecutionTrace, AgentExecutionTraceSummary, GraphLoopExecutionStatus, HookDispatcher,
    HookRegistry, ProviderRuntime, RuntimeContext, RuntimeEnvironmentRequest,
    RuntimeEnvironmentResolver, RuntimeEvent, RuntimeFuture, RuntimeTaskOutcome, TokioAgentRuntime,
};
use tokio_stream::StreamExt;

#[tokio::test]
async fn runtime_spawns_tasks_and_streams_events() {
    let (runtime, mut events) = TokioAgentRuntime::new(8);
    let context = runtime.context();

    let task = runtime.spawn(async move {
        context
            .emit(RuntimeEvent::new("provider", "started"))
            .await
            .expect("runtime event should be delivered");
        7
    });

    assert_eq!(task.join().await.expect("task should finish"), 7);
    assert_eq!(
        events.next().await,
        Some(RuntimeEvent::new("provider", "started"))
    );
}

#[tokio::test]
async fn child_runtime_observes_parent_cancellation() {
    let (runtime, _events) = TokioAgentRuntime::new(8);
    let child = runtime.child_runtime();

    assert!(!child.cancellation_token().is_cancelled());
    runtime.cancellation_token().cancel();
    assert!(child.cancellation_token().is_cancelled());
}

#[tokio::test]
async fn runtime_spawns_blocking_work() {
    let (runtime, _events) = TokioAgentRuntime::new(8);
    let task = runtime.spawn_blocking(|| 21 * 2);

    assert_eq!(task.join().await.expect("blocking task should finish"), 42);
}

#[tokio::test]
async fn runtime_cancellable_task_finishes_as_cancelled() {
    let (runtime, _events) = TokioAgentRuntime::new(8);
    let task = runtime.spawn_cancellable(pending::<usize>());

    runtime.cancellation_token().cancel();

    assert_eq!(
        task.join().await.expect("task should join"),
        RuntimeTaskOutcome::Cancelled
    );
}

struct EchoProvider;

impl ProviderRuntime for EchoProvider {
    type Request = String;
    type Response = String;

    fn run_provider(
        &self,
        request: Self::Request,
        context: RuntimeContext,
    ) -> RuntimeFuture<Self::Response> {
        Box::pin(async move {
            context
                .emit(RuntimeEvent::new("provider", "echo"))
                .await
                .expect("provider event should be delivered");
            request
        })
    }
}

#[tokio::test]
async fn runtime_spawns_provider_contract_with_context() {
    let (runtime, mut events) = TokioAgentRuntime::new(8);
    let task = runtime.spawn_provider(Arc::new(EchoProvider), "hello".to_owned());

    assert_eq!(
        task.join().await.expect("provider task should finish"),
        "hello"
    );
    assert_eq!(
        events.next().await,
        Some(RuntimeEvent::new("provider", "echo"))
    );
}

#[test]
fn core_facade_exposes_environment_resolver() {
    let environment = RuntimeEnvironmentResolver::new().resolve(
        RuntimeEnvironmentRequest::default()
            .with_custom_home("/tmp/marlin-home")
            .with_cwd("/tmp/workspace"),
    );

    assert_eq!(
        environment
            .home
            .as_ref()
            .expect("home should be resolved")
            .path,
        std::path::PathBuf::from("/tmp/marlin-home")
    );
}

#[test]
fn core_facade_exposes_hook_dispatcher() {
    let dispatcher = HookDispatcher::new(HookRegistry::new());

    assert_eq!(dispatcher.registry().registrations().len(), 0);
}

#[test]
fn core_facade_exposes_execution_trace_summary() {
    let trace = AgentExecutionTrace::new("run", "graph", GraphLoopExecutionStatus::Completed);
    let summary: AgentExecutionTraceSummary = trace.summary();

    assert_eq!(summary.run_id.as_str(), "run");
    assert_eq!(summary.graph_id.as_str(), "graph");
    assert_eq!(summary.status, GraphLoopExecutionStatus::Completed);
}
