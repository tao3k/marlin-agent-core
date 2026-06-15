//! Tokio runtime handle and spawn helpers.

use std::{collections::VecDeque, future::Future, sync::Arc, time::Duration};

use crate::observability;
use tokio::sync::{oneshot, watch};
use tokio::task::{JoinError, JoinSet};
use tracing::Instrument;

use super::{
    AgentSessionContext, CancellationToken, ContextVisibility, HookRuntime, ProviderRuntime,
    RuntimeContext, RuntimeEnvironment, RuntimeEventSink, RuntimeEventStream,
    RuntimeExecutionIdentity, RuntimeTask, RuntimeTaskOutcome, RuntimeTaskShutdownReceipt,
    RuntimeTaskShutdownRequest, RuntimeTaskTracker, RuntimeTaskTrackerPolicy, SessionId,
    SessionIsolationReceipt, SessionKind, SubAgentRuntime, SubAgentSpawnConfig,
    SubAgentSpawnReceipt, ToolRuntime, WorkingCopyIsolationReceipt,
};
use crate::tokio_runtime::context::context_visibility_from_sub_agent_policy;
use crate::tokio_runtime::receipt::{
    RuntimeFanoutOutput, RuntimeFanoutReceipt, RuntimeFanoutResult, RuntimeFanoutTaskReceipt,
};
use marlin_agent_sessions::RuntimeFanoutJoinPolicy;

/// Tokio-backed runtime handle shared by providers, tools, and sub-agents.
#[derive(Clone, Debug)]
pub struct TokioAgentRuntime {
    cancellation: CancellationToken,
    events: RuntimeEventSink,
    environment: RuntimeEnvironment,
    execution: Option<RuntimeExecutionIdentity>,
    session: AgentSessionContext,
    graph_loop_runs: crate::GraphLoopRunRegistryHandle,
    process_registry: observability::RuntimeProcessRegistryHandle,
    process_cleanup_policy: observability::RuntimeProcessCleanupPolicy,
    task_tracker: RuntimeTaskTracker,
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
                graph_loop_runs: crate::GraphLoopRunRegistryHandle::new(),
                process_registry: observability::RuntimeProcessRegistryHandle::new(),
                process_cleanup_policy: observability::RuntimeProcessCleanupPolicy::default(),
                task_tracker: RuntimeTaskTracker::new(),
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
            graph_loop_runs: self.graph_loop_runs.clone(),
            process_registry: self.process_registry.clone(),
            process_cleanup_policy: self.process_cleanup_policy.clone(),
        }
    }

    pub fn graph_loop_runs(&self) -> crate::GraphLoopRunRegistryHandle {
        self.graph_loop_runs.clone()
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

    pub fn task_tracker(&self) -> RuntimeTaskTracker {
        self.task_tracker.clone()
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
            graph_loop_runs: self.graph_loop_runs.clone(),
            process_registry: self.process_registry.clone(),
            process_cleanup_policy: self.process_cleanup_policy.clone(),
            task_tracker: self.task_tracker.clone(),
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
                graph_loop_runs: self.graph_loop_runs.clone(),
                process_registry: self.process_registry.clone(),
                process_cleanup_policy: self.process_cleanup_policy.clone(),
                task_tracker: self.task_tracker.clone(),
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
            graph_loop_runs: self.graph_loop_runs.clone(),
            process_registry: self.process_registry.clone(),
            process_cleanup_policy: self.process_cleanup_policy.clone(),
            task_tracker: self.task_tracker.clone(),
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
                graph_loop_runs: self.graph_loop_runs.clone(),
                process_registry: self.process_registry.clone(),
                process_cleanup_policy: self.process_cleanup_policy.clone(),
                task_tracker: self.task_tracker.clone(),
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
        let task_guard = self.task_tracker.track_task();
        RuntimeTask::new(tokio::task::spawn_blocking(move || {
            let _task_guard = task_guard;
            span.in_scope(operation)
        }))
    }

    pub async fn shutdown_tasks(
        &self,
        policy: &RuntimeTaskTrackerPolicy,
    ) -> RuntimeTaskShutdownReceipt {
        let request = if policy.cancel_on_shutdown_enabled() {
            self.cancellation.cancel();
            RuntimeTaskShutdownRequest::CancelTasksAndWait
        } else {
            RuntimeTaskShutdownRequest::WaitForTasks
        };
        self.task_tracker
            .close_and_wait(policy.shutdown_timeout_duration(), request)
            .await
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
    ) -> RuntimeTask<RuntimeFanoutResult<A::Output>>
    where
        A: SubAgentRuntime,
    {
        self.spawn_sub_agents_with_working_copy_environments_with_policy(
            sub_agent,
            fanout,
            RuntimeFanoutJoinPolicy::bounded(max_parallelism),
        )
    }

    pub fn spawn_sub_agents_with_working_copy_environments_with_policy<A>(
        &self,
        sub_agent: Arc<A>,
        fanout: impl IntoIterator<Item = WorkingCopySubAgentFanoutItem<A::Input>>,
        policy: RuntimeFanoutJoinPolicy,
    ) -> RuntimeTask<RuntimeFanoutResult<A::Output>>
    where
        A: SubAgentRuntime,
    {
        let runtime = self.clone();
        let fanout = fanout.into_iter().collect::<Vec<_>>();

        self.spawn_with_span(
            Self::run_sub_agent_fanout_with_policy(runtime, sub_agent, fanout, policy),
            observability::runtime_sub_agent_span(),
        )
    }

    fn effective_working_copy_fanout_parallelism(
        policy: &RuntimeFanoutJoinPolicy,
        item_count: usize,
    ) -> usize {
        policy
            .max_parallelism()
            .unwrap_or_else(|| item_count.max(1))
            .max(1)
    }

    async fn run_sub_agent_fanout_with_policy<A>(
        runtime: Self,
        sub_agent: Arc<A>,
        fanout: Vec<WorkingCopySubAgentFanoutItem<A::Input>>,
        policy: RuntimeFanoutJoinPolicy,
    ) -> RuntimeFanoutResult<A::Output>
    where
        A: SubAgentRuntime,
    {
        WorkingCopySubAgentFanoutRunner::new(runtime, sub_agent, fanout, policy)
            .run()
            .await
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
        let task_guard = self.task_tracker.track_task();
        RuntimeTask::new(tokio::spawn(async move {
            let _task_guard = task_guard;
            future.instrument(span).await
        }))
    }
}

struct WorkingCopySubAgentFanoutRunner<A>
where
    A: SubAgentRuntime,
{
    runtime: TokioAgentRuntime,
    sub_agent: Arc<A>,
    pending: VecDeque<(usize, WorkingCopySubAgentFanoutItem<A::Input>)>,
    tasks: JoinSet<(usize, A::Output)>,
    outputs: Vec<(usize, A::Output)>,
    receipt: RuntimeFanoutReceipt,
    policy: RuntimeFanoutJoinPolicy,
    effective_parallelism: usize,
}

impl<A> WorkingCopySubAgentFanoutRunner<A>
where
    A: SubAgentRuntime,
{
    fn new(
        runtime: TokioAgentRuntime,
        sub_agent: Arc<A>,
        fanout: Vec<WorkingCopySubAgentFanoutItem<A::Input>>,
        policy: RuntimeFanoutJoinPolicy,
    ) -> Self {
        let effective_parallelism =
            TokioAgentRuntime::effective_working_copy_fanout_parallelism(&policy, fanout.len());
        Self {
            runtime,
            sub_agent,
            pending: fanout.into_iter().enumerate().collect(),
            tasks: JoinSet::new(),
            outputs: Vec::new(),
            receipt: RuntimeFanoutReceipt::new(policy.clone()),
            policy,
            effective_parallelism,
        }
    }

    async fn run(mut self) -> RuntimeFanoutResult<A::Output> {
        while self.has_work() {
            self.spawn_ready_tasks();
            match self.record_next_join().await {
                WorkingCopyFanoutStep::Continue => {}
                WorkingCopyFanoutStep::Cancelled(receipt) => return Err(receipt),
            }
        }

        self.finish()
    }

    fn has_work(&self) -> bool {
        !self.pending.is_empty() || !self.tasks.is_empty()
    }

    fn spawn_ready_tasks(&mut self) {
        while self.tasks.len() < self.effective_parallelism {
            let Some((index, item)) = self.pending.pop_front() else {
                break;
            };
            let sub_agent = Arc::clone(&self.sub_agent);
            let context = self
                .runtime
                .context()
                .child_context_with_environment(item.environment)
                .with_working_copy_receipt(item.working_copy_receipt);
            self.tasks.spawn(
                async move { (index, sub_agent.run_sub_agent(item.input, context).await) }
                    .instrument(observability::runtime_sub_agent_span()),
            );
        }
    }

    async fn record_next_join(&mut self) -> WorkingCopyFanoutStep {
        let Some(joined) = self.tasks.join_next().await else {
            return WorkingCopyFanoutStep::Continue;
        };

        match joined {
            Ok((index, output)) => self.record_completed_task(index, output),
            Err(error) => self.record_join_error(error),
        }
    }

    fn record_completed_task(&mut self, index: usize, output: A::Output) -> WorkingCopyFanoutStep {
        self.receipt
            .push_task_receipt(RuntimeFanoutTaskReceipt::completed(index));
        self.outputs.push((index, output));
        WorkingCopyFanoutStep::Continue
    }

    fn record_join_error(&mut self, error: JoinError) -> WorkingCopyFanoutStep {
        self.receipt
            .push_task_receipt(RuntimeFanoutTaskReceipt::join_error(error.to_string()));
        if !self.policy.cancel_on_first_error() {
            return WorkingCopyFanoutStep::Continue;
        }

        WorkingCopyFanoutStep::Cancelled(self.cancel_remaining_work())
    }

    fn cancel_remaining_work(&mut self) -> RuntimeFanoutReceipt {
        self.tasks.abort_all();
        while let Some((index, _)) = self.pending.pop_front() {
            self.receipt
                .push_task_receipt(RuntimeFanoutTaskReceipt::cancelled_before_start(index));
        }
        self.take_receipt()
    }

    fn finish(mut self) -> RuntimeFanoutResult<A::Output> {
        if self.receipt.has_join_errors() {
            return Err(self.receipt);
        }
        if self.policy.preserve_input_order() {
            self.outputs.sort_by_key(|(index, _)| *index);
        }
        Ok(RuntimeFanoutOutput::new(
            self.outputs.into_iter().map(|(_, output)| output).collect(),
            self.receipt,
        ))
    }

    fn take_receipt(&mut self) -> RuntimeFanoutReceipt {
        std::mem::replace(
            &mut self.receipt,
            RuntimeFanoutReceipt::new(self.policy.clone()),
        )
    }
}

enum WorkingCopyFanoutStep {
    Continue,
    Cancelled(RuntimeFanoutReceipt),
}
