use marlin_agent_hooks::{HookDispatcher, HookInvocation, HookRegistry};
use marlin_agent_protocol::{
    HookEventName, HookHandlerType, HookScope, HookSource, HookTrustStatus,
};
use marlin_agent_runtime::TokioAgentRuntime;

use crate::hooks::support::summary_hook_registration;

#[tokio::test]
async fn dispatcher_orders_hooks_and_applies_registration_metadata() {
    let mut registry = HookRegistry::new();
    registry.register(
        summary_hook_registration(
            "late",
            HookEventName::PreToolUse,
            HookHandlerType::Command,
            "late-run",
        )
        .with_display_order(20)
        .with_source(HookSource::User)
        .with_trust(HookTrustStatus::Trusted),
    );
    registry.register(
        summary_hook_registration(
            "early",
            HookEventName::PreToolUse,
            HookHandlerType::Agent,
            "early-run",
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
    assert_eq!(report.selection.candidate_count, 2);
    assert_eq!(report.selection.selected_count, 2);
    assert_eq!(report.policy.evaluated_count, 2);
    assert_eq!(report.policy.allowed_count, 2);
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
