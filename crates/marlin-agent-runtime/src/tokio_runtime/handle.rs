// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Tokio runtime handle and spawn helpers.

use marlin_agent_gitops::working_copy::WorkingCopyIsolationReceipt;
use std::{future::Future, sync::Arc, time::Duration};

use crate::observability;
use marlin_agent_protocol::ProcessMemorySampler;
use tokio::sync::{oneshot, watch};
use tracing::Instrument;

use super::context_visibility_from_sub_agent_policy;
use super::{
    AgentSessionContext, CancellationToken, ContextVisibility, HookRuntime, ProviderRuntime,
    RuntimeContext, RuntimeEnvironment, RuntimeEventSink, RuntimeEventStream,
    RuntimeExecutionIdentity, RuntimeOperationId, RuntimeProcessBudget, RuntimeTask,
    RuntimeTaskOutcome, RuntimeTaskShutdownReceipt, RuntimeTaskShutdownRequest, RuntimeTaskTracker,
    RuntimeTaskTrackerPolicy, SessionId, SessionIsolationReceipt, SessionKind,
    SessionRuntimeSnapshot, SubAgentRuntime, SubAgentSpawnConfig, SubAgentSpawnReceipt,
    TokioRuntimePolicy, TokioRuntimePolicyReceipt, ToolRuntime,
};
#[path = "fanout.rs"]
mod fanout;

/// Tokio-backed runtime handle shared by providers, tools, and sub-agents.
#[derive(Clone, Debug)]
pub struct TokioAgentRuntime {
    cancellation: CancellationToken,
    events: RuntimeEventSink,
    environment: RuntimeEnvironment,
    project_envrc: Option<crate::ProjectEnvrcDiscovery>,
    execution: Option<RuntimeExecutionIdentity>,
    session: AgentSessionContext,
    graph_loop_runs: crate::GraphLoopRunRegistryHandle,
    process_registry: crate::process::RuntimeProcessRegistryHandle,
    process_quota: Option<Arc<tokio::sync::Semaphore>>,
    process_cleanup_policy: crate::process::RuntimeProcessCleanupPolicy,
    task_tracker: RuntimeTaskTracker,
    runtime_snapshot: SessionRuntimeSnapshot,
}

/// Isolated cancellation and retirement accounting for Tokio tasks.
#[derive(Clone, Debug)]
pub struct RuntimeTaskScope(TokioAgentRuntime);

impl RuntimeTaskScope {
    pub fn cancellation_token(&self) -> CancellationToken {
        self.0.cancellation_token()
    }

    pub fn spawn<F>(&self, future: F) -> RuntimeTask<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.0.spawn(future)
    }

    pub async fn shutdown_tasks(
        &self,
        policy: &RuntimeTaskTrackerPolicy,
    ) -> RuntimeTaskShutdownReceipt {
        self.0.shutdown_tasks(policy).await
    }
}

/// Named request for constructing a Tokio-backed agent runtime.
#[derive(Clone, Debug)]
pub struct TokioAgentRuntimeBuildRequest {
    event_buffer: usize,
    process_limit: Option<std::num::NonZeroUsize>,
    process_budget: Option<(RuntimeProcessBudget, Arc<dyn ProcessMemorySampler>)>,
    host_run_id: Option<crate::process::RuntimeHostRunId>,
    process_ledger: Option<Arc<dyn crate::process::ProcessLedgerStore>>,
    cancellation: CancellationToken,
    environment: RuntimeEnvironment,
    project_envrc: Option<crate::ProjectEnvrcDiscovery>,
    session: AgentSessionContext,
    runtime_snapshot: SessionRuntimeSnapshot,
}

impl TokioAgentRuntimeBuildRequest {
    pub fn runtime_root(event_buffer: usize) -> Self {
        let session = AgentSessionContext::runtime_root();
        let runtime_snapshot = SessionRuntimeSnapshot::new(
            session.session_id().clone(),
            TokioRuntimePolicy::default(),
        );
        Self {
            event_buffer,
            process_limit: None,
            process_budget: None,
            host_run_id: None,
            process_ledger: None,
            cancellation: CancellationToken::new(),
            environment: RuntimeEnvironment::default(),
            project_envrc: None,
            session,
            runtime_snapshot,
        }
    }

    /// Attach a custom persistent ledger store for durable process tracking and
    /// post-restart orphan reconciliation.
    pub fn with_process_ledger(
        mut self,
        ledger: Arc<dyn crate::process::ProcessLedgerStore>,
    ) -> Self {
        self.process_ledger = Some(ledger);
        self
    }

    /// Persist managed processes to a crash-safe JSON ledger file at `path`.
    pub fn with_process_ledger_path(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.process_ledger = Some(Arc::new(crate::process::FileProcessLedgerStore::new(path)));
        self
    }

    /// Pin an explicit host boot run identifier for this runtime instance.
    pub fn with_host_run_id(mut self, host_run_id: crate::process::RuntimeHostRunId) -> Self {
        self.host_run_id = Some(host_run_id);
        self
    }

    /// Limits managed child admissions shared across all child contexts.
    /// On Unix, a permit remains held until the group is observed absent.
    /// Descendants that escape the group and externally owned pipes are excluded.
    pub fn with_process_limit(mut self, limit: std::num::NonZeroUsize) -> Self {
        self.process_limit = Some(limit);
        self
    }

    /// Apply a shared budget across every managed process in this Runtime.
    pub fn with_process_budget(
        mut self,
        budget: RuntimeProcessBudget,
        sampler: Arc<dyn ProcessMemorySampler>,
    ) -> Self {
        self.process_limit = Some(budget.concurrent);
        self.process_budget = Some((budget, sampler));
        self
    }

    pub fn with_project_envrc(self, binding: crate::ProjectEnvrc) -> Self {
        self.with_project_envrc_discovery(crate::ProjectEnvrcDiscovery::resolved(binding))
    }

    pub fn with_project_envrc_discovery(mut self, discovery: crate::ProjectEnvrcDiscovery) -> Self {
        self.project_envrc = Some(discovery);
        self
    }

    pub fn with_cancellation(mut self, cancellation: CancellationToken) -> Self {
        self.cancellation = cancellation;
        self
    }

    pub fn with_environment(mut self, environment: RuntimeEnvironment) -> Self {
        self.environment = environment;
        self
    }

    pub fn with_session(mut self, session: AgentSessionContext) -> Self {
        self.runtime_snapshot = runtime_snapshot_for_session(&session, &self.runtime_snapshot);
        self.session = session;
        self
    }

    pub fn with_runtime_snapshot(mut self, runtime_snapshot: SessionRuntimeSnapshot) -> Self {
        self.runtime_snapshot = runtime_snapshot_for_session(&self.session, &runtime_snapshot);
        self
    }
}

fn runtime_snapshot_for_session(
    session: &AgentSessionContext,
    snapshot: &SessionRuntimeSnapshot,
) -> SessionRuntimeSnapshot {
    SessionRuntimeSnapshot::new(
        session.session_id().clone(),
        snapshot.tokio_policy().clone(),
    )
    .with_task_tracker(snapshot.task_tracker().clone())
    .with_fanout_join(snapshot.fanout_join().clone())
    .with_blocking_bridge(snapshot.blocking_bridge().clone())
}

impl TokioAgentRuntime {
    pub fn new(event_buffer: usize) -> (Self, RuntimeEventStream) {
        Self::from_build_request(TokioAgentRuntimeBuildRequest::runtime_root(event_buffer))
    }

    pub fn with_cancellation(
        event_buffer: usize,
        cancellation: CancellationToken,
    ) -> (Self, RuntimeEventStream) {
        Self::from_build_request(
            TokioAgentRuntimeBuildRequest::runtime_root(event_buffer)
                .with_cancellation(cancellation),
        )
    }

    pub fn with_environment(
        event_buffer: usize,
        cancellation: CancellationToken,
        environment: RuntimeEnvironment,
    ) -> (Self, RuntimeEventStream) {
        Self::from_build_request(
            TokioAgentRuntimeBuildRequest::runtime_root(event_buffer)
                .with_cancellation(cancellation)
                .with_environment(environment),
        )
    }

    pub fn with_session(
        event_buffer: usize,
        cancellation: CancellationToken,
        environment: RuntimeEnvironment,
        session: AgentSessionContext,
    ) -> (Self, RuntimeEventStream) {
        Self::from_build_request(
            TokioAgentRuntimeBuildRequest::runtime_root(event_buffer)
                .with_cancellation(cancellation)
                .with_environment(environment)
                .with_session(session),
        )
    }

    pub fn from_build_request(
        request: TokioAgentRuntimeBuildRequest,
    ) -> (Self, RuntimeEventStream) {
        let (events, stream) = RuntimeEventSink::channel(request.event_buffer);
        let host_run_id = request
            .host_run_id
            .unwrap_or_else(crate::process::RuntimeHostRunId::generate);
        let process_registry = match request.process_ledger {
            Some(ledger) => {
                crate::process::RuntimeProcessRegistryHandle::with_ledger(host_run_id, ledger)
            }
            None => crate::process::RuntimeProcessRegistryHandle::new(),
        };
        (
            Self {
                cancellation: request.cancellation,
                events,
                environment: request.environment,
                project_envrc: request.project_envrc,
                execution: None,
                session: request.session,
                graph_loop_runs: crate::GraphLoopRunRegistryHandle::new(),
                process_registry,
                process_quota: request
                    .process_limit
                    .map(|limit| Arc::new(tokio::sync::Semaphore::new(limit.get()))),
                process_cleanup_policy: crate::process::RuntimeProcessCleanupPolicy::default(),
                task_tracker: RuntimeTaskTracker::with_process_budget(request.process_budget),
                runtime_snapshot: request.runtime_snapshot,
            },
            stream,
        )
    }

    /// Unique identifier for this Marlin host runtime boot.
    pub fn host_run_id(&self) -> &crate::process::RuntimeHostRunId {
        self.process_registry.host_run_id()
    }

    /// Reconcile any non-terminal processes left in the persistent ledger from
    /// previous host runs against live OS state.
    pub fn reconcile_persisted_processes(
        &self,
    ) -> std::io::Result<crate::process::PersistedProcessReconciliationReport> {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
            .unwrap_or_default();
        self.process_registry.reconcile_persisted_processes(now_ms)
    }

    pub fn project_envrc(&self) -> Option<crate::ProjectEnvrc> {
        self.project_envrc.as_ref()?.binding()
    }

    pub fn project_envrc_status(&self) -> Option<crate::EnvrcStatus> {
        self.project_envrc.as_ref()?.status()
    }

    pub fn context(&self) -> RuntimeContext {
        RuntimeContext {
            cancellation: self.cancellation.clone(),
            events: self.events.clone(),
            environment: self.environment.clone(),
            project_envrc: self.project_envrc.clone(),
            execution: self.execution.clone(),
            session: self.session.clone(),
            active_working_copy: None,
            working_copy_receipts: Vec::new(),
            graph_loop_runs: self.graph_loop_runs.clone(),
            process_registry: self.process_registry.clone(),
            process_quota: self.process_quota.clone(),
            process_cleanup_policy: self.process_cleanup_policy.clone(),
            task_tracker: self.task_tracker.clone(),
        }
    }

    pub fn graph_loop_runs(&self) -> crate::GraphLoopRunRegistryHandle {
        self.graph_loop_runs.clone()
    }

    pub fn with_process_cleanup_policy(
        mut self,
        process_cleanup_policy: crate::process::RuntimeProcessCleanupPolicy,
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

    pub fn runtime_snapshot(&self) -> &SessionRuntimeSnapshot {
        &self.runtime_snapshot
    }

    pub fn runtime_policy_receipt(&self) -> TokioRuntimePolicyReceipt {
        TokioRuntimePolicyReceipt::from_snapshot(&self.runtime_snapshot)
    }

    pub fn event_sink(&self) -> RuntimeEventSink {
        self.events.clone()
    }

    pub fn process_registry(&self) -> crate::process::RuntimeProcessRegistryHandle {
        self.process_registry.clone()
    }

    pub fn process_cleanup_policy(&self) -> &crate::process::RuntimeProcessCleanupPolicy {
        &self.process_cleanup_policy
    }

    pub fn child_runtime(&self) -> Self {
        Self {
            cancellation: self.cancellation.child_token(),
            events: self.events.clone(),
            environment: self.environment.clone(),
            project_envrc: self.project_envrc.clone(),
            execution: self.execution.clone(),
            session: self.session.clone(),
            graph_loop_runs: self.graph_loop_runs.clone(),
            process_registry: self.process_registry.clone(),
            process_quota: self.process_quota.clone(),
            process_cleanup_policy: self.process_cleanup_policy.clone(),
            task_tracker: self.task_tracker.clone(),
            runtime_snapshot: self.runtime_snapshot.clone(),
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
                project_envrc: self.project_envrc.clone(),
                execution: self.execution.clone(),
                session: self.session.clone(),
                graph_loop_runs: self.graph_loop_runs.clone(),
                process_registry: self.process_registry.clone(),
                process_quota: self.process_quota.clone(),
                process_cleanup_policy: self.process_cleanup_policy.clone(),
                task_tracker: self.task_tracker.clone(),
                runtime_snapshot: self.runtime_snapshot.clone(),
            },
            stream,
        )
    }

    /// A child cancellation domain with its own task retirement accounting.
    pub fn child_task_scope(&self) -> RuntimeTaskScope {
        let mut child = self.child_runtime();
        child.task_tracker = RuntimeTaskTracker::new();
        RuntimeTaskScope(child)
    }

    pub fn child_runtime_with_environment(&self, environment: RuntimeEnvironment) -> Self {
        Self {
            cancellation: self.cancellation.child_token(),
            events: self.events.clone(),
            environment,
            project_envrc: self.project_envrc.clone(),
            execution: self.execution.clone(),
            session: self.session.clone(),
            graph_loop_runs: self.graph_loop_runs.clone(),
            process_registry: self.process_registry.clone(),
            process_quota: self.process_quota.clone(),
            process_cleanup_policy: self.process_cleanup_policy.clone(),
            task_tracker: self.task_tracker.clone(),
            runtime_snapshot: self.runtime_snapshot.clone(),
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
        let runtime_snapshot = runtime_snapshot_for_session(&session, &self.runtime_snapshot);
        (
            Self {
                cancellation: self.cancellation.child_token(),
                events: self.events.clone(),
                environment: self.environment.clone(),
                project_envrc: self.project_envrc.clone(),
                execution: self.execution.clone(),
                session,
                graph_loop_runs: self.graph_loop_runs.clone(),
                process_registry: self.process_registry.clone(),
                process_quota: self.process_quota.clone(),
                process_cleanup_policy: self.process_cleanup_policy.clone(),
                task_tracker: self.task_tracker.clone(),
                runtime_snapshot,
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

    /// Spawn tracked work from a synchronous owner using its Tokio runtime.
    pub fn spawn_on<F>(
        &self,
        runtime: &tokio::runtime::Runtime,
        future: F,
    ) -> RuntimeTask<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let _entered = runtime.enter();
        self.spawn(future)
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
        let task_guard = match self.task_tracker.admit(
            self.session.session_id(),
            RuntimeOperationId::new("task.blocking"),
        ) {
            Ok(guard) => guard,
            Err(error) => return RuntimeTask::rejected(error),
        };
        RuntimeTask::new(
            task_guard.id().clone(),
            tokio::task::spawn_blocking(move || {
                let _task_guard = task_guard;
                span.in_scope(operation)
            }),
        )
    }

    pub async fn shutdown_tasks(
        &self,
        policy: &RuntimeTaskTrackerPolicy,
    ) -> RuntimeTaskShutdownReceipt {
        let count = self.task_tracker.close_admission();
        let request = if policy.cancel_on_shutdown_enabled() {
            self.cancellation.cancel();
            RuntimeTaskShutdownRequest::CancelTasksAndWait
        } else {
            RuntimeTaskShutdownRequest::WaitForTasks
        };
        self.task_tracker
            .wait_for_shutdown(policy.shutdown_timeout_duration(), request, count)
            .await
    }

    pub async fn shutdown_session_tasks(&self) -> RuntimeTaskShutdownReceipt {
        self.shutdown_tasks(self.runtime_snapshot.task_tracker())
            .await
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
        let session = context.session().session_id().clone();
        self.spawn_scoped(
            &session,
            RuntimeOperationId::new("provider"),
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
        let session = context.session().session_id().clone();
        (
            self.spawn_scoped(
                &session,
                RuntimeOperationId::new("provider"),
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
        let session = context.session().session_id().clone();
        self.spawn_scoped(
            &session,
            RuntimeOperationId::new("tool"),
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
        let session = context.session().session_id().clone();
        (
            self.spawn_scoped(
                &session,
                RuntimeOperationId::new("tool"),
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
        let session = context.session().session_id().clone();
        self.spawn_scoped(
            &session,
            RuntimeOperationId::new("subagent"),
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
        let session = context.session().session_id().clone();
        (
            self.spawn_scoped(
                &session,
                RuntimeOperationId::new("subagent"),
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
        let session = context.session().session_id().clone();
        self.spawn_scoped(
            &session,
            RuntimeOperationId::new("subagent"),
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
        let session = context.session().session_id().clone();
        self.spawn_scoped(
            &session,
            RuntimeOperationId::new("subagent"),
            async move { sub_agent.run_sub_agent(input, context).await },
            observability::runtime_sub_agent_span(),
        )
    }

    pub fn spawn_hook<H>(&self, hook: Arc<H>, request: H::Request) -> RuntimeTask<H::Output>
    where
        H: HookRuntime,
    {
        let context = self.context();
        let session = context.session().session_id().clone();
        self.spawn_scoped(
            &session,
            RuntimeOperationId::new("hook"),
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
        let session = context.session().session_id().clone();
        (
            self.spawn_scoped(
                &session,
                RuntimeOperationId::new("hook"),
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
        self.spawn_scoped(
            self.session.session_id(),
            RuntimeOperationId::new("task"),
            future,
            span,
        )
    }

    /// Spawn tracked work for an explicit session and operation.
    pub fn spawn_in_session_on<F>(
        &self,
        runtime: &tokio::runtime::Runtime,
        session: &SessionId,
        operation: RuntimeOperationId,
        future: F,
    ) -> RuntimeTask<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let _entered = runtime.enter();
        self.spawn_scoped(
            session,
            operation,
            future,
            observability::runtime_task_span(observability::RUNTIME_KIND_GENERIC),
        )
    }

    fn spawn_scoped<F>(
        &self,
        session: &SessionId,
        operation: RuntimeOperationId,
        future: F,
        span: tracing::Span,
    ) -> RuntimeTask<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let guard = match self.task_tracker.admit(session, operation) {
            Ok(guard) => guard,
            Err(error) => return RuntimeTask::rejected(error),
        };
        RuntimeTask::new(
            guard.id().clone(),
            tokio::spawn(async move {
                let _guard = guard;
                future.instrument(span).await
            }),
        )
    }
}
