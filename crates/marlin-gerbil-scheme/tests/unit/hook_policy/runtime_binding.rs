use marlin_agent_protocol::{
    HookAgentScope, HookDecisionContext, HookDispatchPolicyReceipt, HookDispatchPolicyReceiptInput,
    HookEventName, HookPolicyDecision, HookPolicyDynamicActionKind, HookPolicyExtension,
    HookPolicyMode,
};
use marlin_gerbil_scheme::{
    DEFAULT_GERBIL_GXI_PROGRAM, GERBIL_HOOK_POLICY_ADAPTER_PATH, GERBIL_LOADPATH_ENV,
    GERBIL_MARLIN_HOOK_POLICY_PATH, GerbilHookPolicyInvocationInput,
    GerbilHookPolicyRuntimeBinding, gerbil_runtime_loadpath,
};
use std::{ffi::OsString, fs, path::Path};
use tempfile::Builder;

fn complex_hook_decision_context() -> HookDecisionContext {
    HookDecisionContext::new()
        .with_session_id("cheap-test-session")
        .with_agent_lineage_node("release")
        .with_workspace_state("dirty")
        .with_workspace_state("project-untrusted")
        .with_org_memory_hit("needs-human-review")
        .with_agent_class("customer-agent")
}

#[test]
fn gerbil_hook_policy_runtime_binding_writes_launcher_assets() {
    let root = Builder::new()
        .prefix("marlin-gerbil-hook-policy-binding-")
        .tempdir()
        .expect("creates hook policy binding root");
    let binding = GerbilHookPolicyRuntimeBinding::new("/bin/sh", root.path())
        .expect("runtime binding should write hook policy assets");
    let launcher = root.path().join(GERBIL_HOOK_POLICY_ADAPTER_PATH);
    let module = root.path().join(GERBIL_MARLIN_HOOK_POLICY_PATH);

    assert_eq!(binding.loadpath_root(), root.path());
    assert!(launcher.exists());
    assert!(module.exists());
    assert!(
        binding
            .written_assets()
            .iter()
            .any(|asset| asset.ends_with(GERBIL_HOOK_POLICY_ADAPTER_PATH))
    );
    assert!(
        binding
            .written_assets()
            .iter()
            .any(|asset| asset.ends_with(GERBIL_MARLIN_HOOK_POLICY_PATH))
    );
    assert_eq!(binding.spec().program, Path::new("/bin/sh"));
    assert_eq!(
        binding.spec().args,
        vec![launcher.as_os_str().to_os_string()]
    );
    assert_eq!(
        binding.spec().env.get(&OsString::from(GERBIL_LOADPATH_ENV)),
        Some(
            &gerbil_runtime_loadpath(root.path())
                .as_os_str()
                .to_os_string()
        )
    );
}

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn gerbil_hook_policy_runtime_binding_real_gxi_evaluates_configured_module() {
    if !Path::new(DEFAULT_GERBIL_GXI_PROGRAM).exists() {
        return;
    }

    let root = Builder::new()
        .prefix("marlin-gerbil-hook-policy-real-gxi-")
        .tempdir()
        .expect("creates real gxi hook policy binding root");
    let binding = GerbilHookPolicyRuntimeBinding::new(DEFAULT_GERBIL_GXI_PROGRAM, root.path())
        .expect("runtime binding should write hook policy assets");
    let policy_module = root.path().join("src/marlin/hooks/policy.ss");
    fs::create_dir_all(policy_module.parent().expect("policy module parent"))
        .expect("creates policy module parent");
    fs::write(
        &policy_module,
        r#";;; -*- Gerbil -*-
package: marlin/hooks

(export decide-hook-policy)

(def (decide-hook-policy request-json)
  "{\"decision\":\"Allowed\",\"diagnostics\":[{\"message\":\"real gxi hook policy allowed\"}]}")
"#,
    )
    .expect("writes policy module");

    let extension = HookPolicyExtension::gerbil_scheme("marlin/hooks/policy", "decide-hook-policy");
    let receipt = binding
        .evaluator()
        .evaluate(GerbilHookPolicyInvocationInput {
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
        .expect("real gxi hook policy should evaluate configured module");

    assert!(receipt.is_allowed());
    assert_eq!(
        receipt.diagnostics[0].message,
        "real gxi hook policy allowed"
    );
}

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn gerbil_hook_policy_runtime_binding_real_gxi_runs_sample_policy_module() {
    if !Path::new(DEFAULT_GERBIL_GXI_PROGRAM).exists() {
        return;
    }

    let root = Builder::new()
        .prefix("marlin-gerbil-hook-policy-sample-real-gxi-")
        .tempdir()
        .expect("creates real gxi hook policy sample binding root");
    let binding = GerbilHookPolicyRuntimeBinding::new(DEFAULT_GERBIL_GXI_PROGRAM, root.path())
        .expect("runtime binding should write hook policy sample assets");

    let extension = HookPolicyExtension::gerbil_scheme(
        "marlin/hooks/policy-samples",
        "decide-hook-policy-sample",
    );
    let receipt = binding
        .evaluator()
        .evaluate(GerbilHookPolicyInvocationInput {
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
        .expect("real gxi hook policy sample should decode dynamic actions");

    assert!(receipt.is_allowed());
    assert_eq!(receipt.actions.len(), 1);
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
}

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn gerbil_hook_policy_runtime_binding_real_gxi_runs_complex_sample_policy_context() {
    if !Path::new(DEFAULT_GERBIL_GXI_PROGRAM).exists() {
        return;
    }

    let root = Builder::new()
        .prefix("marlin-gerbil-hook-policy-complex-sample-real-gxi-")
        .tempdir()
        .expect("creates real gxi complex hook policy sample binding root");
    let binding = GerbilHookPolicyRuntimeBinding::new(DEFAULT_GERBIL_GXI_PROGRAM, root.path())
        .expect("runtime binding should write hook policy sample assets");

    let extension = HookPolicyExtension::gerbil_scheme(
        "marlin/hooks/policy-samples",
        "decide-hook-policy-sample",
    );
    let receipt = binding
        .evaluator()
        .evaluate(GerbilHookPolicyInvocationInput {
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
        .expect("real gxi hook policy sample should evaluate complex decision context");

    assert!(receipt.is_allowed());
    assert_eq!(receipt.actions.len(), 5);
    assert_eq!(
        receipt.actions[0].kind,
        HookPolicyDynamicActionKind::Register
    );
    assert_eq!(receipt.actions[1].kind, HookPolicyDynamicActionKind::Defer);
    assert_eq!(receipt.actions[2].kind, HookPolicyDynamicActionKind::Deny);
    assert_eq!(
        receipt.actions[3].kind,
        HookPolicyDynamicActionKind::Unregister
    );
    assert_eq!(
        receipt.actions[3]
            .target
            .as_ref()
            .map(|target| target.as_str()),
        Some("catalog:live-project-hook")
    );
    assert_eq!(
        receipt.actions[4].kind,
        HookPolicyDynamicActionKind::Rewrite
    );
    assert_eq!(
        receipt.actions[4]
            .replacement
            .as_ref()
            .map(|replacement| replacement.as_str()),
        Some("cargo test --locked")
    );
}

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn gerbil_hook_policy_runtime_binding_real_gxi_decodes_dynamic_actions() {
    if !Path::new(DEFAULT_GERBIL_GXI_PROGRAM).exists() {
        return;
    }

    let root = Builder::new()
        .prefix("marlin-gerbil-hook-policy-actions-real-gxi-")
        .tempdir()
        .expect("creates real gxi hook policy action binding root");
    let binding = GerbilHookPolicyRuntimeBinding::new(DEFAULT_GERBIL_GXI_PROGRAM, root.path())
        .expect("runtime binding should write hook policy assets");
    let policy_module = root.path().join("src/marlin/hooks/actions.ss");
    fs::create_dir_all(policy_module.parent().expect("policy module parent"))
        .expect("creates policy module parent");
    fs::write(
        &policy_module,
        r#";;; -*- Gerbil -*-
package: marlin/hooks

(export decide-hook-policy-actions)

(def (decide-hook-policy-actions request-json)
  "{\"decision\":\"Rejected\",\"diagnostics\":[{\"message\":\"real gxi hook policy actions\"}],\"actions\":[{\"kind\":\"Deny\",\"target\":\"tool:rm\",\"reason\":\"workspace policy\"},{\"kind\":\"Rewrite\",\"target\":\"command\",\"replacement\":\"cargo test --locked\",\"reason\":\"prefer locked tests\"},{\"kind\":\"Register\",\"target\":\"hook:customer-agent\",\"reason\":\"session requires customer hook\"},{\"kind\":\"Unregister\",\"target\":\"hook:legacy\",\"reason\":\"legacy hook disabled\"},{\"kind\":\"Defer\",\"target\":\"session:release\",\"reason\":\"wait for org memory query\"}]}")
"#,
    )
    .expect("writes policy module");

    let extension =
        HookPolicyExtension::gerbil_scheme("marlin/hooks/actions", "decide-hook-policy-actions");
    let receipt = binding
        .evaluator()
        .evaluate(GerbilHookPolicyInvocationInput {
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
        .expect("real gxi hook policy should decode dynamic actions");

    assert!(!receipt.is_allowed());
    assert_eq!(receipt.decision, HookPolicyDecision::Rejected);
    assert_eq!(receipt.actions.len(), 5);
    assert_eq!(receipt.actions[0].kind, HookPolicyDynamicActionKind::Deny);
    assert_eq!(
        receipt.actions[0]
            .target
            .as_ref()
            .map(|target| target.as_str()),
        Some("tool:rm")
    );
    assert_eq!(
        receipt.actions[1].kind,
        HookPolicyDynamicActionKind::Rewrite
    );
    assert_eq!(
        receipt.actions[1]
            .replacement
            .as_ref()
            .map(|replacement| replacement.as_str()),
        Some("cargo test --locked")
    );
    assert_eq!(
        receipt.actions[2].kind,
        HookPolicyDynamicActionKind::Register
    );
    assert_eq!(
        receipt.actions[3].kind,
        HookPolicyDynamicActionKind::Unregister
    );
    assert_eq!(receipt.actions[4].kind, HookPolicyDynamicActionKind::Defer);
    assert_eq!(
        receipt.actions[4]
            .reason
            .as_ref()
            .map(|reason| reason.as_str()),
        Some("wait for org memory query")
    );
}
