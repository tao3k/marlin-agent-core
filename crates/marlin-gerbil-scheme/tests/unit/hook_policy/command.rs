use marlin_agent_protocol::{
    HookAgentScope, HookDispatchPolicyReceipt, HookDispatchPolicyReceiptInput, HookEventName,
    HookPolicyExtension, HookPolicyMode,
};
use marlin_gerbil_scheme::{
    GerbilCommandSpec, GerbilHookPolicyCommandEvaluator, GerbilHookPolicyEvaluationError,
    GerbilHookPolicyInvocationInput,
};

#[test]
fn gerbil_hook_policy_command_evaluator_decodes_command_stdout() {
    let extension = HookPolicyExtension::gerbil_scheme("marlin/hooks/policy", "decide-hook-policy");
    let evaluator = GerbilHookPolicyCommandEvaluator::new(GerbilCommandSpec::new("/bin/sh").arg(
        "-c",
    ).arg(
        "payload=$(cat); case \"$payload\" in *'marlin/hooks/policy'*'decide-hook-policy'*) printf '%s\n' '{\"decision\":\"Allowed\",\"diagnostics\":[{\"message\":\"command policy allowed\"}]}' ;; *) printf '%s\n' 'missing hook policy invocation packet' >&2; exit 64 ;; esac",
    ));

    let receipt = evaluator
        .evaluate(GerbilHookPolicyInvocationInput {
            extension: extension.clone(),
            event_name: HookEventName::PreToolUse,
            agent_scope: HookAgentScope::CustomerAgent,
            policy_receipt: HookDispatchPolicyReceipt::new(HookDispatchPolicyReceiptInput {
                event_name: HookEventName::PreToolUse,
                invocation_agent_scope: HookAgentScope::CustomerAgent,
                mode: HookPolicyMode::ObserveOnly,
                extension,
                actions: Vec::new(),
                decisions: Vec::new(),
            }),
        })
        .expect("command stdout should decode to a policy receipt");

    assert!(receipt.is_allowed());
    assert_eq!(receipt.diagnostics[0].message, "command policy allowed");
}

#[test]
fn gerbil_hook_policy_command_evaluator_reports_command_failure_diagnostics() {
    let extension = HookPolicyExtension::gerbil_scheme("marlin/hooks/policy", "decide-hook-policy");
    let evaluator = GerbilHookPolicyCommandEvaluator::new(GerbilCommandSpec::new("/bin/sh").arg(
        "-c",
    ).arg(
        "cat >/dev/null; printf '%s\n' 'policy stdout failure'; printf '%s\n' 'policy stderr failure' >&2; exit 70",
    ));

    let error = evaluator
        .evaluate(GerbilHookPolicyInvocationInput {
            extension: extension.clone(),
            event_name: HookEventName::PreToolUse,
            agent_scope: HookAgentScope::SubAgent,
            policy_receipt: HookDispatchPolicyReceipt::new(HookDispatchPolicyReceiptInput {
                event_name: HookEventName::PreToolUse,
                invocation_agent_scope: HookAgentScope::SubAgent,
                mode: HookPolicyMode::ObserveOnly,
                extension,
                actions: Vec::new(),
                decisions: Vec::new(),
            }),
        })
        .expect_err("command failure should be reported before decoding");

    match error {
        GerbilHookPolicyEvaluationError::Command { message } => {
            assert!(message.contains("exit status: 70"));
            assert!(message.contains("stderr: policy stderr failure"));
            assert!(message.contains("stdout: policy stdout failure"));
        }
        other => panic!("expected command error, got {other:?}"),
    }
}
