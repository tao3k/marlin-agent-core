use super::support::{gerbil_backend_failure_receipt, test_root};
use marlin_gerbil_scheme::{GerbilAotBackendRepairStatus, GerbilAotBackendShimStatus};
use std::fs;

#[test]
fn gerbil_aot_backend_repair_plan_reports_repo_shim_available_without_writing() {
    let root = test_root("aot-backend-repair-plan");
    fs::create_dir_all(root.path()).expect("create aot backend repair root");
    let gsc = root.path().join("real-gsc");
    let backend_gsc = root
        .path()
        .join("gerbil")
        .join("v0.18.2")
        .join("bin")
        .join("gsc");
    fs::write(&gsc, "#!/bin/sh\nexit 0\n").expect("write configured gsc");

    let receipt = gerbil_backend_failure_receipt(root.path(), &gsc, &backend_gsc);
    let plan = receipt
        .plan_backend_gsc_repair(root.path())
        .expect("plan backend gsc repair");

    assert_eq!(plan.status, GerbilAotBackendRepairStatus::RepoShimAvailable);
    assert!(plan.can_create_shim);
    assert!(!plan.requires_system_write);
    assert!(!backend_gsc.exists());
}

#[test]
fn gerbil_aot_backend_repair_plan_reports_system_write_requirement() {
    let root = test_root("aot-backend-repair-system");
    fs::create_dir_all(root.path()).expect("create aot backend repair root");
    let gsc = root.path().join("real-gsc");
    let backend_gsc = root
        .path()
        .join("outside")
        .join("v0.18.2")
        .join("bin")
        .join("gsc");
    let allowed = root.path().join("allowed");
    fs::write(&gsc, "#!/bin/sh\nexit 0\n").expect("write configured gsc");

    let receipt = gerbil_backend_failure_receipt(root.path(), &gsc, &backend_gsc);
    let plan = receipt
        .plan_backend_gsc_repair(&allowed)
        .expect("plan backend gsc repair");

    assert_eq!(
        plan.status,
        GerbilAotBackendRepairStatus::RequiresSystemWrite
    );
    assert!(!plan.can_create_shim);
    assert!(plan.requires_system_write);
    assert!(
        plan.recommended_action
            .contains("authorize a system-level shim")
    );
    assert!(!backend_gsc.exists());
}

#[cfg(unix)]
#[test]
fn gerbil_aot_backend_shim_links_configured_gsc_within_allowed_root() {
    let root = test_root("aot-backend-shim");
    fs::create_dir_all(root.path()).expect("create aot backend shim root");
    let gsc = root.path().join("real-gsc");
    let backend_gsc = root
        .path()
        .join("gerbil")
        .join("v0.18.2")
        .join("bin")
        .join("gsc");
    fs::write(&gsc, "#!/bin/sh\nexit 0\n").expect("write configured gsc");

    let receipt = gerbil_backend_failure_receipt(root.path(), &gsc, &backend_gsc);
    let shim = receipt
        .prepare_backend_gsc_shim(root.path())
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
        .prepare_backend_gsc_shim(root.path())
        .expect("prepare already-ready backend gsc shim");
    assert_eq!(
        already_ready.status,
        GerbilAotBackendShimStatus::AlreadyReady
    );
}

#[test]
fn gerbil_aot_backend_shim_refuses_paths_outside_allowed_root() {
    let root = test_root("aot-backend-shim-outside");
    fs::create_dir_all(root.path()).expect("create aot backend shim root");
    let gsc = root.path().join("real-gsc");
    let backend_gsc = root
        .path()
        .join("outside")
        .join("v0.18.2")
        .join("bin")
        .join("gsc");
    let allowed = root.path().join("allowed");
    fs::write(&gsc, "#!/bin/sh\nexit 0\n").expect("write configured gsc");

    let receipt = gerbil_backend_failure_receipt(root.path(), &gsc, &backend_gsc);
    let shim = receipt
        .prepare_backend_gsc_shim(&allowed)
        .expect("prepare refused backend gsc shim");
    assert_eq!(shim.status, GerbilAotBackendShimStatus::OutsideAllowedRoot);
    assert!(!backend_gsc.exists());
}

#[test]
fn gerbil_aot_backend_shim_refuses_parent_dir_escape_from_allowed_root() {
    let root = test_root("aot-backend-shim-parent-dir");
    fs::create_dir_all(root.path()).expect("create aot backend shim root");
    let gsc = root.path().join("real-gsc");
    let allowed = root.path().join("allowed");
    let escaped_backend_gsc = allowed
        .join("..")
        .join("outside")
        .join("v0.18.2")
        .join("bin")
        .join("gsc");
    fs::write(&gsc, "#!/bin/sh\nexit 0\n").expect("write configured gsc");

    let receipt = gerbil_backend_failure_receipt(root.path(), &gsc, &escaped_backend_gsc);
    let shim = receipt
        .prepare_backend_gsc_shim(&allowed)
        .expect("prepare refused escaped backend gsc shim");
    assert_eq!(shim.status, GerbilAotBackendShimStatus::OutsideAllowedRoot);
    assert!(!root.path().join("outside").exists());
}
