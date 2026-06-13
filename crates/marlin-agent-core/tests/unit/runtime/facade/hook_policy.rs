use marlin_agent_core::{
    GerbilCommandSpec, GerbilHookPolicyCommandEvaluator, GerbilHookPolicyFinalizer,
    GerbilHookPolicyRuntimeBinding, HookAgentScope, HookDispatchPolicy, HookDispatchPolicyReceipt,
    HookDispatcher, HookEventName, HookHandlerType, HookInvocation, HookPolicyDecisionReason,
    HookPolicyDynamicActionApplicationEffect, HookPolicyDynamicActionApplicationReason,
    HookPolicyDynamicActionKind, HookPolicyDynamicActionTarget, HookPolicyExtension,
    HookRegistration, HookRegistrationCatalog, HookRegistry, HookRunSummary, HookRuntime,
    RuntimeContext, RuntimeFuture,
};
use std::{collections::BTreeMap, sync::Arc};
use tempfile::Builder;

#[derive(Clone, Debug)]
struct CoreSummaryHook;

impl HookRuntime for CoreSummaryHook {
    type Request = HookInvocation;
    type Output = HookRunSummary;

    fn run_hook(
        &self,
        request: Self::Request,
        _context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output> {
        Box::pin(async move {
            HookRunSummary::running(
                "core-gerbil-run",
                request.event_name,
                HookHandlerType::Command,
            )
            .completed()
        })
    }
}

#[derive(Clone, Default)]
struct StaticCoreRegistrationCatalog {
    registrations: BTreeMap<String, HookRegistration>,
}

impl StaticCoreRegistrationCatalog {
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

impl HookRegistrationCatalog<HookRegistration> for StaticCoreRegistrationCatalog {
    fn resolve_registration(
        &self,
        target: &HookPolicyDynamicActionTarget,
    ) -> Option<HookRegistration> {
        self.registrations.get(target.as_str()).cloned()
    }
}

fn core_summary_registration(id: &str) -> HookRegistration {
    HookRegistration::new(
        id,
        HookEventName::PreToolUse,
        HookHandlerType::Command,
        Arc::new(CoreSummaryHook),
    )
}

#[tokio::test]
async fn core_facade_wires_gerbil_hook_policy_finalizer() {
    let registry = HookRegistry::new().with_registration(core_summary_registration("core-gerbil"));
    let evaluator = GerbilHookPolicyCommandEvaluator::new(
        GerbilCommandSpec::new("/bin/sh").arg("-c").arg(
            "cat >/dev/null; printf '%s\n' '{\"decision\":\"Rejected\",\"diagnostics\":[{\"message\":\"core finalizer rejected\"}],\"actions\":[{\"kind\":\"Deny\",\"target\":\"core-gerbil\"}]}'",
        ),
    );
    let finalizer = GerbilHookPolicyFinalizer::new(evaluator);
    let (runtime, _events) = marlin_agent_core::TokioAgentRuntime::new(4);

    let report = HookDispatcher::new(registry)
        .with_policy(HookDispatchPolicy::observe_only().with_extension(
            HookPolicyExtension::gerbil_scheme("marlin/hooks/policy", "decide-hook-policy"),
        ))
        .with_policy_finalizer(Arc::new(finalizer))
        .dispatch(
            &runtime,
            HookInvocation::new(HookEventName::PreToolUse)
                .with_agent_scope(HookAgentScope::CustomerAgent),
        )
        .await;

    assert_eq!(report.policy.allowed_count, 0);
    assert_eq!(report.policy.rejected_count, 1);
    assert_eq!(
        report.policy.decisions[0].reason,
        HookPolicyDecisionReason::ExtensionRejected
    );
    assert_eq!(report.policy.actions.len(), 1);
    assert_eq!(
        report.policy.actions[0].kind,
        HookPolicyDynamicActionKind::Deny
    );
    assert!(report.runs.is_empty());
    assert!(!report.is_success());
}

#[tokio::test]
async fn core_facade_gerbil_hook_policy_finalizer_feeds_complex_actions_to_dispatcher() {
    let extension = HookPolicyExtension::gerbil_scheme(
        "marlin/hooks/policy-samples",
        "decide-hook-policy-sample",
    );
    let registry = HookRegistry::new()
        .with_registration(core_summary_registration("dangerous-shell"))
        .with_registration(core_summary_registration("regular-shell"));
    let catalog = StaticCoreRegistrationCatalog::default().with_registration(
        "catalog:customer-agent-hook",
        core_summary_registration("customer-agent-hook"),
    );
    let evaluator = GerbilHookPolicyCommandEvaluator::new(
        GerbilCommandSpec::new("/bin/sh").arg("-c").arg(
            "cat >/dev/null; printf '%s\n' '{\"decision\":\"Allowed\",\"diagnostics\":[{\"message\":\"complex core finalizer allowed\"}],\"actions\":[{\"kind\":\"Register\",\"target\":\"catalog:customer-agent-hook\",\"reason\":\"customer agent session requires runtime catalog hook\"},{\"kind\":\"Defer\",\"target\":\"session:release\",\"reason\":\"release lineage waits for org memory review\"},{\"kind\":\"Deny\",\"target\":\"dangerous-shell\",\"reason\":\"dirty workspace blocks dangerous shell hook\"},{\"kind\":\"Rewrite\",\"target\":\"command\",\"replacement\":\"cargo test --locked\",\"reason\":\"session policy prefers locked tests\"}]}'",
        ),
    );
    let finalizer = GerbilHookPolicyFinalizer::new(evaluator);
    let (runtime, _events) = marlin_agent_core::TokioAgentRuntime::new(4);

    let report = HookDispatcher::new(registry)
        .with_policy(HookDispatchPolicy::enforce_trusted().with_extension(extension.clone()))
        .with_registration_catalog(Arc::new(catalog))
        .with_policy_finalizer(Arc::new(finalizer))
        .dispatch(
            &runtime,
            HookInvocation::new(HookEventName::PreToolUse)
                .with_agent_scope(HookAgentScope::CustomerAgent)
                .with_message("cargo test"),
        )
        .await;

    assert_eq!(report.policy.extension, extension);
    assert_eq!(report.selection.selected_count, 3);
    assert_eq!(report.policy.actions.len(), 4);
    assert_eq!(report.policy.allowed_count, 0);
    assert_eq!(report.policy.rejected_count, 3);
    assert_eq!(
        core_policy_decision_reason(&report.policy, "dangerous-shell"),
        HookPolicyDecisionReason::ExtensionRejected,
    );
    assert_eq!(
        core_policy_decision_reason(&report.policy, "regular-shell"),
        HookPolicyDecisionReason::ExtensionDeferred,
    );
    assert_eq!(
        core_policy_decision_reason(&report.policy, "customer-agent-hook"),
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
    assert!(!report.is_success());
}

#[test]
fn core_facade_builds_gerbil_hook_policy_finalizer_from_runtime_binding() {
    let root = Builder::new()
        .prefix("marlin-core-gerbil-hook-policy-binding-")
        .tempdir()
        .expect("creates core hook policy binding root");
    let binding = GerbilHookPolicyRuntimeBinding::new("/bin/sh", root.path())
        .expect("runtime binding should write hook policy assets");
    let finalizer = GerbilHookPolicyFinalizer::from_runtime_binding(binding);

    assert_eq!(
        finalizer.evaluator().spec().program,
        std::path::Path::new("/bin/sh")
    );
    assert!(
        finalizer
            .evaluator()
            .spec()
            .args
            .iter()
            .any(|arg| arg.to_string_lossy().contains("hook-policy-adapter.ss"))
    );
}

fn core_policy_decision_reason(
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
