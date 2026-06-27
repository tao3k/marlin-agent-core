use std::{fs, path::Path};

use marlin_rust_project_harness_policy::{
    GerbilRuntimeAssetManifestStatus, GerbilRuntimeHarnessContractStatus,
    inspect_gerbil_runtime_assets, inspect_gerbil_runtime_harness_contract,
};

#[test]
fn gerbil_runtime_asset_receipt_reports_absent_runtime_root_as_non_blocking() {
    let tempdir = tempfile::tempdir().expect("create temp project");

    let receipt = inspect_gerbil_runtime_assets(tempdir.path());

    assert_eq!(receipt.status, GerbilRuntimeAssetManifestStatus::NotPresent);
    assert!(receipt.is_success());
    assert!(!receipt.has_runtime_assets());
    assert_eq!(receipt.asset_count, 0);
}

#[test]
fn gerbil_runtime_asset_receipt_accepts_complete_marlin_runtime_package() {
    let tempdir = tempfile::tempdir().expect("create temp project");
    write_runtime_assets(
        tempdir.path(),
        &[
            "gerbil.pkg",
            "build.ss",
            "src/marlin/protocol-types.ss",
            "src/marlin/protocol.ss",
            "src/marlin/request.ss",
            "src/marlin/adapter.ss",
            "src/marlin/deck-runtime.ss",
            "src/marlin/_deck-runtime-native.ssi",
            "src/marlin/deck-runtime-native.ss",
            "src/marlin/_agent-policy-routing-native.ssi",
            "src/marlin/deck-runtime-native-projection.ss",
            "src/marlin/graph-loop-continuation-native-projection.ss",
            "src/marlin/deck-runtime-script.ss",
            "src/marlin/deck-runtime-strategy.ss",
            "src/marlin/extra-policy.ss",
        ],
    );

    let receipt = inspect_gerbil_runtime_assets(tempdir.path());

    assert_eq!(receipt.status, GerbilRuntimeAssetManifestStatus::Complete);
    assert!(receipt.is_success());
    assert!(receipt.has_runtime_assets());
    assert_eq!(receipt.asset_count, 15);
    assert_eq!(receipt.package_manifest_count, 1);
    assert_eq!(receipt.scheme_source_count, 12);
    assert!(receipt.missing_required_assets.is_empty());
}

#[test]
fn gerbil_runtime_asset_receipt_ignores_transient_runtime_cache_dirs() {
    let tempdir = tempfile::tempdir().expect("create temp project");
    write_runtime_assets(
        tempdir.path(),
        &[
            "gerbil.pkg",
            "build.ss",
            "src/marlin/protocol-types.ss",
            "src/marlin/protocol.ss",
            "src/marlin/request.ss",
            "src/marlin/adapter.ss",
            "src/marlin/deck-runtime.ss",
            "src/marlin/_deck-runtime-native.ssi",
            "src/marlin/deck-runtime-native.ss",
            "src/marlin/_agent-policy-routing-native.ssi",
            "src/marlin/deck-runtime-native-projection.ss",
            "src/marlin/graph-loop-continuation-native-projection.ss",
            "src/marlin/deck-runtime-script.ss",
            "src/marlin/deck-runtime-strategy.ss",
            ".run/marlin-harness-policy-negative/gerbil.pkg",
            ".run/marlin-harness-policy-negative/src/macros/core.ss",
            ".direnv/cache.ss",
            ".devenv/cache.ss",
            ".cache/cache.ss",
        ],
    );

    let receipt = inspect_gerbil_runtime_assets(tempdir.path());

    assert_eq!(receipt.status, GerbilRuntimeAssetManifestStatus::Complete);
    assert!(receipt.is_success());
    assert_eq!(receipt.asset_count, 14);
    assert_eq!(receipt.package_manifest_count, 1);
    assert_eq!(receipt.scheme_source_count, 11);
    assert!(
        receipt.asset_paths.iter().all(|path| {
            !path.starts_with(".run/")
                && !path.starts_with(".direnv/")
                && !path.starts_with(".devenv/")
                && !path.starts_with(".cache/")
        }),
        "transient cache paths should not be embedded as runtime assets: {:?}",
        receipt.asset_paths
    );
}

#[test]
fn gerbil_runtime_asset_receipt_rejects_missing_required_runtime_package_files() {
    let tempdir = tempfile::tempdir().expect("create temp project");
    write_runtime_assets(
        tempdir.path(),
        &["gerbil.pkg", "src/marlin/deck-runtime.ss"],
    );

    let receipt = inspect_gerbil_runtime_assets(tempdir.path());

    assert_eq!(
        receipt.status,
        GerbilRuntimeAssetManifestStatus::MissingRequiredAssets
    );
    assert!(!receipt.is_success());
    assert!(receipt.has_runtime_assets());
    assert!(
        receipt
            .missing_required_assets
            .contains(&"build.ss".to_owned())
    );
    assert!(
        receipt
            .missing_required_assets
            .contains(&"src/marlin/adapter.ss".to_owned())
    );
}

#[test]
fn gerbil_runtime_harness_contract_receipt_reports_absent_runtime_root_as_non_blocking() {
    let tempdir = tempfile::tempdir().expect("create temp project");

    let receipt = inspect_gerbil_runtime_harness_contract(tempdir.path());

    assert_eq!(
        receipt.status,
        GerbilRuntimeHarnessContractStatus::NotPresent
    );
    assert!(receipt.is_success());
    assert!(!receipt.has_runtime_assets());
    assert_eq!(
        receipt.runtime_asset_status,
        GerbilRuntimeAssetManifestStatus::NotPresent
    );
    assert_eq!(receipt.asset_count, 0);
}

#[test]
fn gerbil_runtime_harness_contract_receipt_accepts_complete_harness_assets() {
    let tempdir = tempfile::tempdir().expect("create temp project");
    write_runtime_assets(
        tempdir.path(),
        &[
            "gerbil.pkg",
            "build.ss",
            "src/marlin/protocol-types.ss",
            "src/marlin/protocol.ss",
            "src/marlin/request.ss",
            "src/marlin/adapter.ss",
            "src/marlin/deck-runtime.ss",
            "src/marlin/_deck-runtime-native.ssi",
            "src/marlin/deck-runtime-native.ss",
            "src/marlin/_agent-policy-routing-native.ssi",
            "src/marlin/deck-runtime-native-projection.ss",
            "src/marlin/graph-loop-continuation-native-projection.ss",
            "src/marlin/deck-runtime-script.ss",
            "src/marlin/deck-runtime-strategy.ss",
            "src/marlin/deck-runtime-script-performance.ss",
            "src/marlin/deck-runtime-policy-receipt-gate-cli.ss",
            "harness-policy/gerbil.ss",
            "t/all-test.ss",
            "t/deck-runtime-script-performance-test.ss",
            "t/deck-runtime-script-performance-gate.ss",
        ],
    );

    let receipt = inspect_gerbil_runtime_harness_contract(tempdir.path());

    assert_eq!(receipt.status, GerbilRuntimeHarnessContractStatus::Complete);
    assert!(receipt.is_success());
    assert!(receipt.has_runtime_assets());
    assert!(receipt.missing_capabilities.is_empty());
    assert!(
        receipt
            .available_capabilities
            .contains(&"build-script".to_owned())
    );
    assert!(
        receipt
            .available_capabilities
            .contains(&"harness-policy-module".to_owned())
    );
    assert!(
        receipt
            .available_capabilities
            .contains(&"script-performance-gate".to_owned())
    );
}

#[test]
fn gerbil_runtime_harness_contract_receipt_rejects_missing_capability_assets() {
    let tempdir = tempfile::tempdir().expect("create temp project");
    write_runtime_assets(
        tempdir.path(),
        &[
            "gerbil.pkg",
            "build.ss",
            "src/marlin/protocol-types.ss",
            "src/marlin/protocol.ss",
            "src/marlin/request.ss",
            "src/marlin/adapter.ss",
            "src/marlin/deck-runtime.ss",
            "src/marlin/_deck-runtime-native.ssi",
            "src/marlin/deck-runtime-native.ss",
            "src/marlin/_agent-policy-routing-native.ssi",
            "src/marlin/deck-runtime-native-projection.ss",
            "src/marlin/graph-loop-continuation-native-projection.ss",
            "src/marlin/deck-runtime-script.ss",
            "src/marlin/deck-runtime-strategy.ss",
        ],
    );

    let receipt = inspect_gerbil_runtime_harness_contract(tempdir.path());

    assert_eq!(
        receipt.status,
        GerbilRuntimeHarnessContractStatus::MissingRequiredCapabilities
    );
    assert!(!receipt.is_success());
    assert!(
        receipt
            .missing_capabilities
            .contains(&"harness-policy-module".to_owned())
    );
    assert!(
        receipt
            .missing_capabilities
            .contains(&"script-performance-gate".to_owned())
    );
}

#[test]
fn gerbil_runtime_asset_receipt_accepts_real_marlin_gerbil_scheme_package() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("build-support crate lives under workspace/build-support");
    let package_root = workspace_root.join("crates/marlin-gerbil-scheme");

    let receipt = inspect_gerbil_runtime_assets(&package_root);

    assert_eq!(receipt.status, GerbilRuntimeAssetManifestStatus::Complete);
    assert!(receipt.is_success());
    assert!(receipt.has_runtime_assets());
    assert!(receipt.asset_count >= receipt.required_assets.len());
    assert!(
        receipt
            .asset_paths
            .contains(&"src/marlin/adapter.ss".to_owned())
    );
    assert!(
        receipt
            .asset_paths
            .contains(&"src/marlin/_deck-runtime-native.ssi".to_owned())
    );
    assert!(
        receipt
            .asset_paths
            .contains(&"src/marlin/deck-runtime-native.ss".to_owned())
    );
    assert!(
        receipt
            .asset_paths
            .contains(&"src/marlin/_agent-policy-routing-native.ssi".to_owned())
    );
    assert!(
        receipt
            .asset_paths
            .contains(&"src/marlin/deck-runtime-native-projection.ss".to_owned())
    );
    assert!(
        receipt
            .asset_paths
            .contains(&"src/marlin/graph-loop-continuation-native-projection.ss".to_owned())
    );
    assert!(
        receipt
            .asset_paths
            .contains(&"src/marlin/deck-runtime-script.ss".to_owned())
    );
}

#[test]
fn gerbil_runtime_harness_contract_receipt_accepts_real_marlin_gerbil_scheme_package() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("build-support crate lives under workspace/build-support");
    let package_root = workspace_root.join("crates/marlin-gerbil-scheme");

    let receipt = inspect_gerbil_runtime_harness_contract(&package_root);

    assert_eq!(receipt.status, GerbilRuntimeHarnessContractStatus::Complete);
    assert!(receipt.is_success());
    assert!(receipt.missing_capabilities.is_empty());
    assert!(
        receipt
            .available_capabilities
            .contains(&"policy-receipt-gate-cli-source".to_owned())
    );
}

fn write_runtime_assets(project_root: &Path, paths: &[&str]) {
    for path in paths {
        let full_path = project_root.join("gerbil").join(path);
        fs::create_dir_all(full_path.parent().expect("asset path has parent"))
            .expect("create asset parent");
        fs::write(full_path, ";; test asset\n").expect("write asset");
    }
}
