use super::support::test_root;
use marlin_gerbil_scheme::{
    DEFAULT_GERBIL_GSC_PROGRAM, DEFAULT_GERBIL_GXC_PROGRAM, DEFAULT_GERBIL_GXI_PROGRAM,
    GERBIL_ADAPTER_MODULE, GERBIL_BUILD_SOURCE, GERBIL_LOADPATH_ENV, GERBIL_MARLIN_ADAPTER_PATH,
    GERBIL_MARLIN_ADAPTER_SOURCE, GERBIL_MARLIN_DECK_RUNTIME_COMPILED_POLICY_PATH,
    GERBIL_MARLIN_DECK_RUNTIME_COMPILED_POLICY_SAMPLE_PATH,
    GERBIL_MARLIN_DECK_RUNTIME_COMPILED_POLICY_SAMPLE_SOURCE,
    GERBIL_MARLIN_DECK_RUNTIME_COMPILED_POLICY_SOURCE, GERBIL_MARLIN_DECK_RUNTIME_NATIVE_PATH,
    GERBIL_MARLIN_DECK_RUNTIME_NATIVE_PROJECTION_PATH,
    GERBIL_MARLIN_DECK_RUNTIME_NATIVE_PROJECTION_SOURCE, GERBIL_MARLIN_DECK_RUNTIME_NATIVE_SOURCE,
    GERBIL_MARLIN_DECK_RUNTIME_PATH, GERBIL_MARLIN_DECK_RUNTIME_SCRIPT_PATH,
    GERBIL_MARLIN_DECK_RUNTIME_SCRIPT_SOURCE, GERBIL_MARLIN_DECK_RUNTIME_SOURCE,
    GERBIL_MARLIN_PROTOCOL_PATH, GERBIL_MARLIN_REQUEST_SOURCE, GERBIL_PACKAGE_MANIFEST_PATH,
    GERBIL_PACKAGE_MANIFEST_SOURCE, GERBIL_PACKAGE_SOURCE_PATH, GERBIL_POO_DEPENDENCY,
    GERBIL_POO_MOP_MODULE, GERBIL_POO_OBJECT_MODULE, GERBIL_POO_PACKAGE_NAME,
    GERBIL_POO_PROTO_MODULE, MARLIN_GERBIL_GSC_ENV, MARLIN_GERBIL_GXC_ENV, MARLIN_GERBIL_GXI_ENV,
    gerbil_runtime_assets, gerbil_runtime_loadpath, write_gerbil_runtime_assets,
};
use std::{fs, path::Path};

#[test]
fn gerbil_runtime_assets_expose_loadpath_contract() {
    let assets = gerbil_runtime_assets();
    let build_has_target = |target: &str| GERBIL_BUILD_SOURCE.contains(&format!("\"{target}\""));

    assert_eq!(GERBIL_LOADPATH_ENV, "GERBIL_LOADPATH");
    assert_eq!(GERBIL_ADAPTER_MODULE, ":marlin/adapter");
    assert_eq!(MARLIN_GERBIL_GXI_ENV, "MARLIN_GERBIL_GXI");
    assert_eq!(MARLIN_GERBIL_GXC_ENV, "MARLIN_GERBIL_GXC");
    assert_eq!(MARLIN_GERBIL_GSC_ENV, "MARLIN_GERBIL_GSC");
    assert!(DEFAULT_GERBIL_GXI_PROGRAM.ends_with("/bin/gxi"));
    assert!(DEFAULT_GERBIL_GXC_PROGRAM.ends_with("/bin/gxc"));
    assert!(DEFAULT_GERBIL_GSC_PROGRAM.ends_with("/bin/gsc"));
    assert_eq!(GERBIL_PACKAGE_SOURCE_PATH, "src");
    assert!(GERBIL_PACKAGE_MANIFEST_SOURCE.contains("marlin-deck-runtime"));
    assert_eq!(
        GERBIL_POO_DEPENDENCY,
        "git.cons.io/mighty-gerbils/gerbil-poo"
    );
    assert_eq!(GERBIL_POO_PACKAGE_NAME, "clan/poo");
    assert_eq!(GERBIL_POO_OBJECT_MODULE, ":clan/poo/object");
    assert_eq!(GERBIL_POO_MOP_MODULE, ":clan/poo/mop");
    assert_eq!(GERBIL_POO_PROTO_MODULE, ":clan/poo/proto");
    assert!(GERBIL_PACKAGE_MANIFEST_SOURCE.contains(GERBIL_POO_DEPENDENCY));
    assert!(GERBIL_BUILD_SOURCE.contains("defmarlin-runtime-build-script"));
    assert!(build_has_target("src/marlin/deck-runtime-native"));
    assert!(build_has_target(
        "src/marlin/deck-runtime-native-projection"
    ));
    assert!(build_has_target("src/marlin/deck-runtime-script"));
    assert!(build_has_target("src/marlin/deck-runtime"));
    assert!(build_has_target("src/marlin/deck-runtime-compiled-policy"));
    assert!(build_has_target(
        "src/marlin/deck-runtime-compiled-policy-sample"
    ));
    assert!(build_has_target("src/marlin/deck-runtime-strategy"));
    assert!(build_has_target("src/marlin/deck-runtime-policy-engine"));
    assert!(!build_has_target("src/marlin/deck-runtime-policy"));
    assert!(!build_has_target("src/marlin/hook-policy"));
    assert!(GERBIL_MARLIN_REQUEST_SOURCE.contains("gerbil-compile-request-contract-facts"));
    assert!(GERBIL_MARLIN_ADAPTER_SOURCE.contains("ensure-marlin-contract-facts-shape"));
    assert!(GERBIL_MARLIN_DECK_RUNTIME_SOURCE.contains("poo-object-system"));
    assert!(GERBIL_MARLIN_DECK_RUNTIME_SOURCE.contains("scheme-policy-runtime"));
    assert!(GERBIL_MARLIN_DECK_RUNTIME_SOURCE.contains("scheme-compiled-policy-macro"));
    assert!(GERBIL_MARLIN_DECK_RUNTIME_SOURCE.contains("scheme-complex-strategy"));
    assert!(GERBIL_MARLIN_DECK_RUNTIME_SOURCE.contains("model-route-policy"));
    assert!(
        GERBIL_MARLIN_DECK_RUNTIME_SOURCE.contains("make-marlin-deck-runtime-model-route-policy")
    );
    assert!(
        GERBIL_MARLIN_DECK_RUNTIME_SOURCE.contains("marlin-deck-runtime-select-model-route-policy")
    );
    assert!(GERBIL_MARLIN_DECK_RUNTIME_SOURCE.contains(GERBIL_POO_OBJECT_MODULE));
    assert!(
        GERBIL_MARLIN_DECK_RUNTIME_SOURCE.contains("marlin-deck-runtime-model-route-selection")
    );
    assert!(GERBIL_MARLIN_DECK_RUNTIME_SOURCE.contains("typed-native-abi"));
    assert!(!GERBIL_MARLIN_DECK_RUNTIME_SOURCE.contains("json-handshake"));
    assert!(!GERBIL_MARLIN_DECK_RUNTIME_SOURCE.contains("json-handshake"));
    assert!(!GERBIL_MARLIN_DECK_RUNTIME_SOURCE.contains("selection-json"));
    assert!(
        GERBIL_MARLIN_DECK_RUNTIME_COMPILED_POLICY_SOURCE
            .contains("defmarlin-deck-runtime-compiled-route-selector")
    );
    assert!(
        GERBIL_MARLIN_DECK_RUNTIME_COMPILED_POLICY_SOURCE
            .contains("defmarlin-deck-runtime-cached-compiled-route-index-selector")
    );
    assert!(
        GERBIL_MARLIN_DECK_RUNTIME_COMPILED_POLICY_SOURCE
            .contains("defmarlin-deck-runtime-direct-compiled-route-index-selector")
    );
    assert!(GERBIL_MARLIN_DECK_RUNTIME_COMPILED_POLICY_SOURCE.contains("compiled-macro-selector"));
    assert!(GERBIL_MARLIN_DECK_RUNTIME_COMPILED_POLICY_SOURCE.contains("policy-index-selector"));
    assert!(
        GERBIL_MARLIN_DECK_RUNTIME_COMPILED_POLICY_SOURCE.contains("direct-policy-index-selector")
    );
    assert!(
        GERBIL_MARLIN_DECK_RUNTIME_COMPILED_POLICY_SAMPLE_SOURCE
            .contains("select-marlin-deck-runtime-sample-compiled-policy")
    );
    assert!(
        GERBIL_MARLIN_DECK_RUNTIME_COMPILED_POLICY_SAMPLE_SOURCE
            .contains("select-marlin-deck-runtime-sample-compiled-policy-index")
    );
    assert!(GERBIL_MARLIN_DECK_RUNTIME_COMPILED_POLICY_SAMPLE_SOURCE.contains("index_elapsed_us"));
    assert!(GERBIL_MARLIN_DECK_RUNTIME_COMPILED_POLICY_SAMPLE_SOURCE.contains("current-jiffy"));
    let strategy_asset = assets
        .iter()
        .find(|asset| asset.path == "src/marlin/deck-runtime-strategy.ss")
        .expect("generated manifest includes deck runtime strategy source");
    assert!(
        strategy_asset
            .source
            .contains("defmarlin-deck-runtime-strategy-rule")
    );
    assert!(strategy_asset.source.contains("dynamic-hook-action"));
    assert!(
        strategy_asset
            .source
            .contains("marlin-deck-runtime-strategy-selection")
    );
    assert!(!strategy_asset.source.contains("selection-json"));
    assert!(GERBIL_MARLIN_DECK_RUNTIME_NATIVE_SOURCE.contains("c-define"));
    assert!(GERBIL_MARLIN_DECK_RUNTIME_NATIVE_SOURCE.contains("begin-ffi"));
    assert!(GERBIL_MARLIN_DECK_RUNTIME_NATIVE_SOURCE.contains("begin-foreign"));
    assert!(
        GERBIL_MARLIN_DECK_RUNTIME_NATIVE_SOURCE.contains("marlin_deck_runtime_select_model_route")
    );
    assert!(
        GERBIL_MARLIN_DECK_RUNTIME_NATIVE_SOURCE.contains("MarlinDeckRuntimeModelRouteSelection")
    );
    assert!(!GERBIL_MARLIN_DECK_RUNTIME_NATIVE_SOURCE.contains("MarlinDeckRuntimeOwnedBytes"));
    assert!(!GERBIL_MARLIN_DECK_RUNTIME_NATIVE_SOURCE.contains("selection-json"));
    assert_eq!(
        GERBIL_MARLIN_DECK_RUNTIME_NATIVE_PROJECTION_PATH,
        "src/marlin/deck-runtime-native-projection.ss"
    );
    assert!(
        GERBIL_MARLIN_DECK_RUNTIME_NATIVE_PROJECTION_SOURCE
            .contains("marlin-deck-runtime-native-projection-abi-id")
    );
    assert!(
        GERBIL_MARLIN_DECK_RUNTIME_NATIVE_PROJECTION_SOURCE
            .contains("marlin_deck_runtime_project_poo_policy")
    );
    assert!(
        GERBIL_MARLIN_DECK_RUNTIME_NATIVE_PROJECTION_SOURCE
            .contains("marlin.deck-runtime.poo-policy-projection.v1")
    );
    assert_eq!(
        GERBIL_MARLIN_DECK_RUNTIME_SCRIPT_PATH,
        "src/marlin/deck-runtime-script.ss"
    );
    assert!(GERBIL_MARLIN_DECK_RUNTIME_SCRIPT_SOURCE.contains("defmarlin-deck-runtime-script"));
    assert!(
        GERBIL_MARLIN_DECK_RUNTIME_SCRIPT_SOURCE
            .contains("marlin-deck-runtime-script-interface-receipt")
    );
    assert!(
        GERBIL_MARLIN_DECK_RUNTIME_SCRIPT_SOURCE
            .contains("marlin-deck-runtime-script-batch-metrics")
    );
    assert!(
        GERBIL_MARLIN_DECK_RUNTIME_SCRIPT_SOURCE
            .contains("make-marlin-deck-runtime-poo-policy-projection")
    );
    assert!(GERBIL_MARLIN_DECK_RUNTIME_SCRIPT_SOURCE.contains("poo-native-api-or-gxi-script"));
    assert!(!GERBIL_MARLIN_DECK_RUNTIME_SCRIPT_SOURCE.contains("selection-json"));

    let mut asset_paths = assets
        .iter()
        .map(|asset| asset.path.to_owned())
        .collect::<Vec<_>>();
    asset_paths.sort();
    assert_eq!(
        asset_paths.as_slice(),
        expected_gerbil_runtime_asset_paths().as_slice()
    );
}

#[test]
fn gerbil_runtime_assets_write_loadpath_tree() {
    let root = test_root("runtime-assets");

    let written = write_gerbil_runtime_assets(root.path()).expect("write runtime assets");

    assert_eq!(written.len(), gerbil_runtime_assets().len());
    assert_eq!(
        gerbil_runtime_loadpath(root.path()),
        root.path().join("src")
    );
    assert!(root.path().join(GERBIL_PACKAGE_MANIFEST_PATH).is_file());
    assert!(root.path().join("build.ss").is_file());
    assert!(root.path().join(GERBIL_MARLIN_ADAPTER_PATH).is_file());
    assert!(root.path().join(GERBIL_MARLIN_DECK_RUNTIME_PATH).is_file());
    assert!(
        root.path()
            .join(GERBIL_MARLIN_DECK_RUNTIME_COMPILED_POLICY_PATH)
            .is_file()
    );
    assert!(
        root.path()
            .join(GERBIL_MARLIN_DECK_RUNTIME_COMPILED_POLICY_SAMPLE_PATH)
            .is_file()
    );
    assert!(
        root.path()
            .join("src/marlin/deck-runtime-strategy.ss")
            .is_file()
    );
    assert!(
        root.path()
            .join(GERBIL_MARLIN_DECK_RUNTIME_NATIVE_PATH)
            .is_file()
    );
    assert!(
        root.path()
            .join(GERBIL_MARLIN_DECK_RUNTIME_NATIVE_PROJECTION_PATH)
            .is_file()
    );
    assert!(
        root.path()
            .join(GERBIL_MARLIN_DECK_RUNTIME_SCRIPT_PATH)
            .is_file()
    );
    assert!(
        fs::read_to_string(root.path().join("build.ss"))
            .expect("read build script")
            .contains("src/marlin/deck-runtime")
    );
    assert!(
        fs::read_to_string(root.path().join(GERBIL_MARLIN_DECK_RUNTIME_NATIVE_PATH))
            .expect("read native deck runtime")
            .contains("marlin_deck_runtime_select_model_route")
    );
    assert!(
        fs::read_to_string(
            root.path()
                .join(GERBIL_MARLIN_DECK_RUNTIME_NATIVE_PROJECTION_PATH)
        )
        .expect("read native projection deck runtime")
        .contains("marlin-deck-runtime-project-poo-policy")
    );
    assert!(
        fs::read_to_string(root.path().join(GERBIL_MARLIN_DECK_RUNTIME_SCRIPT_PATH))
            .expect("read quick script deck runtime")
            .contains("defmarlin-deck-runtime-script")
    );
    assert!(
        fs::read_to_string(root.path().join(GERBIL_MARLIN_DECK_RUNTIME_PATH))
            .expect("read deck runtime")
            .contains("marlin-deck-runtime-capability-fact")
    );
    assert!(
        fs::read_to_string(
            root.path()
                .join(GERBIL_MARLIN_DECK_RUNTIME_COMPILED_POLICY_PATH)
        )
        .expect("read compiled deck runtime policy")
        .contains("defmarlin-deck-runtime-compiled-route-selector")
    );
    assert!(
        fs::read_to_string(
            root.path()
                .join(GERBIL_MARLIN_DECK_RUNTIME_COMPILED_POLICY_SAMPLE_PATH)
        )
        .expect("read compiled deck runtime policy sample")
        .contains("display-marlin-deck-runtime-sample-compiled-policy-batch-metrics")
    );
    assert!(
        fs::read_to_string(root.path().join("src/marlin/deck-runtime-strategy.ss"))
            .expect("read deck runtime strategy")
            .contains("marlin-deck-runtime-strategy-selection")
    );
    assert!(
        fs::read_to_string(root.path().join(GERBIL_MARLIN_PROTOCOL_PATH))
            .expect("read protocol")
            .contains("marlin-workspace-patch-intent-artifact-kind")
    );
}

fn expected_gerbil_runtime_asset_paths() -> Vec<String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("gerbil");
    let mut paths = Vec::new();
    collect_gerbil_runtime_asset_paths(&root, &root, &mut paths);
    paths.sort();
    paths
}

fn collect_gerbil_runtime_asset_paths(root: &Path, dir: &Path, paths: &mut Vec<String>) {
    let mut entries = fs::read_dir(dir)
        .expect("read Gerbil runtime asset directory")
        .map(|entry| entry.expect("read Gerbil runtime asset entry"))
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect_gerbil_runtime_asset_paths(root, &path, paths);
            continue;
        }
        if !is_gerbil_runtime_asset(&path) {
            continue;
        }
        let relative = path
            .strip_prefix(root)
            .expect("runtime asset path under Gerbil root")
            .to_string_lossy()
            .replace(std::path::MAIN_SEPARATOR, "/");
        paths.push(relative);
    }
}

fn is_gerbil_runtime_asset(path: &Path) -> bool {
    path.file_name().is_some_and(|name| name == "gerbil.pkg")
        || path.extension().is_some_and(|extension| extension == "ss")
}
