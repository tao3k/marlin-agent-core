use super::support::{local_gxi, local_gxtest_for_gxi, test_root};
use marlin_gerbil_scheme::{
    GERBIL_DECK_RUNTIME_SCRIPT_BATCH_METRICS_SCHEMA_ID,
    GERBIL_DECK_RUNTIME_SCRIPT_BATCH_METRICS_TYPE_ID, GERBIL_LOADPATH_ENV,
    GerbilDeckRuntimeScriptBatchPerformanceBudget, GerbilSchemeSchemaId, GerbilSchemeTypeId,
    GerbilSchemeTypeRegistry, GerbilSchemeTypedValue, GerbilSchemeValue,
    decode_gerbil_deck_runtime_script_batch_metrics,
    evaluate_gerbil_deck_runtime_script_batch_performance,
    gerbil_deck_runtime_script_interface_type_manifest, gerbil_runtime_dependency_loadpath,
    gerbil_runtime_loadpath, write_gerbil_runtime_assets,
};
use std::{env, fs, path::Path, process::Command};

const USER_INTERFACE_MODULE_CONFIG_EXAMPLE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/examples/user-interface-module-config"
);
const MARLINE_CONFIG_INTERFACE_WORKSPACE: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/marline-config-interface");
const MARLINE_REAL_LLM_CASE_ENV: &str = "MARLIN_RUN_REAL_LLM_CASES";

#[test]
#[ignore = "requires local Gerbil gxtest executable and installed poo-flow package dependency"]
fn command_compiler_real_gxtest_runs_user_interface_module_config_example() {
    let Some(stdout) = run_real_gxtest_workspace(
        "user-interface-module-config",
        Path::new(USER_INTERFACE_MODULE_CONFIG_EXAMPLE),
        "t/user-interface-module-config-test.ss",
        "run real gxtest user interface worker space script workflow",
    ) else {
        return;
    };

    assert_no_json_handoff(&stdout);
    assert!(stdout.contains("user-interface-script-workflow-ok"));
    assert!(stdout.contains("script-id=user-interface-worker-script"));
    assert!(stdout.contains("extension-id=user-interface-worker-extension"));
    assert!(stdout.contains("continuation-kind=continue_with_graph"));
    assert!(stdout.contains("loop-profile-source=custom/marlin-user-interface/profiles/loops.ss"));
    assert!(stdout.contains("runtime-handoff-status=handoff-ready"));
    assert!(stdout.contains("runtime-execution-owner=marlin-agent-core"));
    assert!(stdout.contains("loops-policy-owner=marlin"));
    assert!(
        stdout.contains("loops-policy-source=marlin/modules/prefabs/user-interface#loops-policy")
    );
    assert!(stdout.contains("loops-policy-receipt-contract-count=8"));
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

#[test]
#[ignore = "requires local Gerbil gxtest executable and installed poo-flow package dependency"]
fn command_compiler_real_gxtest_runs_marline_config_interface() {
    let Some(stdout) = run_real_gxtest_workspace(
        "marline-config-interface",
        Path::new(MARLINE_CONFIG_INTERFACE_WORKSPACE),
        "t/marline-config-interface-test.ss",
        "run real gxtest marline config interface workflow",
    ) else {
        return;
    };

    assert_no_json_handoff(&stdout);
    assert!(stdout.contains("marline-config-interface-ok"));
    assert!(stdout.contains("loop-policy-owner=marlin"));
    assert!(stdout.contains("loop-receipt-contract-count=8"));
    assert!(stdout.contains("kernel-loop-use-cases=4"));
    assert!(stdout.contains("kernel-loop-profiles=4"));
    assert!(stdout.contains("kernel-loop-llm-cases=4"));
    assert_marline_loop_real_llm_case_assets();
}

#[test]
#[ignore = "requires MARLIN_RUN_REAL_LLM_CASES=1 and live Codex CLI access"]
fn command_compiler_real_llm_loop_cases_run_through_debug_cli_when_enabled() {
    if env::var(MARLINE_REAL_LLM_CASE_ENV).ok().as_deref() != Some("1") {
        return;
    }

    for (case_file, case_id, continuation_planner, terminal_status, process_exit_status) in [
        (
            "runtime-handoff-llm.loop.json",
            "marlin-runtime-handoff-real-llm",
            "repeat-graph",
            "Completed",
            "0",
        ),
        (
            "policy-receipt-gate-llm.loop.json",
            "marlin-policy-receipt-gate-real-llm",
            "repeat-graph",
            "Completed",
            "0",
        ),
        (
            "loop-contract-llm.loop.json",
            "marlin-loop-contract-real-llm",
            "repeat-graph",
            "Completed",
            "0",
        ),
        (
            "failure-retry-llm.loop.json",
            "marlin-failure-retry-real-llm",
            "retry-on-failure",
            "Failed",
            "17",
        ),
    ] {
        let stdout = run_marline_real_llm_loop_case(case_file, continuation_planner);
        assert!(
            stdout.contains(&format!("\"terminal_status\": \"{terminal_status}\"")),
            "real LLM loop case {case_file} ended with unexpected terminal status:\n{stdout}"
        );
        assert!(
            stdout.contains("\"iteration_count\": 3"),
            "real LLM loop case {case_file} should run three controller iterations:\n{stdout}"
        );
        assert!(
            stdout.contains(&format!(
                "process-command.exit_status:{process_exit_status}"
            )),
            "real LLM loop case {case_file} did not return the expected process receipt:\n{stdout}"
        );
        assert!(
            stdout.contains(&format!("marlin-real-llm-case.case_id={case_id}")),
            "real LLM loop case {case_file} did not emit its case marker:\n{stdout}"
        );
        assert!(
            stdout.contains("marlin-real-llm-case.result=pass"),
            "real LLM loop case {case_file} did not pass policy simulation:\n{stdout}"
        );
        assert!(
            stdout.contains("marlin-real-llm-case.rounds_used="),
            "real LLM loop case {case_file} did not report LLM rounds used:\n{stdout}"
        );
        if continuation_planner == "retry-on-failure" {
            assert!(
                stdout.contains("continuation_planner=retry-on-failure"),
                "failure retry loop case {case_file} did not emit retry planner diagnostics:\n{stdout}"
            );
            assert!(
                stdout.contains("\"failure_classification_receipt\""),
                "failure retry loop case {case_file} did not emit failure classification receipt:\n{stdout}"
            );
            assert!(
                stdout.contains("\"governance_receipt\""),
                "failure retry loop case {case_file} did not emit governance receipt:\n{stdout}"
            );
            assert!(
                stdout.contains("\"backend\": \"nono\""),
                "failure retry loop case {case_file} did not materialize nono sandbox:\n{stdout}"
            );
            assert!(
                stdout.contains("\"decision\": \"human-audit\""),
                "failure retry loop case {case_file} did not escalate exhausted retries to human audit:\n{stdout}"
            );
        }
        eprintln!(
            "real-llm-case-summary case={case_id} controller_iterations=3 planner={continuation_planner} terminal_status={terminal_status} rounds_used={} result={}",
            real_llm_marker_value(&stdout, "marlin-real-llm-case.rounds_used="),
            real_llm_marker_value(&stdout, "marlin-real-llm-case.result=")
        );
    }
}

fn run_real_gxtest_workspace(
    fixture_name: &str,
    source: &Path,
    test_script_relative: &str,
    failure_context: &str,
) -> Option<String> {
    let Some(gxi) = local_gxi() else {
        return None;
    };
    let Some(gxtest) = local_gxtest_for_gxi(&gxi) else {
        return None;
    };
    let root = test_root(fixture_name);
    let runtime_root = root.path().join("runtime");
    let workspace_root = root.path().join(fixture_name);
    write_gerbil_runtime_assets(&runtime_root).expect("write gerbil runtime assets");
    copy_workspace_tree(source, &workspace_root);
    let test_script = workspace_root.join(test_script_relative);
    let loadpath = env::join_paths([
        gerbil_runtime_loadpath(&runtime_root),
        gerbil_runtime_dependency_loadpath(),
        workspace_root.clone(),
        workspace_root.join("t"),
    ])
    .expect("Gerbil loadpath entries should be joinable");

    let output = Command::new(gxtest)
        .env(GERBIL_LOADPATH_ENV, loadpath)
        .current_dir(&workspace_root)
        .arg(&test_script)
        .output()
        .expect(failure_context);

    assert!(
        output.status.success(),
        "{fixture_name} gxtest failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Some(String::from_utf8(output.stdout).expect("workspace workflow stdout should be UTF-8"))
}

fn assert_no_json_handoff(stdout: &str) {
    for forbidden in ["json-handshake", "selection-json", "{\""] {
        assert!(
            !stdout.contains(forbidden),
            "real gxtest output leaked serialized handoff marker {forbidden}: {stdout}"
        );
    }
}

fn assert_marline_loop_real_llm_case_assets() {
    let case_root = Path::new(MARLINE_CONFIG_INTERFACE_WORKSPACE)
        .join("custom")
        .join("marline-kernel")
        .join("policies")
        .join("loops")
        .join("cases");
    for case_file in [
        "runtime-handoff-llm.loop.json",
        "policy-receipt-gate-llm.loop.json",
        "loop-contract-llm.loop.json",
        "failure-retry-llm.loop.json",
    ] {
        assert!(
            case_root.join(case_file).is_file(),
            "missing real LLM loop case asset {case_file}"
        );
    }
    let catalog = fs::read_to_string(case_root.join("real-llm-catalog.toml"))
        .expect("read real LLM loop case catalog");
    for executor in [
        "marlin.real-llm.runtime-handoff",
        "marlin.real-llm.policy-receipt-gate",
        "marlin.real-llm.loop-contract",
        "marlin.real-llm.failure-retry",
    ] {
        assert!(
            catalog.contains(executor),
            "real LLM catalog missing executor {executor}"
        );
    }
    assert!(catalog.contains("command = \"sh\""));
    assert!(catalog.contains("run-real-llm-case.sh"));
    for case_file in [
        "runtime-handoff-llm.loop.json",
        "policy-receipt-gate-llm.loop.json",
        "loop-contract-llm.loop.json",
        "failure-retry-llm.loop.json",
    ] {
        let case_config =
            fs::read_to_string(case_root.join(case_file)).expect("read real LLM loop case");
        assert!(case_config.contains("\"policy_profile\""));
        assert!(case_config.contains("\"failure_policy\""));
        assert!(case_config.contains("\"max_iterations\": 3"));
        if case_file == "failure-retry-llm.loop.json" {
            assert!(case_config.contains("\"governance_policy\""));
            assert!(case_config.contains("\"backend\": \"nono\""));
            assert!(case_config.contains("\"profile_ref\": \"nono-profile\""));
        }
    }
}

fn run_marline_real_llm_loop_case(case_file: &str, continuation_planner: &str) -> String {
    let case_root = Path::new(MARLINE_CONFIG_INTERFACE_WORKSPACE)
        .join("custom")
        .join("marline-kernel")
        .join("policies")
        .join("loops")
        .join("cases");
    let input = case_root.join(case_file);
    let catalog = case_root.join("real-llm-catalog.toml");
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crate should live under workspace/crates");
    let manifest_path = repo_root.join("Cargo.toml");

    let output = Command::new("cargo")
        .current_dir(Path::new(MARLINE_CONFIG_INTERFACE_WORKSPACE))
        .args([
            "run",
            "--manifest-path",
            manifest_path.to_str().expect("utf8 manifest path"),
            "-p",
            "marlin-agent-core",
            "--bin",
            "marlin",
            "--",
            "loop",
            "run",
            "--input",
        ])
        .arg(&input)
        .arg("--catalog")
        .arg(&catalog)
        .args(["--continuation-planner", continuation_planner])
        .arg("--no-store")
        .output()
        .unwrap_or_else(|error| panic!("run real LLM loop case {case_file}: {error}"));

    assert!(
        output.status.success(),
        "real LLM loop case {case_file} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("real LLM loop stdout should be UTF-8")
}

fn real_llm_marker_value(stdout: &str, marker: &str) -> String {
    let start = stdout
        .find(marker)
        .unwrap_or_else(|| panic!("missing marker {marker} in stdout:\n{stdout}"))
        + marker.len();
    let rest = &stdout[start..];
    let end = rest
        .find(|character| matches!(character, '\\' | '"' | '\n' | '\r'))
        .unwrap_or(rest.len());
    rest[..end].to_owned()
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

fn copy_workspace_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("create destination workspace");
    for entry in fs::read_dir(source).expect("read workspace") {
        let entry = entry.expect("read workspace entry");
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            copy_workspace_tree(&source_path, &destination_path);
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
