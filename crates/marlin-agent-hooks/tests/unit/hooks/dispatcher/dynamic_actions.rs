use marlin_agent_hooks::{
    HookDispatchPolicy, HookDispatchPolicyFinalizer, HookDispatchPolicyFinalizerInput,
    HookDispatcher, HookInvocation, HookRegistration, HookRegistrationCatalog, HookRegistry,
};
use marlin_agent_protocol::{
    HookAgentScope, HookDispatchPolicyReceipt, HookDispatchPolicyReceiptInput, HookEventName,
    HookHandlerType, HookPolicyDynamicAction, HookPolicyDynamicActionApplicationEffect,
    HookPolicyDynamicActionApplicationReason, HookPolicyDynamicActionApplicationStatus,
    HookPolicyDynamicActionKind, HookPolicyDynamicActionTarget, HookRegistryUpdateKind,
    HookTrustStatus,
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
