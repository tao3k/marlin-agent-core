use marlin_agent_protocol::{
    HookAgentScope, HookDecisionContext, HookDispatchPolicyReceipt, HookDispatchPolicyReceiptInput,
    HookEventName, HookPolicyDecision, HookPolicyDynamicActionKind, HookPolicyExtension,
    HookPolicyExtensionKind, HookPolicyMode,
};
use marlin_gerbil_scheme::{
    GerbilHookPolicyEvaluationDecodeInput, GerbilHookPolicyEvaluationError,
    GerbilHookPolicyInvocationError, GerbilHookPolicyInvocationInput,
    decode_gerbil_hook_policy_evaluation,
};

#[test]
fn gerbil_hook_policy_evaluation_decodes_typed_receipt() {
    let extension = HookPolicyExtension::gerbil_scheme("marlin/hooks/policy", "decide-hook-policy");
    let receipt = decode_gerbil_hook_policy_evaluation(GerbilHookPolicyEvaluationDecodeInput {
        invocation: GerbilHookPolicyInvocationInput {
            extension: extension.clone(),
            event_name: HookEventName::PreToolUse,
            agent_scope: HookAgentScope::CustomerAgent,
            policy_receipt: HookDispatchPolicyReceipt::new(HookDispatchPolicyReceiptInput {
                event_name: HookEventName::PreToolUse,
                invocation_agent_scope: HookAgentScope::CustomerAgent,
                decision_context: HookDecisionContext::default(),
                mode: HookPolicyMode::ObserveOnly,
                extension: extension.clone(),
                actions: Vec::new(),
                decisions: Vec::new(),
            }),
        },
        output_json: serde_json::json!({
            "decision": "Rejected",
            "diagnostics": [
                {
                    "message": "blocked by Gerbil policy"
                }
            ]
        })
        .to_string(),
    })
    .expect("evaluation output decodes");

    assert_eq!(receipt.event_name, HookEventName::PreToolUse);
    assert_eq!(receipt.agent_scope, HookAgentScope::CustomerAgent);
    assert_eq!(receipt.extension, extension);
    assert_eq!(receipt.decision, HookPolicyDecision::Rejected);
    assert_eq!(receipt.diagnostics[0].message, "blocked by Gerbil policy");
    assert_eq!(receipt.policy_evaluated_count, 0);
    assert_eq!(receipt.policy_rejected_count, 0);
    assert!(!receipt.is_allowed());
}

#[test]
fn gerbil_hook_policy_evaluation_decodes_complex_scheme_policy_actions_without_llm() {
    let extension = HookPolicyExtension::gerbil_scheme(
        "marlin/hooks/policy-samples",
        "decide-hook-policy-sample",
    );
    let receipt = decode_gerbil_hook_policy_evaluation(GerbilHookPolicyEvaluationDecodeInput {
        invocation: GerbilHookPolicyInvocationInput {
            extension: extension.clone(),
            event_name: HookEventName::PreToolUse,
            agent_scope: HookAgentScope::CustomerAgent,
            policy_receipt: HookDispatchPolicyReceipt::new(HookDispatchPolicyReceiptInput {
                event_name: HookEventName::PreToolUse,
                invocation_agent_scope: HookAgentScope::CustomerAgent,
                decision_context: HookDecisionContext::new()
                    .with_session_id("cheap-test-session")
                    .with_agent_lineage_node("release")
                    .with_workspace_state("dirty")
                    .with_org_memory_hit("needs-human-review")
                    .with_agent_class("customer-agent"),
                mode: HookPolicyMode::ObserveOnly,
                extension: extension.clone(),
                actions: Vec::new(),
                decisions: Vec::new(),
            }),
        },
        output_json: serde_json::json!({
            "decision": "Allowed",
            "diagnostics": [
                {
                    "message": "sample Gerbil hook policy evaluated"
                }
            ],
            "actions": [
                {
                    "kind": "Register",
                    "target": "catalog:customer-agent-hook",
                    "reason": "customer agent session requires runtime catalog hook"
                },
                {
                    "kind": "Defer",
                    "target": "session:release",
                    "reason": "release lineage waits for org memory review"
                },
                {
                    "kind": "Deny",
                    "target": "dangerous-shell",
                    "reason": "dirty workspace blocks dangerous shell hook"
                },
                {
                    "kind": "Rewrite",
                    "target": "command",
                    "replacement": "cargo test --locked",
                    "reason": "session policy prefers locked tests"
                }
            ]
        })
        .to_string(),
    })
    .expect("complex Gerbil policy actions decode without LLM");

    assert!(receipt.is_allowed());
    assert_eq!(receipt.decision, HookPolicyDecision::Allowed);
    assert_eq!(receipt.actions.len(), 4);
    assert_eq!(
        receipt.actions[0].kind,
        HookPolicyDynamicActionKind::Register
    );
    assert_eq!(
        receipt.actions[0]
            .target
            .as_ref()
            .map(|target| target.as_str()),
        Some("catalog:customer-agent-hook")
    );
    assert_eq!(receipt.actions[1].kind, HookPolicyDynamicActionKind::Defer);
    assert_eq!(
        receipt.actions[1]
            .reason
            .as_ref()
            .map(|reason| reason.as_str()),
        Some("release lineage waits for org memory review")
    );
    assert_eq!(receipt.actions[2].kind, HookPolicyDynamicActionKind::Deny);
    assert_eq!(
        receipt.actions[2]
            .target
            .as_ref()
            .map(|target| target.as_str()),
        Some("dangerous-shell")
    );
    assert_eq!(
        receipt.actions[3].kind,
        HookPolicyDynamicActionKind::Rewrite
    );
    assert_eq!(
        receipt.actions[3]
            .replacement
            .as_ref()
            .map(|replacement| replacement.as_str()),
        Some("cargo test --locked")
    );
}

#[test]
fn gerbil_hook_policy_evaluation_rejects_invalid_json() {
    let extension = HookPolicyExtension::gerbil_scheme("marlin/hooks/policy", "decide-hook-policy");
    let error = decode_gerbil_hook_policy_evaluation(GerbilHookPolicyEvaluationDecodeInput {
        invocation: GerbilHookPolicyInvocationInput {
            extension: extension.clone(),
            event_name: HookEventName::PreToolUse,
            agent_scope: HookAgentScope::SubAgent,
            policy_receipt: HookDispatchPolicyReceipt::new(HookDispatchPolicyReceiptInput {
                event_name: HookEventName::PreToolUse,
                invocation_agent_scope: HookAgentScope::SubAgent,
                decision_context: HookDecisionContext::default(),
                mode: HookPolicyMode::ObserveOnly,
                extension,
                actions: Vec::new(),
                decisions: Vec::new(),
            }),
        },
        output_json: "not-json".to_owned(),
    })
    .expect_err("invalid JSON should fail");

    assert!(matches!(
        error,
        GerbilHookPolicyEvaluationError::Decode { .. }
    ));
}

#[test]
fn gerbil_hook_policy_evaluation_reuses_invocation_validation() {
    let error = decode_gerbil_hook_policy_evaluation(GerbilHookPolicyEvaluationDecodeInput {
        invocation: GerbilHookPolicyInvocationInput {
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
        },
        output_json: serde_json::json!({
            "decision": "Allowed"
        })
        .to_string(),
    })
    .expect_err("non-scheme extension should fail before decoding output");

    assert_eq!(
        error,
        GerbilHookPolicyEvaluationError::Invocation {
            source: GerbilHookPolicyInvocationError::UnsupportedExtension {
                kind: HookPolicyExtensionKind::None
            }
        }
    );
}
