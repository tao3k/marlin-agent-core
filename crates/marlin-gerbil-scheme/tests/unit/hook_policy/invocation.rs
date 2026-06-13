use marlin_agent_protocol::{
    HookAgentScope, HookDecisionContext, HookDispatchPolicyReceipt, HookDispatchPolicyReceiptInput,
    HookEventName, HookPolicyExtension, HookPolicyExtensionKind, HookPolicyMode,
};
use marlin_gerbil_scheme::{
    GerbilHookPolicyInvocationError, GerbilHookPolicyInvocationInput,
    build_gerbil_hook_policy_invocation,
};

fn complex_hook_decision_context() -> HookDecisionContext {
    HookDecisionContext::new()
        .with_session_id("cheap-test-session")
        .with_agent_lineage_node("release")
        .with_workspace_state("dirty")
        .with_org_memory_hit("needs-human-review")
        .with_agent_class("customer-agent")
}

#[test]
fn gerbil_hook_policy_invocation_builds_typed_request() {
    let extension = HookPolicyExtension::gerbil_scheme("marlin/hooks/policy", "decide-hook-policy");
    let invocation = build_gerbil_hook_policy_invocation(GerbilHookPolicyInvocationInput {
        extension: extension.clone(),
        event_name: HookEventName::PreToolUse,
        agent_scope: HookAgentScope::CustomerAgent,
        policy_receipt: HookDispatchPolicyReceipt::new(HookDispatchPolicyReceiptInput {
            event_name: HookEventName::PreToolUse,
            invocation_agent_scope: HookAgentScope::CustomerAgent,
            decision_context: HookDecisionContext::default(),
            mode: HookPolicyMode::ObserveOnly,
            extension,
            actions: Vec::new(),
            decisions: Vec::new(),
        }),
    })
    .expect("gerbil hook policy invocation builds");

    assert_eq!(invocation.module.as_str(), "marlin/hooks/policy");
    assert_eq!(invocation.procedure.as_str(), "decide-hook-policy");
    assert!(invocation.request_json.contains("\"CustomerAgent\""));
    assert!(invocation.request_json.contains("\"PreToolUse\""));
}

#[test]
fn gerbil_hook_policy_invocation_serializes_nested_policy_context_for_scheme_policy() {
    let extension = HookPolicyExtension::gerbil_scheme(
        "marlin/hooks/policy-samples",
        "decide-hook-policy-sample",
    );
    let invocation = build_gerbil_hook_policy_invocation(GerbilHookPolicyInvocationInput {
        extension: extension.clone(),
        event_name: HookEventName::PreToolUse,
        agent_scope: HookAgentScope::CustomerAgent,
        policy_receipt: HookDispatchPolicyReceipt::new(HookDispatchPolicyReceiptInput {
            event_name: HookEventName::PreToolUse,
            invocation_agent_scope: HookAgentScope::CustomerAgent,
            decision_context: complex_hook_decision_context(),
            mode: HookPolicyMode::ObserveOnly,
            extension,
            actions: Vec::new(),
            decisions: Vec::new(),
        }),
    })
    .expect("gerbil hook policy invocation builds for complex context");

    assert!(invocation.request_json.contains("\"policy_receipt\""));
    assert!(invocation.request_json.contains("\"decision_context\""));
    assert!(invocation.request_json.contains("\"cheap-test-session\""));
    assert!(invocation.request_json.contains("\"release\""));
    assert!(invocation.request_json.contains("\"dirty\""));
    assert!(invocation.request_json.contains("\"needs-human-review\""));
    assert!(invocation.request_json.contains("\"customer-agent\""));
}

#[test]
fn gerbil_hook_policy_invocation_rejects_non_scheme_extension() {
    let error = build_gerbil_hook_policy_invocation(GerbilHookPolicyInvocationInput {
        extension: HookPolicyExtension::none(),
        event_name: HookEventName::PreToolUse,
        agent_scope: HookAgentScope::SubAgent,
        policy_receipt: HookDispatchPolicyReceipt::new(HookDispatchPolicyReceiptInput {
            event_name: HookEventName::PreToolUse,
            invocation_agent_scope: HookAgentScope::SubAgent,
            decision_context: HookDecisionContext::default(),
            mode: HookPolicyMode::ObserveOnly,
            extension: HookPolicyExtension::none(),
            actions: Vec::new(),
            decisions: Vec::new(),
        }),
    })
    .expect_err("non-scheme extension should fail");

    assert_eq!(
        error,
        GerbilHookPolicyInvocationError::UnsupportedExtension {
            kind: HookPolicyExtensionKind::None
        }
    );
}

#[test]
fn gerbil_hook_policy_invocation_rejects_agent_scope_mismatch() {
    let extension = HookPolicyExtension::gerbil_scheme("marlin/hooks/policy", "decide-hook-policy");
    let error = build_gerbil_hook_policy_invocation(GerbilHookPolicyInvocationInput {
        extension: extension.clone(),
        event_name: HookEventName::PreToolUse,
        agent_scope: HookAgentScope::SubAgent,
        policy_receipt: HookDispatchPolicyReceipt::new(HookDispatchPolicyReceiptInput {
            event_name: HookEventName::PreToolUse,
            invocation_agent_scope: HookAgentScope::CustomerAgent,
            decision_context: HookDecisionContext::default(),
            mode: HookPolicyMode::ObserveOnly,
            extension,
            actions: Vec::new(),
            decisions: Vec::new(),
        }),
    })
    .expect_err("scope mismatch should fail before invoking Gerbil");

    assert_eq!(
        error,
        GerbilHookPolicyInvocationError::AgentScopeMismatch {
            input: HookAgentScope::SubAgent,
            receipt: HookAgentScope::CustomerAgent
        }
    );
}
