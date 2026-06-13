use super::support::test_root;
use marlin_gerbil_scheme::{
    DEFAULT_GERBIL_GSC_PROGRAM, DEFAULT_GERBIL_GXC_PROGRAM, DEFAULT_GERBIL_GXI_PROGRAM,
    GERBIL_ADAPTER_MODULE, GERBIL_BUILD_SOURCE, GERBIL_COMMAND_ADAPTER_BATCH_PATH,
    GERBIL_COMMAND_ADAPTER_PATH, GERBIL_DECK_RUNTIME_POLICY_ADAPTER_PATH,
    GERBIL_DECK_RUNTIME_POLICY_ADAPTER_SOURCE, GERBIL_HOOK_POLICY_ADAPTER_PATH,
    GERBIL_LOADPATH_ENV, GERBIL_MARLIN_ADAPTER_PATH, GERBIL_MARLIN_ADAPTER_SOURCE,
    GERBIL_MARLIN_DECK_RUNTIME_NATIVE_PATH, GERBIL_MARLIN_DECK_RUNTIME_NATIVE_SOURCE,
    GERBIL_MARLIN_DECK_RUNTIME_PATH, GERBIL_MARLIN_DECK_RUNTIME_POLICY_PATH,
    GERBIL_MARLIN_DECK_RUNTIME_POLICY_SOURCE, GERBIL_MARLIN_DECK_RUNTIME_SOURCE,
    GERBIL_MARLIN_DECK_RUNTIME_STRATEGY_PATH, GERBIL_MARLIN_DECK_RUNTIME_STRATEGY_SOURCE,
    GERBIL_MARLIN_HOOK_POLICY_PATH, GERBIL_MARLIN_PARSER_PATH, GERBIL_MARLIN_PROTOCOL_PATH,
    GERBIL_MARLIN_REQUEST_PATH, GERBIL_MARLIN_REQUEST_SOURCE, GERBIL_PACKAGE_MANIFEST_PATH,
    GERBIL_PACKAGE_MANIFEST_SOURCE, GERBIL_PACKAGE_SOURCE_PATH, GERBIL_POO_DEPENDENCY,
    GERBIL_POO_MOP_MODULE, GERBIL_POO_OBJECT_MODULE, GERBIL_POO_PACKAGE_NAME,
    GERBIL_POO_PROTO_MODULE, GERBIL_SMOKE_PATH, MARLIN_GERBIL_GSC_ENV, MARLIN_GERBIL_GXC_ENV,
    MARLIN_GERBIL_GXI_ENV, gerbil_runtime_assets, gerbil_runtime_loadpath,
    write_gerbil_runtime_assets,
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
    assert!(GERBIL_BUILD_SOURCE.contains("src/marlin/deck-runtime-native"));
    assert!(GERBIL_BUILD_SOURCE.contains("src/marlin/deck-runtime"));
    assert!(GERBIL_BUILD_SOURCE.contains("src/marlin/deck-runtime-strategy"));
    assert!(GERBIL_BUILD_SOURCE.contains("src/marlin/deck-runtime-policy"));
    assert!(GERBIL_DECK_RUNTIME_POLICY_ADAPTER_SOURCE.contains(":marlin/deck-runtime-policy"));
    assert!(GERBIL_MARLIN_REQUEST_SOURCE.contains("gerbil-compile-request-contract-facts"));
    assert!(GERBIL_MARLIN_ADAPTER_SOURCE.contains("ensure-marlin-contract-facts-shape"));
    assert!(
        GERBIL_MARLIN_DECK_RUNTIME_POLICY_SOURCE.contains("run-marlin-deck-runtime-policy-adapter")
    );
    assert!(GERBIL_MARLIN_DECK_RUNTIME_SOURCE.contains("poo-object-system"));
    assert!(GERBIL_MARLIN_DECK_RUNTIME_SOURCE.contains("scheme-policy-runtime"));
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
        GERBIL_MARLIN_DECK_RUNTIME_SOURCE.contains("display-marlin-deck-runtime-object-model-json")
    );
    assert!(
        GERBIL_MARLIN_DECK_RUNTIME_SOURCE
            .contains("display-marlin-deck-runtime-model-route-selection-json")
    );
    assert!(
        GERBIL_MARLIN_DECK_RUNTIME_STRATEGY_SOURCE.contains("defmarlin-deck-runtime-strategy-rule")
    );
    assert!(GERBIL_MARLIN_DECK_RUNTIME_STRATEGY_SOURCE.contains("dynamic-hook-action"));
    assert!(
        GERBIL_MARLIN_DECK_RUNTIME_STRATEGY_SOURCE
            .contains("display-marlin-deck-runtime-strategy-selection-json")
    );
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

    let asset_paths = assets.iter().map(|asset| asset.path).collect::<Vec<_>>();
    assert_eq!(
        asset_paths.as_slice(),
        &[
            GERBIL_PACKAGE_MANIFEST_PATH,
            GERBIL_COMMAND_ADAPTER_PATH,
            GERBIL_COMMAND_ADAPTER_BATCH_PATH,
            GERBIL_HOOK_POLICY_ADAPTER_PATH,
            GERBIL_DECK_RUNTIME_POLICY_ADAPTER_PATH,
            "build.ss",
            GERBIL_SMOKE_PATH,
            GERBIL_MARLIN_ADAPTER_PATH,
            GERBIL_MARLIN_DECK_RUNTIME_PATH,
            GERBIL_MARLIN_DECK_RUNTIME_STRATEGY_PATH,
            GERBIL_MARLIN_DECK_RUNTIME_NATIVE_PATH,
            GERBIL_MARLIN_DECK_RUNTIME_POLICY_PATH,
            GERBIL_MARLIN_HOOK_POLICY_PATH,
            GERBIL_MARLIN_PARSER_PATH,
            GERBIL_MARLIN_PROTOCOL_PATH,
            GERBIL_MARLIN_REQUEST_PATH,
        ]
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
    assert!(root.path().join(GERBIL_COMMAND_ADAPTER_PATH).is_file());
    assert!(
        root.path()
            .join(GERBIL_DECK_RUNTIME_POLICY_ADAPTER_PATH)
            .is_file()
    );
    assert!(root.path().join("build.ss").is_file());
    assert!(root.path().join(GERBIL_MARLIN_ADAPTER_PATH).is_file());
    assert!(root.path().join(GERBIL_MARLIN_DECK_RUNTIME_PATH).is_file());
    assert!(
        root.path()
            .join(GERBIL_MARLIN_DECK_RUNTIME_STRATEGY_PATH)
            .is_file()
    );
    assert!(
        root.path()
            .join(GERBIL_MARLIN_DECK_RUNTIME_NATIVE_PATH)
            .is_file()
    );
    assert!(
        root.path()
            .join(GERBIL_MARLIN_DECK_RUNTIME_POLICY_PATH)
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
        fs::read_to_string(root.path().join(GERBIL_MARLIN_DECK_RUNTIME_PATH))
            .expect("read deck runtime")
            .contains("display-marlin-deck-runtime-capability-json")
    );
    assert!(
        fs::read_to_string(root.path().join(GERBIL_MARLIN_DECK_RUNTIME_STRATEGY_PATH))
            .expect("read deck runtime strategy")
            .contains("display-marlin-deck-runtime-strategy-selection-json")
    );
    assert!(
        fs::read_to_string(root.path().join(GERBIL_MARLIN_PROTOCOL_PATH))
            .expect("read protocol")
            .contains("marlin-workspace-patch-intent-artifact-kind")
    );
}
