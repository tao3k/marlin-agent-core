use marlin_agent_kernel::{GraphNodeExecutionReceipt, GraphNodeExecutor, GraphNodeInvocation};
use marlin_agent_runtime::{
    HookRuntime, RuntimeContext, RuntimeEnvironment, RuntimeEvent, RuntimeFuture, SubAgentRuntime,
};

#[derive(Clone, Debug)]
pub(super) struct EnvironmentEchoHook;

impl HookRuntime for EnvironmentEchoHook {
    type Request = String;
    type Output = RuntimeEnvironment;

    fn run_hook(
        &self,
        _request: Self::Request,
        context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output> {
        let environment = context.environment().clone();
        Box::pin(async move { environment })
    }
}

#[derive(Clone, Debug)]
pub(super) struct EnvironmentEchoSubAgent;

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
pub(super) struct QuietExecutor;

impl GraphNodeExecutor for QuietExecutor {
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

#[derive(Clone, Debug)]
pub(super) struct EventfulExecutor;

impl GraphNodeExecutor for EventfulExecutor {
    fn execute_node(
        &self,
        invocation: GraphNodeInvocation,
        context: RuntimeContext,
    ) -> RuntimeFuture<GraphNodeExecutionReceipt> {
        Box::pin(async move {
            let node_id = invocation.node_id;
            let executor = invocation.executor;
            context
                .emit(RuntimeEvent::new(
                    "test.harness",
                    format!("node {} observed", node_id.as_str()),
                ))
                .await
                .expect("harness event should be emitted");
            GraphNodeExecutionReceipt::completed(node_id, executor)
        })
    }
}

#[derive(Clone, Debug)]
pub(super) struct FailingExecutor;

impl GraphNodeExecutor for FailingExecutor {
    fn execute_node(
        &self,
        invocation: GraphNodeInvocation,
        _context: RuntimeContext,
    ) -> RuntimeFuture<GraphNodeExecutionReceipt> {
        Box::pin(async move {
            GraphNodeExecutionReceipt::failed(
                invocation.node_id,
                invocation.executor,
                vec!["node node-1 failed intentionally".to_owned()],
            )
        })
    }
}
