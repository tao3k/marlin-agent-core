use marlin_agent_hooks::{HookDispatcher, HookInvocation, HookRegistry};
use marlin_agent_protocol::{
    HookAgentScope, HookDecisionContext, HookEventName, HookHandlerType, HookSelectionSkipReason,
};
use marlin_agent_runtime::{RuntimeExecutionIdentity, TokioAgentRuntime};

use crate::hooks::support::summary_hook_registration;

#[tokio::test]
async fn dispatcher_ignores_other_hook_events() {
    let registry = HookRegistry::new().with_registration(summary_hook_registration(
        "post-tool",
        HookEventName::PostToolUse,
        HookHandlerType::Command,
        "post-tool-run",
    ));
    let (runtime, _events) = TokioAgentRuntime::new(4);

    let report = HookDispatcher::new(registry)
        .dispatch(&runtime, HookInvocation::new(HookEventName::PreToolUse))
        .await;

    assert!(report.runs.is_empty());
    assert_eq!(report.selection.candidate_count, 0);
    assert_eq!(report.selection.selected_count, 0);
    assert_eq!(report.policy.evaluated_count, 0);
    assert!(report.is_success());
}

#[tokio::test]
async fn dispatcher_does_not_confuse_stop_with_sub_agent_stop() {
    let registry = HookRegistry::new()
        .with_registration(summary_hook_registration(
            "stop",
            HookEventName::Stop,
            HookHandlerType::Command,
            "stop-run",
        ))
        .with_registration(summary_hook_registration(
            "sub-agent-stop",
            HookEventName::SubAgentStop,
            HookHandlerType::Agent,
            "sub-agent-stop-run",
        ));
    let (runtime, _events) = TokioAgentRuntime::new(4);

    let report = HookDispatcher::new(registry)
        .dispatch(&runtime, HookInvocation::new(HookEventName::SubAgentStop))
        .await;

    assert_eq!(
        report
            .runs
            .iter()
            .map(|run| run.id.as_str())
            .collect::<Vec<_>>(),
        vec!["sub-agent-stop-run"]
    );
    assert_eq!(report.selection.candidate_count, 1);
    assert_eq!(report.selection.selected_count, 1);
    assert_eq!(report.policy.allowed_count, 1);
}

#[tokio::test]
async fn dispatcher_filters_hooks_by_agent_scope() {
    let registry = HookRegistry::new()
        .with_registration(
            summary_hook_registration(
                "root",
                HookEventName::PreToolUse,
                HookHandlerType::Command,
                "root-run",
            )
            .with_agent_scope(HookAgentScope::RootAgent),
        )
        .with_registration(
            summary_hook_registration(
                "sub-agent",
                HookEventName::PreToolUse,
                HookHandlerType::Agent,
                "sub-agent-run",
            )
            .with_agent_scope(HookAgentScope::SubAgent),
        );
    let (runtime, _events) = TokioAgentRuntime::new(4);

    let report = HookDispatcher::new(registry)
        .dispatch(
            &runtime,
            HookInvocation::new(HookEventName::PreToolUse)
                .with_agent_scope(HookAgentScope::SubAgent),
        )
        .await;

    assert_eq!(report.selection.candidate_count, 2);
    assert_eq!(report.selection.selected_count, 1);
    assert_eq!(
        report
            .selection
            .candidates
            .iter()
            .find(|candidate| candidate.hook_id.as_str() == "root")
            .expect("root candidate")
            .skip_reason,
        Some(HookSelectionSkipReason::AgentScopeMismatch)
    );
    assert_eq!(
        report
            .runs
            .iter()
            .map(|run| run.id.as_str())
            .collect::<Vec<_>>(),
        vec!["sub-agent-run"]
    );
    assert_eq!(report.runs[0].agent_scope, HookAgentScope::SubAgent);
}

#[tokio::test]
async fn dispatcher_carries_typed_policy_context_into_receipts() {
    let registry = HookRegistry::new().with_registration(summary_hook_registration(
        "customer-policy",
        HookEventName::PreToolUse,
        HookHandlerType::Command,
        "customer-policy-run",
    ));
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let context = HookDecisionContext::new()
        .with_session_id("session-42")
        .with_agent_lineage_node("root-agent")
        .with_agent_lineage_node("customer-agent")
        .with_workspace_state("native-aot-ready")
        .with_org_memory_hit("hook-roadmap")
        .with_agent_class("customer-reviewer");

    let report = HookDispatcher::new(registry)
        .dispatch(
            &runtime,
            HookInvocation::new(HookEventName::PreToolUse)
                .with_agent_scope(HookAgentScope::CustomerAgent)
                .with_decision_context(context.clone()),
        )
        .await;

    assert_eq!(report.selection.decision_context, context);
    assert_eq!(report.policy.decision_context, context);
    assert_eq!(
        report
            .selection
            .decision_context
            .session_id
            .as_ref()
            .map(|session_id| session_id.as_str()),
        Some("session-42")
    );
    assert_eq!(
        report
            .policy
            .decision_context
            .agent_class
            .as_ref()
            .map(|agent_class| agent_class.as_str()),
        Some("customer-reviewer")
    );
    assert_eq!(report.runs.len(), 1);
}

#[tokio::test]
async fn dispatcher_projects_runtime_context_into_hook_decision_context() {
    let registry = HookRegistry::new().with_registration(summary_hook_registration(
        "runtime-context",
        HookEventName::PreToolUse,
        HookHandlerType::Command,
        "runtime-context-run",
    ));
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let context = runtime
        .context()
        .with_execution_identity(RuntimeExecutionIdentity::new("run-7", "graph-9"));

    let report = HookDispatcher::new(registry)
        .dispatch(
            &runtime,
            HookInvocation::new(HookEventName::PreToolUse).with_runtime_context(&context),
        )
        .await;

    assert_eq!(
        report
            .selection
            .decision_context
            .session_id
            .as_ref()
            .map(|session_id| session_id.as_str()),
        Some("runtime.root")
    );
    assert!(
        report
            .policy
            .decision_context
            .agent_lineage
            .iter()
            .any(|node| node.as_str() == "root_session:runtime.root")
    );
    assert!(
        report
            .policy
            .decision_context
            .workspace_state
            .iter()
            .any(|fact| fact.as_str() == "run_id=run-7")
    );
    assert!(
        report
            .policy
            .decision_context
            .workspace_state
            .iter()
            .any(|fact| fact.as_str() == "graph_id=graph-9")
    );
}
