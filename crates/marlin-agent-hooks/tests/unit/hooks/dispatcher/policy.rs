use marlin_agent_hooks::{
    HookDispatchPolicy, HookDispatchPolicyFinalizer, HookDispatchPolicyFinalizerInput,
    HookDispatcher, HookInvocation, HookRegistry,
};
use marlin_agent_protocol::{
    HookAgentScope, HookDispatchPolicyReceipt, HookDispatchPolicyReceiptInput, HookEventName,
    HookHandlerType, HookPolicyDecision, HookPolicyDecisionReason, HookPolicyDynamicAction,
    HookPolicyDynamicActionApplicationEffect, HookPolicyDynamicActionApplicationStatus,
    HookPolicyDynamicActionKind, HookPolicyDynamicActionReplacement, HookPolicyDynamicActionTarget,
    HookPolicyExtension, HookTrustStatus,
};
use marlin_agent_runtime::TokioAgentRuntime;
use std::sync::Arc;

use crate::hooks::support::summary_hook_registration;

#[tokio::test]
async fn dispatcher_default_policy_observes_untrusted_hooks_without_blocking() {
    let registry = HookRegistry::new().with_registration(summary_hook_registration(
        "untrusted",
        HookEventName::PreToolUse,
        HookHandlerType::Command,
        "untrusted-run",
    ));
    let (runtime, _events) = TokioAgentRuntime::new(4);

    let report = HookDispatcher::new(registry)
        .dispatch(
            &runtime,
            HookInvocation::new(HookEventName::PreToolUse)
                .with_agent_scope(HookAgentScope::CustomerAgent),
        )
        .await;

    assert_eq!(
        report.policy.invocation_agent_scope,
        HookAgentScope::CustomerAgent
    );
    assert_eq!(report.policy.allowed_count, 1);
    assert_eq!(
        report.policy.decisions[0].reason,
        HookPolicyDecisionReason::UntrustedAllowedByObserveOnly
    );
    assert_eq!(report.runs.len(), 1);
    assert!(report.is_success());
}

#[tokio::test]
async fn dispatcher_enforce_trusted_policy_rejects_untrusted_hooks_before_execution() {
    let registry = HookRegistry::new()
        .with_registration(
            summary_hook_registration(
                "trusted",
                HookEventName::PreToolUse,
                HookHandlerType::Command,
                "trusted-run",
            )
            .with_trust(HookTrustStatus::Trusted),
        )
        .with_registration(summary_hook_registration(
            "untrusted",
            HookEventName::PreToolUse,
            HookHandlerType::Command,
            "untrusted-run",
        ));
    let (runtime, _events) = TokioAgentRuntime::new(4);

    let report = HookDispatcher::new(registry)
        .with_policy(HookDispatchPolicy::enforce_trusted())
        .dispatch(
            &runtime,
            HookInvocation::new(HookEventName::PreToolUse)
                .with_agent_scope(HookAgentScope::SubAgent),
        )
        .await;

    assert_eq!(report.policy.evaluated_count, 2);
    assert_eq!(
        report.policy.invocation_agent_scope,
        HookAgentScope::SubAgent
    );
    assert_eq!(report.policy.allowed_count, 1);
    assert_eq!(report.policy.rejected_count, 1);
    assert_eq!(
        report
            .policy
            .decisions
            .iter()
            .find(|decision| decision.decision == HookPolicyDecision::Rejected)
            .expect("rejected decision")
            .reason,
        HookPolicyDecisionReason::UntrustedRejected
    );
    assert_eq!(
        report
            .runs
            .iter()
            .map(|run| run.id.as_str())
            .collect::<Vec<_>>(),
        vec!["trusted-run"]
    );
    assert!(!report.is_success());
}

#[derive(Clone, Debug)]
struct RejectingPolicyFinalizer;

impl HookDispatchPolicyFinalizer for RejectingPolicyFinalizer {
    fn finalize(&self, input: HookDispatchPolicyFinalizerInput) -> HookDispatchPolicyReceipt {
        let decisions = input
            .policy_receipt
            .decisions
            .into_iter()
            .map(|mut decision| {
                decision.decision = HookPolicyDecision::Rejected;
                decision.reason = HookPolicyDecisionReason::ExtensionRejected;
                decision
            })
            .collect();

        HookDispatchPolicyReceipt::new(HookDispatchPolicyReceiptInput {
            event_name: input.invocation.event_name,
            invocation_agent_scope: input.invocation.agent_scope,
            decision_context: input.invocation.decision_context.clone(),
            mode: input.policy_receipt.mode,
            extension: input.policy_receipt.extension,
            actions: Vec::new(),
            decisions,
        })
    }
}

#[tokio::test]
async fn dispatcher_policy_finalizer_can_reject_after_rust_policy_allows() {
    let registry = HookRegistry::new().with_registration(summary_hook_registration(
        "scheme-owned",
        HookEventName::PreToolUse,
        HookHandlerType::Command,
        "scheme-run",
    ));
    let (runtime, _events) = TokioAgentRuntime::new(4);

    let report = HookDispatcher::new(registry)
        .with_policy(HookDispatchPolicy::observe_only().with_extension(
            HookPolicyExtension::gerbil_scheme("marlin/hooks/policy", "decide-hook-policy"),
        ))
        .with_policy_finalizer(Arc::new(RejectingPolicyFinalizer))
        .dispatch(
            &runtime,
            HookInvocation::new(HookEventName::PreToolUse)
                .with_agent_scope(HookAgentScope::CustomerAgent),
        )
        .await;

    assert_eq!(
        report.policy.invocation_agent_scope,
        HookAgentScope::CustomerAgent
    );
    assert_eq!(report.policy.evaluated_count, 1);
    assert_eq!(report.policy.allowed_count, 0);
    assert_eq!(report.policy.rejected_count, 1);
    assert_eq!(
        report.policy.extension,
        HookPolicyExtension::gerbil_scheme("marlin/hooks/policy", "decide-hook-policy")
    );
    assert_eq!(
        report.policy.decisions[0].reason,
        HookPolicyDecisionReason::ExtensionRejected
    );
    assert!(report.runs.is_empty());
    assert!(!report.is_success());
}

#[derive(Clone, Debug)]
struct DynamicActionPolicyFinalizer {
    actions: Vec<HookPolicyDynamicAction>,
}

impl DynamicActionPolicyFinalizer {
    fn new(actions: Vec<HookPolicyDynamicAction>) -> Self {
        Self { actions }
    }
}

impl HookDispatchPolicyFinalizer for DynamicActionPolicyFinalizer {
    fn finalize(&self, input: HookDispatchPolicyFinalizerInput) -> HookDispatchPolicyReceipt {
        HookDispatchPolicyReceipt::new(HookDispatchPolicyReceiptInput {
            event_name: input.invocation.event_name,
            invocation_agent_scope: input.invocation.agent_scope,
            decision_context: input.invocation.decision_context.clone(),
            mode: input.policy_receipt.mode,
            extension: input.policy_receipt.extension,
            actions: self.actions.clone(),
            decisions: input.policy_receipt.decisions,
        })
    }
}

#[tokio::test]
async fn dispatcher_dynamic_deny_action_blocks_target_registration() {
    let registry = HookRegistry::new()
        .with_registration(
            summary_hook_registration(
                "allowed",
                HookEventName::PreToolUse,
                HookHandlerType::Command,
                "allowed-run",
            )
            .with_trust(HookTrustStatus::Trusted),
        )
        .with_registration(
            summary_hook_registration(
                "blocked",
                HookEventName::PreToolUse,
                HookHandlerType::Command,
                "blocked-run",
            )
            .with_trust(HookTrustStatus::Trusted),
        );
    let (runtime, _events) = TokioAgentRuntime::new(4);

    let report = HookDispatcher::new(registry)
        .with_policy(HookDispatchPolicy::enforce_trusted())
        .with_policy_finalizer(Arc::new(DynamicActionPolicyFinalizer::new(vec![
            HookPolicyDynamicAction {
                kind: HookPolicyDynamicActionKind::Deny,
                target: Some(HookPolicyDynamicActionTarget::from("blocked")),
                replacement: None,
                reason: None,
            },
        ])))
        .dispatch(
            &runtime,
            HookInvocation::new(HookEventName::PreToolUse)
                .with_agent_scope(HookAgentScope::SubAgent),
        )
        .await;

    assert_eq!(report.policy.allowed_count, 1);
    assert_eq!(report.policy.rejected_count, 1);
    assert_eq!(
        report
            .policy
            .decisions
            .iter()
            .find(|decision| decision.hook_id.as_str() == "blocked")
            .expect("blocked decision")
            .reason,
        HookPolicyDecisionReason::ExtensionRejected
    );
    assert_eq!(
        report
            .runs
            .iter()
            .map(|run| run.id.as_str())
            .collect::<Vec<_>>(),
        vec!["allowed-run"]
    );
    assert_eq!(
        report.applied_actions[0].status,
        HookPolicyDynamicActionApplicationStatus::Applied
    );
    assert_eq!(
        report.applied_actions[0].effect,
        HookPolicyDynamicActionApplicationEffect::DecisionRejected
    );
}

#[tokio::test]
async fn dispatcher_dynamic_defer_action_suppresses_current_dispatch() {
    let registry = HookRegistry::new().with_registration(
        summary_hook_registration(
            "deferred",
            HookEventName::PreToolUse,
            HookHandlerType::Command,
            "deferred-run",
        )
        .with_trust(HookTrustStatus::Trusted),
    );
    let (runtime, _events) = TokioAgentRuntime::new(4);

    let report = HookDispatcher::new(registry)
        .with_policy(HookDispatchPolicy::enforce_trusted())
        .with_policy_finalizer(Arc::new(DynamicActionPolicyFinalizer::new(vec![
            HookPolicyDynamicAction {
                kind: HookPolicyDynamicActionKind::Defer,
                target: None,
                replacement: None,
                reason: None,
            },
        ])))
        .dispatch(
            &runtime,
            HookInvocation::new(HookEventName::PreToolUse)
                .with_agent_scope(HookAgentScope::SubAgent),
        )
        .await;

    assert_eq!(report.policy.allowed_count, 0);
    assert_eq!(report.policy.rejected_count, 1);
    assert_eq!(
        report.policy.decisions[0].reason,
        HookPolicyDecisionReason::ExtensionDeferred
    );
    assert!(report.runs.is_empty());
    assert_eq!(
        report.applied_actions[0].effect,
        HookPolicyDynamicActionApplicationEffect::DispatchDeferred
    );
}

#[tokio::test]
async fn dispatcher_dynamic_rewrite_action_updates_hook_invocation_message() {
    let registry = HookRegistry::new().with_registration(
        summary_hook_registration(
            "rewritten",
            HookEventName::PreToolUse,
            HookHandlerType::Command,
            "rewritten-run",
        )
        .with_trust(HookTrustStatus::Trusted),
    );
    let (runtime, _events) = TokioAgentRuntime::new(4);

    let report = HookDispatcher::new(registry)
        .with_policy(HookDispatchPolicy::enforce_trusted())
        .with_policy_finalizer(Arc::new(DynamicActionPolicyFinalizer::new(vec![
            HookPolicyDynamicAction {
                kind: HookPolicyDynamicActionKind::Rewrite,
                target: Some(HookPolicyDynamicActionTarget::from("message")),
                replacement: Some(HookPolicyDynamicActionReplacement::from(
                    "cargo test --locked",
                )),
                reason: None,
            },
        ])))
        .dispatch(
            &runtime,
            HookInvocation::new(HookEventName::PreToolUse)
                .with_agent_scope(HookAgentScope::SubAgent)
                .with_message("cargo test"),
        )
        .await;

    assert_eq!(report.policy.allowed_count, 1);
    assert_eq!(report.runs.len(), 1);
    assert_eq!(
        report.runs[0].entries[0].text.as_str(),
        "cargo test --locked"
    );
    assert_eq!(
        report.applied_actions[0].effect,
        HookPolicyDynamicActionApplicationEffect::InvocationRewritten
    );
}
