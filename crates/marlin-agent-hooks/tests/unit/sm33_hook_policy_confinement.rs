// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! SM33 (ADV-L4) hostile runtime exploit suite for `marlin-agent-hooks`.
//!
//! Proves LIVE RED -> LIVE GREEN across 3 real hook security vulnerabilities:
//! 1. Silent fail-open downgrade to `ObserveOnly` on unknown/misspelled TOML keys,
//!    unvalidated `GerbilScheme` extension paths, and unbounded `from_toml_path`.
//! 2. Duplicate `hook_id` ghost-shadow persistence after `set_enabled(false)` / `unregister`
//!    plus unbounded concurrent `HookExecutionMode::Async` hook fan-out (>16).
//! 3. Extension `Defer`/`Deny` erasing Rust `UntrustedRejected` trust-floor audit reasons
//!    plus unbounded dynamic action flood & oversize/NUL `Rewrite` payloads.

use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use marlin_agent_hooks::{
    HookActionContext, HookActionProvider, HookConfigurationEnvelope, HookDispatchPolicy,
    HookDispatcher, HookInvocation, HookRegistration, HookRegistry,
};
use marlin_agent_protocol::{
    HookEventName, HookExecutionMode, HookHandlerType, HookPolicyDecision,
    HookPolicyDecisionReason, HookPolicyDynamicAction, HookPolicyDynamicActionKind,
    HookPolicyDynamicActionReplacement, HookPolicyDynamicActionTarget, HookRunId, HookRunSummary,
    HookTrustStatus,
};
use marlin_agent_runtime::{HookRuntime, RuntimeContext, RuntimeFuture, TokioAgentRuntime};

struct CountingHook {
    label: &'static str,
    invocations: Arc<AtomicUsize>,
}

impl HookRuntime for CountingHook {
    type Request = HookInvocation;
    type Output = HookRunSummary;

    fn run_hook(
        &self,
        request: Self::Request,
        _context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output> {
        let invocations = Arc::clone(&self.invocations);
        let label = self.label;
        Box::pin(async move {
            invocations.fetch_add(1, Ordering::SeqCst);
            HookRunSummary::running(label, request.event_name, HookHandlerType::Command).completed()
        })
    }
}

struct ConcurrencyProbeHook {
    active: Arc<AtomicUsize>,
    peak: Arc<AtomicUsize>,
}

impl HookRuntime for ConcurrencyProbeHook {
    type Request = HookInvocation;
    type Output = HookRunSummary;

    fn run_hook(
        &self,
        request: Self::Request,
        _context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output> {
        let active = Arc::clone(&self.active);
        let peak = Arc::clone(&self.peak);
        Box::pin(async move {
            let now = active.fetch_add(1, Ordering::SeqCst) + 1;
            peak.fetch_max(now, Ordering::SeqCst);
            tokio::time::sleep(Duration::from_millis(25)).await;
            active.fetch_sub(1, Ordering::SeqCst);
            HookRunSummary::running("probe", request.event_name, HookHandlerType::Command)
                .completed()
        })
    }
}

struct StaticActionsProvider(Vec<HookPolicyDynamicAction>);

impl HookActionProvider for StaticActionsProvider {
    fn propose_actions(&self, _input: HookActionContext<'_>) -> Vec<HookPolicyDynamicAction> {
        self.0.clone()
    }
}

#[test]
fn sm33_v1_hook_toml_rejects_unknown_fields_malformed_extension_and_oversize_file() {
    // Typo `[polcy]` must NOT silently fall back to `ObserveOnly`.
    let typo_res = HookConfigurationEnvelope::from_toml_str(
        r#"
[polcy]
mode = "EnforceTrusted"
"#,
    );
    assert!(
        typo_res.is_err(),
        "SM33-V1: unknown top-level section `[polcy]` must fail closed instead of defaulting to ObserveOnly, got {typo_res:?}"
    );

    let unknown_policy_key = HookConfigurationEnvelope::from_toml_str(
        r#"
[policy]
enforce_trusted = true
"#,
    );
    assert!(
        unknown_policy_key.is_err(),
        "SM33-V1: unknown policy key must fail closed instead of defaulting to ObserveOnly"
    );

    // GerbilScheme extension with empty or traversal module must fail closed.
    for hostile_ext in [
        r#"
[policy]
mode = "EnforceTrusted"
[policy.extension]
kind = "GerbilScheme"
module = ""
procedure = "decide"
"#,
        r#"
[policy]
mode = "EnforceTrusted"
[policy.extension]
kind = "GerbilScheme"
module = "../../../etc/evil"
procedure = "decide"
"#,
        r#"
[policy]
mode = "EnforceTrusted"
[policy.extension]
kind = "GerbilScheme"
module = "marlin/hooks/policy"
"#,
    ] {
        assert!(
            HookConfigurationEnvelope::from_toml_str(hostile_ext).is_err(),
            "SM33-V1: malformed GerbilScheme extension must be refused:\n{hostile_ext}"
        );
    }

    // Oversize config file (>256 KiB) must be refused without unbounded read_to_string.
    let big_file = std::env::temp_dir().join(format!(
        "marlin-sm33-big-hooks-{}.toml",
        std::process::id()
    ));
    let padding = "#".repeat(300 * 1024);
    std::fs::write(&big_file, format!("{padding}\n")).expect("write big config");
    let big_res = HookConfigurationEnvelope::from_toml_path(&big_file);
    let _ = std::fs::remove_file(&big_file);
    assert!(
        big_res.is_err(),
        "SM33-V1: >256 KiB hook TOML config file must be refused"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sm33_v2_duplicate_hook_id_cannot_leave_ghost_shadow_and_async_fanout_is_bounded() {
    let original_hits = Arc::new(AtomicUsize::new(0));
    let shadow_hits = Arc::new(AtomicUsize::new(0));

    let mut registry = HookRegistry::new();
    registry.register(
        HookRegistration::new(
            "audit-hook",
            HookEventName::PreToolUse,
            HookHandlerType::Command,
            Arc::new(CountingHook {
                label: "original",
                invocations: Arc::clone(&original_hits),
            }),
        )
        .with_trust(HookTrustStatus::Trusted),
    );
    registry.register(
        HookRegistration::new(
            "audit-hook",
            HookEventName::PreToolUse,
            HookHandlerType::Command,
            Arc::new(CountingHook {
                label: "shadow",
                invocations: Arc::clone(&shadow_hits),
            }),
        )
        .with_trust(HookTrustStatus::Trusted),
    );

    assert_eq!(
        registry.registrations().len(),
        1,
        "SM33-V2: HookRegistry::register must upsert unique hook_id without duplicate shadows"
    );

    // Disabling by hook_id must stop ALL executions of that hook_id.
    registry
        .set_enabled(&HookRunId::new("audit-hook"), false, None)
        .expect("disable audit-hook");
    let (runtime, _events) = TokioAgentRuntime::new(16);
    let report = HookDispatcher::new(registry.clone())
        .with_policy(HookDispatchPolicy::enforce_trusted())
        .dispatch(&runtime, HookInvocation::new(HookEventName::PreToolUse))
        .await;
    assert_eq!(
        original_hits.load(Ordering::SeqCst) + shadow_hits.load(Ordering::SeqCst),
        0,
        "SM33-V2: disabled hook_id must not execute via duplicate ghost shadow! runs={:?}",
        report.runs
    );

    // Unregistering must leave zero copies.
    registry
        .unregister(&HookRunId::new("audit-hook"), None)
        .expect("unregister audit-hook");
    assert!(
        registry.registrations().is_empty(),
        "SM33-V2: unregister must remove all registrations for hook_id"
    );

    // Unbounded async hook fan-out must be bounded (<= 16 concurrent).
    let active = Arc::new(AtomicUsize::new(0));
    let peak = Arc::new(AtomicUsize::new(0));
    let mut fanout_registry = HookRegistry::new();
    for idx in 0..48 {
        fanout_registry.register(
            HookRegistration::new(
                format!("async-hook-{idx}"),
                HookEventName::PreToolUse,
                HookHandlerType::Command,
                Arc::new(ConcurrencyProbeHook {
                    active: Arc::clone(&active),
                    peak: Arc::clone(&peak),
                }),
            )
            .with_execution_mode(HookExecutionMode::Async)
            .with_trust(HookTrustStatus::Trusted),
        );
    }
    let fanout_report = HookDispatcher::new(fanout_registry)
        .with_policy(HookDispatchPolicy::enforce_trusted())
        .dispatch(&runtime, HookInvocation::new(HookEventName::PreToolUse))
        .await;
    assert_eq!(fanout_report.runs.len(), 48);
    assert!(
        peak.load(Ordering::SeqCst) <= 16,
        "SM33-V2: concurrent async hook fan-out peaked at {} (> 16 semaphore cap)",
        peak.load(Ordering::SeqCst)
    );
}

#[tokio::test]
async fn sm33_v3_extension_actions_cannot_mask_trust_rejection_or_flood_rewrites() {
    let untrusted_hits = Arc::new(AtomicUsize::new(0));
    let registry = HookRegistry::new().with_registration(
        HookRegistration::new(
            "rogue-hook",
            HookEventName::PreToolUse,
            HookHandlerType::Command,
            Arc::new(CountingHook {
                label: "rogue",
                invocations: Arc::clone(&untrusted_hits),
            }),
        )
        .with_trust(HookTrustStatus::Untrusted),
    );

    // Hostile extension tries to mask UntrustedRejected via Defer AND inject a NUL / oversize Rewrite.
    let mut actions = vec![
        HookPolicyDynamicAction {
            kind: HookPolicyDynamicActionKind::Defer,
            target: None,
            replacement: None,
            reason: None,
        },
        HookPolicyDynamicAction {
            kind: HookPolicyDynamicActionKind::Rewrite,
            target: Some(HookPolicyDynamicActionTarget::from("message")),
            replacement: Some(HookPolicyDynamicActionReplacement::from("evil\0payload")),
            reason: None,
        },
    ];
    // Append a flood of 200 extra actions beyond the cap.
    for idx in 0..200 {
        actions.push(HookPolicyDynamicAction {
            kind: HookPolicyDynamicActionKind::Deny,
            target: Some(HookPolicyDynamicActionTarget::from(format!("missing-{idx}"))),
            replacement: None,
            reason: None,
        });
    }

    let (runtime, _events) = TokioAgentRuntime::new(8);
    let report = HookDispatcher::new(registry)
        .with_policy(HookDispatchPolicy::enforce_trusted())
        .with_action_provider(Arc::new(StaticActionsProvider(actions)))
        .dispatch(
            &runtime,
            HookInvocation::new(HookEventName::PreToolUse).with_message("clean-original"),
        )
        .await;

    assert_eq!(untrusted_hits.load(Ordering::SeqCst), 0);
    assert_eq!(report.policy.decisions[0].decision, HookPolicyDecision::Rejected);
    assert_eq!(
        report.policy.decisions[0].reason,
        HookPolicyDecisionReason::UntrustedRejected,
        "SM33-V3: extension Defer/Deny must NEVER overwrite Rust UntrustedRejected trust-floor reason!"
    );
    assert!(
        report.applied_actions.len() <= 64,
        "SM33-V3: dynamic policy action application must be bounded (<= 64), got {}",
        report.applied_actions.len()
    );
}
