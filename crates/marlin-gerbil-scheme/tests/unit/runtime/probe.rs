use super::support::{line_count, test_root};
use marlin_gerbil_scheme::{GerbilAotProbeConfig, GerbilAotProbeStatus};
use std::fs;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[test]
fn gerbil_aot_probe_reports_missing_gxc_without_writing_assets() {
    let root = test_root("aot-missing-gxc");
    let missing_gxc = root.join("missing-gxc");
    let missing_gsc = root.join("missing-gsc");

    let receipt = GerbilAotProbeConfig::new(&root)
        .with_gxc(&missing_gxc)
        .with_gsc(&missing_gsc)
        .probe();

    assert_eq!(receipt.status, GerbilAotProbeStatus::MissingGxc);
    assert_eq!(receipt.gxc, missing_gxc);
    assert_eq!(receipt.gsc, missing_gsc);
    assert_eq!(receipt.backend_gsc, None);
    assert!(!root.join("command-adapter.ss").exists());
}

#[test]
fn gerbil_aot_probe_reports_missing_gsc_before_compile() {
    let root = test_root("aot-missing-gsc");
    fs::create_dir_all(&root).expect("create aot root");
    let fake_gxc = root.join("gxc");
    fs::write(&fake_gxc, "#!/bin/sh\nexit 0\n").expect("write fake gxc");
    let missing_gsc = root.join("missing-gsc");

    let receipt = GerbilAotProbeConfig::new(&root)
        .with_gxc(&fake_gxc)
        .with_gsc(&missing_gsc)
        .probe();

    assert_eq!(receipt.status, GerbilAotProbeStatus::MissingGsc);
    assert_eq!(receipt.gxc, fake_gxc);
    assert_eq!(receipt.gsc, missing_gsc);
    assert_eq!(receipt.backend_gsc, None);
    assert!(receipt.module_compile.is_none());
    let _ = fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn gerbil_aot_probe_reports_missing_backend_gsc_path() {
    let root = test_root("aot-backend-gsc");
    fs::create_dir_all(&root).expect("create aot root");
    let fake_gxc = root.join("gxc");
    let fake_gsc = root.join("gsc");
    let expected_backend_gsc = root.join("gerbil").join("v0.18.2").join("bin").join("gsc");
    fs::write(&fake_gsc, "#!/bin/sh\nexit 0\n").expect("write fake gsc");
    fs::write(
        &fake_gxc,
        format!(
            "#!/bin/sh\ncat <<'EOF'\n*** ERROR IN gxc#gsc-compile-file -- No such file or directory\n(open-process '(path: \"{}\" arguments: (\"-target\" \"C\" \"protocol~0.scm\")))\nEOF\nexit 70\n",
            expected_backend_gsc.display()
        ),
    )
    .expect("write fake gxc");
    let mut permissions = fs::metadata(&fake_gxc)
        .expect("fake gxc metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&fake_gxc, permissions).expect("mark fake gxc executable");

    let receipt = GerbilAotProbeConfig::new(&root)
        .with_gxc(&fake_gxc)
        .with_gsc(&fake_gsc)
        .probe();

    assert_eq!(receipt.status, GerbilAotProbeStatus::GscBackendUnavailable);
    assert_eq!(
        receipt.backend_gsc.as_deref(),
        Some(expected_backend_gsc.as_path())
    );
    assert_eq!(
        receipt
            .module_compile
            .as_ref()
            .and_then(|compile| compile.status_code),
        Some(70)
    );
    assert!(receipt.executable_compile.is_none());
    let _ = fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn gerbil_aot_probe_cache_reuses_backend_failure_until_backend_exists() {
    let root = test_root("aot-cache-backend-gsc");
    fs::create_dir_all(&root).expect("create aot cache root");
    let fake_gxc = root.join("gxc");
    let fake_gsc = root.join("gsc");
    let invocations = root.join("gxc-invocations");
    let cache = root.join("probe-cache.json");
    let first_probe_root = root.join("first-probe");
    let second_probe_root = root.join("second-probe");
    let third_probe_root = root.join("third-probe");
    let expected_backend_gsc = root.join("gerbil").join("v0.18.2").join("bin").join("gsc");

    fs::write(&fake_gsc, "#!/bin/sh\nexit 0\n").expect("write fake gsc");
    fs::write(
        &fake_gxc,
        format!(
            "#!/bin/sh\nprintf '1\\n' >> '{}'\ncat <<'EOF'\n*** ERROR IN gxc#gsc-compile-file -- No such file or directory\n(open-process '(path: \"{}\" arguments: (\"-target\" \"C\" \"protocol~0.scm\")))\nEOF\nexit 70\n",
            invocations.display(),
            expected_backend_gsc.display()
        ),
    )
    .expect("write fake gxc");
    let mut permissions = fs::metadata(&fake_gxc)
        .expect("fake gxc metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&fake_gxc, permissions).expect("mark fake gxc executable");

    let first = GerbilAotProbeConfig::new(&first_probe_root)
        .with_gxc(&fake_gxc)
        .with_gsc(&fake_gsc)
        .probe_with_toolchain_cache(&cache)
        .expect("first cached probe");
    assert_eq!(first.status, GerbilAotProbeStatus::GscBackendUnavailable);
    assert!(first.module_compile.is_some());
    assert_eq!(
        first.backend_gsc.as_deref(),
        Some(expected_backend_gsc.as_path())
    );
    assert_eq!(line_count(&invocations), 1);

    let second = GerbilAotProbeConfig::new(&second_probe_root)
        .with_gxc(&fake_gxc)
        .with_gsc(&fake_gsc)
        .probe_with_toolchain_cache(&cache)
        .expect("second cached probe");
    assert_eq!(second.status, GerbilAotProbeStatus::GscBackendUnavailable);
    assert_eq!(second.root, second_probe_root);
    assert!(
        second
            .detail
            .as_deref()
            .is_some_and(|detail| detail.contains("cached"))
    );
    assert!(second.module_compile.is_none());
    assert_eq!(line_count(&invocations), 1);

    fs::create_dir_all(expected_backend_gsc.parent().expect("backend parent"))
        .expect("create backend parent");
    fs::write(&expected_backend_gsc, "#!/bin/sh\nexit 0\n").expect("write backend gsc");
    let third = GerbilAotProbeConfig::new(&third_probe_root)
        .with_gxc(&fake_gxc)
        .with_gsc(&fake_gsc)
        .probe_with_toolchain_cache(&cache)
        .expect("third cached probe");
    assert_eq!(third.status, GerbilAotProbeStatus::GscBackendUnavailable);
    assert!(third.module_compile.is_some());
    assert_eq!(line_count(&invocations), 2);

    let _ = fs::remove_dir_all(root);
}

#[test]
#[ignore = "requires local Gerbil gxc/gsc toolchain"]
fn gerbil_aot_probe_reports_local_toolchain_status() {
    let root = test_root("aot-local-toolchain");

    let receipt = GerbilAotProbeConfig::new(&root).probe();
    eprintln!("{receipt:?}");

    assert_ne!(receipt.status, GerbilAotProbeStatus::MissingGxc);
    if receipt.status == GerbilAotProbeStatus::GscBackendUnavailable {
        assert!(receipt.backend_gsc.is_some());
    }
    if let Some(module_compile) = &receipt.module_compile {
        assert!(
            module_compile.stdout.contains("ERROR")
                || module_compile.stderr.contains("ERROR")
                || module_compile.status_code == Some(0)
        );
    }
    let _ = fs::remove_dir_all(root);
}
