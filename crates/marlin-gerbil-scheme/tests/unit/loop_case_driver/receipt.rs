use marlin_gerbil_scheme::{
    GERBIL_LOOP_CASE_DRIVER_RUST_LOOP_RECEIPT_SCHEMA_ID,
    GERBIL_LOOP_CASE_DRIVER_SCHEME_RECEIPT_KIND, GerbilLoopCaseDriverSchemeReceipt,
    GerbilLoopCaseDriverSchemeReceiptKind, GerbilLoopCaseRuntimeHandoffStatus,
    GerbilLoopCaseRuntimeMode, GerbilLoopCaseSchemeBoundary, GerbilLoopCaseSerializationBoundary,
    project_gerbil_loop_case_driver_rust_loop_receipt,
};

#[test]
fn config_interface_scheme_case_projects_to_rust_loop_receipt() {
    let scheme_receipt = GerbilLoopCaseDriverSchemeReceipt::runtime_handoff_no_live_llm_fixture(
        "scheme-projected-runtime-handoff",
        "none",
    );

    assert_eq!(
        scheme_receipt.kind,
        GerbilLoopCaseDriverSchemeReceiptKind::MarlineKernelLoopCaseDriverReceipt
    );
    assert_eq!(
        scheme_receipt.kind.as_str(),
        GERBIL_LOOP_CASE_DRIVER_SCHEME_RECEIPT_KIND
    );
    assert_eq!(
        scheme_receipt.runtime_mode,
        GerbilLoopCaseRuntimeMode::LoopRuntime
    );
    assert_eq!(scheme_receipt.runtime_mode.as_str(), "loop-runtime");

    let rust_receipt = project_gerbil_loop_case_driver_rust_loop_receipt(&scheme_receipt);

    assert_eq!(
        rust_receipt.schema_id,
        GERBIL_LOOP_CASE_DRIVER_RUST_LOOP_RECEIPT_SCHEMA_ID
    );
    assert_eq!(
        rust_receipt.case_id.as_str(),
        "scheme-projected-runtime-handoff"
    );
    assert_eq!(
        rust_receipt.runtime_handoff_status,
        GerbilLoopCaseRuntimeHandoffStatus::DeferredNoLiveLlm
    );
    assert_eq!(rust_receipt.runtime_execution_owner, "rust-loop-runtime");
    assert_eq!(
        rust_receipt.module_kind.as_str(),
        "poo-flow.modules.user-selection.v1"
    );
    assert_eq!(rust_receipt.module_user_module.as_str(), "funflow");
    assert_eq!(
        rust_receipt
            .module_selection_tags
            .iter()
            .map(|tag| tag.as_str())
            .collect::<Vec<_>>(),
        vec![
            "+functional",
            "+dag",
            "+typed-receipts",
            "+runtime-manifest"
        ]
    );
    assert_eq!(rust_receipt.module_source_ref, "none");
    assert_eq!(rust_receipt.module_entrypoint, "none");
    assert!(rust_receipt.module_enabled);
    assert!(!rust_receipt.live_llm_allowed);
    assert!(rust_receipt.stable_fixture);
    assert_eq!(
        rust_receipt.scheme_boundary,
        GerbilLoopCaseSchemeBoundary::SchemeTypesToRustTypes
    );
    assert_eq!(
        rust_receipt.serialization_boundary,
        GerbilLoopCaseSerializationBoundary::RustOwnedCliTraceCrossProcess
    );
    assert!(
        !format!("{:?}", rust_receipt.scheme_boundary)
            .to_ascii_lowercase()
            .contains("json")
    );
}

#[test]
fn config_interface_typed_loop_projection_case_projects_to_ready_rust_loop_receipt() {
    let scheme_receipt = GerbilLoopCaseDriverSchemeReceipt::typed_loop_projection_fixture(
        "scheme-projected-loop",
        "scheme-profile/reactive-tool-loop",
        ["+scripted-e2e", "+tool-handoff", "+verification"],
    );

    assert_eq!(
        scheme_receipt.runtime_mode,
        GerbilLoopCaseRuntimeMode::TypedLoopProjection
    );
    assert_eq!(
        scheme_receipt.runtime_mode.as_str(),
        "typed-loop-projection"
    );

    let rust_receipt = project_gerbil_loop_case_driver_rust_loop_receipt(&scheme_receipt);

    assert_eq!(rust_receipt.case_id.as_str(), "scheme-projected-loop");
    assert_eq!(
        rust_receipt.profile_ref.as_str(),
        "scheme-profile/reactive-tool-loop"
    );
    assert_eq!(
        rust_receipt.runtime_handoff_status,
        GerbilLoopCaseRuntimeHandoffStatus::Ready
    );
    assert!(!rust_receipt.live_llm_required);
    assert!(!rust_receipt.live_llm_allowed);
    assert!(!rust_receipt.stable_fixture);
    assert_eq!(
        rust_receipt.command_vector,
        vec![
            "marlin",
            "loop",
            "program",
            "run",
            "--profile",
            "scheme-profile/reactive-tool-loop"
        ]
    );
    assert_eq!(
        rust_receipt.module_kind.as_str(),
        "marlin.config-interface.loop-policy-profile-projection.v1"
    );
    assert_eq!(
        rust_receipt.module_entrypoint,
        "marlinLoopPolicyProfileCompilerReceipts"
    );
    assert_eq!(
        rust_receipt
            .module_selection_tags
            .iter()
            .map(|tag| tag.as_str())
            .collect::<Vec<_>>(),
        vec!["+scripted-e2e", "+tool-handoff", "+verification"]
    );
    assert_eq!(
        rust_receipt.scheme_boundary,
        GerbilLoopCaseSchemeBoundary::SchemeTypesToRustTypes
    );
    assert_eq!(
        rust_receipt.serialization_boundary,
        GerbilLoopCaseSerializationBoundary::RustOwnedCliTraceCrossProcess
    );
}
