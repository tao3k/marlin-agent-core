//! Applies protocol-owned dynamic hook actions without owning dispatcher state.

use marlin_agent_protocol::{
    HookDispatchPolicyReceipt, HookDispatchPolicyReceiptInput, HookPolicyDecision,
    HookPolicyDecisionReason, HookPolicyDecisionReceipt, HookPolicyDynamicAction,
    HookPolicyDynamicActionApplicationEffect, HookPolicyDynamicActionApplicationReason,
    HookPolicyDynamicActionApplicationReceipt, HookPolicyDynamicActionApplicationStatus,
    HookPolicyDynamicActionKind, HookPolicyDynamicActionTarget, HookRegistryUpdateReceipt,
    HookRunId,
};

/// Runtime catalog that resolves extension-requested hook registrations.
pub(crate) type DynamicHookRegistrationCatalog<Registration> =
    dyn HookRegistrationCatalog<Registration> + Send + Sync + 'static;

/// Catalog boundary for extension-requested hook registration actions.
pub trait HookRegistrationCatalog<Registration> {
    fn resolve_registration(&self, target: &HookPolicyDynamicActionTarget) -> Option<Registration>;
}

pub(crate) trait HookRegistryActionTarget<Registration> {
    fn register_dynamic_hook(&mut self, registration: Registration) -> HookRegistryUpdateReceipt;

    fn unregister_dynamic_hook(&mut self, hook_id: &HookRunId)
    -> Option<HookRegistryUpdateReceipt>;
}

pub(crate) struct DynamicRegistryActionApplication {
    pub(crate) changed: bool,
    pub(crate) receipts: Vec<HookPolicyDynamicActionApplicationReceipt>,
}

pub(crate) struct DynamicPolicyActionApplication {
    pub(crate) message: Option<String>,
    pub(crate) policy: HookDispatchPolicyReceipt,
    pub(crate) receipts: Vec<HookPolicyDynamicActionApplicationReceipt>,
}

pub(crate) fn attach_dynamic_actions(
    policy: HookDispatchPolicyReceipt,
    actions: Vec<HookPolicyDynamicAction>,
) -> HookDispatchPolicyReceipt {
    HookDispatchPolicyReceipt::new(HookDispatchPolicyReceiptInput {
        event_name: policy.event_name,
        invocation_agent_scope: policy.invocation_agent_scope,
        mode: policy.mode,
        extension: policy.extension,
        actions,
        decisions: policy.decisions,
    })
}

pub(crate) fn apply_dynamic_registry_actions<Registration>(
    registry: &mut impl HookRegistryActionTarget<Registration>,
    actions: &[HookPolicyDynamicAction],
    catalog: Option<&DynamicHookRegistrationCatalog<Registration>>,
) -> DynamicRegistryActionApplication {
    let (changed, receipts) = actions.iter().fold(
        (false, Vec::new()),
        |(changed, mut receipts), action| match &action.kind {
            HookPolicyDynamicActionKind::Register => {
                let (did_change, receipt) =
                    apply_dynamic_register_action(registry, action, catalog);
                receipts.push(receipt);
                (changed || did_change, receipts)
            }
            HookPolicyDynamicActionKind::Unregister => {
                let (did_change, receipt) = apply_dynamic_unregister_action(registry, action);
                receipts.push(receipt);
                (changed || did_change, receipts)
            }
            HookPolicyDynamicActionKind::Deny
            | HookPolicyDynamicActionKind::Rewrite
            | HookPolicyDynamicActionKind::Defer => (changed, receipts),
        },
    );

    DynamicRegistryActionApplication { changed, receipts }
}

fn apply_dynamic_register_action<Registration>(
    registry: &mut impl HookRegistryActionTarget<Registration>,
    action: &HookPolicyDynamicAction,
    catalog: Option<&DynamicHookRegistrationCatalog<Registration>>,
) -> (bool, HookPolicyDynamicActionApplicationReceipt) {
    let Some(target) = action.target.as_ref() else {
        return (
            false,
            dynamic_action_receipt(
                action,
                HookPolicyDynamicActionApplicationStatus::Failed,
                HookPolicyDynamicActionApplicationEffect::Noop,
                HookPolicyDynamicActionApplicationReason::MissingTarget,
                None,
                None,
            ),
        );
    };
    let Some(catalog) = catalog else {
        return (
            false,
            dynamic_action_receipt(
                action,
                HookPolicyDynamicActionApplicationStatus::Failed,
                HookPolicyDynamicActionApplicationEffect::Noop,
                HookPolicyDynamicActionApplicationReason::CatalogUnavailable,
                None,
                None,
            ),
        );
    };
    let Some(registration) = catalog.resolve_registration(target) else {
        return (
            false,
            dynamic_action_receipt(
                action,
                HookPolicyDynamicActionApplicationStatus::Failed,
                HookPolicyDynamicActionApplicationEffect::Noop,
                HookPolicyDynamicActionApplicationReason::CatalogMiss,
                None,
                None,
            ),
        );
    };

    let update = registry.register_dynamic_hook(registration);
    let hook_id = update.hook_id.clone();
    (
        true,
        dynamic_action_receipt(
            action,
            HookPolicyDynamicActionApplicationStatus::Applied,
            HookPolicyDynamicActionApplicationEffect::RegistrationRegistered,
            HookPolicyDynamicActionApplicationReason::CatalogResolved,
            Some(hook_id),
            Some(update),
        ),
    )
}

fn apply_dynamic_unregister_action<Registration>(
    registry: &mut impl HookRegistryActionTarget<Registration>,
    action: &HookPolicyDynamicAction,
) -> (bool, HookPolicyDynamicActionApplicationReceipt) {
    let Some(target) = action.target.as_ref() else {
        return (
            false,
            dynamic_action_receipt(
                action,
                HookPolicyDynamicActionApplicationStatus::Failed,
                HookPolicyDynamicActionApplicationEffect::Noop,
                HookPolicyDynamicActionApplicationReason::MissingTarget,
                None,
                None,
            ),
        );
    };
    let hook_id = HookRunId::new(target.as_str());
    let Some(update) = registry.unregister_dynamic_hook(&hook_id) else {
        return (
            false,
            dynamic_action_receipt(
                action,
                HookPolicyDynamicActionApplicationStatus::Failed,
                HookPolicyDynamicActionApplicationEffect::Noop,
                HookPolicyDynamicActionApplicationReason::RegistryMiss,
                Some(hook_id),
                None,
            ),
        );
    };
    let hook_id = update.hook_id.clone();
    (
        true,
        dynamic_action_receipt(
            action,
            HookPolicyDynamicActionApplicationStatus::Applied,
            HookPolicyDynamicActionApplicationEffect::RegistrationUnregistered,
            HookPolicyDynamicActionApplicationReason::RegistryUpdated,
            Some(hook_id),
            Some(update),
        ),
    )
}

pub(crate) fn apply_dynamic_policy_actions(
    mut message: Option<String>,
    mut policy: HookDispatchPolicyReceipt,
) -> DynamicPolicyActionApplication {
    let receipts = policy
        .actions
        .clone()
        .iter()
        .filter_map(|action| match &action.kind {
            HookPolicyDynamicActionKind::Deny => Some(apply_dynamic_decision_action(
                &mut policy.decisions,
                action,
                HookPolicyDecisionReason::ExtensionRejected,
                HookPolicyDynamicActionApplicationEffect::DecisionRejected,
            )),
            HookPolicyDynamicActionKind::Defer => Some(apply_dynamic_decision_action(
                &mut policy.decisions,
                action,
                HookPolicyDecisionReason::ExtensionDeferred,
                HookPolicyDynamicActionApplicationEffect::DispatchDeferred,
            )),
            HookPolicyDynamicActionKind::Rewrite => {
                Some(apply_dynamic_rewrite_action(&mut message, action))
            }
            HookPolicyDynamicActionKind::Register | HookPolicyDynamicActionKind::Unregister => None,
        })
        .collect::<Vec<_>>();

    let policy = HookDispatchPolicyReceipt::new(HookDispatchPolicyReceiptInput {
        event_name: policy.event_name,
        invocation_agent_scope: policy.invocation_agent_scope,
        mode: policy.mode,
        extension: policy.extension,
        actions: policy.actions,
        decisions: policy.decisions,
    });

    DynamicPolicyActionApplication {
        message,
        policy,
        receipts,
    }
}

fn apply_dynamic_decision_action(
    decisions: &mut [HookPolicyDecisionReceipt],
    action: &HookPolicyDynamicAction,
    reason: HookPolicyDecisionReason,
    effect: HookPolicyDynamicActionApplicationEffect,
) -> HookPolicyDynamicActionApplicationReceipt {
    let matched = decisions.iter_mut().find_map(|decision| {
        dynamic_action_targets_decision(action, decision).then(|| {
            decision.decision = HookPolicyDecision::Rejected;
            decision.reason = reason.clone();
            decision.hook_id.clone()
        })
    });

    match matched {
        Some(hook_id) => dynamic_action_receipt(
            action,
            HookPolicyDynamicActionApplicationStatus::Applied,
            effect,
            HookPolicyDynamicActionApplicationReason::TargetMatched,
            Some(hook_id),
            None,
        ),
        None => dynamic_action_receipt(
            action,
            HookPolicyDynamicActionApplicationStatus::Ignored,
            HookPolicyDynamicActionApplicationEffect::Noop,
            HookPolicyDynamicActionApplicationReason::TargetNotMatched,
            None,
            None,
        ),
    }
}

fn dynamic_action_targets_decision(
    action: &HookPolicyDynamicAction,
    decision: &HookPolicyDecisionReceipt,
) -> bool {
    match action.target.as_ref() {
        Some(target) => target.as_str() == decision.hook_id.as_str(),
        None => true,
    }
}

fn apply_dynamic_rewrite_action(
    message: &mut Option<String>,
    action: &HookPolicyDynamicAction,
) -> HookPolicyDynamicActionApplicationReceipt {
    if !dynamic_action_targets_invocation_rewrite(action) {
        return dynamic_action_receipt(
            action,
            HookPolicyDynamicActionApplicationStatus::Ignored,
            HookPolicyDynamicActionApplicationEffect::Noop,
            HookPolicyDynamicActionApplicationReason::UnsupportedTarget,
            None,
            None,
        );
    }

    let Some(replacement) = &action.replacement else {
        return dynamic_action_receipt(
            action,
            HookPolicyDynamicActionApplicationStatus::Failed,
            HookPolicyDynamicActionApplicationEffect::Noop,
            HookPolicyDynamicActionApplicationReason::MissingReplacement,
            None,
            None,
        );
    };

    *message = Some(replacement.as_str().to_owned());
    dynamic_action_receipt(
        action,
        HookPolicyDynamicActionApplicationStatus::Applied,
        HookPolicyDynamicActionApplicationEffect::InvocationRewritten,
        HookPolicyDynamicActionApplicationReason::TargetMatched,
        None,
        None,
    )
}

fn dynamic_action_targets_invocation_rewrite(action: &HookPolicyDynamicAction) -> bool {
    match action.target.as_ref().map(|target| target.as_str()) {
        Some("command" | "message") | None => true,
        Some(_) => false,
    }
}

fn dynamic_action_receipt(
    action: &HookPolicyDynamicAction,
    status: HookPolicyDynamicActionApplicationStatus,
    effect: HookPolicyDynamicActionApplicationEffect,
    reason: HookPolicyDynamicActionApplicationReason,
    target_hook_id: Option<HookRunId>,
    registry_update: Option<HookRegistryUpdateReceipt>,
) -> HookPolicyDynamicActionApplicationReceipt {
    HookPolicyDynamicActionApplicationReceipt {
        action: action.clone(),
        status,
        effect,
        reason,
        target_hook_id,
        registry_update,
    }
}
