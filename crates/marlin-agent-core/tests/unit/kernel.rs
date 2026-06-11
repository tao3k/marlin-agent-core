use std::collections::BTreeMap;

use marlin_agent_core::{
    ExecutorName, GraphLoopExecutionRequest, GraphLoopExecutionStatus, GraphLoopKernel,
    GraphNodeExecutionReceipt, GraphNodeExecutor, GraphNodeInvocation, LoopGraph, LoopNodeSpec,
    NodeId, ProviderNodeAdapter, ProviderRuntime, RuntimeContext, RuntimeEvent, RuntimeFuture,
    SubAgentNodeAdapter, SubAgentRuntime, TokioAgentRuntime, TokioGraphLoopKernel, ToolNodeAdapter,
    ToolRuntime,
};
use tokio::time::{Duration, timeout};
use tokio_stream::StreamExt;

fn test_graph() -> LoopGraph {
    LoopGraph {
        graph_id: "graph-1".to_owned(),
        nodes: vec![
            LoopNodeSpec {
                id: "plan".to_owned(),
                executor: "provider".to_owned(),
                config: BTreeMap::new(),
            },
            LoopNodeSpec {
                id: "apply".to_owned(),
                executor: "tool".to_owned(),
                config: BTreeMap::new(),
            },
        ],
        edges: Vec::new(),
    }
}

fn adapter_graph() -> LoopGraph {
    LoopGraph {
        graph_id: "adapter-graph".to_owned(),
        nodes: vec![
            LoopNodeSpec {
                id: "plan".to_owned(),
                executor: "provider".to_owned(),
                config: BTreeMap::new(),
            },
            LoopNodeSpec {
                id: "apply".to_owned(),
                executor: "tool".to_owned(),
                config: BTreeMap::new(),
            },
            LoopNodeSpec {
                id: "review".to_owned(),
                executor: "sub-agent".to_owned(),
                config: BTreeMap::new(),
            },
        ],
        edges: Vec::new(),
    }
}

async fn next_event(events: &mut marlin_agent_core::RuntimeEventStream) -> RuntimeEvent {
    timeout(Duration::from_millis(100), events.next())
        .await
        .expect("runtime event should arrive before timeout")
        .expect("runtime event stream should stay open")
}

struct RecordingExecutor;

impl GraphNodeExecutor for RecordingExecutor {
    fn execute_node(
        &self,
        invocation: GraphNodeInvocation,
        context: RuntimeContext,
    ) -> RuntimeFuture<GraphNodeExecutionReceipt> {
        Box::pin(async move {
            context
                .emit(RuntimeEvent::new(
                    "test.executor",
                    format!(
                        "executor {} ran node {}",
                        invocation.executor.as_str(),
                        invocation.node_id.as_str()
                    ),
                ))
                .await
                .expect("executor event should be delivered");
            GraphNodeExecutionReceipt::completed(invocation.node_id, invocation.executor)
        })
    }
}

fn test_kernel() -> TokioGraphLoopKernel {
    TokioGraphLoopKernel::new("pending-run", "pending-graph")
        .with_executor("provider", RecordingExecutor)
        .with_executor("tool", RecordingExecutor)
}

struct RecordingProviderRuntime;

impl ProviderRuntime for RecordingProviderRuntime {
    type Request = GraphNodeInvocation;
    type Response = ();

    fn run_provider(
        &self,
        request: Self::Request,
        context: RuntimeContext,
    ) -> RuntimeFuture<Self::Response> {
        Box::pin(async move {
            context
                .emit(RuntimeEvent::new(
                    "test.provider",
                    format!("provider planned node {}", request.node_id.as_str()),
                ))
                .await
                .expect("provider event should be delivered");
        })
    }
}

struct RecordingToolRuntime;

impl ToolRuntime for RecordingToolRuntime {
    type Invocation = GraphNodeInvocation;
    type Output = ();

    fn run_tool(
        &self,
        invocation: Self::Invocation,
        context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output> {
        Box::pin(async move {
            context
                .emit(RuntimeEvent::new(
                    "test.tool",
                    format!("tool applied node {}", invocation.node_id.as_str()),
                ))
                .await
                .expect("tool event should be delivered");
        })
    }
}

struct RecordingSubAgentRuntime;

impl SubAgentRuntime for RecordingSubAgentRuntime {
    type Input = GraphNodeInvocation;
    type Output = ();

    fn run_sub_agent(
        &self,
        input: Self::Input,
        context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output> {
        Box::pin(async move {
            context
                .emit(RuntimeEvent::new(
                    "test.sub-agent",
                    format!("sub-agent reviewed node {}", input.node_id.as_str()),
                ))
                .await
                .expect("sub-agent event should be delivered");
        })
    }
}

fn adapter_kernel() -> TokioGraphLoopKernel {
    TokioGraphLoopKernel::new("pending-run", "pending-graph")
        .with_executor(
            "provider",
            ProviderNodeAdapter::new(RecordingProviderRuntime, std::convert::identity),
        )
        .with_executor(
            "tool",
            ToolNodeAdapter::new(RecordingToolRuntime, std::convert::identity),
        )
        .with_executor(
            "sub-agent",
            SubAgentNodeAdapter::new(RecordingSubAgentRuntime, std::convert::identity),
        )
}

fn receipt_from_backend(
    receipt: GraphNodeExecutionReceipt,
    _node_id: NodeId,
    _executor: ExecutorName,
) -> GraphNodeExecutionReceipt {
    receipt
}

struct FailingProviderRuntime;

impl ProviderRuntime for FailingProviderRuntime {
    type Request = GraphNodeInvocation;
    type Response = GraphNodeExecutionReceipt;

    fn run_provider(
        &self,
        request: Self::Request,
        context: RuntimeContext,
    ) -> RuntimeFuture<Self::Response> {
        Box::pin(async move {
            context
                .emit(RuntimeEvent::new(
                    "test.provider",
                    format!("provider failed node {}", request.node_id.as_str()),
                ))
                .await
                .expect("provider event should be delivered");
            GraphNodeExecutionReceipt::failed(
                request.node_id,
                request.executor,
                vec!["provider failed plan".to_owned()],
            )
        })
    }
}

struct FailingToolRuntime;

impl ToolRuntime for FailingToolRuntime {
    type Invocation = GraphNodeInvocation;
    type Output = GraphNodeExecutionReceipt;

    fn run_tool(
        &self,
        invocation: Self::Invocation,
        context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output> {
        Box::pin(async move {
            context
                .emit(RuntimeEvent::new(
                    "test.tool",
                    format!("tool failed node {}", invocation.node_id.as_str()),
                ))
                .await
                .expect("tool event should be delivered");
            GraphNodeExecutionReceipt::failed(
                invocation.node_id,
                invocation.executor,
                vec!["tool failed apply".to_owned()],
            )
        })
    }
}

struct FailingSubAgentRuntime;

impl SubAgentRuntime for FailingSubAgentRuntime {
    type Input = GraphNodeInvocation;
    type Output = GraphNodeExecutionReceipt;

    fn run_sub_agent(
        &self,
        input: Self::Input,
        context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output> {
        Box::pin(async move {
            context
                .emit(RuntimeEvent::new(
                    "test.sub-agent",
                    format!("sub-agent failed node {}", input.node_id.as_str()),
                ))
                .await
                .expect("sub-agent event should be delivered");
            GraphNodeExecutionReceipt::failed(
                input.node_id,
                input.executor,
                vec!["sub-agent failed review".to_owned()],
            )
        })
    }
}

#[tokio::test]
async fn tokio_kernel_driver_emits_events_and_completes() {
    let (runtime, mut events) = TokioAgentRuntime::new(16);
    let kernel = test_kernel();
    let request = GraphLoopExecutionRequest::new("run-1", test_graph());

    let task = kernel.spawn_execution(request, &runtime);
    let result = task.join().await.expect("kernel task should finish");

    assert_eq!(result.status, GraphLoopExecutionStatus::Completed);
    assert_eq!(result.visited_nodes, vec!["plan", "apply"]);
    assert_eq!(result.snapshot.run_id, "run-1");
    assert_eq!(result.snapshot.graph_id, "graph-1");
    assert_eq!(result.snapshot.active_node, None);
    assert_eq!(kernel.snapshot(), result.snapshot);

    assert_eq!(
        next_event(&mut events).await,
        RuntimeEvent::new("kernel.execution", "run run-1 started graph graph-1")
    );
    assert_eq!(
        next_event(&mut events).await,
        RuntimeEvent::new("kernel.node", "node plan started executor provider")
    );
    assert_eq!(
        next_event(&mut events).await,
        RuntimeEvent::new("test.executor", "executor provider ran node plan")
    );
    assert_eq!(
        next_event(&mut events).await,
        RuntimeEvent::new("kernel.node", "node plan completed executor provider")
    );
    assert_eq!(
        next_event(&mut events).await,
        RuntimeEvent::new("kernel.node", "node apply started executor tool")
    );
    assert_eq!(
        next_event(&mut events).await,
        RuntimeEvent::new("test.executor", "executor tool ran node apply")
    );
    assert_eq!(
        next_event(&mut events).await,
        RuntimeEvent::new("kernel.node", "node apply completed executor tool")
    );
    assert_eq!(
        next_event(&mut events).await,
        RuntimeEvent::new("kernel.execution", "run run-1 completed graph graph-1")
    );
}

#[tokio::test]
async fn tokio_kernel_driver_observes_parent_cancellation() {
    let (runtime, mut events) = TokioAgentRuntime::new(16);
    let kernel = test_kernel();
    let request = GraphLoopExecutionRequest::new("run-1", test_graph());

    runtime.cancellation_token().cancel();
    let result = kernel
        .spawn_execution(request, &runtime)
        .join()
        .await
        .expect("kernel task should finish");

    assert_eq!(result.status, GraphLoopExecutionStatus::Cancelled);
    assert!(result.visited_nodes.is_empty());
    assert_eq!(result.snapshot.active_node, None);
    assert_eq!(kernel.snapshot(), result.snapshot);

    assert_eq!(
        next_event(&mut events).await,
        RuntimeEvent::new("kernel.execution", "run run-1 started graph graph-1")
    );
    assert_eq!(
        next_event(&mut events).await,
        RuntimeEvent::new("kernel.execution", "run run-1 cancelled graph graph-1")
    );
}

#[tokio::test]
async fn tokio_kernel_driver_fails_missing_executor() {
    let (runtime, mut events) = TokioAgentRuntime::new(16);
    let kernel = TokioGraphLoopKernel::new("pending-run", "pending-graph");
    let request = GraphLoopExecutionRequest::new("run-1", test_graph());

    let result = kernel
        .spawn_execution(request, &runtime)
        .join()
        .await
        .expect("kernel task should finish");

    assert_eq!(result.status, GraphLoopExecutionStatus::Failed);
    assert!(result.visited_nodes.is_empty());
    assert_eq!(
        result.diagnostics,
        vec!["missing graph node executor `provider` for node plan"]
    );

    assert_eq!(
        next_event(&mut events).await,
        RuntimeEvent::new("kernel.execution", "run run-1 started graph graph-1")
    );
    assert_eq!(
        next_event(&mut events).await,
        RuntimeEvent::new("kernel.node", "node plan started executor provider")
    );
    assert_eq!(
        next_event(&mut events).await,
        RuntimeEvent::new("kernel.execution", "run run-1 failed graph graph-1")
    );
}

#[tokio::test]
async fn tokio_kernel_driver_runs_provider_tool_and_subagent_adapters() {
    let (runtime, mut events) = TokioAgentRuntime::new(24);
    let kernel = adapter_kernel();
    let request = GraphLoopExecutionRequest::new("run-adapter", adapter_graph());

    let result = kernel
        .spawn_execution(request, &runtime)
        .join()
        .await
        .expect("kernel task should finish");

    assert_eq!(result.status, GraphLoopExecutionStatus::Completed);
    assert_eq!(result.visited_nodes, vec!["plan", "apply", "review"]);

    assert_eq!(
        next_event(&mut events).await,
        RuntimeEvent::new(
            "kernel.execution",
            "run run-adapter started graph adapter-graph"
        )
    );
    assert_eq!(
        next_event(&mut events).await,
        RuntimeEvent::new("kernel.node", "node plan started executor provider")
    );
    assert_eq!(
        next_event(&mut events).await,
        RuntimeEvent::new("test.provider", "provider planned node plan")
    );
    assert_eq!(
        next_event(&mut events).await,
        RuntimeEvent::new("kernel.node", "node plan completed executor provider")
    );
    assert_eq!(
        next_event(&mut events).await,
        RuntimeEvent::new("kernel.node", "node apply started executor tool")
    );
    assert_eq!(
        next_event(&mut events).await,
        RuntimeEvent::new("test.tool", "tool applied node apply")
    );
    assert_eq!(
        next_event(&mut events).await,
        RuntimeEvent::new("kernel.node", "node apply completed executor tool")
    );
    assert_eq!(
        next_event(&mut events).await,
        RuntimeEvent::new("kernel.node", "node review started executor sub-agent")
    );
    assert_eq!(
        next_event(&mut events).await,
        RuntimeEvent::new(
            "kernel.sub_agent",
            "sub-agent sub-agent Started source ThreadSpawn { parent_run_id: Some(RunId(\"run-adapter\")), depth: 1, agent_path: None, agent_nickname: Some(\"sub-agent\"), agent_role: None } status node review"
        )
    );
    assert_eq!(
        next_event(&mut events).await,
        RuntimeEvent::new("test.sub-agent", "sub-agent reviewed node review")
    );
    assert_eq!(
        next_event(&mut events).await,
        RuntimeEvent::new(
            "kernel.sub_agent",
            "sub-agent sub-agent Stopped source ThreadSpawn { parent_run_id: Some(RunId(\"run-adapter\")), depth: 1, agent_path: None, agent_nickname: Some(\"sub-agent\"), agent_role: None } status node review"
        )
    );
    assert_eq!(
        next_event(&mut events).await,
        RuntimeEvent::new("kernel.node", "node review completed executor sub-agent")
    );
    assert_eq!(
        next_event(&mut events).await,
        RuntimeEvent::new(
            "kernel.execution",
            "run run-adapter completed graph adapter-graph"
        )
    );
}

#[tokio::test]
async fn provider_adapter_failure_fails_graph_execution() {
    let (runtime, mut events) = TokioAgentRuntime::new(16);
    let kernel = TokioGraphLoopKernel::new("pending-run", "pending-graph").with_executor(
        "provider",
        ProviderNodeAdapter::with_receipt_mapper(
            FailingProviderRuntime,
            std::convert::identity,
            receipt_from_backend,
        ),
    );
    let request = GraphLoopExecutionRequest::new("run-provider-fail", adapter_graph());

    let result = kernel
        .spawn_execution(request, &runtime)
        .join()
        .await
        .expect("kernel task should finish");

    assert_eq!(result.status, GraphLoopExecutionStatus::Failed);
    assert!(result.visited_nodes.is_empty());
    assert_eq!(result.diagnostics, vec!["provider failed plan"]);

    assert_eq!(
        next_event(&mut events).await,
        RuntimeEvent::new(
            "kernel.execution",
            "run run-provider-fail started graph adapter-graph"
        )
    );
    assert_eq!(
        next_event(&mut events).await,
        RuntimeEvent::new("kernel.node", "node plan started executor provider")
    );
    assert_eq!(
        next_event(&mut events).await,
        RuntimeEvent::new("test.provider", "provider failed node plan")
    );
    assert_eq!(
        next_event(&mut events).await,
        RuntimeEvent::new(
            "kernel.execution",
            "run run-provider-fail failed graph adapter-graph"
        )
    );
}

#[tokio::test]
async fn tool_adapter_failure_fails_graph_execution() {
    let (runtime, mut events) = TokioAgentRuntime::new(24);
    let kernel = TokioGraphLoopKernel::new("pending-run", "pending-graph")
        .with_executor(
            "provider",
            ProviderNodeAdapter::new(RecordingProviderRuntime, std::convert::identity),
        )
        .with_executor(
            "tool",
            ToolNodeAdapter::with_receipt_mapper(
                FailingToolRuntime,
                std::convert::identity,
                receipt_from_backend,
            ),
        );
    let request = GraphLoopExecutionRequest::new("run-tool-fail", adapter_graph());

    let result = kernel
        .spawn_execution(request, &runtime)
        .join()
        .await
        .expect("kernel task should finish");

    assert_eq!(result.status, GraphLoopExecutionStatus::Failed);
    assert_eq!(result.visited_nodes, vec!["plan"]);
    assert_eq!(result.diagnostics, vec!["tool failed apply"]);

    assert_eq!(
        next_event(&mut events).await,
        RuntimeEvent::new(
            "kernel.execution",
            "run run-tool-fail started graph adapter-graph"
        )
    );
    assert_eq!(
        next_event(&mut events).await,
        RuntimeEvent::new("kernel.node", "node plan started executor provider")
    );
    assert_eq!(
        next_event(&mut events).await,
        RuntimeEvent::new("test.provider", "provider planned node plan")
    );
    assert_eq!(
        next_event(&mut events).await,
        RuntimeEvent::new("kernel.node", "node plan completed executor provider")
    );
    assert_eq!(
        next_event(&mut events).await,
        RuntimeEvent::new("kernel.node", "node apply started executor tool")
    );
    assert_eq!(
        next_event(&mut events).await,
        RuntimeEvent::new("test.tool", "tool failed node apply")
    );
    assert_eq!(
        next_event(&mut events).await,
        RuntimeEvent::new(
            "kernel.execution",
            "run run-tool-fail failed graph adapter-graph"
        )
    );
}

#[tokio::test]
async fn subagent_adapter_failure_fails_graph_execution() {
    let (runtime, mut events) = TokioAgentRuntime::new(32);
    let kernel = TokioGraphLoopKernel::new("pending-run", "pending-graph")
        .with_executor(
            "provider",
            ProviderNodeAdapter::new(RecordingProviderRuntime, std::convert::identity),
        )
        .with_executor(
            "tool",
            ToolNodeAdapter::new(RecordingToolRuntime, std::convert::identity),
        )
        .with_executor(
            "sub-agent",
            SubAgentNodeAdapter::with_receipt_mapper(
                FailingSubAgentRuntime,
                std::convert::identity,
                receipt_from_backend,
            ),
        );
    let request = GraphLoopExecutionRequest::new("run-subagent-fail", adapter_graph());

    let result = kernel
        .spawn_execution(request, &runtime)
        .join()
        .await
        .expect("kernel task should finish");

    assert_eq!(result.status, GraphLoopExecutionStatus::Failed);
    assert_eq!(result.visited_nodes, vec!["plan", "apply"]);
    assert_eq!(result.diagnostics, vec!["sub-agent failed review"]);

    assert_eq!(
        next_event(&mut events).await,
        RuntimeEvent::new(
            "kernel.execution",
            "run run-subagent-fail started graph adapter-graph"
        )
    );
    assert_eq!(
        next_event(&mut events).await,
        RuntimeEvent::new("kernel.node", "node plan started executor provider")
    );
    assert_eq!(
        next_event(&mut events).await,
        RuntimeEvent::new("test.provider", "provider planned node plan")
    );
    assert_eq!(
        next_event(&mut events).await,
        RuntimeEvent::new("kernel.node", "node plan completed executor provider")
    );
    assert_eq!(
        next_event(&mut events).await,
        RuntimeEvent::new("kernel.node", "node apply started executor tool")
    );
    assert_eq!(
        next_event(&mut events).await,
        RuntimeEvent::new("test.tool", "tool applied node apply")
    );
    assert_eq!(
        next_event(&mut events).await,
        RuntimeEvent::new("kernel.node", "node apply completed executor tool")
    );
    assert_eq!(
        next_event(&mut events).await,
        RuntimeEvent::new("kernel.node", "node review started executor sub-agent")
    );
    assert_eq!(
        next_event(&mut events).await,
        RuntimeEvent::new(
            "kernel.sub_agent",
            "sub-agent sub-agent Started source ThreadSpawn { parent_run_id: Some(RunId(\"run-subagent-fail\")), depth: 1, agent_path: None, agent_nickname: Some(\"sub-agent\"), agent_role: None } status node review"
        )
    );
    assert_eq!(
        next_event(&mut events).await,
        RuntimeEvent::new("test.sub-agent", "sub-agent failed node review")
    );
    assert_eq!(
        next_event(&mut events).await,
        RuntimeEvent::new(
            "kernel.sub_agent",
            "sub-agent sub-agent Stopped source ThreadSpawn { parent_run_id: Some(RunId(\"run-subagent-fail\")), depth: 1, agent_path: None, agent_nickname: Some(\"sub-agent\"), agent_role: None } status node review"
        )
    );
    assert_eq!(
        next_event(&mut events).await,
        RuntimeEvent::new(
            "kernel.execution",
            "run run-subagent-fail failed graph adapter-graph"
        )
    );
}
