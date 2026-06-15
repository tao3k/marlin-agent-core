use marlin_gerbil_scheme::{
    GERBIL_ADAPTER_MODULE, GERBIL_LOADPATH_ENV, GerbilArtifactKind, GerbilCommandProfile,
    GerbilCommandSpec, GerbilCompileRequest, GerbilSource, GerbilWorkspaceContractFacts,
};
use marlin_org_model::{
    OrgContractRegistry, OrgContractResolutionReport, OrgContractValidationReport,
};

#[test]
fn compile_request_defaults_workspace_patch_intent_contract_facts() {
    let request = GerbilCompileRequest::new(
        GerbilSource::new(
            "audit/workspace-patch-intent",
            "(workspace-patch-intent \"intent:memory\")",
        ),
        GerbilArtifactKind::WorkspacePatchIntent,
    );
    let schema_request = GerbilCompileRequest::new(
        GerbilSource::new(
            "audit/workspace-schema",
            "(workspace-schema workspace-record)",
        ),
        GerbilArtifactKind::WorkspaceSchema,
    );

    assert!(
        request.contract_facts.is_some(),
        "workspace patch-intent requests carry contract facts by default"
    );
    assert!(schema_request.contract_facts.is_none());
}

#[test]
fn compile_request_accepts_explicit_contract_facts_for_any_kind() {
    let request = GerbilCompileRequest::with_contract_facts(
        GerbilSource::new("audit/control-plane", "(module audit/control-plane)"),
        GerbilArtifactKind::LoopGraph,
        GerbilWorkspaceContractFacts {
            registry: OrgContractRegistry::default(),
            resolutions: OrgContractResolutionReport::default(),
            validations: OrgContractValidationReport::default(),
        },
    );

    assert_eq!(request.expected, GerbilArtifactKind::LoopGraph);
    assert!(request.contract_facts.is_some());
}

#[test]
fn command_profile_projects_to_command_spec_without_compiler_binding() {
    let profile = GerbilCommandProfile::new("/opt/gerbil/bin/gxi")
        .arg(":marlin/adapter")
        .current_dir("/tmp")
        .env("CUSTOM_GERBIL_FLAG", "enabled");

    let spec: GerbilCommandSpec = profile.into();

    assert_eq!(
        spec.program,
        std::path::PathBuf::from("/opt/gerbil/bin/gxi")
    );
    assert_eq!(
        spec.current_dir.as_deref(),
        Some(std::path::Path::new("/tmp"))
    );
    assert_eq!(spec.args, [std::ffi::OsString::from(":marlin/adapter")]);
    assert_eq!(
        spec.env
            .get(&std::ffi::OsString::from("CUSTOM_GERBIL_FLAG"))
            .map(std::ffi::OsString::as_os_str),
        Some(std::ffi::OsStr::new("enabled"))
    );
}

#[test]
fn command_spec_marlin_runtime_module_uses_typed_adapter_module() {
    let spec = GerbilCommandSpec::marlin_runtime_module("/opt/gerbil/bin/gxi", "/runtime/root");

    assert_eq!(spec.args, [std::ffi::OsString::from(GERBIL_ADAPTER_MODULE)]);
    assert!(
        spec.env
            .contains_key(&std::ffi::OsString::from(GERBIL_LOADPATH_ENV)),
        "runtime module specs still carry a Rust-owned loadpath projection"
    );
}
