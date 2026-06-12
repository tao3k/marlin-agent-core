use marlin_agent_protocol::{
    HookAgentScope, HookDispatchPolicyReceipt, HookDispatchPolicyReceiptInput, HookEventName,
    HookPolicyDecision, HookPolicyExtension, HookPolicyExtensionKind, HookPolicyMode,
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
                mode: HookPolicyMode::ObserveOnly,
                extension: extension.clone(),
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
                mode: HookPolicyMode::ObserveOnly,
                extension,
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
                mode: HookPolicyMode::ObserveOnly,
                extension: HookPolicyExtension::none(),
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
