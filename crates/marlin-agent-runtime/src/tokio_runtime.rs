//! Tokio-backed execution substrate for agent providers, tools, and sub-agents.

use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};

use crate::observability;
pub use marlin_agent_protocol::{AgentEvent as RuntimeEvent, GraphId, RunId, RuntimeEnvironment};
pub use marlin_agent_sessions::{
    AgentSessionContext, ContextExpansionPolicy, ContextNamespace, ContextVisibility, SessionId,
    SessionIdError, SessionIdentity, SessionIsolationPolicy, SessionIsolationReceipt, SessionKind,
};
use tokio::sync::{mpsc, oneshot, watch};
use tokio::task::{JoinError, JoinSet};
pub use tokio_util::sync::CancellationToken;
use tracing::Instrument;

/// Boxed async work item used by runtime extension traits.
pub type RuntimeFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

/// Tokio-backed runtime handle shared by providers, tools, and sub-agents.
#[derive(Clone, Debug)]
pub struct TokioAgentRuntime {
    cancellation: CancellationToken,
    events: RuntimeEventSink,
    environment: RuntimeEnvironment,
    execution: Option<RuntimeExecutionIdentity>,
    session: AgentSessionContext,
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
        }
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

    pub fn child_runtime(&self) -> Self {
        Self {
            cancellation: self.cancellation.child_token(),
            events: self.events.clone(),
            environment: self.environment.clone(),
            execution: self.execution.clone(),
            session: self.session.clone(),
        }
    }

    pub fn child_runtime_with_environment(&self, environment: RuntimeEnvironment) -> Self {
        Self {
            cancellation: self.cancellation.child_token(),
            events: self.events.clone(),
            environment,
            execution: self.execution.clone(),
            session: self.session.clone(),
        }
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

/// Per-call runtime context passed into provider, tool, and sub-agent work.
#[derive(Clone, Debug)]
pub struct RuntimeContext {
    cancellation: CancellationToken,
    events: RuntimeEventSink,
    environment: RuntimeEnvironment,
    execution: Option<RuntimeExecutionIdentity>,
    session: AgentSessionContext,
}

impl RuntimeContext {
    pub fn cancellation_token(&self) -> CancellationToken {
        self.cancellation.clone()
    }

    pub fn event_sink(&self) -> RuntimeEventSink {
        self.events.clone()
    }

    pub fn environment(&self) -> &RuntimeEnvironment {
        &self.environment
    }

    pub fn session(&self) -> &AgentSessionContext {
        &self.session
    }

    pub fn execution_identity(&self) -> Option<&RuntimeExecutionIdentity> {
        self.execution.as_ref()
    }

    pub fn with_execution_identity(mut self, execution: RuntimeExecutionIdentity) -> Self {
        self.execution = Some(execution);
        self
    }

    pub fn with_session_context(mut self, session: AgentSessionContext) -> Self {
        self.session = session;
        self
    }

    pub fn child_context(&self) -> Self {
        Self {
            cancellation: self.cancellation.child_token(),
            events: self.events.clone(),
            environment: self.environment.clone(),
            execution: self.execution.clone(),
            session: self.session.clone(),
        }
    }

    pub fn child_context_with_environment(&self, environment: RuntimeEnvironment) -> Self {
        Self {
            cancellation: self.cancellation.child_token(),
            events: self.events.clone(),
            environment,
            execution: self.execution.clone(),
            session: self.session.clone(),
        }
    }

    pub fn child_context_for_session(
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
            },
            receipt,
        )
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancellation.is_cancelled()
    }

    pub async fn emit(
        &self,
        event: RuntimeEvent,
    ) -> Result<(), mpsc::error::SendError<RuntimeEvent>> {
        self.events.emit(event).await
    }
}

/// Graph-loop execution identity propagated through runtime child contexts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeExecutionIdentity {
    run_id: RunId,
    graph_id: GraphId,
}

impl RuntimeExecutionIdentity {
    pub fn new(run_id: impl Into<String>, graph_id: impl Into<String>) -> Self {
        Self {
            run_id: RunId::new(run_id),
            graph_id: GraphId::new(graph_id),
        }
    }

    pub fn from_parts(run_id: RunId, graph_id: GraphId) -> Self {
        Self { run_id, graph_id }
    }

    pub fn run_id(&self) -> &str {
        self.run_id.as_str()
    }

    pub fn graph_id(&self) -> &str {
        self.graph_id.as_str()
    }
}

/// Cloneable Tokio mpsc sender for runtime observations and receipts.
#[derive(Clone, Debug)]
pub struct RuntimeEventSink {
    sender: mpsc::Sender<RuntimeEvent>,
}

impl RuntimeEventSink {
    pub fn channel(event_buffer: usize) -> (Self, RuntimeEventStream) {
        let (sender, receiver) = mpsc::channel(event_buffer);
        (Self { sender }, RuntimeEventStream::new(receiver))
    }

    pub async fn emit(
        &self,
        event: RuntimeEvent,
    ) -> Result<(), mpsc::error::SendError<RuntimeEvent>> {
        self.sender.send(event).await
    }
}

/// Tokio stream of runtime observations and receipts.
#[derive(Debug)]
pub struct RuntimeEventStream {
    receiver: mpsc::Receiver<RuntimeEvent>,
}

impl RuntimeEventStream {
    /// Wrap a Tokio event receiver in the Marlin runtime event stream boundary.
    pub fn new(receiver: mpsc::Receiver<RuntimeEvent>) -> Self {
        Self { receiver }
    }

    /// Return one already-buffered runtime event without waiting.
    pub fn try_next(&mut self) -> Option<RuntimeEvent> {
        self.receiver.try_recv().ok()
    }

    /// Close the stream so no further runtime events can be received.
    pub fn close(&mut self) {
        self.receiver.close();
    }
}

impl tokio_stream::Stream for RuntimeEventStream {
    type Item = RuntimeEvent;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.receiver.poll_recv(cx)
    }
}

/// Compatibility alias for the runtime event stream boundary.
pub type EventStream = RuntimeEventStream;

/// Tokio task handle with a stable marlin-owned name.
#[derive(Debug)]
pub struct RuntimeTask<T> {
    handle: tokio::task::JoinHandle<T>,
}

/// Outcome for work spawned through Tokio cancellation-aware helpers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeTaskOutcome<T> {
    Completed(T),
    Cancelled,
}

impl<T> RuntimeTask<T> {
    pub fn new(handle: tokio::task::JoinHandle<T>) -> Self {
        Self { handle }
    }

    pub fn abort(&self) {
        self.handle.abort();
    }

    pub fn is_finished(&self) -> bool {
        self.handle.is_finished()
    }

    pub async fn join(self) -> Result<T, JoinError> {
        self.handle.await
    }
}

/// Provider boundary for model or completion runtimes.
pub trait ProviderRuntime: Send + Sync + 'static {
    type Request: Send + 'static;
    type Response: Send + 'static;

    fn run_provider(
        &self,
        request: Self::Request,
        context: RuntimeContext,
    ) -> RuntimeFuture<Self::Response>;
}

/// Tool boundary for native or external tool execution.
pub trait ToolRuntime: Send + Sync + 'static {
    type Invocation: Send + 'static;
    type Output: Send + 'static;

    fn run_tool(
        &self,
        invocation: Self::Invocation,
        context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output>;
}

/// Hook boundary for runtime interception and observation.
pub trait HookRuntime: Send + Sync + 'static {
    type Request: Send + 'static;
    type Output: Send + 'static;

    fn run_hook(
        &self,
        request: Self::Request,
        context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output>;
}

/// Sub-agent boundary for delegated graph-loop work.
pub trait SubAgentRuntime: Send + Sync + 'static {
    type Input: Send + 'static;
    type Output: Send + 'static;

    fn run_sub_agent(
        &self,
        input: Self::Input,
        context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output>;
}
