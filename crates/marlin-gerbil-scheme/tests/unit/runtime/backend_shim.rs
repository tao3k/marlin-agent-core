use super::support::{gerbil_backend_failure_receipt, test_root};
use marlin_gerbil_scheme::GerbilAotBackendShimStatus;
use std::fs;

#[cfg(unix)]
#[test]
fn gerbil_aot_backend_shim_links_configured_gsc_within_allowed_root() {
    let root = test_root("aot-backend-shim");
    fs::create_dir_all(&root).expect("create aot backend shim root");
    let gsc = root.join("real-gsc");
    let backend_gsc = root.join("gerbil").join("v0.18.2").join("bin").join("gsc");
    fs::write(&gsc, "#!/bin/sh\nexit 0\n").expect("write configured gsc");

    let receipt = gerbil_backend_failure_receipt(&root, &gsc, &backend_gsc);
    let shim = receipt
        .prepare_backend_gsc_shim(&root)
        .expect("prepare backend gsc shim");
    assert_eq!(shim.status, GerbilAotBackendShimStatus::Created);
    assert_eq!(shim.backend_gsc.as_deref(), Some(backend_gsc.as_path()));
    assert!(backend_gsc.is_file());
    assert!(
        fs::symlink_metadata(&backend_gsc)
            .expect("backend shim metadata")
            .file_type()
            .is_symlink()
    );

    let already_ready = receipt
        .prepare_backend_gsc_shim(&root)
        .expect("prepare already-ready backend gsc shim");
    assert_eq!(
        already_ready.status,
        GerbilAotBackendShimStatus::AlreadyReady
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn gerbil_aot_backend_shim_refuses_paths_outside_allowed_root() {
    let root = test_root("aot-backend-shim-outside");
    fs::create_dir_all(&root).expect("create aot backend shim root");
    let gsc = root.join("real-gsc");
    let backend_gsc = root.join("outside").join("v0.18.2").join("bin").join("gsc");
    let allowed = root.join("allowed");
    fs::write(&gsc, "#!/bin/sh\nexit 0\n").expect("write configured gsc");

    let receipt = gerbil_backend_failure_receipt(&root, &gsc, &backend_gsc);
    let shim = receipt
        .prepare_backend_gsc_shim(&allowed)
        .expect("prepare refused backend gsc shim");
    assert_eq!(shim.status, GerbilAotBackendShimStatus::OutsideAllowedRoot);
    assert!(!backend_gsc.exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn gerbil_aot_backend_shim_refuses_parent_dir_escape_from_allowed_root() {
    let root = test_root("aot-backend-shim-parent-dir");
    fs::create_dir_all(&root).expect("create aot backend shim root");
    let gsc = root.join("real-gsc");
    let allowed = root.join("allowed");
    let escaped_backend_gsc = allowed
        .join("..")
        .join("outside")
        .join("v0.18.2")
        .join("bin")
        .join("gsc");
    fs::write(&gsc, "#!/bin/sh\nexit 0\n").expect("write configured gsc");

    let receipt = gerbil_backend_failure_receipt(&root, &gsc, &escaped_backend_gsc);
    let shim = receipt
        .prepare_backend_gsc_shim(&allowed)
        .expect("prepare refused escaped backend gsc shim");
    assert_eq!(shim.status, GerbilAotBackendShimStatus::OutsideAllowedRoot);
    assert!(!root.join("outside").exists());
    let _ = fs::remove_dir_all(root);
}
