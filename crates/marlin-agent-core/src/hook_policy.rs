//! Core-level bridge between hook dispatch policy and `Gerbil Scheme`.

use marlin_agent_hooks::{HookDispatchPolicyFinalizer, HookDispatchPolicyFinalizerInput};
use marlin_agent_protocol::{
    HookDispatchPolicyReceipt, HookDispatchPolicyReceiptInput, HookPolicyDecision,
    HookPolicyDecisionReason,
};
use marlin_gerbil_scheme::{
    GerbilHookPolicyCommandEvaluator, GerbilHookPolicyInvocationInput,
    GerbilHookPolicyRuntimeBinding,
};
use std::{io, path::PathBuf};

/// Hook policy finalizer backed by a `Gerbil Scheme` command evaluator.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilHookPolicyFinalizer {
    evaluator: GerbilHookPolicyCommandEvaluator,
}

impl GerbilHookPolicyFinalizer {
    pub fn new(evaluator: GerbilHookPolicyCommandEvaluator) -> Self {
        Self { evaluator }
    }

    pub fn evaluator(&self) -> &GerbilHookPolicyCommandEvaluator {
        &self.evaluator
    }

    pub fn into_evaluator(self) -> GerbilHookPolicyCommandEvaluator {
        self.evaluator
    }

    pub fn from_runtime_binding(binding: GerbilHookPolicyRuntimeBinding) -> Self {
        Self::new(binding.into_evaluator())
    }

    pub fn from_default_gxi(loadpath_root: impl Into<PathBuf>) -> io::Result<Self> {
        GerbilHookPolicyRuntimeBinding::from_default_gxi(loadpath_root)
            .map(Self::from_runtime_binding)
    }
}

impl HookDispatchPolicyFinalizer for GerbilHookPolicyFinalizer {
    fn finalize(&self, input: HookDispatchPolicyFinalizerInput) -> HookDispatchPolicyReceipt {
        let policy_receipt = input.policy_receipt;
        let evaluation = self.evaluator.evaluate(GerbilHookPolicyInvocationInput {
            extension: policy_receipt.extension.clone(),
            event_name: input.invocation.event_name,
            agent_scope: input.invocation.agent_scope,
            policy_receipt: policy_receipt.clone(),
        });

        match evaluation {
            Ok(receipt) if receipt.is_allowed() => policy_receipt,
            Ok(_) | Err(_) => reject_policy_receipt(policy_receipt),
        }
    }
}

fn reject_policy_receipt(policy_receipt: HookDispatchPolicyReceipt) -> HookDispatchPolicyReceipt {
    let decisions = policy_receipt
        .decisions
        .into_iter()
        .map(|mut decision| {
            decision.decision = HookPolicyDecision::Rejected;
            decision.reason = HookPolicyDecisionReason::ExtensionRejected;
            decision
        })
        .collect();

    HookDispatchPolicyReceipt::new(HookDispatchPolicyReceiptInput {
        event_name: policy_receipt.event_name,
        invocation_agent_scope: policy_receipt.invocation_agent_scope,
        mode: policy_receipt.mode,
        extension: policy_receipt.extension,
        decisions,
    })
}

impl From<GerbilHookPolicyCommandEvaluator> for GerbilHookPolicyFinalizer {
    fn from(evaluator: GerbilHookPolicyCommandEvaluator) -> Self {
        Self::new(evaluator)
    }
}

impl From<GerbilHookPolicyRuntimeBinding> for GerbilHookPolicyFinalizer {
    fn from(binding: GerbilHookPolicyRuntimeBinding) -> Self {
        Self::from_runtime_binding(binding)
    }
}
