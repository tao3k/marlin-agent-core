//! Runtime-backed graph node executor adapters.

use std::sync::Arc;

use marlin_agent_hooks::{HookDispatchReport, HookDispatcher, HookInvocation};
use marlin_agent_protocol::{
    ExecutorName, GraphNodeExecutionReceipt, GraphNodeInvocation, HookEventName, NodeId,
    SubAgentActivity, SubAgentActivityKind, SubAgentSource,
};
use marlin_agent_runtime::{
    ProviderRuntime, RuntimeContext, RuntimeFuture, SubAgentRuntime, ToolRuntime, observability,
};
use tracing::Instrument;

use crate::GraphNodeExecutor;

type ProviderRequestMapper<P> =
    dyn Fn(GraphNodeInvocation) -> <P as ProviderRuntime>::Request + Send + Sync;
type ProviderReceiptMapper<P> = dyn Fn(<P as ProviderRuntime>::Response, NodeId, ExecutorName) -> GraphNodeExecutionReceipt
    + Send
    + Sync;
type ToolInvocationMapper<T> =
    dyn Fn(GraphNodeInvocation) -> <T as ToolRuntime>::Invocation + Send + Sync;
type ToolReceiptMapper<T> = dyn Fn(<T as ToolRuntime>::Output, NodeId, ExecutorName) -> GraphNodeExecutionReceipt
    + Send
    + Sync;
type SubAgentInputMapper<A> =
    dyn Fn(GraphNodeInvocation) -> <A as SubAgentRuntime>::Input + Send + Sync;
type SubAgentReceiptMapper<A> = dyn Fn(<A as SubAgentRuntime>::Output, NodeId, ExecutorName) -> GraphNodeExecutionReceipt
    + Send
    + Sync;

/// Graph node executor backed by a provider runtime.
pub struct ProviderNodeAdapter<P>
where
    P: ProviderRuntime,
{
    provider: Arc<P>,
    request_mapper: Arc<ProviderRequestMapper<P>>,
    receipt_mapper: Arc<ProviderReceiptMapper<P>>,
    hook_dispatcher: HookDispatcher,
}

impl<P> ProviderNodeAdapter<P>
where
    P: ProviderRuntime,
{
    pub fn new<RequestMapper>(provider: P, request_mapper: RequestMapper) -> Self
    where
        RequestMapper: Fn(GraphNodeInvocation) -> P::Request + Send + Sync + 'static,
    {
        Self::with_receipt_mapper(provider, request_mapper, completed_receipt::<P::Response>)
    }

    pub fn with_receipt_mapper<RequestMapper, ReceiptMapper>(
        provider: P,
        request_mapper: RequestMapper,
        receipt_mapper: ReceiptMapper,
    ) -> Self
    where
        RequestMapper: Fn(GraphNodeInvocation) -> P::Request + Send + Sync + 'static,
        ReceiptMapper: Fn(P::Response, NodeId, ExecutorName) -> GraphNodeExecutionReceipt
            + Send
            + Sync
            + 'static,
    {
        Self {
            provider: Arc::new(provider),
            request_mapper: Arc::new(request_mapper),
            receipt_mapper: Arc::new(receipt_mapper),
            hook_dispatcher: HookDispatcher::default(),
        }
    }

    pub fn with_hook_dispatcher(mut self, hook_dispatcher: HookDispatcher) -> Self {
        self.hook_dispatcher = hook_dispatcher;
        self
    }
}

impl<P> GraphNodeExecutor for ProviderNodeAdapter<P>
where
    P: ProviderRuntime,
{
    fn execute_node(
        &self,
        invocation: GraphNodeInvocation,
        context: RuntimeContext,
    ) -> RuntimeFuture<GraphNodeExecutionReceipt> {
        let provider = Arc::clone(&self.provider);
        let request = (self.request_mapper)(invocation.clone());
        let receipt_mapper = Arc::clone(&self.receipt_mapper);
        let hook_dispatcher = self.hook_dispatcher.clone();
        let node_id = invocation.node_id;
        let executor = invocation.executor;
        let hook_message = format!("node {} executor {}", node_id.as_str(), executor.as_str());
        let span = observability::agent_provider_span(&node_id, &executor);

        Box::pin(
            async move {
                let pre_report = hook_dispatcher
                    .dispatch_with_context(
                        context.child_context(),
                        HookInvocation::new(HookEventName::PreToolUse)
                            .with_message(hook_message.clone()),
                    )
                    .await;
                emit_hook_report(&context, &pre_report).await;

                let output = provider
                    .run_provider(request, context.child_context())
                    .await;

                let post_report = hook_dispatcher
                    .dispatch_with_context(
                        context.child_context(),
                        HookInvocation::new(HookEventName::PostToolUse).with_message(hook_message),
                    )
                    .await;
                emit_hook_report(&context, &post_report).await;

                receipt_mapper(output, node_id, executor)
            }
            .instrument(span),
        )
    }
}

/// Graph node executor backed by a tool runtime.
pub struct ToolNodeAdapter<T>
where
    T: ToolRuntime,
{
    tool: Arc<T>,
    invocation_mapper: Arc<ToolInvocationMapper<T>>,
    receipt_mapper: Arc<ToolReceiptMapper<T>>,
    hook_dispatcher: HookDispatcher,
}

impl<T> ToolNodeAdapter<T>
where
    T: ToolRuntime,
{
    pub fn new<InvocationMapper>(tool: T, invocation_mapper: InvocationMapper) -> Self
    where
        InvocationMapper: Fn(GraphNodeInvocation) -> T::Invocation + Send + Sync + 'static,
    {
        Self::with_receipt_mapper(tool, invocation_mapper, completed_receipt::<T::Output>)
    }

    pub fn with_receipt_mapper<InvocationMapper, ReceiptMapper>(
        tool: T,
        invocation_mapper: InvocationMapper,
        receipt_mapper: ReceiptMapper,
    ) -> Self
    where
        InvocationMapper: Fn(GraphNodeInvocation) -> T::Invocation + Send + Sync + 'static,
        ReceiptMapper: Fn(T::Output, NodeId, ExecutorName) -> GraphNodeExecutionReceipt
            + Send
            + Sync
            + 'static,
    {
        Self {
            tool: Arc::new(tool),
            invocation_mapper: Arc::new(invocation_mapper),
            receipt_mapper: Arc::new(receipt_mapper),
            hook_dispatcher: HookDispatcher::default(),
        }
    }

    pub fn with_hook_dispatcher(mut self, hook_dispatcher: HookDispatcher) -> Self {
        self.hook_dispatcher = hook_dispatcher;
        self
    }
}

impl<T> GraphNodeExecutor for ToolNodeAdapter<T>
where
    T: ToolRuntime,
{
    fn execute_node(
        &self,
        invocation: GraphNodeInvocation,
        context: RuntimeContext,
    ) -> RuntimeFuture<GraphNodeExecutionReceipt> {
        let tool = Arc::clone(&self.tool);
        let tool_invocation = (self.invocation_mapper)(invocation.clone());
        let receipt_mapper = Arc::clone(&self.receipt_mapper);
        let hook_dispatcher = self.hook_dispatcher.clone();
        let node_id = invocation.node_id;
        let executor = invocation.executor;
        let hook_message = format!("node {} executor {}", node_id.as_str(), executor.as_str());
        let span = observability::agent_tool_span(&node_id, &executor);

        Box::pin(
            async move {
                let pre_report = hook_dispatcher
                    .dispatch_with_context(
                        context.child_context(),
                        HookInvocation::new(HookEventName::PreToolUse)
                            .with_message(hook_message.clone()),
                    )
                    .await;
                emit_hook_report(&context, &pre_report).await;

                let output = tool
                    .run_tool(tool_invocation, context.child_context())
                    .await;

                let post_report = hook_dispatcher
                    .dispatch_with_context(
                        context.child_context(),
                        HookInvocation::new(HookEventName::PostToolUse).with_message(hook_message),
                    )
                    .await;
                emit_hook_report(&context, &post_report).await;

                receipt_mapper(output, node_id, executor)
            }
            .instrument(span),
        )
    }
}

/// Graph node executor backed by a delegated sub-agent runtime.
pub struct SubAgentNodeAdapter<A>
where
    A: SubAgentRuntime,
{
    sub_agent: Arc<A>,
    input_mapper: Arc<SubAgentInputMapper<A>>,
    receipt_mapper: Arc<SubAgentReceiptMapper<A>>,
    hook_dispatcher: HookDispatcher,
}

impl<A> SubAgentNodeAdapter<A>
where
    A: SubAgentRuntime,
{
    pub fn new<InputMapper>(sub_agent: A, input_mapper: InputMapper) -> Self
    where
        InputMapper: Fn(GraphNodeInvocation) -> A::Input + Send + Sync + 'static,
    {
        Self::with_receipt_mapper(sub_agent, input_mapper, completed_receipt::<A::Output>)
    }

    pub fn with_receipt_mapper<InputMapper, ReceiptMapper>(
        sub_agent: A,
        input_mapper: InputMapper,
        receipt_mapper: ReceiptMapper,
    ) -> Self
    where
        InputMapper: Fn(GraphNodeInvocation) -> A::Input + Send + Sync + 'static,
        ReceiptMapper: Fn(A::Output, NodeId, ExecutorName) -> GraphNodeExecutionReceipt
            + Send
            + Sync
            + 'static,
    {
        Self {
            sub_agent: Arc::new(sub_agent),
            input_mapper: Arc::new(input_mapper),
            receipt_mapper: Arc::new(receipt_mapper),
            hook_dispatcher: HookDispatcher::default(),
        }
    }

    pub fn with_hook_dispatcher(mut self, hook_dispatcher: HookDispatcher) -> Self {
        self.hook_dispatcher = hook_dispatcher;
        self
    }
}

impl<A> GraphNodeExecutor for SubAgentNodeAdapter<A>
where
    A: SubAgentRuntime,
{
    fn execute_node(
        &self,
        invocation: GraphNodeInvocation,
        context: RuntimeContext,
    ) -> RuntimeFuture<GraphNodeExecutionReceipt> {
        let sub_agent = Arc::clone(&self.sub_agent);
        let input = (self.input_mapper)(invocation.clone());
        let receipt_mapper = Arc::clone(&self.receipt_mapper);
        let hook_dispatcher = self.hook_dispatcher.clone();
        let node_id = invocation.node_id;
        let executor = invocation.executor;
        let hook_message = format!(
            "sub-agent node {} executor {}",
            node_id.as_str(),
            executor.as_str()
        );
        let activity_status = format!("node {}", node_id.as_str());
        let agent_reference = executor.as_str().to_owned();
        let sub_agent_source =
            SubAgentSource::Other(observability::SUB_AGENT_SOURCE_KERNEL_NODE.to_owned());
        let span = observability::agent_sub_agent_span_with_source(
            &node_id,
            &executor,
            observability::SUB_AGENT_SOURCE_KERNEL_NODE,
            agent_reference.as_str(),
        );
        if let Some(execution) = context.execution_identity() {
            span.record(observability::FIELD_PARENT_RUN_ID, execution.run_id());
        }

        Box::pin(
            async move {
                let start_report = hook_dispatcher
                    .dispatch_with_context(
                        context.child_context(),
                        HookInvocation::new(HookEventName::SubAgentStart)
                            .with_message(hook_message.clone()),
                    )
                    .await;
                emit_hook_report(&context, &start_report).await;
                emit_sub_agent_activity(
                    &context,
                    SubAgentActivity::new(
                        agent_reference.clone(),
                        sub_agent_source.clone(),
                        SubAgentActivityKind::Started,
                    )
                    .with_status_message(activity_status.clone()),
                )
                .await;

                let output = sub_agent
                    .run_sub_agent(input, context.child_context())
                    .await;

                let stop_report = hook_dispatcher
                    .dispatch_with_context(
                        context.child_context(),
                        HookInvocation::new(HookEventName::SubAgentStop).with_message(hook_message),
                    )
                    .await;
                emit_hook_report(&context, &stop_report).await;
                emit_sub_agent_activity(
                    &context,
                    SubAgentActivity::new(
                        agent_reference,
                        sub_agent_source,
                        SubAgentActivityKind::Stopped,
                    )
                    .with_status_message(activity_status),
                )
                .await;

                receipt_mapper(output, node_id, executor)
            }
            .instrument(span),
        )
    }
}

fn completed_receipt<T>(
    _output: T,
    node_id: NodeId,
    executor: ExecutorName,
) -> GraphNodeExecutionReceipt {
    GraphNodeExecutionReceipt::completed(node_id, executor)
}

async fn emit_hook_report(context: &RuntimeContext, report: &HookDispatchReport) {
    for run in &report.runs {
        let message = format!(
            "hook {} {:?} {:?} status {:?}",
            run.id.as_str(),
            run.event_name,
            run.execution_mode,
            run.status
        );
        let _ = context
            .emit(observability::kernel_hook_event(message))
            .await;
    }
}

async fn emit_sub_agent_activity(context: &RuntimeContext, activity: SubAgentActivity) {
    let message = format!(
        "sub-agent {} {:?} source {:?} status {}",
        activity.agent_reference,
        activity.kind,
        activity.source,
        activity.status_message.as_deref().unwrap_or_default()
    );
    let _ = context
        .emit(observability::kernel_sub_agent_event(message))
        .await;
}
