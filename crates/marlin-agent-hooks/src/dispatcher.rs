//! Dispatches registered `HookRuntime` handlers and returns protocol-owned hook receipts.

use std::sync::Arc;

use marlin_agent_protocol::{
    HookEventName, HookExecutionMode, HookHandlerType, HookRunStatus, HookRunSummary, HookScope,
    HookSource, HookSourcePath, HookTrustStatus,
};
use marlin_agent_runtime::{HookRuntime, RuntimeContext, TokioAgentRuntime};
use tracing::Instrument;

/// Runtime shape accepted by hook registrations.
pub type RegisteredHookRuntime =
    dyn HookRuntime<Request = HookInvocation, Output = HookRunSummary> + Send + Sync + 'static;

/// Input passed to registered hook runtimes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HookInvocation {
    pub event_name: HookEventName,
    pub message: Option<String>,
}

impl HookInvocation {
    /// Creates a hook invocation for an event.
    pub fn new(event_name: HookEventName) -> Self {
        Self {
            event_name,
            message: None,
        }
    }

    /// Adds a human-readable message to the invocation.
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

/// Registered hook handler and its ordering metadata.
#[derive(Clone)]
pub struct HookRegistration {
    pub id: String,
    pub event_name: HookEventName,
    pub handler_type: HookHandlerType,
    pub execution_mode: HookExecutionMode,
    pub scope: HookScope,
    pub source_path: Option<HookSourcePath>,
    pub source: HookSource,
    pub trust: HookTrustStatus,
    pub display_order: i64,
    handler: Arc<RegisteredHookRuntime>,
}

impl HookRegistration {
    /// Creates a hook registration with conservative default metadata.
    pub fn new<H>(
        id: impl Into<String>,
        event_name: HookEventName,
        handler_type: HookHandlerType,
        handler: Arc<H>,
    ) -> Self
    where
        H: HookRuntime<Request = HookInvocation, Output = HookRunSummary>,
    {
        let handler: Arc<RegisteredHookRuntime> = handler;
        Self {
            id: id.into(),
            event_name,
            handler_type,
            execution_mode: HookExecutionMode::Sync,
            scope: HookScope::Turn,
            source_path: None,
            source: HookSource::Unknown,
            trust: HookTrustStatus::Untrusted,
            display_order: 0,
            handler,
        }
    }

    /// Sets the hook execution mode.
    pub fn with_execution_mode(mut self, execution_mode: HookExecutionMode) -> Self {
        self.execution_mode = execution_mode;
        self
    }

    /// Sets the hook scope.
    pub fn with_scope(mut self, scope: HookScope) -> Self {
        self.scope = scope;
        self
    }

    /// Sets the hook source path.
    pub fn with_source_path(mut self, source_path: impl Into<HookSourcePath>) -> Self {
        self.source_path = Some(source_path.into());
        self
    }

    /// Sets the hook source.
    pub fn with_source(mut self, source: HookSource) -> Self {
        self.source = source;
        self
    }

    /// Sets the hook trust status.
    pub fn with_trust(mut self, trust: HookTrustStatus) -> Self {
        self.trust = trust;
        self
    }

    /// Sets the hook display order. Lower values run first.
    pub fn with_display_order(mut self, display_order: i64) -> Self {
        self.display_order = display_order;
        self
    }

    fn run(
        &self,
        context: RuntimeContext,
        invocation: HookInvocation,
    ) -> tokio::task::JoinHandle<HookRunSummary> {
        let handler = self.handler.clone();
        let span = hook_run_span(self);
        tokio::spawn(async move { handler.run_hook(invocation, context).await }.instrument(span))
    }
}

/// Ordered registry of hook registrations.
#[derive(Clone, Default)]
pub struct HookRegistry {
    registrations: Vec<HookRegistration>,
}

impl HookRegistry {
    /// Creates an empty hook registry.
    pub fn new() -> Self {
        Self {
            registrations: Vec::new(),
        }
    }

    /// Registers one hook handler.
    pub fn register(&mut self, registration: HookRegistration) {
        self.registrations.push(registration);
        self.sort();
    }

    /// Returns a new registry containing one additional hook registration.
    pub fn with_registration(mut self, registration: HookRegistration) -> Self {
        self.register(registration);
        self
    }

    /// Returns all registrations in dispatch order.
    pub fn registrations(&self) -> &[HookRegistration] {
        &self.registrations
    }

    fn matching(&self, event_name: &HookEventName) -> Vec<HookRegistration> {
        self.registrations
            .iter()
            .filter(|registration| &registration.event_name == event_name)
            .cloned()
            .collect()
    }

    fn sort(&mut self) {
        self.registrations
            .sort_by_key(|registration| registration.display_order);
    }
}

/// Dispatches hook invocations through a registry.
#[derive(Clone, Default)]
pub struct HookDispatcher {
    registry: HookRegistry,
}

impl HookDispatcher {
    /// Creates a dispatcher from an ordered registry.
    pub fn new(registry: HookRegistry) -> Self {
        Self { registry }
    }

    /// Returns the dispatcher registry.
    pub fn registry(&self) -> &HookRegistry {
        &self.registry
    }

    /// Runs matching hooks and returns their receipts.
    pub async fn dispatch(
        &self,
        runtime: &TokioAgentRuntime,
        invocation: HookInvocation,
    ) -> HookDispatchReport {
        self.dispatch_with_context(runtime.context(), invocation)
            .await
    }

    /// Runs matching hooks with an existing runtime context and returns their receipts.
    pub async fn dispatch_with_context(
        &self,
        context: RuntimeContext,
        invocation: HookInvocation,
    ) -> HookDispatchReport {
        let registrations = self.registry.matching(&invocation.event_name);
        let dispatch_span = tracing::info_span!(
            "hook.dispatch",
            hook_event = ?invocation.event_name,
            hook_count = registrations.len()
        );

        async move {
            let mut runs = Vec::new();
            let mut async_runs = Vec::new();

            for registration in registrations {
                match registration.execution_mode {
                    HookExecutionMode::Sync => {
                        let span = hook_run_span(&registration);
                        let summary = registration
                            .handler
                            .run_hook(invocation.clone(), context.child_context())
                            .instrument(span)
                            .await;
                        runs.push(apply_registration_metadata(&registration, summary));
                    }
                    HookExecutionMode::Async => {
                        let task = registration.run(context.child_context(), invocation.clone());
                        async_runs.push((registration, task));
                    }
                }
            }

            for (registration, task) in async_runs {
                let summary = match task.await {
                    Ok(summary) => summary,
                    Err(error) => {
                        tracing::warn!(
                            hook_id = %registration.id,
                            hook_event = ?registration.event_name,
                            error = %error,
                            "async hook task join failed"
                        );
                        failed_join_summary(&registration, &invocation, error.to_string())
                    }
                };
                runs.push(apply_registration_metadata(&registration, summary));
            }

            HookDispatchReport {
                event_name: invocation.event_name,
                runs,
            }
        }
        .instrument(dispatch_span)
        .await
    }
}

/// Receipt collection for one hook event dispatch.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HookDispatchReport {
    pub event_name: HookEventName,
    pub runs: Vec<HookRunSummary>,
}

impl HookDispatchReport {
    /// Returns true when every hook completed successfully.
    pub fn is_success(&self) -> bool {
        self.runs
            .iter()
            .all(|run| run.status == HookRunStatus::Completed)
    }
}

fn apply_registration_metadata(
    registration: &HookRegistration,
    mut summary: HookRunSummary,
) -> HookRunSummary {
    summary.event_name = registration.event_name.clone();
    summary.handler_type = registration.handler_type.clone();
    summary.execution_mode = registration.execution_mode.clone();
    summary.scope = registration.scope.clone();
    summary.source_path = registration.source_path.clone();
    summary.source = registration.source.clone();
    summary.trust = registration.trust.clone();
    summary.display_order = registration.display_order;
    summary
}

fn hook_run_span(registration: &HookRegistration) -> tracing::Span {
    tracing::info_span!(
        "hook.run",
        hook_id = %registration.id,
        hook_event = ?registration.event_name,
        hook_mode = ?registration.execution_mode,
        hook_handler = ?registration.handler_type
    )
}

fn failed_join_summary(
    registration: &HookRegistration,
    invocation: &HookInvocation,
    status_message: String,
) -> HookRunSummary {
    let mut summary = HookRunSummary::running(
        registration.id.clone(),
        invocation.event_name.clone(),
        registration.handler_type.clone(),
    );
    summary.status = HookRunStatus::Failed;
    summary.status_message = Some(status_message);
    summary
}
