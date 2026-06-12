use marlin_agent_protocol::{
    HookAgentScope, HookDispatchPolicyReceipt, HookDispatchPolicyReceiptInput, HookEventName,
    HookPolicyExtension, HookPolicyExtensionKind, HookPolicyMode,
};
use marlin_gerbil_scheme::{
    GerbilHookPolicyInvocationError, GerbilHookPolicyInvocationInput,
    build_gerbil_hook_policy_invocation,
};

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
            mode: HookPolicyMode::ObserveOnly,
            extension,
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
fn gerbil_hook_policy_invocation_rejects_non_scheme_extension() {
    let error = build_gerbil_hook_policy_invocation(GerbilHookPolicyInvocationInput {
        extension: HookPolicyExtension::none(),
        event_name: HookEventName::PreToolUse,
        agent_scope: HookAgentScope::SubAgent,
        policy_receipt: HookDispatchPolicyReceipt::new(HookDispatchPolicyReceiptInput {
            event_name: HookEventName::PreToolUse,
            invocation_agent_scope: HookAgentScope::SubAgent,
            mode: HookPolicyMode::ObserveOnly,
            extension: HookPolicyExtension::none(),
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
            mode: HookPolicyMode::ObserveOnly,
            extension,
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
