use marlin_agent_protocol::{ModelRouteAgentScope, ModelRouteRequest};
use marlin_gerbil_scheme::{
    DEFAULT_GERBIL_GXI_PROGRAM, GERBIL_DECK_RUNTIME_POLICY_ADAPTER_PATH, GERBIL_LOADPATH_ENV,
    GERBIL_MARLIN_DECK_RUNTIME_POLICY_PATH, GerbilDeckRuntimeModelRoutePolicy,
    GerbilDeckRuntimeModelRoutePolicyRequest, GerbilDeckRuntimeModelRoutePolicyRuntimeBinding,
    decode_gerbil_deck_runtime_model_route_selection, gerbil_runtime_loadpath,
};
use std::{ffi::OsString, path::Path};
use tempfile::Builder;

#[test]
fn gerbil_deck_runtime_policy_request_uses_typed_agent_scope() {
    let policy =
        GerbilDeckRuntimeModelRoutePolicy::new("cheap-test-runner", "openai", "gpt-5-mini")
            .with_command_prefix("cargo test")
            .with_agent_scope("sub-agent");
    let request = ModelRouteRequest::command(["cargo", "test"])
        .with_agent_scope(ModelRouteAgentScope::SubAgent)
        .with_sub_agent_role("tester");

    let scheme_request =
        GerbilDeckRuntimeModelRoutePolicyRequest::from_model_route_request([policy], &request);

    assert_eq!(scheme_request.command, "cargo test");
    assert_eq!(scheme_request.agent_scope, "sub-agent");
    assert_eq!(scheme_request.policies[0].provider, "openai");
    assert_eq!(scheme_request.policies[0].model, "gpt-5-mini");
}

#[test]
fn gerbil_deck_runtime_policy_decode_preserves_provider_model_identity() {
    let receipt = decode_gerbil_deck_runtime_model_route_selection(
        r#"{"schema_id":"marlin-deck-runtime.model-route-selection.v1","command":"cargo test","agent_scope":"sub-agent","matched":true,"policy":{"kind":"marlin-deck-runtime.model-route-policy.v1","name":"cheap-test-runner","provider":"openai","model":"gpt-5-mini","command_prefixes":["cargo test"],"agent_scopes":["sub-agent"],"context_mode":"forked-context","isolation_mode":"workspace-isolated"}}"#,
    )
    .expect("decode selection receipt");

    let policy = receipt.selected_policy().expect("selected policy");
    assert!(receipt.matched);
    assert_eq!(policy.provider, "openai");
    assert_eq!(policy.model, "gpt-5-mini");
    assert_eq!(policy.context_mode.as_str(), "forked-context");
}

#[test]
fn gerbil_deck_runtime_policy_runtime_binding_writes_launcher_assets() {
    let root = Builder::new()
        .prefix("marlin-gerbil-deck-runtime-policy-binding-")
        .tempdir()
        .expect("creates deck runtime policy binding root");
    let binding = GerbilDeckRuntimeModelRoutePolicyRuntimeBinding::new("/bin/sh", root.path())
        .expect("runtime binding should write deck runtime policy assets");
    let launcher = root.path().join(GERBIL_DECK_RUNTIME_POLICY_ADAPTER_PATH);
    let module = root.path().join(GERBIL_MARLIN_DECK_RUNTIME_POLICY_PATH);

    assert_eq!(binding.loadpath_root(), root.path());
    assert!(launcher.exists());
    assert!(module.exists());
    assert!(
        binding
            .written_assets()
            .iter()
            .any(|asset| asset.ends_with(GERBIL_DECK_RUNTIME_POLICY_ADAPTER_PATH))
    );
    assert!(
        binding
            .written_assets()
            .iter()
            .any(|asset| asset.ends_with(GERBIL_MARLIN_DECK_RUNTIME_POLICY_PATH))
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
#[ignore = "requires a local Gerbil gxi executable and installed gerbil-poo dependency"]
fn gerbil_deck_runtime_policy_runtime_binding_real_gxi_selects_policy() {
    if !Path::new(DEFAULT_GERBIL_GXI_PROGRAM).exists() {
        return;
    }

    let root = Builder::new()
        .prefix("marlin-gerbil-deck-runtime-policy-real-gxi-")
        .tempdir()
        .expect("creates real gxi deck runtime policy root");
    let binding = GerbilDeckRuntimeModelRoutePolicyRuntimeBinding::new(
        DEFAULT_GERBIL_GXI_PROGRAM,
        root.path(),
    )
    .expect("runtime binding should write deck runtime policy assets");
    let policy =
        GerbilDeckRuntimeModelRoutePolicy::new("cheap-test-runner", "openai", "gpt-5-mini")
            .with_command_prefix("cargo test")
            .with_agent_scope("sub-agent")
            .with_context_mode("forked-context")
            .with_isolation_mode("workspace-isolated");
    let request = GerbilDeckRuntimeModelRoutePolicyRequest::from_model_route_request(
        [policy],
        &ModelRouteRequest::command(["cargo", "test", "-p", "marlin-gerbil-scheme"])
            .with_agent_scope(ModelRouteAgentScope::SubAgent),
    );

    let receipt = binding
        .evaluator()
        .evaluate(request)
        .expect("real gxi deck runtime policy selector should select policy");
    let selected = receipt.selected_policy().expect("selected policy");

    assert!(receipt.matched);
    assert_eq!(selected.name, "cheap-test-runner");
    assert_eq!(selected.provider, "openai");
    assert_eq!(selected.model, "gpt-5-mini");
}
