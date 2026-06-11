use super::support::test_root;
use marlin_gerbil_scheme::{
    DEFAULT_GERBIL_GSC_PROGRAM, DEFAULT_GERBIL_GXC_PROGRAM, DEFAULT_GERBIL_GXI_PROGRAM,
    GERBIL_ADAPTER_MODULE, GERBIL_BUILD_SOURCE, GERBIL_LOADPATH_ENV, GERBIL_MARLIN_ADAPTER_SOURCE,
    GERBIL_MARLIN_REQUEST_SOURCE, MARLIN_GERBIL_GSC_ENV, MARLIN_GERBIL_GXC_ENV,
    MARLIN_GERBIL_GXI_ENV, gerbil_runtime_assets, write_gerbil_runtime_assets,
};
use std::fs;

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
