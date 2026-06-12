use marlin_agent_hooks::{HookDispatcher, HookInvocation, HookRegistry};
use marlin_agent_protocol::{HookEventName, HookExecutionMode, HookHandlerType, HookRunStatus};
use marlin_agent_runtime::TokioAgentRuntime;

use crate::hooks::support::summary_hook_registration;

#[tokio::test]
async fn dispatcher_executes_async_hooks_and_marks_execution_mode() {
    let registry = HookRegistry::new().with_registration(
        summary_hook_registration(
            "async-hook",
            HookEventName::SubAgentStart,
            HookHandlerType::Agent,
            "async-run",
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
