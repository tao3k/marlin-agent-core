use std::{sync::Arc, time::Duration};

use marlin_agent_hooks::{HookDispatcher, HookInvocation, HookRegistration, HookRegistry};
use marlin_agent_kernel::{
    GraphLoopExecutionRequest, GraphLoopExecutionStatus, GraphLoopKernel, GraphNodeInvocation,
    LoopGraph, LoopNodeSpec, ProviderNodeAdapter, SubAgentNodeAdapter, TokioGraphLoopKernel,
    ToolNodeAdapter,
};
use marlin_agent_protocol::{HookEventName, HookHandlerType, HookRunSummary};
use marlin_agent_runtime::{
    HookRuntime, ProviderRuntime, RuntimeContext, RuntimeEvent, RuntimeFuture, SubAgentRuntime,
    TokioAgentRuntime, ToolRuntime,
};
use tokio_stream::StreamExt;

#[tokio::test]
async fn tool_node_adapter_dispatches_pre_and_post_tool_hooks() {
    let tool_adapter = ToolNodeAdapter::new(OrderingTool, |invocation: GraphNodeInvocation| {
        invocation.node_id.into_string()
    })
    .with_hook_dispatcher(ordering_hook_dispatcher());
    let request = GraphLoopExecutionRequest::new("run", single_node_graph("tool-node", "tool"));
    let kernel = TokioGraphLoopKernel::new("run", "graph").with_executor("tool", tool_adapter);
    let (runtime, mut events) = TokioAgentRuntime::new(16);

    let result = kernel
        .spawn_execution(request, &runtime)
        .join()
        .await
        .expect("kernel task should join");
    let order = collect_order_events(&mut events).await;

    assert_eq!(result.status, GraphLoopExecutionStatus::Completed);
    assert_eq!(order, vec!["pre", "tool", "post"]);
}

#[tokio::test]
async fn provider_node_adapter_dispatches_pre_and_post_tool_hooks() {
    let provider_adapter =
        ProviderNodeAdapter::new(OrderingProvider, |invocation: GraphNodeInvocation| {
            invocation.node_id.into_string()
        })
        .with_hook_dispatcher(ordering_hook_dispatcher());
    let request =
        GraphLoopExecutionRequest::new("run", single_node_graph("provider-node", "provider"));
    let kernel =
        TokioGraphLoopKernel::new("run", "graph").with_executor("provider", provider_adapter);
    let (runtime, mut events) = TokioAgentRuntime::new(16);

    let result = kernel
        .spawn_execution(request, &runtime)
        .join()
        .await
        .expect("kernel task should join");
    let order = collect_order_events(&mut events).await;

    assert_eq!(result.status, GraphLoopExecutionStatus::Completed);
    assert_eq!(order, vec!["pre", "provider", "post"]);
}

#[tokio::test]
async fn sub_agent_node_adapter_dispatches_lifecycle_hooks_and_activity() {
    let sub_agent_adapter =
        SubAgentNodeAdapter::new(OrderingSubAgent, |invocation: GraphNodeInvocation| {
            invocation.node_id.into_string()
        })
        .with_hook_dispatcher(sub_agent_hook_dispatcher());
    let request =
        GraphLoopExecutionRequest::new("run", single_node_graph("sub-agent-node", "sub-agent"));
    let kernel =
        TokioGraphLoopKernel::new("run", "graph").with_executor("sub-agent", sub_agent_adapter);
    let (runtime, mut events) = TokioAgentRuntime::new(16);

    let result = kernel
        .spawn_execution(request, &runtime)
        .join()
        .await
        .expect("kernel task should join");
    let (order, activity) = collect_sub_agent_events(&mut events).await;

    assert_eq!(result.status, GraphLoopExecutionStatus::Completed);
    assert_eq!(
        order,
        vec!["sub-agent-start", "sub-agent", "sub-agent-stop"]
    );
    assert_eq!(activity.len(), 2);
    assert!(activity[0].contains("Started"));
    assert!(activity[0].contains("sub-agent"));
    assert!(activity[1].contains("Stopped"));
    assert!(activity[1].contains("sub-agent"));
}

fn ordering_hook_dispatcher() -> HookDispatcher {
    HookDispatcher::new(
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
    )
}

fn sub_agent_hook_dispatcher() -> HookDispatcher {
    HookDispatcher::new(
        HookRegistry::new()
            .with_registration(HookRegistration::new(
                "sub-agent-start",
                HookEventName::SubAgentStart,
                HookHandlerType::Command,
                Arc::new(OrderingHook::new("sub-agent-start-run", "sub-agent-start")),
            ))
            .with_registration(HookRegistration::new(
                "sub-agent-stop",
                HookEventName::SubAgentStop,
                HookHandlerType::Command,
                Arc::new(OrderingHook::new("sub-agent-stop-run", "sub-agent-stop")),
            )),
    )
}

fn single_node_graph(node_id: &str, executor: &str) -> LoopGraph {
    LoopGraph {
        graph_id: "graph".to_owned(),
        nodes: vec![LoopNodeSpec {
            id: node_id.to_owned(),
            executor: executor.to_owned(),
            config: Default::default(),
        }],
        edges: Vec::new(),
    }
}

async fn collect_order_events(
    events: &mut marlin_agent_runtime::RuntimeEventStream,
) -> Vec<String> {
    tokio::time::timeout(Duration::from_secs(1), async {
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
    .expect("order events should arrive")
}

async fn collect_sub_agent_events(
    events: &mut marlin_agent_runtime::RuntimeEventStream,
) -> (Vec<String>, Vec<String>) {
    tokio::time::timeout(Duration::from_secs(1), async {
        let mut order = Vec::new();
        let mut activity = Vec::new();
        while order.len() < 3 || activity.len() < 2 {
            let event = events.next().await.expect("event should be emitted");
            if event.topic == "test.order" {
                order.push(event.message);
            } else if event.topic == "kernel.sub_agent" {
                activity.push(event.message);
            }
        }
        (order, activity)
    })
    .await
    .expect("sub-agent events should arrive")
}

#[derive(Clone, Debug)]
struct OrderingProvider;

impl ProviderRuntime for OrderingProvider {
    type Request = String;
    type Response = String;

    fn run_provider(
        &self,
        request: Self::Request,
        context: RuntimeContext,
    ) -> RuntimeFuture<Self::Response> {
        Box::pin(async move {
            context
                .emit(RuntimeEvent::new("test.order", "provider"))
                .await
                .expect("provider order event should be delivered");
            request
        })
    }
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
struct OrderingSubAgent;

impl SubAgentRuntime for OrderingSubAgent {
    type Input = String;
    type Output = String;

    fn run_sub_agent(
        &self,
        input: Self::Input,
        context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output> {
        Box::pin(async move {
            context
                .emit(RuntimeEvent::new("test.order", "sub-agent"))
                .await
                .expect("sub-agent order event should be delivered");
            input
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
