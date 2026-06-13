use marlin_agent_protocol::{
    HookAgentScope, HookDecisionContext, HookDispatchPolicyReceipt, HookDispatchPolicyReceiptInput,
    HookDispatchSelectionInput, HookDispatchSelectionReceipt, HookEventName, HookHandlerType,
    HookMatcherStrategy, HookMatcherToken, HookOutputEntry, HookOutputEntryKind,
    HookPolicyDecision, HookPolicyDecisionReason, HookPolicyDecisionReceipt, HookPolicyExtension,
    HookPolicyExtensionKind, HookPolicyMode, HookRunStatus, HookRunSummary, HookScope,
    HookSelectedCandidateInput, HookSelectionCandidateReceipt, HookSelectionSkipReason,
    HookSkippedCandidateInput, HookSource, HookTrustStatus,
};

fn sample_decision_context() -> HookDecisionContext {
    HookDecisionContext::new()
        .with_session_id("session-1")
        .with_agent_lineage_node("root-agent")
        .with_workspace_state("dirty-worktree=false")
        .with_org_memory_hit("hook-policy")
        .with_agent_class("sub-agent")
}

#[test]
fn hook_run_summary_tracks_status_and_output_entries() {
    let summary = HookRunSummary::running(
        "hook-1",
        HookEventName::SubAgentStart,
        HookHandlerType::Agent,
    )
    .with_entry(HookOutputEntry::new(
        HookOutputEntryKind::Context,
        "spawn reviewer",
    ))
    .completed();

    assert_eq!(summary.id.as_str(), "hook-1");
    assert_eq!(summary.event_name, HookEventName::SubAgentStart);
    assert_eq!(summary.handler_type, HookHandlerType::Agent);
    assert_eq!(summary.status, HookRunStatus::Completed);
    assert_eq!(summary.entries.len(), 1);
    assert_eq!(summary.entries[0].kind, HookOutputEntryKind::Context);
}

#[test]
fn hook_dispatch_selection_receipt_records_candidates_and_strategy() {
    let receipt = HookDispatchSelectionReceipt::new(HookDispatchSelectionInput {
        event_name: HookEventName::PreToolUse,
        invocation_agent_scope: HookAgentScope::SubAgent,
        decision_context: sample_decision_context(),
        matcher_strategy: HookMatcherStrategy::AhoCorasickEventIndex,
        matched_tokens: vec![HookMatcherToken::new("|PreToolUse|")],
        candidates: vec![
            HookSelectionCandidateReceipt::selected(HookSelectedCandidateInput {
                hook_id: "trusted-command".into(),
                event_name: HookEventName::PreToolUse,
                registration_agent_scope: HookAgentScope::SubAgent,
                invocation_agent_scope: HookAgentScope::SubAgent,
            }),
            HookSelectionCandidateReceipt::skipped(HookSkippedCandidateInput {
                hook_id: "post-tool".into(),
                event_name: HookEventName::PostToolUse,
                registration_agent_scope: HookAgentScope::RootAgent,
                invocation_agent_scope: HookAgentScope::SubAgent,
                skip_reason: HookSelectionSkipReason::EventMismatch,
            }),
        ],
    });

    assert_eq!(receipt.candidate_count, 2);
    assert_eq!(receipt.selected_count, 1);
    assert_eq!(
        receipt
            .decision_context
            .session_id
            .as_ref()
            .expect("session context")
            .as_str(),
        "session-1"
    );
    assert_eq!(receipt.matched_tokens[0].as_str(), "|PreToolUse|");
    assert_eq!(
        receipt.candidates[1].skip_reason,
        Some(HookSelectionSkipReason::EventMismatch)
    );
}

#[test]
fn hook_dispatch_policy_receipt_counts_allowed_and_rejected_decisions() {
    let receipt = HookDispatchPolicyReceipt::new(HookDispatchPolicyReceiptInput {
        event_name: HookEventName::PreToolUse,
        invocation_agent_scope: HookAgentScope::CustomerAgent,
        decision_context: sample_decision_context(),
        mode: HookPolicyMode::EnforceTrusted,
        extension: HookPolicyExtension::gerbil_scheme("marlin/hooks/policy", "decide-hook-policy"),
        actions: Vec::new(),
        decisions: vec![
            HookPolicyDecisionReceipt {
                hook_id: "trusted".into(),
                event_name: HookEventName::PreToolUse,
                handler_type: HookHandlerType::Command,
                scope: HookScope::Turn,
                agent_scope: HookAgentScope::SubAgent,
                source_path: None,
                source: HookSource::User,
                trust: HookTrustStatus::Trusted,
                decision: HookPolicyDecision::Allowed,
                reason: HookPolicyDecisionReason::TrustedSource,
            },
            HookPolicyDecisionReceipt {
                hook_id: "untrusted".into(),
                event_name: HookEventName::PreToolUse,
                handler_type: HookHandlerType::Command,
                scope: HookScope::Turn,
                agent_scope: HookAgentScope::CustomerAgent,
                source_path: None,
                source: HookSource::Project,
                trust: HookTrustStatus::Untrusted,
                decision: HookPolicyDecision::Rejected,
                reason: HookPolicyDecisionReason::UntrustedRejected,
            },
        ],
    });

    assert_eq!(receipt.evaluated_count, 2);
    assert_eq!(
        receipt.invocation_agent_scope,
        HookAgentScope::CustomerAgent
    );
    assert_eq!(
        receipt
            .decision_context
            .agent_class
            .as_ref()
            .expect("agent class")
            .as_str(),
        "sub-agent"
    );
    assert_eq!(receipt.allowed_count, 1);
    assert_eq!(receipt.rejected_count, 1);
    assert_eq!(
        receipt.extension.kind,
        HookPolicyExtensionKind::GerbilScheme
    );
    assert!(!receipt.is_success());
}
