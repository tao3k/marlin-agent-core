use std::{path::Path, process::Command};

use marlin_gerbil_scheme::{
    GERBIL_LOOP_CASE_DRIVER_RUST_LOOP_RECEIPT_SCHEMA_ID,
    GERBIL_LOOP_CASE_DRIVER_SCHEME_RECEIPT_KIND, GerbilLoopCaseDriverSchemeReceipt,
    GerbilLoopCaseDriverSchemeReceiptKind, GerbilLoopCaseRuntimeHandoffStatus,
    GerbilLoopCaseRuntimeMode, GerbilLoopCaseSchemeBoundary, GerbilLoopCaseSerializationBoundary,
    default_gerbil_gxi_program, project_gerbil_loop_case_driver_rust_loop_receipt,
};

#[test]
fn config_interface_scheme_case_projects_to_rust_loop_receipt() {
    let scheme_receipt = GerbilLoopCaseDriverSchemeReceipt::runtime_handoff_no_live_llm_fixture(
        "runtime-handoff-llm",
        "custom/marline-kernel/policies/loops/cases/runtime-handoff-llm.json",
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
    assert_eq!(rust_receipt.case_id, "runtime-handoff-llm");
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
fn config_interface_case_driver_scheme_smoke_runs_real_policy_cases() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let gerbil_root = manifest_dir.join("gerbil");
    let gxi = default_gerbil_gxi_program();
    let loadpath = gerbil_loadpath_with_src(&gerbil_root);

    let output = Command::new(&gxi)
        .current_dir(&gerbil_root)
        .env("GERBIL_LOADPATH", loadpath)
        .arg("t/config-interface-case-driver-test.ss")
        .output()
        .unwrap_or_else(|error| panic!("run {:?}: {error}", gxi));

    assert!(
        output.status.success(),
        "gxi case-driver smoke failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("config-interface-case-driver-ok"));
    assert!(stdout.contains("case-driver-receipts=4"));
}

fn gerbil_loadpath_with_src(gerbil_root: &Path) -> std::ffi::OsString {
    let mut paths = vec![gerbil_root.join("src")];
    if let Some(existing) = std::env::var_os("GERBIL_LOADPATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    std::env::join_paths(paths).expect("join Gerbil loadpath")
}
