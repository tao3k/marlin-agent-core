//! Tokio runtime handle and spawn helpers.

use std::{future::Future, sync::Arc, time::Duration};

use crate::observability;
use tokio::sync::{oneshot, watch};
use tokio::task::JoinSet;
use tracing::Instrument;

use super::{
    AgentSessionContext, CancellationToken, ContextVisibility, HookRuntime, ProviderRuntime,
    RuntimeContext, RuntimeEnvironment, RuntimeEventSink, RuntimeEventStream,
    RuntimeExecutionIdentity, RuntimeTask, RuntimeTaskOutcome, SessionId, SessionIsolationReceipt,
    SessionKind, SubAgentRuntime, SubAgentSpawnConfig, SubAgentSpawnReceipt, ToolRuntime,
    WorkingCopyIsolationReceipt,
};
use crate::tokio_runtime::context::context_visibility_from_sub_agent_policy;

/// Tokio-backed runtime handle shared by providers, tools, and sub-agents.
#[derive(Clone, Debug)]
pub struct TokioAgentRuntime {
    cancellation: CancellationToken,
    events: RuntimeEventSink,
    environment: RuntimeEnvironment,
    execution: Option<RuntimeExecutionIdentity>,
    session: AgentSessionContext,
    process_registry: observability::RuntimeProcessRegistryHandle,
    process_cleanup_policy: observability::RuntimeProcessCleanupPolicy,
}

/// One working-copy-bound sub-agent spawn in a runtime fanout.
#[derive(Clone, Debug)]
pub struct WorkingCopySubAgentFanoutItem<I> {
    input: I,
    environment: RuntimeEnvironment,
    working_copy_receipt: WorkingCopyIsolationReceipt,
}

impl<I> WorkingCopySubAgentFanoutItem<I> {
    pub fn new(
        input: I,
        environment: RuntimeEnvironment,
        working_copy_receipt: WorkingCopyIsolationReceipt,
    ) -> Self {
        Self {
            input,
            environment,
            working_copy_receipt,
        }
    }

    pub fn input(&self) -> &I {
        &self.input
    }

    pub fn environment(&self) -> &RuntimeEnvironment {
        &self.environment
    }

    pub fn working_copy_receipt(&self) -> &WorkingCopyIsolationReceipt {
        &self.working_copy_receipt
    }

    pub fn into_parts(self) -> (I, RuntimeEnvironment, WorkingCopyIsolationReceipt) {
        (self.input, self.environment, self.working_copy_receipt)
    }
}

impl TokioAgentRuntime {
    pub fn new(event_buffer: usize) -> (Self, RuntimeEventStream) {
        Self::with_cancellation(event_buffer, CancellationToken::new())
    }

    pub fn with_cancellation(
        event_buffer: usize,
        cancellation: CancellationToken,
    ) -> (Self, RuntimeEventStream) {
        Self::with_environment(event_buffer, cancellation, RuntimeEnvironment::default())
    }

    pub fn with_environment(
        event_buffer: usize,
        cancellation: CancellationToken,
        environment: RuntimeEnvironment,
    ) -> (Self, RuntimeEventStream) {
        Self::with_session(
            event_buffer,
            cancellation,
            environment,
            AgentSessionContext::runtime_root(),
        )
    }

    pub fn with_session(
        event_buffer: usize,
        cancellation: CancellationToken,
        environment: RuntimeEnvironment,
        session: AgentSessionContext,
    ) -> (Self, RuntimeEventStream) {
        let (events, stream) = RuntimeEventSink::channel(event_buffer);
        (
            Self {
                cancellation,
                events,
                environment,
                execution: None,
                session,
                process_registry: observability::RuntimeProcessRegistryHandle::new(),
                process_cleanup_policy: observability::RuntimeProcessCleanupPolicy::default(),
            },
            stream,
        )
    }

    pub fn context(&self) -> RuntimeContext {
        RuntimeContext {
            cancellation: self.cancellation.clone(),
            events: self.events.clone(),
            environment: self.environment.clone(),
            execution: self.execution.clone(),
            session: self.session.clone(),
            active_working_copy: None,
            working_copy_receipts: Vec::new(),
            process_registry: self.process_registry.clone(),
            process_cleanup_policy: self.process_cleanup_policy.clone(),
        }
    }

    pub fn with_process_cleanup_policy(
        mut self,
        process_cleanup_policy: observability::RuntimeProcessCleanupPolicy,
    ) -> Self {
        self.process_cleanup_policy = process_cleanup_policy;
        self
    }

    pub fn environment(&self) -> &RuntimeEnvironment {
        &self.environment
    }

    pub fn session(&self) -> &AgentSessionContext {
        &self.session
    }

    pub fn cancellation_token(&self) -> CancellationToken {
        self.cancellation.clone()
    }

    pub fn event_sink(&self) -> RuntimeEventSink {
        self.events.clone()
    }

    pub fn process_registry(&self) -> observability::RuntimeProcessRegistryHandle {
        self.process_registry.clone()
    }

    pub fn process_cleanup_policy(&self) -> &observability::RuntimeProcessCleanupPolicy {
        &self.process_cleanup_policy
    }

    pub fn child_runtime(&self) -> Self {
        Self {
            cancellation: self.cancellation.child_token(),
            events: self.events.clone(),
            environment: self.environment.clone(),
            execution: self.execution.clone(),
            session: self.session.clone(),
            process_registry: self.process_registry.clone(),
            process_cleanup_policy: self.process_cleanup_policy.clone(),
        }
    }

    pub fn child_runtime_with_event_capture(
        &self,
        event_buffer: usize,
    ) -> (Self, RuntimeEventStream) {
        let (events, stream) = self.events.with_capture(event_buffer);
        (
            Self {
                cancellation: self.cancellation.child_token(),
                events,
                environment: self.environment.clone(),
                execution: self.execution.clone(),
                session: self.session.clone(),
                process_registry: self.process_registry.clone(),
                process_cleanup_policy: self.process_cleanup_policy.clone(),
            },
            stream,
        )
    }

    pub fn child_runtime_with_environment(&self, environment: RuntimeEnvironment) -> Self {
        Self {
            cancellation: self.cancellation.child_token(),
            events: self.events.clone(),
            environment,
            execution: self.execution.clone(),
            session: self.session.clone(),
            process_registry: self.process_registry.clone(),
            process_cleanup_policy: self.process_cleanup_policy.clone(),
        }
    }

    pub fn with_runtime_environment(mut self, environment: RuntimeEnvironment) -> Self {
        self.environment = environment;
        self
    }

    pub fn child_runtime_for_session(
        &self,
        kind: SessionKind,
        child_session_id: impl Into<SessionId>,
        requested_visibility: ContextVisibility,
    ) -> (Self, SessionIsolationReceipt) {
        let (session, receipt) =
            self.session
                .child_session(kind, child_session_id, requested_visibility);
        (
            Self {
                cancellation: self.cancellation.child_token(),
                events: self.events.clone(),
                environment: self.environment.clone(),
                execution: self.execution.clone(),
                session,
                process_registry: self.process_registry.clone(),
                process_cleanup_policy: self.process_cleanup_policy.clone(),
            },
            receipt,
        )
    }

    pub fn spawn<F>(&self, future: F) -> RuntimeTask<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.spawn_with_span(
            future,
            observability::runtime_task_span(observability::RUNTIME_KIND_GENERIC),
        )
    }

    pub fn spawn_cancellable<F>(&self, future: F) -> RuntimeTask<RuntimeTaskOutcome<F::Output>>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let cancellation = self.cancellation.clone();
        self.spawn_with_span(
            async move {
                tokio::select! {
                    output = future => RuntimeTaskOutcome::Completed(output),
                    () = cancellation.cancelled() => {
                        tracing::debug!(
                            runtime_kind = observability::RUNTIME_KIND_CANCELLABLE,
                            "runtime task cancelled"
                        );
                        RuntimeTaskOutcome::Cancelled
                    },
                }
            },
            observability::runtime_task_span(observability::RUNTIME_KIND_CANCELLABLE),
        )
    }

    pub fn spawn_blocking<F, T>(&self, operation: F) -> RuntimeTask<T>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        let span = observability::runtime_task_span(observability::RUNTIME_KIND_BLOCKING);
        RuntimeTask::new(tokio::task::spawn_blocking(move || {
            span.in_scope(operation)
        }))
    }

    pub fn join_set<T>(&self) -> JoinSet<T>
    where
        T: Send + 'static,
    {
        JoinSet::new()
    }

    pub fn oneshot_channel<T>(&self) -> (oneshot::Sender<T>, oneshot::Receiver<T>) {
        oneshot::channel()
    }

    pub fn watch_channel<T>(&self, initial: T) -> (watch::Sender<T>, watch::Receiver<T>) {
        watch::channel(initial)
    }

    pub fn sleep(&self, duration: Duration) -> tokio::time::Sleep {
        tokio::time::sleep(duration)
    }

    pub fn timeout<F>(&self, duration: Duration, future: F) -> tokio::time::Timeout<F>
    where
        F: Future,
    {
        tokio::time::timeout(duration, future)
    }

    pub fn spawn_provider<P>(
        &self,
        provider: Arc<P>,
        request: P::Request,
    ) -> RuntimeTask<P::Response>
    where
        P: ProviderRuntime,
    {
        let context = self.context();
        self.spawn_with_span(
            async move { provider.run_provider(request, context).await },
            observability::runtime_provider_span(),
        )
    }

    pub fn spawn_provider_with_session<P>(
        &self,
        provider: Arc<P>,
        request: P::Request,
        child_session_id: impl Into<SessionId>,
        requested_visibility: ContextVisibility,
    ) -> (RuntimeTask<P::Response>, SessionIsolationReceipt)
    where
        P: ProviderRuntime,
    {
        let (context, receipt) = self.context().child_context_for_session(
            SessionKind::Provider,
            child_session_id,
            requested_visibility,
        );
        (
            self.spawn_with_span(
                async move { provider.run_provider(request, context).await },
                observability::runtime_provider_span(),
            ),
            receipt,
        )
    }

    pub fn spawn_tool<T>(&self, tool: Arc<T>, invocation: T::Invocation) -> RuntimeTask<T::Output>
    where
        T: ToolRuntime,
    {
        let context = self.context();
        self.spawn_with_span(
            async move { tool.run_tool(invocation, context).await },
            observability::runtime_tool_span(),
        )
    }

    pub fn spawn_tool_with_session<T>(
        &self,
        tool: Arc<T>,
        invocation: T::Invocation,
        child_session_id: impl Into<SessionId>,
        requested_visibility: ContextVisibility,
    ) -> (RuntimeTask<T::Output>, SessionIsolationReceipt)
    where
        T: ToolRuntime,
    {
        let (context, receipt) = self.context().child_context_for_session(
            SessionKind::Tool,
            child_session_id,
            requested_visibility,
        );
        (
            self.spawn_with_span(
                async move { tool.run_tool(invocation, context).await },
                observability::runtime_tool_span(),
            ),
            receipt,
        )
    }

    pub fn spawn_sub_agent<A>(&self, sub_agent: Arc<A>, input: A::Input) -> RuntimeTask<A::Output>
    where
        A: SubAgentRuntime,
    {
        let context = self.context();
        self.spawn_with_span(
            async move { sub_agent.run_sub_agent(input, context).await },
            observability::runtime_sub_agent_span(),
        )
    }

    pub fn spawn_sub_agent_with_session<A>(
        &self,
        sub_agent: Arc<A>,
        input: A::Input,
        child_session_id: impl Into<SessionId>,
        requested_visibility: ContextVisibility,
    ) -> (RuntimeTask<A::Output>, SessionIsolationReceipt)
    where
        A: SubAgentRuntime,
    {
        let (context, receipt) = self.context().child_context_for_session(
            SessionKind::SubAgent,
            child_session_id,
            requested_visibility,
        );
        (
            self.spawn_with_span(
                async move { sub_agent.run_sub_agent(input, context).await },
                observability::runtime_sub_agent_span(),
            ),
            receipt,
        )
    }

    pub fn spawn_sub_agent_with_config<A>(
        &self,
        sub_agent: Arc<A>,
        input: A::Input,
        config: SubAgentSpawnConfig,
    ) -> (RuntimeTask<A::Output>, SubAgentSpawnReceipt)
    where
        A: SubAgentRuntime,
    {
        let child_session_id = config.child_session_id().to_owned();
        let requested_visibility = context_visibility_from_sub_agent_policy(&config.policy.context);
        let (task, isolation_receipt) = self.spawn_sub_agent_with_session(
            sub_agent,
            input,
            child_session_id,
            requested_visibility,
        );
        (task, SubAgentSpawnReceipt::new(config, isolation_receipt))
    }

    pub fn spawn_sub_agent_with_environment<A>(
        &self,
        sub_agent: Arc<A>,
        input: A::Input,
        environment: RuntimeEnvironment,
    ) -> RuntimeTask<A::Output>
    where
        A: SubAgentRuntime,
    {
        let context = self.context().child_context_with_environment(environment);
        self.spawn_with_span(
            async move { sub_agent.run_sub_agent(input, context).await },
            observability::runtime_sub_agent_span(),
        )
    }

    pub fn spawn_sub_agent_with_working_copy_environment<A>(
        &self,
        sub_agent: Arc<A>,
        input: A::Input,
        environment: RuntimeEnvironment,
        working_copy_receipt: WorkingCopyIsolationReceipt,
    ) -> RuntimeTask<A::Output>
    where
        A: SubAgentRuntime,
    {
        let context = self
            .context()
            .child_context_with_environment(environment)
            .with_working_copy_receipt(working_copy_receipt);
        self.spawn_with_span(
            async move { sub_agent.run_sub_agent(input, context).await },
            observability::runtime_sub_agent_span(),
        )
    }

    pub fn spawn_sub_agents_with_working_copy_environments<A>(
        &self,
        sub_agent: Arc<A>,
        fanout: impl IntoIterator<Item = WorkingCopySubAgentFanoutItem<A::Input>>,
        max_parallelism: usize,
    ) -> RuntimeTask<Vec<A::Output>>
    where
        A: SubAgentRuntime,
    {
        let runtime = self.clone();
        let fanout = fanout.into_iter().collect::<Vec<_>>();
        let effective_parallelism = max_parallelism.max(1);

        self.spawn_with_span(
            async move {
                let mut pending = fanout.into_iter().enumerate();
                let mut tasks = JoinSet::new();
                let mut outputs = Vec::new();

                loop {
                    while tasks.len() < effective_parallelism {
                        let Some((index, item)) = pending.next() else {
                            break;
                        };
                        let sub_agent = Arc::clone(&sub_agent);
                        let context = runtime
                            .context()
                            .child_context_with_environment(item.environment)
                            .with_working_copy_receipt(item.working_copy_receipt);
                        tasks.spawn(
                            async move { (index, sub_agent.run_sub_agent(item.input, context).await) }
                                .instrument(observability::runtime_sub_agent_span()),
                        );
                    }

                    let Some(joined) = tasks.join_next().await else {
                        break;
                    };
                    let (index, output) = joined.unwrap_or_else(|error| {
                        panic!("working-copy sub-agent fanout task failed: {error}")
                    });
                    outputs.push((index, output));
                }

                outputs.sort_by_key(|(index, _)| *index);
                outputs.into_iter().map(|(_, output)| output).collect()
            },
            observability::runtime_sub_agent_span(),
        )
    }

    pub fn spawn_hook<H>(&self, hook: Arc<H>, request: H::Request) -> RuntimeTask<H::Output>
    where
        H: HookRuntime,
    {
        let context = self.context();
        self.spawn_with_span(
            async move { hook.run_hook(request, context).await },
            observability::runtime_hook_span(),
        )
    }

    pub fn spawn_hook_with_session<H>(
        &self,
        hook: Arc<H>,
        request: H::Request,
        child_session_id: impl Into<SessionId>,
        requested_visibility: ContextVisibility,
    ) -> (RuntimeTask<H::Output>, SessionIsolationReceipt)
    where
        H: HookRuntime,
    {
        let (context, receipt) = self.context().child_context_for_session(
            SessionKind::Hook,
            child_session_id,
            requested_visibility,
        );
        (
            self.spawn_with_span(
                async move { hook.run_hook(request, context).await },
                observability::runtime_hook_span(),
            ),
            receipt,
        )
    }

    fn spawn_with_span<F>(&self, future: F, span: tracing::Span) -> RuntimeTask<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        RuntimeTask::new(tokio::spawn(future.instrument(span)))
    }
}
