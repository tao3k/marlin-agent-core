use std::{fs, path::Path};

use marlin_rust_project_harness_policy::{
    GerbilRuntimeAssetManifestStatus, inspect_gerbil_runtime_assets,
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
            "bin/command-adapter-batch.ss",
            "src/marlin/deck-runtime.ss",
            "src/marlin/deck-runtime-native.ss",
            "src/marlin/hook-policy.ss",
            "src/marlin/extra-policy.ss",
        ],
    );

    let receipt = inspect_gerbil_runtime_assets(tempdir.path());

    assert_eq!(receipt.status, GerbilRuntimeAssetManifestStatus::Complete);
    assert!(receipt.is_success());
    assert!(receipt.has_runtime_assets());
    assert_eq!(receipt.asset_count, 7);
    assert_eq!(receipt.package_manifest_count, 1);
    assert_eq!(receipt.scheme_source_count, 6);
    assert!(receipt.missing_required_assets.is_empty());
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
            .contains(&"bin/command-adapter-batch.ss".to_owned())
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
            .contains(&"bin/command-adapter-batch.ss".to_owned())
    );
    assert!(
        receipt
            .asset_paths
            .contains(&"src/marlin/deck-runtime-native.ss".to_owned())
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
