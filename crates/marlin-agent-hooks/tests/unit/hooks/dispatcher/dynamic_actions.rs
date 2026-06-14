use marlin_agent_hooks::{
    HookDispatchPolicy, HookDispatchPolicyFinalizer, HookDispatchPolicyFinalizerInput,
    HookDispatcher, HookInvocation, HookRegistration, HookRegistrationCatalog, HookRegistry,
};
use marlin_agent_protocol::{
    HookAgentScope, HookDecisionContext, HookDispatchPolicyReceipt, HookDispatchPolicyReceiptInput,
    HookEventName, HookHandlerType, HookPolicyDecisionReason, HookPolicyDynamicAction,
    HookPolicyDynamicActionApplicationEffect, HookPolicyDynamicActionApplicationReason,
    HookPolicyDynamicActionApplicationStatus, HookPolicyDynamicActionKind,
    HookPolicyDynamicActionReplacement, HookPolicyDynamicActionTarget, HookPolicyExtension,
    HookRegistryUpdateKind, HookTrustStatus,
};
use marlin_agent_runtime::TokioAgentRuntime;
use std::{collections::BTreeMap, sync::Arc};

use crate::hooks::support::summary_hook_registration;

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

#[derive(Clone, Default)]
struct StaticRegistrationCatalog {
    registrations: BTreeMap<String, HookRegistration>,
}

impl StaticRegistrationCatalog {
    fn with_registration(
        mut self,
        target: impl Into<HookPolicyDynamicActionTarget>,
        registration: HookRegistration,
    ) -> Self {
        self.registrations
            .insert(target.into().into_string(), registration);
        self
    }
}

impl HookRegistrationCatalog<HookRegistration> for StaticRegistrationCatalog {
    fn resolve_registration(
        &self,
        target: &HookPolicyDynamicActionTarget,
    ) -> Option<HookRegistration> {
        self.registrations.get(target.as_str()).cloned()
    }
}

fn complex_scheme_hook_decision_context() -> HookDecisionContext {
    HookDecisionContext::new()
        .with_session_id("cheap-test-session")
        .with_agent_lineage_node("release")
        .with_workspace_state("dirty")
        .with_org_memory_hit("needs-human-review")
        .with_agent_class("customer-agent")
}

fn complex_scheme_policy_actions() -> Vec<HookPolicyDynamicAction> {
    vec![
        dynamic_action(
            HookPolicyDynamicActionKind::Register,
            Some("catalog:customer-agent-hook"),
            None,
        ),
        dynamic_action(
            HookPolicyDynamicActionKind::Defer,
            Some("session:release"),
            None,
        ),
        dynamic_action(
            HookPolicyDynamicActionKind::Deny,
            Some("dangerous-shell"),
            None,
        ),
        dynamic_action(
            HookPolicyDynamicActionKind::Rewrite,
            Some("command"),
            Some("cargo test --locked"),
        ),
    ]
}

fn dynamic_action(
    kind: HookPolicyDynamicActionKind,
    target: Option<&str>,
    replacement: Option<&str>,
) -> HookPolicyDynamicAction {
    HookPolicyDynamicAction {
        kind,
        target: target.map(HookPolicyDynamicActionTarget::from),
        replacement: replacement.map(HookPolicyDynamicActionReplacement::from),
        reason: None,
    }
}

fn policy_decision_reason(
    policy: &HookDispatchPolicyReceipt,
    hook_id: &str,
) -> HookPolicyDecisionReason {
    policy
        .decisions
        .iter()
        .find(|decision| decision.hook_id.as_str() == hook_id)
        .expect("policy decision should exist")
        .reason
        .clone()
}

#[tokio::test]
async fn dispatcher_dynamic_register_action_resolves_catalog_registration() {
    let registered = summary_hook_registration(
        "catalog-hook",
        HookEventName::PreToolUse,
        HookHandlerType::Command,
        "catalog-run",
    )
    .with_trust(HookTrustStatus::Trusted);
    let catalog = StaticRegistrationCatalog::default()
        .with_registration("catalog:customer-agent", registered);
    let (runtime, _events) = TokioAgentRuntime::new(4);

    let report = HookDispatcher::new(HookRegistry::new())
        .with_policy(HookDispatchPolicy::enforce_trusted())
        .with_registration_catalog(Arc::new(catalog))
        .with_policy_finalizer(Arc::new(DynamicActionPolicyFinalizer::new(vec![
            HookPolicyDynamicAction {
                kind: HookPolicyDynamicActionKind::Register,
                target: Some(HookPolicyDynamicActionTarget::from(
                    "catalog:customer-agent",
                )),
                replacement: None,
                reason: None,
            },
        ])))
        .dispatch(
            &runtime,
            HookInvocation::new(HookEventName::PreToolUse)
                .with_agent_scope(HookAgentScope::CustomerAgent),
        )
        .await;

    assert_eq!(
        report
            .runs
            .iter()
            .map(|run| run.id.as_str())
            .collect::<Vec<_>>(),
        vec!["catalog-run"]
    );
    assert_eq!(report.selection.selected_count, 1);
    assert_eq!(report.policy.allowed_count, 1);
    assert_eq!(report.applied_actions.len(), 1);
    assert_eq!(
        report.applied_actions[0].status,
        HookPolicyDynamicActionApplicationStatus::Applied
    );
    assert_eq!(
        report.applied_actions[0].effect,
        HookPolicyDynamicActionApplicationEffect::RegistrationRegistered
    );
    assert_eq!(
        report.applied_actions[0].reason,
        HookPolicyDynamicActionApplicationReason::CatalogResolved
    );
    assert_eq!(
        report.applied_actions[0]
            .registry_update
            .as_ref()
            .expect("registry update")
            .kind,
        HookRegistryUpdateKind::Registered
    );
}

#[tokio::test]
async fn dispatcher_complex_scheme_policy_action_set_applies_catalog_session_deny_and_rewrite() {
    let registry = HookRegistry::new()
        .with_registration(
            summary_hook_registration(
                "dangerous-shell",
                HookEventName::PreToolUse,
                HookHandlerType::Command,
                "dangerous-run",
            )
            .with_trust(HookTrustStatus::Trusted),
        )
        .with_registration(
            summary_hook_registration(
                "regular-shell",
                HookEventName::PreToolUse,
                HookHandlerType::Command,
                "regular-run",
            )
            .with_trust(HookTrustStatus::Trusted),
        );
    let catalog = StaticRegistrationCatalog::default().with_registration(
        "catalog:customer-agent-hook",
        summary_hook_registration(
            "customer-agent-hook",
            HookEventName::PreToolUse,
            HookHandlerType::Command,
            "catalog-run",
        )
        .with_trust(HookTrustStatus::Trusted),
    );
    let extension =
        HookPolicyExtension::gerbil_scheme("marlin/hooks/dynamic-policy", "decide-hook-policy");
    let (runtime, _events) = TokioAgentRuntime::new(4);

    let report = HookDispatcher::new(registry)
        .with_policy(HookDispatchPolicy::enforce_trusted().with_extension(extension.clone()))
        .with_registration_catalog(Arc::new(catalog))
        .with_policy_finalizer(Arc::new(DynamicActionPolicyFinalizer::new(
            complex_scheme_policy_actions(),
        )))
        .dispatch(
            &runtime,
            HookInvocation::new(HookEventName::PreToolUse)
                .with_agent_scope(HookAgentScope::CustomerAgent)
                .with_decision_context(complex_scheme_hook_decision_context())
                .with_message("cargo test"),
        )
        .await;

    assert_eq!(report.policy.extension, extension);
    assert_eq!(report.selection.selected_count, 3);
    assert_eq!(report.policy.actions.len(), 4);
    assert_eq!(report.policy.evaluated_count, 3);
    assert_eq!(report.policy.allowed_count, 0);
    assert_eq!(report.policy.rejected_count, 3);
    assert_eq!(
        policy_decision_reason(&report.policy, "dangerous-shell"),
        HookPolicyDecisionReason::ExtensionRejected,
    );
    assert_eq!(
        policy_decision_reason(&report.policy, "regular-shell"),
        HookPolicyDecisionReason::ExtensionDeferred,
    );
    assert_eq!(
        policy_decision_reason(&report.policy, "customer-agent-hook"),
        HookPolicyDecisionReason::ExtensionDeferred,
    );
    assert_eq!(
        report
            .applied_actions
            .iter()
            .map(|receipt| receipt.effect.clone())
            .collect::<Vec<_>>(),
        vec![
            HookPolicyDynamicActionApplicationEffect::RegistrationRegistered,
            HookPolicyDynamicActionApplicationEffect::DispatchDeferred,
            HookPolicyDynamicActionApplicationEffect::DecisionRejected,
            HookPolicyDynamicActionApplicationEffect::InvocationRewritten,
        ],
    );
    assert_eq!(
        report.applied_actions[0].reason,
        HookPolicyDynamicActionApplicationReason::CatalogResolved,
    );
    assert!(report.runs.is_empty());
}

#[tokio::test]
async fn dispatcher_dynamic_unregister_action_removes_registration_before_run() {
    let registry = HookRegistry::new()
        .with_registration(
            summary_hook_registration(
                "kept",
                HookEventName::PreToolUse,
                HookHandlerType::Command,
                "kept-run",
            )
            .with_trust(HookTrustStatus::Trusted),
        )
        .with_registration(
            summary_hook_registration(
                "removed",
                HookEventName::PreToolUse,
                HookHandlerType::Command,
                "removed-run",
            )
            .with_trust(HookTrustStatus::Trusted),
        );
    let (runtime, _events) = TokioAgentRuntime::new(4);

    let report = HookDispatcher::new(registry)
        .with_policy(HookDispatchPolicy::enforce_trusted())
        .with_policy_finalizer(Arc::new(DynamicActionPolicyFinalizer::new(vec![
            HookPolicyDynamicAction {
                kind: HookPolicyDynamicActionKind::Unregister,
                target: Some(HookPolicyDynamicActionTarget::from("removed")),
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

    assert_eq!(
        report
            .runs
            .iter()
            .map(|run| run.id.as_str())
            .collect::<Vec<_>>(),
        vec!["kept-run"]
    );
    assert_eq!(report.selection.selected_count, 1);
    assert_eq!(report.policy.allowed_count, 1);
    assert_eq!(report.applied_actions.len(), 1);
    assert_eq!(
        report.applied_actions[0].effect,
        HookPolicyDynamicActionApplicationEffect::RegistrationUnregistered
    );
    assert_eq!(
        report.applied_actions[0].reason,
        HookPolicyDynamicActionApplicationReason::RegistryUpdated
    );
    assert_eq!(
        report.applied_actions[0]
            .registry_update
            .as_ref()
            .expect("registry update")
            .kind,
        HookRegistryUpdateKind::Unregistered
    );
}
