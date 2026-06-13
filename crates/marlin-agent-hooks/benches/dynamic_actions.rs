use criterion::{Criterion, criterion_group, criterion_main};
use marlin_agent_hooks::{
    HookDispatchPolicy, HookDispatchPolicyFinalizer, HookDispatchPolicyFinalizerInput,
    HookDispatcher, HookInvocation, HookRegistration, HookRegistrationCatalog, HookRegistry,
};
use marlin_agent_protocol::{
    HookAgentScope, HookDispatchPolicyReceipt, HookDispatchPolicyReceiptInput, HookEventName,
    HookHandlerType, HookPolicyDynamicAction, HookPolicyDynamicActionKind,
    HookPolicyDynamicActionReplacement, HookPolicyDynamicActionTarget, HookRunSummary,
    HookTrustStatus,
};
use marlin_agent_runtime::{HookRuntime, RuntimeContext, RuntimeFuture, TokioAgentRuntime};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone, Debug)]
struct BenchHook;

impl HookRuntime for BenchHook {
    type Request = HookInvocation;
    type Output = HookRunSummary;

    fn run_hook(
        &self,
        request: Self::Request,
        _context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output> {
        Box::pin(async move {
            HookRunSummary::running(
                "dynamic-action-run",
                request.event_name,
                HookHandlerType::Command,
            )
            .completed()
        })
    }
}

#[derive(Clone, Debug)]
struct BenchFinalizer {
    actions: Vec<HookPolicyDynamicAction>,
}

impl HookDispatchPolicyFinalizer for BenchFinalizer {
    fn finalize(&self, input: HookDispatchPolicyFinalizerInput) -> HookDispatchPolicyReceipt {
        HookDispatchPolicyReceipt::new(HookDispatchPolicyReceiptInput {
            event_name: input.invocation.event_name,
            invocation_agent_scope: input.invocation.agent_scope,
            decision_context: input.policy_receipt.decision_context,
            mode: input.policy_receipt.mode,
            extension: input.policy_receipt.extension,
            actions: self.actions.clone(),
            decisions: input.policy_receipt.decisions,
        })
    }
}

#[derive(Clone, Default)]
struct BenchCatalog {
    registrations: BTreeMap<String, HookRegistration>,
}

impl HookRegistrationCatalog<HookRegistration> for BenchCatalog {
    fn resolve_registration(
        &self,
        target: &HookPolicyDynamicActionTarget,
    ) -> Option<HookRegistration> {
        self.registrations.get(target.as_str()).cloned()
    }
}

fn bench_dynamic_action_dispatch(c: &mut Criterion) {
    let mut registrations = BTreeMap::new();
    registrations.insert(
        "catalog:dynamic-action".to_string(),
        HookRegistration::new(
            "dynamic-action-hook",
            HookEventName::PreToolUse,
            HookHandlerType::Command,
            Arc::new(BenchHook),
        )
        .with_trust(HookTrustStatus::Trusted),
    );
    let catalog = BenchCatalog { registrations };
    let dispatcher = HookDispatcher::new(HookRegistry::new())
        .with_policy(HookDispatchPolicy::enforce_trusted())
        .with_registration_catalog(Arc::new(catalog))
        .with_policy_finalizer(Arc::new(BenchFinalizer {
            actions: vec![
                HookPolicyDynamicAction {
                    kind: HookPolicyDynamicActionKind::Register,
                    target: Some(HookPolicyDynamicActionTarget::from(
                        "catalog:dynamic-action",
                    )),
                    replacement: None,
                    reason: None,
                },
                HookPolicyDynamicAction {
                    kind: HookPolicyDynamicActionKind::Rewrite,
                    target: Some(HookPolicyDynamicActionTarget::from("message")),
                    replacement: Some(HookPolicyDynamicActionReplacement::from(
                        "cargo test --locked",
                    )),
                    reason: None,
                },
            ],
        }));
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let tokio = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("create benchmark tokio runtime");

    c.bench_function(
        "hook_dynamic_actions_register_rewrite_dispatch",
        |bencher| {
            bencher.iter(|| {
                let report = tokio.block_on(
                    dispatcher.dispatch(
                        &runtime,
                        HookInvocation::new(HookEventName::PreToolUse)
                            .with_agent_scope(HookAgentScope::CustomerAgent)
                            .with_message("cargo test"),
                    ),
                );
                std::hint::black_box(report);
            });
        },
    );
}

criterion_group!(benches, bench_dynamic_action_dispatch);
criterion_main!(benches);
