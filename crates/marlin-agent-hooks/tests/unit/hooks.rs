use std::sync::Arc;

use marlin_agent_hooks::{HookDispatcher, HookInvocation, HookRegistration, HookRegistry};
use marlin_agent_protocol::{
    HookEventName, HookExecutionMode, HookHandlerType, HookRunStatus, HookRunSummary, HookScope,
    HookSource, HookTrustStatus,
};
use marlin_agent_runtime::{HookRuntime, RuntimeContext, RuntimeFuture, TokioAgentRuntime};

#[tokio::test]
async fn dispatcher_orders_hooks_and_applies_registration_metadata() {
    let mut registry = HookRegistry::new();
    registry.register(
        HookRegistration::new(
            "late",
            HookEventName::PreToolUse,
            HookHandlerType::Command,
            Arc::new(SummaryHook::new("late-run")),
        )
        .with_display_order(20)
        .with_source(HookSource::User)
        .with_trust(HookTrustStatus::Trusted),
    );
    registry.register(
        HookRegistration::new(
            "early",
            HookEventName::PreToolUse,
            HookHandlerType::Agent,
            Arc::new(SummaryHook::new("early-run")),
        )
        .with_display_order(10)
        .with_scope(HookScope::Thread)
        .with_source(HookSource::Project)
        .with_trust(HookTrustStatus::Managed),
    );
    let (runtime, _events) = TokioAgentRuntime::new(4);

    let report = HookDispatcher::new(registry)
        .dispatch(
            &runtime,
            HookInvocation::new(HookEventName::PreToolUse).with_message("tool"),
        )
        .await;

    assert_eq!(report.event_name, HookEventName::PreToolUse);
    assert!(report.is_success());
    assert_eq!(
        report
            .runs
            .iter()
            .map(|run| run.id.as_str())
            .collect::<Vec<_>>(),
        vec!["early-run", "late-run"]
    );
    assert_eq!(report.runs[0].display_order, 10);
    assert_eq!(report.runs[0].handler_type, HookHandlerType::Agent);
    assert_eq!(report.runs[0].scope, HookScope::Thread);
    assert_eq!(report.runs[0].source, HookSource::Project);
    assert_eq!(report.runs[0].trust, HookTrustStatus::Managed);
    assert_eq!(report.runs[1].display_order, 20);
}

#[tokio::test]
async fn dispatcher_ignores_other_hook_events() {
    let registry = HookRegistry::new().with_registration(HookRegistration::new(
        "post-tool",
        HookEventName::PostToolUse,
        HookHandlerType::Command,
        Arc::new(SummaryHook::new("post-tool-run")),
    ));
    let (runtime, _events) = TokioAgentRuntime::new(4);

    let report = HookDispatcher::new(registry)
        .dispatch(&runtime, HookInvocation::new(HookEventName::PreToolUse))
        .await;

    assert!(report.runs.is_empty());
    assert!(report.is_success());
}

#[tokio::test]
async fn dispatcher_executes_async_hooks_and_marks_execution_mode() {
    let registry = HookRegistry::new().with_registration(
        HookRegistration::new(
            "async-hook",
            HookEventName::SubAgentStart,
            HookHandlerType::Agent,
            Arc::new(SummaryHook::new("async-run")),
        )
        .with_execution_mode(HookExecutionMode::Async)
        .with_display_order(5),
    );
    let (runtime, _events) = TokioAgentRuntime::new(4);

    let report = HookDispatcher::new(registry)
        .dispatch(&runtime, HookInvocation::new(HookEventName::SubAgentStart))
        .await;

    assert_eq!(report.runs.len(), 1);
    assert_eq!(report.runs[0].id.as_str(), "async-run");
    assert_eq!(report.runs[0].execution_mode, HookExecutionMode::Async);
    assert_eq!(report.runs[0].status, HookRunStatus::Completed);
}

#[derive(Clone, Debug)]
struct SummaryHook {
    id: &'static str,
}

impl SummaryHook {
    fn new(id: &'static str) -> Self {
        Self { id }
    }
}

impl HookRuntime for SummaryHook {
    type Request = HookInvocation;
    type Output = HookRunSummary;

    fn run_hook(
        &self,
        request: Self::Request,
        _context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output> {
        let id = self.id;
        Box::pin(async move {
            HookRunSummary::running(id, request.event_name, HookHandlerType::Command).completed()
        })
    }
}
