use super::support::test_root;
use marlin_gerbil_scheme::{
    GERBIL_ADAPTER_MODULE, GERBIL_LOADPATH_ENV, GERBIL_PACKAGE_BUILD_SCRIPT,
    GERBIL_PACKAGE_ROOT_PATH, GERBIL_PACKAGE_SOURCE_PATH, GERBIL_POO_MOP_MODULE,
    GERBIL_POO_OBJECT_MODULE, GERBIL_POO_PACKAGE_NAME, GERBIL_POO_PROTO_MODULE,
    GERBIL_RUNTIME_BUILD_DEPS, GERBIL_RUNTIME_BUILD_STAGES, GERBIL_RUNTIME_BUILD_WORKER_ENV,
    GERBIL_RUNTIME_COVERAGE_ROOTS, GERBIL_RUNTIME_EXCLUDED_PACKAGE_SOURCE_FILES,
    GERBIL_RUNTIME_PACKAGE_NAME, GERBIL_RUNTIME_SOURCE_ROOTS, GERBIL_RUNTIME_SPECIAL_SOURCE_FILES,
    MARLIN_GERBIL_GSC_ENV, MARLIN_GERBIL_GXC_ENV, MARLIN_GERBIL_GXI_ENV,
    default_gerbil_gsc_program, default_gerbil_gxc_program, default_gerbil_gxi_program,
    gerbil_package_build_script, gerbil_package_root, gerbil_runtime_asset, gerbil_runtime_assets,
    gerbil_runtime_build_script_contract, gerbil_runtime_loadpath, write_gerbil_runtime_assets,
};
use std::{fs, path::Path};

#[test]
fn gerbil_runtime_assets_expose_loadpath_contract() {
    let assets = gerbil_runtime_assets();
    let build_source = runtime_asset_source("build.ss");
    let package_manifest_source = runtime_asset_source("gerbil.pkg");
    let adapter_source = runtime_asset_source("src/marlin/adapter.ss");
    let request_source = runtime_asset_source("src/marlin/request.ss");
    let deck_runtime_source = runtime_asset_source("src/marlin/deck-runtime.ss");
    let resident_strategy_source =
        runtime_asset_source("src/marlin/deck-runtime-resident-strategy.ss");
    let compiled_policy_source = runtime_asset_source("src/marlin/deck-runtime-compiled-policy.ss");
    let compiled_policy_sample_source =
        runtime_asset_source("src/marlin/deck-runtime-compiled-policy-sample.ss");
    let native_source = runtime_asset_source("src/marlin/deck-runtime-native.ss");
    let native_projection_source =
        runtime_asset_source("src/marlin/deck-runtime-native-projection.ss");
    let continuation_projection_source =
        runtime_asset_source("src/marlin/graph-loop-continuation-native-projection.ss");
    let script_source = runtime_asset_source("src/marlin/deck-runtime-script.ss");
    let script_performance_source =
        runtime_asset_source("src/marlin/deck-runtime-script-performance.ss");
    let build_contract = gerbil_runtime_build_script_contract();
    assert_eq!(GERBIL_LOADPATH_ENV, "GERBIL_LOADPATH");
    assert_eq!(GERBIL_ADAPTER_MODULE, ":marlin/adapter");
    assert_eq!(MARLIN_GERBIL_GXI_ENV, "MARLIN_GERBIL_GXI");
    assert_eq!(MARLIN_GERBIL_GXC_ENV, "MARLIN_GERBIL_GXC");
    assert_eq!(MARLIN_GERBIL_GSC_ENV, "MARLIN_GERBIL_GSC");
    assert_eq!(default_gerbil_gxi_program(), Path::new("gxi"));
    assert_eq!(default_gerbil_gxc_program(), Path::new("gxc"));
    assert_eq!(
        default_gerbil_gsc_program()
            .file_name()
            .and_then(|file_name| file_name.to_str()),
        Some("gsc")
    );
    assert_eq!(GERBIL_PACKAGE_SOURCE_PATH, "src");
    assert!(package_manifest_source.contains("marlin-deck-runtime"));
    assert_eq!(GERBIL_POO_PACKAGE_NAME, "clan/poo");
    assert_eq!(GERBIL_POO_OBJECT_MODULE, ":clan/poo/object");
    assert_eq!(GERBIL_POO_MOP_MODULE, ":clan/poo/mop");
    assert_eq!(GERBIL_POO_PROTO_MODULE, ":clan/poo/proto");
    assert_eq!(build_contract.package_name, GERBIL_RUNTIME_PACKAGE_NAME);
    assert_eq!(build_contract.stages, GERBIL_RUNTIME_BUILD_STAGES);
    assert_eq!(build_contract.deps, GERBIL_RUNTIME_BUILD_DEPS);
    assert_eq!(build_contract.source_roots, GERBIL_RUNTIME_SOURCE_ROOTS);
    assert_eq!(build_contract.coverage_roots, GERBIL_RUNTIME_COVERAGE_ROOTS);
    assert_eq!(
        build_contract.special_source_files,
        GERBIL_RUNTIME_SPECIAL_SOURCE_FILES
    );
    assert_eq!(
        build_contract.excluded_package_source_files,
        GERBIL_RUNTIME_EXCLUDED_PACKAGE_SOURCE_FILES
    );
    assert_eq!(build_contract.worker_env, GERBIL_RUNTIME_BUILD_WORKER_ENV);
    assert_eq!(
        build_contract.special_source_files,
        build_contract.excluded_package_source_files
    );
    assert!(package_manifest_source.contains("github.com/tao3k/poo-flow"));
    assert!(
        package_manifest_source.contains("github.com/tao3k/gerbil-scheme-language-project-harness")
    );
    assert!(
        !package_manifest_source.contains("/Users/"),
        "crate-shipped gerbil.pkg must not lock runtime deps to a local checkout"
    );
    assert!(build_source.contains(":std/make"));
    assert!(build_source.contains(":std/source"));
    assert!(build_source.contains(":clan/building"));
    assert!(build_source.contains("gslph-source-coverage"));
    assert!(build_source.contains(build_contract.package_name));
    assert!(build_source.contains("+marlin-special-source-files+"));
    assert!(build_source.contains("+marlin-excluded-package-source-files+"));
    assert!(build_source.contains("marlin-package-source-file?"));
    assert!(build_source.contains("marlin-runtime-build-spec"));
    assert!(build_source.contains("marlin-package-build-spec"));
    for root in build_contract.coverage_roots {
        assert!(
            build_source.contains(&format!("runtime-roots: '(\"{root}\")")),
            "build script should activate runtime coverage root {root}"
        );
    }
    for dep in build_contract.deps {
        assert!(
            build_source.contains(&format!("\"{dep}\"")),
            "build script should declare package dependency {dep}"
        );
    }
    for stage in build_contract.stages {
        assert!(
            build_source.contains(&format!("\"{stage}\"")),
            "build script should expose build stage {stage}"
        );
    }
    assert!(!build_source.contains("stage-native-aot"));
    assert!(!build_source.contains("marlin-build-stage-native-aot"));
    assert!(build_source.contains(":gerbil/compiler/base"));
    assert!(build_source.contains("__available-cores"));
    for env_var in build_contract.worker_env {
        assert!(
            build_source.contains(env_var),
            "build script should consume worker env {env_var}"
        );
    }
    assert!(build_source.contains("##cpu-count"));
    assert!(build_source.contains("marlin-build-worker-count"));
    assert!(build_source.contains("marlin-sync-build-worker-count!"));
    assert!(build_source.contains("parallelize:"));
    assert!(build_source.contains("all-gerbil-modules"));
    for source_file in build_contract.special_source_files {
        assert!(
            build_source.contains(&format!("\"{source_file}\"")),
            "build script should list special native source {source_file}"
        );
    }
    assert!(!build_source.contains("marlin-package-module?"));
    assert!(!build_source.contains("+marlin-native-aot-only-modules+"));
    assert!(!build_source.contains("git.cons.io/mighty-gerbils/gerbil-poo"));
    assert!(!build_source.contains("github.com/tao3k/poo-flow"));
    assert!(request_source.contains("gerbil-compile-request-contract-facts"));
    assert!(adapter_source.contains("ensure-marlin-contract-facts-shape"));
    assert!(deck_runtime_source.contains("poo-object-system"));
    assert!(deck_runtime_source.contains("scheme-policy-runtime"));
    assert!(deck_runtime_source.contains("scheme-compiled-policy-macro"));
    assert!(deck_runtime_source.contains("scheme-complex-strategy"));
    assert!(deck_runtime_source.contains("model-route-policy"));
    assert!(deck_runtime_source.contains("make-marlin-deck-runtime-model-route-policy"));
    assert!(deck_runtime_source.contains("marlin-deck-runtime-select-model-route-policy"));
    assert!(deck_runtime_source.contains(GERBIL_POO_OBJECT_MODULE));
    assert!(deck_runtime_source.contains("marlin-deck-runtime-model-route-selection"));
    assert!(deck_runtime_source.contains("typed-native-abi"));
    assert!(!deck_runtime_source.contains("json-handshake"));
    assert!(!deck_runtime_source.contains("selection-json"));
    assert!(resident_strategy_source.contains("marlin.resident.strategy.procedure-response.v1"));
    assert!(resident_strategy_source.contains("\"status\" \"executed\""));
    assert!(!resident_strategy_source.contains("selection-json"));
    assert!(compiled_policy_source.contains("defmarlin-deck-runtime-compiled-route-selector"));
    assert!(
        compiled_policy_source
            .contains("defmarlin-deck-runtime-cached-compiled-route-index-selector")
    );
    assert!(
        compiled_policy_source
            .contains("defmarlin-deck-runtime-direct-compiled-route-index-selector")
    );
    assert!(compiled_policy_source.contains("compiled-macro-selector"));
    assert!(compiled_policy_source.contains("policy-index-selector"));
    assert!(compiled_policy_source.contains("direct-policy-index-selector"));
    assert!(
        compiled_policy_sample_source.contains("select-marlin-deck-runtime-sample-compiled-policy")
    );
    assert!(
        compiled_policy_sample_source
            .contains("select-marlin-deck-runtime-sample-compiled-policy-index")
    );
    assert!(compiled_policy_sample_source.contains("index_elapsed_us"));
    assert!(compiled_policy_sample_source.contains("current-jiffy"));
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
    assert!(
        strategy_asset
            .source
            .contains("marlin-deck-runtime-resident-strategy-execute")
    );
    assert!(!strategy_asset.source.contains("selection-json"));
    assert!(native_source.contains("c-define"));
    assert!(native_source.contains("begin-ffi"));
    assert!(native_source.contains("begin-foreign"));
    assert!(native_source.contains("marlin_deck_runtime_select_model_route"));
    assert!(native_source.contains("MarlinDeckRuntimeModelRouteSelection"));
    assert!(!native_source.contains("MarlinDeckRuntimeOwnedBytes"));
    assert!(!native_source.contains("selection-json"));
    assert!(native_projection_source.contains("marlin-deck-runtime-native-projection-abi-id"));
    assert!(native_projection_source.contains("marlin_deck_runtime_project_poo_policy"));
    assert!(native_projection_source.contains("marlin.deck-runtime.poo-policy-projection.v1"));
    assert!(
        continuation_projection_source
            .contains("marlin-graph-loop-continuation-native-projection-abi-id")
    );
    assert!(continuation_projection_source.contains("marlin_graph_loop_continuation_next_action"));
    assert!(
        continuation_projection_source.contains("marlin.agent.gerbil_loop_graph_continuation.v1")
    );
    assert!(continuation_projection_source.contains("defmarlin-graph-loop-continuation-profile"));
    assert!(script_source.contains("defmarlin-deck-runtime-script"));
    assert!(script_source.contains("marlin-deck-runtime-script-interface-receipt"));
    assert!(script_source.contains("marlin-deck-runtime-script-batch-metrics"));
    assert!(script_source.contains("make-marlin-deck-runtime-poo-policy-projection"));
    assert!(script_source.contains("poo-native-api-or-gxi-script"));
    assert!(!script_source.contains("selection-json"));
    assert!(script_performance_source.contains("deck-runtime-script-performance-run-batch"));
    assert!(script_performance_source.contains("deck-runtime-script-performance-count-runs"));
    assert!(script_performance_source.contains("performance-script-extension"));

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
    assert!(root.path().join("gerbil.pkg").is_file());
    assert!(root.path().join("build.ss").is_file());
    assert!(root.path().join("src/marlin/adapter.ss").is_file());
    assert!(root.path().join("src/marlin/deck-runtime.ss").is_file());
    assert!(
        root.path()
            .join("src/marlin/deck-runtime-compiled-policy.ss")
            .is_file()
    );
    assert!(
        root.path()
            .join("src/marlin/deck-runtime-compiled-policy-sample.ss")
            .is_file()
    );
    assert!(
        root.path()
            .join("src/marlin/deck-runtime-strategy.ss")
            .is_file()
    );
    assert!(
        root.path()
            .join("src/marlin/deck-runtime-native.ss")
            .is_file()
    );
    assert!(
        root.path()
            .join("src/marlin/deck-runtime-native-projection.ss")
            .is_file()
    );
    assert!(
        root.path()
            .join("src/marlin/graph-loop-continuation-native-projection.ss")
            .is_file()
    );
    assert!(
        root.path()
            .join("src/marlin/deck-runtime-script.ss")
            .is_file()
    );
    assert!(
        fs::read_to_string(root.path().join("build.ss"))
            .expect("read build script")
            .contains("marlin-runtime-build-spec")
    );
    assert!(
        fs::read_to_string(root.path().join("src/marlin/deck-runtime-native.ss"))
            .expect("read native deck runtime")
            .contains("marlin_deck_runtime_select_model_route")
    );
    assert!(
        fs::read_to_string(
            root.path()
                .join("src/marlin/deck-runtime-native-projection.ss")
        )
        .expect("read native projection deck runtime")
        .contains("marlin-deck-runtime-project-poo-policy")
    );
    assert!(
        fs::read_to_string(
            root.path()
                .join("src/marlin/graph-loop-continuation-native-projection.ss")
        )
        .expect("read graph loop continuation native projection")
        .contains("marlin-graph-loop-continuation-next-action")
    );
    assert!(
        fs::read_to_string(root.path().join("src/marlin/deck-runtime-script.ss"))
            .expect("read quick script deck runtime")
            .contains("defmarlin-deck-runtime-script")
    );
    assert!(
        fs::read_to_string(
            root.path()
                .join("src/marlin/deck-runtime-script-performance.ss")
        )
        .expect("read quick script performance helper")
        .contains("deck-runtime-script-performance-run-batch")
    );
    assert!(
        fs::read_to_string(root.path().join("src/marlin/deck-runtime.ss"))
            .expect("read deck runtime")
            .contains("marlin-deck-runtime-capability-fact")
    );
    assert!(
        fs::read_to_string(
            root.path()
                .join("src/marlin/deck-runtime-compiled-policy.ss")
        )
        .expect("read compiled deck runtime policy")
        .contains("defmarlin-deck-runtime-compiled-route-selector")
    );
    assert!(
        fs::read_to_string(
            root.path()
                .join("src/marlin/deck-runtime-compiled-policy-sample.ss")
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
        fs::read_to_string(root.path().join("src/marlin/protocol.ss"))
            .expect("read protocol")
            .contains("marlin-workspace-patch-intent-artifact-kind")
    );
}

#[test]
fn gerbil_runtime_package_path_points_at_owned_build_script() {
    let package_root = gerbil_package_root();
    let build_script = gerbil_package_build_script();

    assert_eq!(GERBIL_PACKAGE_ROOT_PATH, "gerbil");
    assert_eq!(GERBIL_PACKAGE_BUILD_SCRIPT, "build.ss");
    assert_eq!(build_script, package_root.join(GERBIL_PACKAGE_BUILD_SCRIPT));
    assert!(package_root.join("gerbil.pkg").is_file());
    assert!(build_script.is_file());
}

fn runtime_asset_source(path: &str) -> &'static str {
    gerbil_runtime_asset(path)
        .unwrap_or_else(|| panic!("generated manifest includes {path}"))
        .source
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
