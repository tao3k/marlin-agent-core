use marlin_agent_protocol::{
    HookAgentScope, HookDispatchPolicyReceipt, HookDispatchPolicyReceiptInput, HookEventName,
    HookPolicyExtension, HookPolicyMode,
};
use marlin_gerbil_scheme::{
    DEFAULT_GERBIL_GXI_PROGRAM, GERBIL_HOOK_POLICY_ADAPTER_PATH, GERBIL_LOADPATH_ENV,
    GERBIL_MARLIN_HOOK_POLICY_PATH, GerbilHookPolicyInvocationInput,
    GerbilHookPolicyRuntimeBinding, gerbil_runtime_loadpath,
};
use std::{ffi::OsString, fs, path::Path};
use tempfile::Builder;

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
                mode: HookPolicyMode::ObserveOnly,
                extension,
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
