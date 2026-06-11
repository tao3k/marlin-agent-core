use marlin_gerbil_scheme::{
    GERBIL_ADAPTER_MODULE, GERBIL_LOADPATH_ENV, gerbil_runtime_assets, write_gerbil_runtime_assets,
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
    assert_eq!(assets.len(), 6);
    assert!(
        assets
            .iter()
            .any(|asset| asset.path == "command-adapter.ss")
    );
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
    assert!(root.join("marlin/adapter.ss").is_file());
    assert!(
        fs::read_to_string(root.join("marlin/protocol.ss"))
            .expect("read protocol")
            .contains("marlin-workspace-patch-intent-artifact-kind")
    );
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
