use std::{future::pending, sync::Arc};

use marlin_agent_core::{
    ProviderRuntime, RuntimeContext, RuntimeEvent, RuntimeFuture, RuntimeTaskOutcome,
    TokioAgentRuntime,
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
