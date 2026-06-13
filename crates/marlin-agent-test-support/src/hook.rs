//! Reusable hook fixtures for custom sub-agent runtime tests.

use marlin_agent_protocol::{
    HookAgentScope, HookDecisionContext, HookDispatchPolicyReceipt, HookDispatchPolicyReceiptInput,
    HookDispatchSelectionInput, HookDispatchSelectionReceipt, HookEventName, HookHandlerType,
    HookMatcherStrategy, HookMatcherToken, HookOutputEntry, HookOutputEntryKind,
    HookPolicyDecision, HookPolicyDecisionReason, HookPolicyDecisionReceipt,
    HookPolicyDynamicAction, HookPolicyDynamicActionKind, HookPolicyExtension,
    HookPolicyExtensionKind, HookPolicyMode, HookRunId, HookRunStatus, HookRunSummary, HookScope,
    HookSelectedCandidateInput, HookSelectionCandidateReceipt, HookSelectionSkipReason,
    HookSkippedCandidateInput, HookSource, HookSourcePath, HookTrustStatus, LoopEvidence,
    LoopEvidenceKind,
};

/// Hook run summary fixture for a configured custom sub-agent startup hook.
pub fn custom_sub_agent_start_hook_summary_fixture() -> HookRunSummary {
    let mut summary = HookRunSummary::running(
        "custom-sub-agent-start",
        HookEventName::SubAgentStart,
        HookHandlerType::Agent,
    )
    .with_entry(HookOutputEntry::new(
        HookOutputEntryKind::Context,
        "spawn reviewer",
    ))
    .completed();
    summary.agent_scope = HookAgentScope::SubAgent;
    summary.source_path = Some(HookSourcePath::new(
        "test-home/root/sub/reviewer/hooks/start.ss",
    ));
    summary.source = HookSource::Project;
    summary.trust = HookTrustStatus::Trusted;
    summary
}

/// Assert the configured custom sub-agent startup hook summary fixture.
pub fn assert_custom_sub_agent_start_hook_summary(summary: &HookRunSummary) {
    assert_eq!(summary.id.as_str(), "custom-sub-agent-start");
    assert_eq!(summary.event_name, HookEventName::SubAgentStart);
    assert_eq!(summary.handler_type, HookHandlerType::Agent);
    assert_eq!(summary.scope, HookScope::Turn);
    assert_eq!(summary.agent_scope, HookAgentScope::SubAgent);
    assert_eq!(
        summary
            .source_path
            .as_ref()
            .expect("hook summary should record source path")
            .as_str(),
        "test-home/root/sub/reviewer/hooks/start.ss",
    );
    assert_eq!(summary.source, HookSource::Project);
    assert_eq!(summary.trust, HookTrustStatus::Trusted);
    assert_eq!(summary.status, HookRunStatus::Completed);
    assert_eq!(summary.entries.len(), 1);
    assert_eq!(summary.entries[0].kind, HookOutputEntryKind::Context);
    assert_eq!(summary.entries[0].text, "spawn reviewer");
}

/// Selection receipt fixture proving sub-agent hook dispatch keeps scoped hooks.
pub fn sub_agent_hook_dispatch_selection_fixture() -> HookDispatchSelectionReceipt {
    HookDispatchSelectionReceipt::new(HookDispatchSelectionInput {
        event_name: HookEventName::PreToolUse,
        invocation_agent_scope: HookAgentScope::SubAgent,
        decision_context: HookDecisionContext::default(),
        matcher_strategy: HookMatcherStrategy::AhoCorasickEventIndex,
        matched_tokens: vec![HookMatcherToken::new("|PreToolUse|")],
        candidates: vec![
            HookSelectionCandidateReceipt::selected(HookSelectedCandidateInput {
                hook_id: HookRunId::new("trusted-sub-agent-command"),
                event_name: HookEventName::PreToolUse,
                registration_agent_scope: HookAgentScope::SubAgent,
                invocation_agent_scope: HookAgentScope::SubAgent,
            }),
            HookSelectionCandidateReceipt::skipped(HookSkippedCandidateInput {
                hook_id: HookRunId::new("root-only-stop"),
                event_name: HookEventName::Stop,
                registration_agent_scope: HookAgentScope::RootAgent,
                invocation_agent_scope: HookAgentScope::SubAgent,
                skip_reason: HookSelectionSkipReason::EventMismatch,
            }),
        ],
    })
}

/// Assert the sub-agent hook dispatch selection fixture.
pub fn assert_sub_agent_hook_dispatch_selection(receipt: &HookDispatchSelectionReceipt) {
    assert_eq!(receipt.event_name, HookEventName::PreToolUse);
    assert_eq!(receipt.invocation_agent_scope, HookAgentScope::SubAgent);
    assert_eq!(
        receipt.matcher_strategy,
        HookMatcherStrategy::AhoCorasickEventIndex,
    );
    assert_eq!(receipt.matched_tokens.len(), 1);
    assert_eq!(receipt.matched_tokens[0].as_str(), "|PreToolUse|");
    assert_eq!(receipt.candidate_count, 2);
    assert_eq!(receipt.selected_count, 1);
    assert!(receipt.candidates[0].selected);
    assert_eq!(
        receipt.candidates[0].hook_id.as_str(),
        "trusted-sub-agent-command",
    );
    assert!(!receipt.candidates[1].selected);
    assert_eq!(
        receipt.candidates[1].skip_reason,
        Some(HookSelectionSkipReason::EventMismatch),
    );
}

/// Policy receipt fixture for custom hook enforcement through the extension plane.
pub fn custom_hook_policy_receipt_fixture() -> HookDispatchPolicyReceipt {
    HookDispatchPolicyReceipt::new(HookDispatchPolicyReceiptInput {
        event_name: HookEventName::PreToolUse,
        invocation_agent_scope: HookAgentScope::CustomAgent,
        decision_context: HookDecisionContext::default(),
        mode: HookPolicyMode::EnforceTrusted,
        extension: HookPolicyExtension::gerbil_scheme("marlin/hooks/policy", "decide-hook-policy"),
        actions: Vec::new(),
        decisions: vec![
            HookPolicyDecisionReceipt {
                hook_id: HookRunId::new("trusted-custom-command"),
                event_name: HookEventName::PreToolUse,
                handler_type: HookHandlerType::Command,
                scope: HookScope::Turn,
                agent_scope: HookAgentScope::CustomAgent,
                source_path: Some(HookSourcePath::new("test-home/root/hooks/trusted.ss")),
                source: HookSource::User,
                trust: HookTrustStatus::Trusted,
                decision: HookPolicyDecision::Allowed,
                reason: HookPolicyDecisionReason::TrustedSource,
            },
            HookPolicyDecisionReceipt {
                hook_id: HookRunId::new("untrusted-custom-command"),
                event_name: HookEventName::PreToolUse,
                handler_type: HookHandlerType::Command,
                scope: HookScope::Turn,
                agent_scope: HookAgentScope::CustomAgent,
                source_path: Some(HookSourcePath::new(
                    "test-workspace/root/.marlin/hooks/untrusted.ss",
                )),
                source: HookSource::Project,
                trust: HookTrustStatus::Untrusted,
                decision: HookPolicyDecision::Rejected,
                reason: HookPolicyDecisionReason::UntrustedRejected,
            },
        ],
    })
}

/// Decision context fixture that triggers the complex Gerbil hook policy sample.
pub fn complex_gerbil_hook_policy_decision_context_fixture() -> HookDecisionContext {
    HookDecisionContext::new()
        .with_session_id("cheap-test-session")
        .with_agent_lineage_node("release")
        .with_workspace_state("dirty")
        .with_org_memory_hit("needs-human-review")
        .with_agent_class("customer-agent")
}

/// Policy receipt fixture for a complex Gerbil hook policy dynamic action set.
pub fn complex_gerbil_hook_policy_receipt_fixture() -> HookDispatchPolicyReceipt {
    complex_gerbil_hook_policy_receipt_with_decision_context(
        complex_gerbil_hook_policy_decision_context_fixture(),
    )
}

/// Policy receipt fixture for a complex Gerbil hook policy using supplied runtime context facts.
pub fn complex_gerbil_hook_policy_receipt_with_decision_context(
    decision_context: HookDecisionContext,
) -> HookDispatchPolicyReceipt {
    HookDispatchPolicyReceipt::new(HookDispatchPolicyReceiptInput {
        event_name: HookEventName::PreToolUse,
        invocation_agent_scope: HookAgentScope::CustomerAgent,
        decision_context,
        mode: HookPolicyMode::ObserveOnly,
        extension: HookPolicyExtension::gerbil_scheme(
            "marlin/hooks/policy-samples",
            "decide-hook-policy-sample",
        ),
        actions: vec![
            dynamic_hook_action(
                HookPolicyDynamicActionKind::Register,
                Some("catalog:customer-agent-hook"),
                None,
                Some("customer agent session requires runtime catalog hook"),
            ),
            dynamic_hook_action(
                HookPolicyDynamicActionKind::Defer,
                Some("session:release"),
                None,
                Some("release lineage waits for org memory review"),
            ),
            dynamic_hook_action(
                HookPolicyDynamicActionKind::Deny,
                Some("dangerous-shell"),
                None,
                Some("dirty workspace blocks dangerous shell hook"),
            ),
            dynamic_hook_action(
                HookPolicyDynamicActionKind::Rewrite,
                Some("command"),
                Some("cargo test --locked"),
                Some("session policy prefers locked tests"),
            ),
        ],
        decisions: vec![HookPolicyDecisionReceipt {
            hook_id: HookRunId::new("gerbil-sample-extension-policy"),
            event_name: HookEventName::PreToolUse,
            handler_type: HookHandlerType::Command,
            scope: HookScope::Turn,
            agent_scope: HookAgentScope::CustomerAgent,
            source_path: Some(HookSourcePath::new(
                "gerbil/src/marlin/hooks/policy-samples.ss",
            )),
            source: HookSource::Project,
            trust: HookTrustStatus::Trusted,
            decision: HookPolicyDecision::Allowed,
            reason: HookPolicyDecisionReason::ExtensionAllowed,
        }],
    })
}

/// Assert the custom hook policy receipt fixture.
pub fn assert_custom_hook_policy_receipt(receipt: &HookDispatchPolicyReceipt) {
    assert_custom_hook_policy_header(receipt);
    assert_custom_hook_policy_extension(receipt);
    assert_custom_hook_policy_counts(receipt);
    assert_custom_hook_policy_decisions(receipt);
}

/// Assert the complex Gerbil hook policy receipt fixture.
pub fn assert_complex_gerbil_hook_policy_receipt(receipt: &HookDispatchPolicyReceipt) {
    assert_complex_gerbil_hook_policy_header(receipt);
    assert_complex_gerbil_hook_policy_extension(receipt);
    assert_complex_gerbil_hook_policy_counts(receipt);
    assert_complex_gerbil_hook_policy_actions(receipt);
}

fn assert_complex_gerbil_hook_policy_header(receipt: &HookDispatchPolicyReceipt) {
    assert_eq!(receipt.event_name, HookEventName::PreToolUse);
    assert_eq!(
        receipt.invocation_agent_scope,
        HookAgentScope::CustomerAgent
    );
    assert_eq!(receipt.mode, HookPolicyMode::ObserveOnly);
}

fn assert_complex_gerbil_hook_policy_extension(receipt: &HookDispatchPolicyReceipt) {
    assert_eq!(
        receipt.extension.kind,
        HookPolicyExtensionKind::GerbilScheme
    );
    assert_eq!(
        receipt
            .extension
            .module
            .as_ref()
            .expect("complex hook policy should record Gerbil module")
            .as_str(),
        "marlin/hooks/policy-samples",
    );
    assert_eq!(
        receipt
            .extension
            .procedure
            .as_ref()
            .expect("complex hook policy should record Gerbil procedure")
            .as_str(),
        "decide-hook-policy-sample",
    );
}

fn assert_complex_gerbil_hook_policy_counts(receipt: &HookDispatchPolicyReceipt) {
    assert_eq!(receipt.evaluated_count, 1);
    assert_eq!(receipt.allowed_count, 1);
    assert_eq!(receipt.rejected_count, 0);
    assert!(receipt.is_success());
}

fn assert_complex_gerbil_hook_policy_actions(receipt: &HookDispatchPolicyReceipt) {
    assert_eq!(receipt.actions.len(), 4);
    assert_eq!(
        receipt.actions[0].kind,
        HookPolicyDynamicActionKind::Register
    );
    assert_eq!(receipt.actions[1].kind, HookPolicyDynamicActionKind::Defer);
    assert_eq!(receipt.actions[2].kind, HookPolicyDynamicActionKind::Deny);
    assert_eq!(
        receipt.actions[3].kind,
        HookPolicyDynamicActionKind::Rewrite
    );
    assert_eq!(
        receipt.actions[3]
            .replacement
            .as_ref()
            .map(|replacement| replacement.as_str()),
        Some("cargo test --locked"),
    );
}

/// Project hook replay receipts into runtime evidence consumed by harness tests.
pub fn hook_dispatch_replay_evidence(
    summary: &HookRunSummary,
    selection: &HookDispatchSelectionReceipt,
    policy: &HookDispatchPolicyReceipt,
) -> LoopEvidence {
    let rejected_decisions = policy
        .decisions
        .iter()
        .filter(|decision| decision.decision == HookPolicyDecision::Rejected)
        .count();
    let action_kinds = policy
        .actions
        .iter()
        .map(|action| format!("{:?}", action.kind))
        .collect::<Vec<_>>()
        .join("|");
    let decision_context = &policy.decision_context;
    let context_session_id = decision_context
        .session_id
        .as_ref()
        .map(|session_id| session_id.as_str())
        .unwrap_or("none");
    let context_agent_lineage = join_detail_values(
        decision_context
            .agent_lineage
            .iter()
            .map(|node| node.as_str()),
    );
    let context_workspace_state = join_detail_values(
        decision_context
            .workspace_state
            .iter()
            .map(|fact| fact.as_str()),
    );
    let context_org_memory_hits = join_detail_values(
        decision_context
            .org_memory_hits
            .iter()
            .map(|hit| hit.as_str()),
    );
    let context_agent_class = decision_context
        .agent_class
        .as_ref()
        .map(|agent_class| agent_class.as_str())
        .unwrap_or("none");
    let action_targets = join_detail_values(policy.actions.iter().map(|action| {
        action
            .target
            .as_ref()
            .map(|target| target.as_str())
            .unwrap_or("none")
    }));
    let action_replacements = join_detail_values(policy.actions.iter().map(|action| {
        action
            .replacement
            .as_ref()
            .map(|replacement| replacement.as_str())
            .unwrap_or("none")
    }));
    let action_reasons = join_detail_values(policy.actions.iter().map(|action| {
        action
            .reason
            .as_ref()
            .map(|reason| reason.as_str())
            .unwrap_or("none")
    }));
    let detail = format!(
        "hook_id={} event={:?} run_status={:?} selected_count={} candidate_count={} matcher_strategy={:?} matched_token_count={} policy_decisions={} policy_mode={:?} policy_extension_kind={:?} allowed_decisions={} rejected_decisions={} policy_action_count={} policy_action_kinds={} policy_action_targets={} policy_action_replacements={} policy_action_reasons={} context_session_id={} context_agent_lineage={} context_workspace_state={} context_org_memory_hits={} context_agent_class={} summary_agent_scope={:?} selection_agent_scope={:?} policy_agent_scope={:?} live_llm=false",
        summary.id.as_str(),
        summary.event_name,
        summary.status,
        selection.selected_count,
        selection.candidate_count,
        selection.matcher_strategy,
        selection.matched_tokens.len(),
        policy.decisions.len(),
        policy.mode,
        policy.extension.kind,
        policy.allowed_count,
        rejected_decisions,
        policy.actions.len(),
        action_kinds,
        action_targets,
        action_replacements,
        action_reasons,
        context_session_id,
        context_agent_lineage,
        context_workspace_state,
        context_org_memory_hits,
        context_agent_class,
        summary.agent_scope,
        selection.invocation_agent_scope,
        policy.invocation_agent_scope,
    );

    LoopEvidence::present(
        LoopEvidenceKind::Runtime,
        format!("hook-dispatch-replay:{}", summary.id.as_str()),
    )
    .with_detail(detail)
}

fn join_detail_values<'a>(values: impl IntoIterator<Item = &'a str>) -> String {
    let values = values.into_iter().collect::<Vec<_>>();
    if values.is_empty() {
        "none".to_owned()
    } else {
        values.join("|")
    }
}

fn assert_custom_hook_policy_header(receipt: &HookDispatchPolicyReceipt) {
    assert_eq!(receipt.event_name, HookEventName::PreToolUse);
    assert_eq!(receipt.invocation_agent_scope, HookAgentScope::CustomAgent);
    assert_eq!(receipt.mode, HookPolicyMode::EnforceTrusted);
}

fn assert_custom_hook_policy_extension(receipt: &HookDispatchPolicyReceipt) {
    assert_eq!(
        receipt.extension.kind,
        HookPolicyExtensionKind::GerbilScheme
    );
    assert_eq!(
        receipt
            .extension
            .module
            .as_ref()
            .expect("custom hook policy should record Gerbil module")
            .as_str(),
        "marlin/hooks/policy",
    );
    assert_eq!(
        receipt
            .extension
            .procedure
            .as_ref()
            .expect("custom hook policy should record Gerbil procedure")
            .as_str(),
        "decide-hook-policy",
    );
}

fn assert_custom_hook_policy_counts(receipt: &HookDispatchPolicyReceipt) {
    assert_eq!(receipt.evaluated_count, 2);
    assert_eq!(receipt.allowed_count, 1);
    assert_eq!(receipt.rejected_count, 1);
    assert!(!receipt.is_success());
}

fn dynamic_hook_action(
    kind: HookPolicyDynamicActionKind,
    target: Option<&str>,
    replacement: Option<&str>,
    reason: Option<&str>,
) -> HookPolicyDynamicAction {
    HookPolicyDynamicAction {
        kind,
        target: target.map(Into::into),
        replacement: replacement.map(Into::into),
        reason: reason.map(Into::into),
    }
}

fn assert_custom_hook_policy_decisions(receipt: &HookDispatchPolicyReceipt) {
    assert_eq!(receipt.decisions[0].decision, HookPolicyDecision::Allowed);
    assert_eq!(
        receipt.decisions[0].reason,
        HookPolicyDecisionReason::TrustedSource,
    );
    assert_eq!(receipt.decisions[1].decision, HookPolicyDecision::Rejected);
    assert_eq!(
        receipt.decisions[1].reason,
        HookPolicyDecisionReason::UntrustedRejected,
    );
}
