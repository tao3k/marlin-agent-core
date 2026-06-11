use marlin_gerbil_scheme::{
    DEFAULT_GERBIL_GSC_PROGRAM, DEFAULT_GERBIL_GXC_PROGRAM, DEFAULT_GERBIL_GXI_PROGRAM,
    GERBIL_ADAPTER_MODULE, GERBIL_BUILD_SOURCE, GERBIL_LOADPATH_ENV, GERBIL_MARLIN_ADAPTER_SOURCE,
    GERBIL_MARLIN_REQUEST_SOURCE, GerbilAotProbeConfig, GerbilAotProbeStatus,
    MARLIN_GERBIL_GSC_ENV, MARLIN_GERBIL_GXC_ENV, MARLIN_GERBIL_GXI_ENV, gerbil_runtime_assets,
    write_gerbil_runtime_assets,
};
use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn gerbil_runtime_assets_expose_loadpath_contract() {
    let assets = gerbil_runtime_assets();

    assert_eq!(GERBIL_LOADPATH_ENV, "GERBIL_LOADPATH");
    assert_eq!(GERBIL_ADAPTER_MODULE, ":marlin/adapter");
    assert_eq!(MARLIN_GERBIL_GXI_ENV, "MARLIN_GERBIL_GXI");
    assert_eq!(MARLIN_GERBIL_GXC_ENV, "MARLIN_GERBIL_GXC");
    assert_eq!(MARLIN_GERBIL_GSC_ENV, "MARLIN_GERBIL_GSC");
    assert!(DEFAULT_GERBIL_GXI_PROGRAM.ends_with("/bin/gxi"));
    assert!(DEFAULT_GERBIL_GXC_PROGRAM.ends_with("/bin/gxc"));
    assert!(DEFAULT_GERBIL_GSC_PROGRAM.ends_with("/bin/gsc"));
    assert!(GERBIL_BUILD_SOURCE.contains("defmarlin-runtime-build-script"));
    assert!(GERBIL_MARLIN_REQUEST_SOURCE.contains("gerbil-compile-request-contract-facts"));
    assert!(GERBIL_MARLIN_ADAPTER_SOURCE.contains("ensure-marlin-contract-facts-shape"));
    assert_eq!(assets.len(), 8);
    assert!(
        assets
            .iter()
            .any(|asset| asset.path == "command-adapter.ss")
    );
    assert!(
        assets
            .iter()
            .any(|asset| asset.path == "command-adapter-batch.ss")
    );
    assert!(assets.iter().any(|asset| asset.path == "build.ss"));
    assert!(assets.iter().any(|asset| asset.path == "smoke.ss"));
    assert!(assets.iter().any(|asset| asset.path == "marlin/adapter.ss"));
    assert!(assets.iter().any(|asset| asset.path == "marlin/parser.ss"));
    assert!(
        assets
            .iter()
            .any(|asset| asset.path == "marlin/protocol.ss")
    );
    assert!(assets.iter().any(|asset| asset.path == "marlin/request.ss"));
}

#[test]
fn gerbil_runtime_assets_write_loadpath_tree() {
    let root = test_root("runtime-assets");

    let written = write_gerbil_runtime_assets(&root).expect("write runtime assets");

    assert_eq!(written.len(), gerbil_runtime_assets().len());
    assert!(root.join("command-adapter.ss").is_file());
    assert!(root.join("build.ss").is_file());
    assert!(root.join("marlin/adapter.ss").is_file());
    assert!(
        fs::read_to_string(root.join("build.ss"))
            .expect("read build script")
            .contains("defmarlin-runtime-build-script")
    );
    assert!(
        fs::read_to_string(root.join("marlin/protocol.ss"))
            .expect("read protocol")
            .contains("marlin-workspace-patch-intent-artifact-kind")
    );
    let _ = fs::remove_dir_all(root);
}

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
    assert!(receipt.module_compile.is_none());
    let _ = fs::remove_dir_all(root);
}

#[test]
#[ignore = "requires local Gerbil gxc/gsc toolchain"]
fn gerbil_aot_probe_reports_local_toolchain_status() {
    let root = test_root("aot-local-toolchain");

    let receipt = GerbilAotProbeConfig::new(&root).probe();
    eprintln!("{receipt:?}");

    assert_ne!(receipt.status, GerbilAotProbeStatus::MissingGxc);
    if let Some(module_compile) = &receipt.module_compile {
        assert!(
            module_compile.stdout.contains("ERROR")
                || module_compile.stderr.contains("ERROR")
                || module_compile.status_code == Some(0)
        );
    }
    let _ = fs::remove_dir_all(root);
}

fn test_root(name: &str) -> std::path::PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    std::env::temp_dir().join(format!(
        "marlin-gerbil-scheme-{name}-{}-{suffix}",
        std::process::id()
    ))
}
