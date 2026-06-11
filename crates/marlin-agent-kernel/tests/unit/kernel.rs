use std::{sync::Arc, time::Duration};

use marlin_agent_hooks::{HookDispatcher, HookInvocation, HookRegistration, HookRegistry};
use marlin_agent_kernel::{
    GraphLoopExecutionRequest, GraphLoopExecutionStatus, GraphLoopKernel,
    GraphNodeExecutionReceipt, GraphNodeExecutor, GraphNodeInvocation, LoopEdgeSpec, LoopGraph,
    LoopNodeSpec, TokioGraphLoopKernel, ToolNodeAdapter,
};
use marlin_agent_protocol::{HookEventName, HookHandlerType, HookRunSummary};
use marlin_agent_runtime::{
    HookRuntime, RuntimeContext, RuntimeEvent, RuntimeFuture, TokioAgentRuntime, ToolRuntime,
};
use tokio_stream::StreamExt;

#[derive(Clone, Debug)]
struct CompletingExecutor;

impl GraphNodeExecutor for CompletingExecutor {
    fn execute_node(
        &self,
        invocation: GraphNodeInvocation,
        _context: RuntimeContext,
    ) -> RuntimeFuture<GraphNodeExecutionReceipt> {
        Box::pin(async move {
            GraphNodeExecutionReceipt::completed(invocation.node_id, invocation.executor)
        })
    }
}

#[tokio::test]
async fn kernel_executes_graph_node_through_registered_executor() {
    let graph = LoopGraph {
        graph_id: "graph".to_owned(),
        nodes: vec![LoopNodeSpec {
            id: "node-1".to_owned(),
            executor: "echo".to_owned(),
            config: Default::default(),
        }],
        edges: Vec::new(),
    };
    let request = GraphLoopExecutionRequest::new("run", graph);
    let kernel =
        TokioGraphLoopKernel::new("run", "graph").with_executor("echo", CompletingExecutor);
    let (runtime, mut events) = TokioAgentRuntime::new(8);

    let result = kernel
        .spawn_execution(request, &runtime)
        .join()
        .await
        .expect("kernel task should join");

    assert_eq!(result.status, GraphLoopExecutionStatus::Completed);
    assert_eq!(result.visited_nodes, vec!["node-1"]);
    assert_eq!(
        events.next().await.expect("start event").topic,
        "kernel.execution"
    );
}

#[tokio::test]
async fn kernel_executes_graph_nodes_in_edge_topology_order() {
    let graph = LoopGraph {
        graph_id: "graph".to_owned(),
        nodes: vec![
            LoopNodeSpec {
                id: "review".to_owned(),
                executor: "echo".to_owned(),
                config: Default::default(),
            },
            LoopNodeSpec {
                id: "apply".to_owned(),
                executor: "echo".to_owned(),
                config: Default::default(),
            },
            LoopNodeSpec {
                id: "plan".to_owned(),
                executor: "echo".to_owned(),
                config: Default::default(),
            },
        ],
        edges: vec![
            LoopEdgeSpec {
                from: "plan".to_owned(),
                to: "apply".to_owned(),
                condition: None,
            },
            LoopEdgeSpec {
                from: "apply".to_owned(),
                to: "review".to_owned(),
                condition: None,
            },
        ],
    };
    let request = GraphLoopExecutionRequest::new("run", graph);
    let kernel =
        TokioGraphLoopKernel::new("run", "graph").with_executor("echo", CompletingExecutor);
    let (runtime, _events) = TokioAgentRuntime::new(8);

    let result = kernel
        .spawn_execution(request, &runtime)
        .join()
        .await
        .expect("kernel task should join");

    assert_eq!(result.status, GraphLoopExecutionStatus::Completed);
    assert_eq!(result.visited_nodes, vec!["plan", "apply", "review"]);
}

#[tokio::test]
async fn kernel_rejects_edges_that_reference_missing_nodes() {
    let graph = LoopGraph {
        graph_id: "graph".to_owned(),
        nodes: vec![LoopNodeSpec {
            id: "plan".to_owned(),
            executor: "echo".to_owned(),
            config: Default::default(),
        }],
        edges: vec![LoopEdgeSpec {
            from: "plan".to_owned(),
            to: "apply".to_owned(),
            condition: None,
        }],
    };
    let request = GraphLoopExecutionRequest::new("run", graph);
    let kernel =
        TokioGraphLoopKernel::new("run", "graph").with_executor("echo", CompletingExecutor);
    let (runtime, _events) = TokioAgentRuntime::new(8);

    let result = kernel
        .spawn_execution(request, &runtime)
        .join()
        .await
        .expect("kernel task should join");

    assert_eq!(result.status, GraphLoopExecutionStatus::Failed);
    assert_eq!(result.visited_nodes, Vec::<String>::new());
    assert_eq!(
        result.diagnostics,
        vec!["graph edge references missing target node `apply`"]
    );
}

#[tokio::test]
async fn kernel_rejects_duplicate_node_ids() {
    let graph = LoopGraph {
        graph_id: "graph".to_owned(),
        nodes: vec![
            LoopNodeSpec {
                id: "plan".to_owned(),
                executor: "echo".to_owned(),
                config: Default::default(),
            },
            LoopNodeSpec {
                id: "plan".to_owned(),
                executor: "echo".to_owned(),
                config: Default::default(),
            },
        ],
        edges: Vec::new(),
    };
    let request = GraphLoopExecutionRequest::new("run", graph);
    let kernel =
        TokioGraphLoopKernel::new("run", "graph").with_executor("echo", CompletingExecutor);
    let (runtime, _events) = TokioAgentRuntime::new(8);

    let result = kernel
        .spawn_execution(request, &runtime)
        .join()
        .await
        .expect("kernel task should join");

    assert_eq!(result.status, GraphLoopExecutionStatus::Failed);
    assert_eq!(
        result.diagnostics,
        vec!["graph contains duplicate node ids: plan"]
    );
}

#[tokio::test]
async fn kernel_rejects_conditional_edges_until_condition_evaluation_exists() {
    let graph = LoopGraph {
        graph_id: "graph".to_owned(),
        nodes: vec![
            LoopNodeSpec {
                id: "plan".to_owned(),
                executor: "echo".to_owned(),
                config: Default::default(),
            },
            LoopNodeSpec {
                id: "apply".to_owned(),
                executor: "echo".to_owned(),
                config: Default::default(),
            },
        ],
        edges: vec![LoopEdgeSpec {
            from: "plan".to_owned(),
            to: "apply".to_owned(),
            condition: Some("approved".to_owned()),
        }],
    };
    let request = GraphLoopExecutionRequest::new("run", graph);
    let kernel =
        TokioGraphLoopKernel::new("run", "graph").with_executor("echo", CompletingExecutor);
    let (runtime, _events) = TokioAgentRuntime::new(8);

    let result = kernel
        .spawn_execution(request, &runtime)
        .join()
        .await
        .expect("kernel task should join");

    assert_eq!(result.status, GraphLoopExecutionStatus::Failed);
    assert_eq!(
        result.diagnostics,
        vec!["conditional graph edge plan -> apply is not supported"]
    );
}

#[tokio::test]
async fn kernel_rejects_cyclic_edge_topologies() {
    let graph = LoopGraph {
        graph_id: "graph".to_owned(),
        nodes: vec![
            LoopNodeSpec {
                id: "plan".to_owned(),
                executor: "echo".to_owned(),
                config: Default::default(),
            },
            LoopNodeSpec {
                id: "apply".to_owned(),
                executor: "echo".to_owned(),
                config: Default::default(),
            },
        ],
        edges: vec![
            LoopEdgeSpec {
                from: "plan".to_owned(),
                to: "apply".to_owned(),
                condition: None,
            },
            LoopEdgeSpec {
                from: "apply".to_owned(),
                to: "plan".to_owned(),
                condition: None,
            },
        ],
    };
    let request = GraphLoopExecutionRequest::new("run", graph);
    let kernel =
        TokioGraphLoopKernel::new("run", "graph").with_executor("echo", CompletingExecutor);
    let (runtime, _events) = TokioAgentRuntime::new(8);

    let result = kernel
        .spawn_execution(request, &runtime)
        .join()
        .await
        .expect("kernel task should join");

    assert_eq!(result.status, GraphLoopExecutionStatus::Failed);
    assert_eq!(
        result.diagnostics,
        vec!["graph edge topology contains a cycle; pending nodes: plan, apply"]
    );
}

#[tokio::test]
async fn tool_node_adapter_dispatches_pre_and_post_tool_hooks() {
    let hook_dispatcher = HookDispatcher::new(
        HookRegistry::new()
            .with_registration(HookRegistration::new(
                "pre",
                HookEventName::PreToolUse,
                HookHandlerType::Command,
                Arc::new(OrderingHook::new("pre-run", "pre")),
            ))
            .with_registration(HookRegistration::new(
                "post",
                HookEventName::PostToolUse,
                HookHandlerType::Command,
                Arc::new(OrderingHook::new("post-run", "post")),
            )),
    );
    let tool_adapter = ToolNodeAdapter::new(OrderingTool, |invocation: GraphNodeInvocation| {
        invocation.node_id.into_string()
    })
    .with_hook_dispatcher(hook_dispatcher);
    let graph = LoopGraph {
        graph_id: "graph".to_owned(),
        nodes: vec![LoopNodeSpec {
            id: "tool-node".to_owned(),
            executor: "tool".to_owned(),
            config: Default::default(),
        }],
        edges: Vec::new(),
    };
    let request = GraphLoopExecutionRequest::new("run", graph);
    let kernel = TokioGraphLoopKernel::new("run", "graph").with_executor("tool", tool_adapter);
    let (runtime, mut events) = TokioAgentRuntime::new(16);

    let result = kernel
        .spawn_execution(request, &runtime)
        .join()
        .await
        .expect("kernel task should join");
    let order = tokio::time::timeout(Duration::from_secs(1), async {
        let mut order = Vec::new();
        while order.len() < 3 {
            let event = events.next().await.expect("event should be emitted");
            if event.topic == "test.order" {
                order.push(event.message);
            }
        }
        order
    })
    .await
    .expect("order events should arrive");

    assert_eq!(result.status, GraphLoopExecutionStatus::Completed);
    assert_eq!(order, vec!["pre", "tool", "post"]);
}

#[derive(Clone, Debug)]
struct OrderingTool;

impl ToolRuntime for OrderingTool {
    type Invocation = String;
    type Output = String;

    fn run_tool(
        &self,
        invocation: Self::Invocation,
        context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output> {
        Box::pin(async move {
            context
                .emit(RuntimeEvent::new("test.order", "tool"))
                .await
                .expect("tool order event should be delivered");
            invocation
        })
    }
}

#[derive(Clone, Debug)]
struct OrderingHook {
    id: &'static str,
    label: &'static str,
}

impl OrderingHook {
    fn new(id: &'static str, label: &'static str) -> Self {
        Self { id, label }
    }
}

impl HookRuntime for OrderingHook {
    type Request = HookInvocation;
    type Output = HookRunSummary;

    fn run_hook(
        &self,
        request: Self::Request,
        context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output> {
        let id = self.id;
        let label = self.label;
        Box::pin(async move {
            context
                .emit(RuntimeEvent::new("test.order", label))
                .await
                .expect("hook order event should be delivered");
            HookRunSummary::running(id, request.event_name, HookHandlerType::Command).completed()
        })
    }
}
