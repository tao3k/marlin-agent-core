use super::support::{local_gxi, local_gxtest_for_gxi, test_root};
use marlin_gerbil_scheme::{
    GERBIL_DECK_RUNTIME_SCRIPT_BATCH_METRICS_SCHEMA_ID,
    GERBIL_DECK_RUNTIME_SCRIPT_BATCH_METRICS_TYPE_ID, GERBIL_LOADPATH_ENV,
    GerbilDeckRuntimeScriptBatchPerformanceBudget, GerbilSchemeSchemaId, GerbilSchemeTypeId,
    GerbilSchemeTypeRegistry, GerbilSchemeTypedValue, GerbilSchemeValue,
    decode_gerbil_deck_runtime_script_batch_metrics,
    evaluate_gerbil_deck_runtime_script_batch_performance,
    gerbil_deck_runtime_script_interface_type_manifest, gerbil_runtime_loadpath,
    write_gerbil_runtime_assets,
};
use std::{fs, path::Path, process::Command};

const USER_INTERFACE_MODULE_CONFIG_EXAMPLE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/examples/user-interface-module-config"
);

#[test]
#[ignore = "requires local Gerbil gxtest executable and installed gerbil-poo dependency"]
fn command_compiler_real_gxtest_runs_user_interface_module_config_example() {
    let Some(gxi) = local_gxi() else {
        return;
    };
    let Some(gxtest) = local_gxtest_for_gxi(&gxi) else {
        return;
    };
    let root = test_root("user-interface-module-config");
    let runtime_root = root.path().join("runtime");
    let workspace_root = root.path().join("user-interface-module-config");
    write_gerbil_runtime_assets(&runtime_root).expect("write gerbil runtime assets");
    copy_example_workspace(
        Path::new(USER_INTERFACE_MODULE_CONFIG_EXAMPLE),
        &workspace_root,
    );
    let test_script = workspace_root.join("t/user-interface-module-config-test.ss");

    let output = Command::new(gxtest)
        .env(GERBIL_LOADPATH_ENV, gerbil_runtime_loadpath(&runtime_root))
        .current_dir(&workspace_root)
        .arg(&test_script)
        .output()
        .expect("run real gxtest user interface worker space script workflow");

    assert!(
        output.status.success(),
        "user interface module config example gxtest failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout =
        String::from_utf8(output.stdout).expect("user interface workflow stdout should be UTF-8");
    assert_no_json_handoff(&stdout);
    assert!(stdout.contains("user-interface-script-workflow-ok"));
    assert!(stdout.contains("script-id=user-interface-worker-script"));
    assert!(stdout.contains("extension-id=user-interface-worker-extension"));
    assert!(stdout.contains("continuation-kind=continue_with_graph"));
    assert!(stdout.contains("has-interface-file=true"));

    let metrics = decode_gerbil_deck_runtime_script_batch_metrics(
        &GerbilSchemeTypeRegistry::new(gerbil_deck_runtime_script_interface_type_manifest())
            .expect("script interface type manifest should validate"),
        &script_batch_metrics_typed_value_from_stdout(&stdout),
    )
    .expect("user interface worker script batch metrics should decode");
    let receipt = evaluate_gerbil_deck_runtime_script_batch_performance(
        metrics,
        &GerbilDeckRuntimeScriptBatchPerformanceBudget::default(),
    );

    assert!(
        receipt.within_budget(),
        "user interface worker script workflow exceeded Rust budget: {receipt:?}"
    );
}

fn assert_no_json_handoff(stdout: &str) {
    for forbidden in ["json-handshake", "selection-json", "{\""] {
        assert!(
            !stdout.contains(forbidden),
            "real gxtest output leaked serialized handoff marker {forbidden}: {stdout}"
        );
    }
}

fn script_batch_metrics_typed_value_from_stdout(stdout: &str) -> GerbilSchemeTypedValue {
    GerbilSchemeTypedValue::new(
        GerbilSchemeTypeId::new(GERBIL_DECK_RUNTIME_SCRIPT_BATCH_METRICS_TYPE_ID),
        GerbilSchemeValue::record([
            ("kind", metric_text(stdout, "metrics-kind").into()),
            ("script-id", "user-interface-worker-script".into()),
            ("interface", metric_text(stdout, "metrics-interface").into()),
            (
                "iterations",
                GerbilSchemeValue::integer(metric_u64(stdout, "iterations") as i64),
            ),
            (
                "runs",
                GerbilSchemeValue::integer(metric_u64(stdout, "runs") as i64),
            ),
            (
                "elapsed-us",
                GerbilSchemeValue::integer(metric_u64(stdout, "elapsed_us") as i64),
            ),
        ]),
    )
    .with_schema_id(GerbilSchemeSchemaId::new(
        GERBIL_DECK_RUNTIME_SCRIPT_BATCH_METRICS_SCHEMA_ID,
    ))
}

fn metric_text(stdout: &str, key: &str) -> String {
    let prefix = format!("{key}=");
    stdout
        .lines()
        .find_map(|line| line.strip_prefix(&prefix))
        .unwrap_or_else(|| panic!("missing metric line {prefix:?} in stdout:\n{stdout}"))
        .to_owned()
}

fn metric_u64(stdout: &str, key: &str) -> u64 {
    metric_text(stdout, key)
        .parse::<u64>()
        .unwrap_or_else(|error| panic!("invalid numeric metric {key} in stdout: {error}"))
}

fn copy_example_workspace(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("create destination example workspace");
    for entry in fs::read_dir(source).expect("read example workspace") {
        let entry = entry.expect("read example workspace entry");
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            copy_example_workspace(&source_path, &destination_path);
        } else {
            fs::copy(&source_path, &destination_path).unwrap_or_else(|error| {
                panic!(
                    "copy example file {} to {}: {error}",
                    source_path.display(),
                    destination_path.display()
                )
            });
        }
    }
}
