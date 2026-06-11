//! Tokio-backed execution substrate for agent providers, tools, and sub-agents.

use std::{future::Future, pin::Pin, sync::Arc, time::Duration};

pub use marlin_agent_protocol::{AgentEvent as RuntimeEvent, RuntimeEnvironment};
use tokio::sync::{mpsc, oneshot, watch};
use tokio::task::{JoinError, JoinSet};
pub use tokio_util::sync::CancellationToken;

/// Boxed async work item used by runtime extension traits.
pub type RuntimeFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

/// Tokio-backed runtime handle shared by providers, tools, and sub-agents.
#[derive(Clone, Debug)]
pub struct TokioAgentRuntime {
    cancellation: CancellationToken,
    events: RuntimeEventSink,
    environment: RuntimeEnvironment,
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
        let (events, stream) = RuntimeEventSink::channel(event_buffer);
        (
            Self {
                cancellation,
                events,
                environment,
            },
            stream,
        )
    }

    pub fn context(&self) -> RuntimeContext {
        RuntimeContext {
            cancellation: self.cancellation.clone(),
            events: self.events.clone(),
            environment: self.environment.clone(),
        }
    }

    pub fn environment(&self) -> &RuntimeEnvironment {
        &self.environment
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
        }
    }

    pub fn child_runtime_with_environment(&self, environment: RuntimeEnvironment) -> Self {
        Self {
            cancellation: self.cancellation.child_token(),
            events: self.events.clone(),
            environment,
        }
    }

    pub fn spawn<F>(&self, future: F) -> RuntimeTask<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        RuntimeTask::new(tokio::spawn(future))
    }

    pub fn spawn_cancellable<F>(&self, future: F) -> RuntimeTask<RuntimeTaskOutcome<F::Output>>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let cancellation = self.cancellation.clone();
        self.spawn(async move {
            tokio::select! {
                output = future => RuntimeTaskOutcome::Completed(output),
                () = cancellation.cancelled() => RuntimeTaskOutcome::Cancelled,
            }
        })
    }

    pub fn spawn_blocking<F, T>(&self, operation: F) -> RuntimeTask<T>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        RuntimeTask::new(tokio::task::spawn_blocking(operation))
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
        self.spawn(async move { provider.run_provider(request, context).await })
    }

    pub fn spawn_tool<T>(&self, tool: Arc<T>, invocation: T::Invocation) -> RuntimeTask<T::Output>
    where
        T: ToolRuntime,
    {
        let context = self.context();
        self.spawn(async move { tool.run_tool(invocation, context).await })
    }

    pub fn spawn_sub_agent<A>(&self, sub_agent: Arc<A>, input: A::Input) -> RuntimeTask<A::Output>
    where
        A: SubAgentRuntime,
    {
        let context = self.context();
        self.spawn(async move { sub_agent.run_sub_agent(input, context).await })
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
        self.spawn(async move { sub_agent.run_sub_agent(input, context).await })
    }

    pub fn spawn_hook<H>(&self, hook: Arc<H>, request: H::Request) -> RuntimeTask<H::Output>
    where
        H: HookRuntime,
    {
        let context = self.context();
        self.spawn(async move { hook.run_hook(request, context).await })
    }
}

/// Per-call runtime context passed into provider, tool, and sub-agent work.
#[derive(Clone, Debug)]
pub struct RuntimeContext {
    cancellation: CancellationToken,
    events: RuntimeEventSink,
    environment: RuntimeEnvironment,
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

    pub fn child_context(&self) -> Self {
        Self {
            cancellation: self.cancellation.child_token(),
            events: self.events.clone(),
            environment: self.environment.clone(),
        }
    }

    pub fn child_context_with_environment(&self, environment: RuntimeEnvironment) -> Self {
        Self {
            cancellation: self.cancellation.child_token(),
            events: self.events.clone(),
            environment,
        }
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

/// Cloneable Tokio mpsc sender for runtime observations and receipts.
#[derive(Clone, Debug)]
pub struct RuntimeEventSink {
    sender: mpsc::Sender<RuntimeEvent>,
}

impl RuntimeEventSink {
    pub fn channel(event_buffer: usize) -> (Self, RuntimeEventStream) {
        let (sender, receiver) = mpsc::channel(event_buffer);
        (
            Self { sender },
            tokio_stream::wrappers::ReceiverStream::new(receiver),
        )
    }

    pub async fn emit(
        &self,
        event: RuntimeEvent,
    ) -> Result<(), mpsc::error::SendError<RuntimeEvent>> {
        self.sender.send(event).await
    }
}

/// Tokio stream of runtime observations and receipts.
pub type RuntimeEventStream = tokio_stream::wrappers::ReceiverStream<RuntimeEvent>;

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
