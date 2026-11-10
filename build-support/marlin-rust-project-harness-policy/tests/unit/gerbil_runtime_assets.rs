use std::{fs, path::Path};

use marlin_rust_project_harness_policy::{
    GerbilRuntimeAssetManifestStatus, GerbilRuntimeHarnessContractStatus,
    inspect_gerbil_runtime_assets, inspect_gerbil_runtime_harness_contract,
};

use crate::workspace::workspace_root;

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
            "init.ss",
            "policy/interface.ss",
            "c-abi/_deck-runtime-native.ssi",
            "c-abi/deck-runtime-native.ss",
            "c-abi/_agent-policy-routing-native.ssi",
            "policy/projection/deck.ss",
            "policy/projection/graph-continuation.ss",
            "policy/projection/script.ss",
            "policy/strategy.ss",
            "policy/extra-policy.ss",
        ],
    );

    let receipt = inspect_gerbil_runtime_assets(tempdir.path());

    assert_eq!(receipt.status, GerbilRuntimeAssetManifestStatus::Complete);
    assert!(receipt.is_success());
    assert!(receipt.has_runtime_assets());
    assert_eq!(receipt.asset_count, 12);
    assert_eq!(receipt.package_manifest_count, 1);
    assert_eq!(receipt.scheme_source_count, 9);
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
            "init.ss",
            "policy/interface.ss",
            "c-abi/_deck-runtime-native.ssi",
            "c-abi/deck-runtime-native.ss",
            "c-abi/_agent-policy-routing-native.ssi",
            "policy/projection/deck.ss",
            "policy/projection/graph-continuation.ss",
            "policy/projection/script.ss",
            "policy/strategy.ss",
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
    assert_eq!(receipt.asset_count, 11);
    assert_eq!(receipt.package_manifest_count, 1);
    assert_eq!(receipt.scheme_source_count, 8);
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
    write_runtime_assets(tempdir.path(), &["gerbil.pkg", "policy/interface.ss"]);

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
            .contains(&"init.ss".to_owned())
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
            "init.ss",
            "policy/interface.ss",
            "c-abi/_deck-runtime-native.ssi",
            "c-abi/deck-runtime-native.ss",
            "c-abi/_agent-policy-routing-native.ssi",
            "policy/projection/deck.ss",
            "policy/projection/graph-continuation.ss",
            "policy/projection/script.ss",
            "policy/strategy.ss",
            "t/support/script-performance.ss",
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
            .contains(&"test-entrypoint".to_owned())
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
            "init.ss",
            "policy/interface.ss",
            "c-abi/_deck-runtime-native.ssi",
            "c-abi/deck-runtime-native.ss",
            "c-abi/_agent-policy-routing-native.ssi",
            "policy/projection/deck.ss",
            "policy/projection/graph-continuation.ss",
            "policy/projection/script.ss",
            "policy/strategy.ss",
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
            .contains(&"test-entrypoint".to_owned())
    );
    assert!(
        receipt
            .missing_capabilities
            .contains(&"script-performance-gate".to_owned())
    );
}

#[test]
fn gerbil_runtime_asset_receipt_accepts_real_marlin_gerbil_scheme_package() {
    let workspace_root = workspace_root();
    let package_root = workspace_root.join("crates/marlin-gerbil-scheme");

    let receipt = inspect_gerbil_runtime_assets(&package_root);

    assert_eq!(receipt.status, GerbilRuntimeAssetManifestStatus::Complete);
    assert!(receipt.is_success());
    assert!(receipt.has_runtime_assets());
    assert!(receipt.asset_count >= receipt.required_assets.len());
    assert!(receipt.asset_paths.contains(&"init.ss".to_owned()));
    assert!(
        receipt
            .asset_paths
            .contains(&"c-abi/_deck-runtime-native.ssi".to_owned())
    );
    assert!(
        receipt
            .asset_paths
            .contains(&"c-abi/deck-runtime-native.ss".to_owned())
    );
    assert!(
        receipt
            .asset_paths
            .contains(&"c-abi/_agent-policy-routing-native.ssi".to_owned())
    );
    assert!(
        receipt
            .asset_paths
            .contains(&"policy/projection/deck.ss".to_owned())
    );
    assert!(
        receipt
            .asset_paths
            .contains(&"policy/projection/graph-continuation.ss".to_owned())
    );
    assert!(
        receipt
            .asset_paths
            .contains(&"policy/projection/script.ss".to_owned())
    );
}

#[test]
fn gerbil_runtime_harness_contract_receipt_accepts_real_marlin_gerbil_scheme_package() {
    let workspace_root = workspace_root();
    let package_root = workspace_root.join("crates/marlin-gerbil-scheme");

    let receipt = inspect_gerbil_runtime_harness_contract(&package_root);

    assert_eq!(receipt.status, GerbilRuntimeHarnessContractStatus::Complete);
    assert!(receipt.is_success());
    assert!(receipt.missing_capabilities.is_empty());
    assert!(
        receipt
            .available_capabilities
            .contains(&"test-entrypoint".to_owned())
    );
}

fn write_runtime_assets(project_root: &Path, paths: &[&str]) {
    for path in paths {
        let full_path = project_root.join("scheme").join(path);
        fs::create_dir_all(full_path.parent().expect("asset path has parent"))
            .expect("create asset parent");
        fs::write(full_path, ";; test asset\n").expect("write asset");
    }
}
