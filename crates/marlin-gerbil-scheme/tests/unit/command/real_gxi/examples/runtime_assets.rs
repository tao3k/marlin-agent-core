use super::support::{local_gxi, test_root, write_deck_runtime_handshake_example};
use marlin_gerbil_scheme::{
    GERBIL_LOADPATH_ENV, GERBIL_POO_MOP_MODULE, GERBIL_POO_OBJECT_MODULE, GERBIL_POO_PACKAGE_NAME,
    GERBIL_POO_PROTO_MODULE, gerbil_runtime_loadpath, write_gerbil_runtime_assets,
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn command_compiler_real_gxi_builds_runtime_package_and_runs_smoke_launcher() {
    let Some(gxi) = local_gxi() else {
        return;
    };
    let root = test_root("runtime-package-smoke");
    write_gerbil_runtime_assets(root.path()).expect("write gerbil runtime assets");

    let build_output = Command::new(&gxi)
        .env(GERBIL_LOADPATH_ENV, gerbil_runtime_loadpath(root.path()))
        .current_dir(root.path())
        .arg(root.path().join("build.ss"))
        .arg("compile")
        .output()
        .expect("run real gxi build script");

    assert!(
        build_output.status.success(),
        "gxi runtime package build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build_output.stdout),
        String::from_utf8_lossy(&build_output.stderr)
    );

    let smoke_output = Command::new(gxi)
        .env(GERBIL_LOADPATH_ENV, gerbil_runtime_loadpath(root.path()))
        .current_dir(root.path())
        .arg(root.path().join("bin/smoke.ss"))
        .output()
        .expect("run real gxi runtime smoke launcher");

    assert!(
        smoke_output.status.success(),
        "gxi runtime smoke launcher failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&smoke_output.stdout),
        String::from_utf8_lossy(&smoke_output.stderr)
    );
    assert_eq!(
        String::from_utf8(smoke_output.stdout).expect("smoke stdout is UTF-8"),
        "marlin-gerbil-smoke\n"
    );

    let manifest =
        fs::read_to_string(root.path().join("gerbil.pkg")).expect("read package manifest");
    assert!(manifest.contains("(package: marlin-deck-runtime"));
    assert!(manifest.contains("github.com/tao3k/poo-flow"));
    assert!(manifest.contains("github.com/tao3k/gerbil-scheme-language-project-harness"));
    assert!(!manifest.contains("/Users/"));

    let native_source = fs::read_to_string(root.path().join("src/marlin/deck-runtime-native.ss"))
        .expect("read native runtime source");
    assert!(native_source.contains("marlin_deck_runtime_initialize"));
    assert!(native_source.contains("marlin_deck_runtime_select_model_route"));
}

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn command_compiler_real_gxi_runs_workspace_patch_intent_example_from_runtime_assets() {
    let Some(gxi) = local_gxi() else {
        return;
    };
    let root = test_root("runtime-example");
    write_gerbil_runtime_assets(root.path()).expect("write gerbil runtime assets");
    let example = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("workspace-patch-intent.ss");

    let output = Command::new(gxi)
        .env(GERBIL_LOADPATH_ENV, gerbil_runtime_loadpath(root.path()))
        .arg(example)
        .output()
        .expect("run real gxi workspace patch intent example");

    assert!(
        output.status.success(),
        "gxi example failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("workspace patch intent stdout is UTF-8");
    assert_eq!(stdout, "workspace-patch-intent-artifact\n");
    assert!(!stdout.contains("{\""));
}

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn command_compiler_real_gxi_deck_runtime_capability_handshake() {
    let Some(gxi) = local_gxi() else {
        return;
    };
    let root = test_root("deck-runtime-handshake");
    write_gerbil_runtime_assets(root.path()).expect("write gerbil runtime assets");
    let example = root.path().join("deck-runtime-handshake.ss");
    write_deck_runtime_handshake_example(&example);

    let output = Command::new(gxi)
        .env(GERBIL_LOADPATH_ENV, gerbil_runtime_loadpath(root.path()))
        .arg(example)
        .output()
        .expect("run real gxi deck runtime handshake");

    assert!(
        output.status.success(),
        "gxi deck runtime handshake failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("handshake output should be UTF-8");
    assert!(stdout.contains("marlin-deck-runtime"));
    assert!(stdout.contains(":marlin/deck-runtime"));
    assert!(stdout.contains("github.com/tao3k/poo-flow"));
    assert!(stdout.contains(GERBIL_POO_PACKAGE_NAME));
    assert!(stdout.contains(GERBIL_POO_OBJECT_MODULE));
    assert!(stdout.contains(GERBIL_POO_MOP_MODULE));
    assert!(stdout.contains(GERBIL_POO_PROTO_MODULE));
    assert!(stdout.contains(".o"));
    assert!(stdout.contains(".defgeneric"));
    assert!(stdout.contains("defmethod"));
    assert!(stdout.contains("rust-bridge"));
    assert!(stdout.contains("poo-object-system"));
    assert!(stdout.contains("real-gxi"));
    assert!(stdout.contains("typed-native-abi"));
}

#[test]
#[ignore = "requires a local Gerbil gxi executable and installed poo-flow package dependency"]
fn command_compiler_real_gxi_deck_runtime_selects_scheme_model_route_policy() {
    let Some(gxi) = local_gxi() else {
        return;
    };
    let root = test_root("deck-runtime-model-route-policy");
    write_gerbil_runtime_assets(root.path()).expect("write gerbil runtime assets");
    let example = root.path().join("deck-runtime-model-route-policy.ss");
    write_deck_runtime_model_route_policy_example(&example);

    let output = Command::new(gxi)
        .env(GERBIL_LOADPATH_ENV, gerbil_runtime_loadpath(root.path()))
        .arg(example)
        .output()
        .expect("run real gxi deck runtime model route policy");

    assert!(
        output.status.success(),
        "gxi deck runtime model route policy failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("policy selection output should be UTF-8");
    assert_no_json_handoff(&stdout);
    assert!(stdout.contains("marlin-deck-runtime.model-route-selection.v1"));
    assert!(stdout.contains("matched"));
    assert!(stdout.contains("marlin-deck-runtime.model-route-policy.v1"));
    assert!(stdout.contains("cheap-test-runner"));
    assert!(stdout.contains("openai"));
    assert!(stdout.contains("gpt-5-mini"));
    assert!(stdout.contains("forked-context"));
    assert!(stdout.contains("workspace-isolated"));
}

#[test]
#[ignore = "requires a local Gerbil gxi executable and installed gerbil-poo dependency"]
fn command_compiler_real_gxi_deck_runtime_runs_scheme_complex_strategy() {
    let Some(gxi) = local_gxi() else {
        return;
    };
    let root = test_root("deck-runtime-complex-strategy");
    write_gerbil_runtime_assets(root.path()).expect("write gerbil runtime assets");
    let example = root.path().join("deck-runtime-complex-strategy.ss");
    write_deck_runtime_complex_strategy_example(&example);

    let output = Command::new(gxi)
        .env(GERBIL_LOADPATH_ENV, gerbil_runtime_loadpath(root.path()))
        .arg(example)
        .output()
        .expect("run real gxi deck runtime complex strategy");

    assert!(
        output.status.success(),
        "gxi deck runtime complex strategy failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout =
        String::from_utf8(output.stdout).expect("strategy selection output should be UTF-8");
    assert_no_json_handoff(&stdout);
    assert!(stdout.contains("marlin-deck-runtime.strategy-selection.v1"));
    assert!(stdout.contains("matched"));
    assert!(stdout.contains("customer-release-subagent"));
    assert!(stdout.contains("defer"));
    assert!(stdout.contains("deep-customer-reviewer"));
    assert!(stdout.contains("anthropic"));
    assert!(stdout.contains("claude-opus-4-8"));

    for expected in [
        "model-route",
        "command-prefix",
        "agent-scope",
        "session",
        "agent-lineage",
        "workspace-state",
        "org-memory",
        "customer-agent",
        "high-order-matcher",
    ] {
        assert!(
            stdout.contains(expected),
            "missing strategy signal {expected}"
        );
    }

    for expected in [
        "session-policy",
        "agent-lineage-policy",
        "workspace-state-policy",
        "org-memory-policy",
        "dynamic-hook-action",
        "customer-agent-policy",
        "high-order-matcher",
        "strategy-template-macro",
    ] {
        assert!(
            stdout.contains(expected),
            "missing strategy capability {expected}"
        );
    }
}

#[test]
#[ignore = "requires a local Gerbil gxi executable and installed gerbil-poo dependency"]
fn command_compiler_real_gxi_deck_runtime_runs_compiled_policy_macro_selector() {
    let Some(gxi) = local_gxi() else {
        return;
    };
    let root = test_root("deck-runtime-compiled-policy");
    write_gerbil_runtime_assets(root.path()).expect("write gerbil runtime assets");
    let example = root.path().join("deck-runtime-compiled-policy.ss");
    write_deck_runtime_compiled_policy_example(&example);

    let output = Command::new(gxi)
        .env(GERBIL_LOADPATH_ENV, gerbil_runtime_loadpath(root.path()))
        .arg(example)
        .output()
        .expect("run real gxi deck runtime compiled policy selector");

    assert!(
        output.status.success(),
        "gxi deck runtime compiled policy failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("compiled policy output should be UTF-8");
    assert_no_json_handoff(&stdout);
    assert!(stdout.contains("marlin-deck-runtime.compiled-policy.v1"));
    assert!(stdout.contains("compiled-policy-match"));
}

#[test]
#[ignore = "requires a local Gerbil gxi executable and installed gerbil-poo dependency"]
fn command_compiler_real_gxi_deck_runtime_runs_compiled_policy_macro_selector_batch() {
    let Some(gxi) = local_gxi() else {
        return;
    };
    let root = test_root("deck-runtime-compiled-policy-batch");
    write_gerbil_runtime_assets(root.path()).expect("write gerbil runtime assets");
    let example = root.path().join("deck-runtime-compiled-policy-batch.ss");
    write_deck_runtime_compiled_policy_batch_example(&example, 10_000);

    let output = Command::new(gxi)
        .env(GERBIL_LOADPATH_ENV, gerbil_runtime_loadpath(root.path()))
        .arg(example)
        .output()
        .expect("run real gxi deck runtime compiled policy selector batch");

    assert!(
        output.status.success(),
        "gxi deck runtime compiled policy batch failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("batch output should be UTF-8");
    assert_no_json_handoff(&stdout);
    assert!(stdout.contains("marlin-deck-runtime.compiled-policy.v1"));
    assert!(stdout.contains("iterations=10000"));
    assert!(stdout.contains("matches=10000"));
    assert!(stdout.contains("policy_elapsed_us="));
    assert!(stdout.contains("index_elapsed_us="));
}

fn assert_no_json_handoff(stdout: &str) {
    for forbidden in ["json-handshake", "selection-json", "{\""] {
        assert!(
            !stdout.contains(forbidden),
            "real gxi output leaked serialized handoff marker {forbidden}: {stdout}"
        );
    }
}

fn write_deck_runtime_model_route_policy_example(path: &Path) {
    fs::write(
        path,
        r#"(import :clan/poo/object
        :marlin/deck-runtime)

(def policies
  (list
   (make-marlin-deck-runtime-model-route-policy
    "cheap-test-runner"
    "openai"
    "gpt-5-mini"
    (list "cargo test" "just gerbil-deps-verify")
    (list "sub-agent" "hook")
    "forked-context"
    "workspace-isolated")
   (make-marlin-deck-runtime-model-route-policy
    "deep-runtime-reviewer"
    "anthropic"
    "claude-opus-4-8"
    (list "cargo clippy" "asp rust check")
    (list "reviewer")
    "shared-context"
    "isolated-session")))

(def selection
  (marlin-deck-runtime-model-route-selection
   policies
   "cargo test -p marlin-gerbil-scheme --test unit_test"
   "sub-agent"))

(display (.get selection kind))
(newline)
(display (if (.get selection matched) "matched" "miss"))
(newline)
(def selected-policy (.get selection policy))
(display (.get selected-policy kind))
(newline)
(display (.get selected-policy name))
(newline)
(display (.get selected-policy provider))
(newline)
(display (.get selected-policy model))
(newline)
(display (.get selected-policy context-mode))
(newline)
(display (.get selected-policy isolation-mode))
(newline)
"#,
    )
    .expect("write deck runtime model route policy example");
}

fn write_deck_runtime_compiled_policy_example(path: &Path) {
    fs::write(
        path,
        r#"(import :marlin/deck-runtime
        :marlin/deck-runtime-compiled-policy)

(defmarlin-deck-runtime-compiled-route-selector
  select-compiled-policy
  ("compiled-cheap-test-runner"
   "openai"
   "gpt-5-mini"
   ("cargo test" "just test")
   ("sub-agent" "hook")
   "forked-context"
   "workspace-isolated")
  ("compiled-deep-reviewer"
   "anthropic"
   "claude-opus-4-8"
   ("gpt-5.5 customer-review" "cargo clippy")
   ("reviewer")
   "shared-context"
   "isolated-session"))

(display marlin-deck-runtime-compiled-policy-kind)
(newline)
(display
 (if (select-compiled-policy
      "cargo test -p marlin-gerbil-scheme"
      "sub-agent")
   "compiled-policy-match"
   "compiled-policy-miss"))
(newline)
"#,
    )
    .expect("write deck runtime compiled policy example");
}

fn write_deck_runtime_compiled_policy_batch_example(path: &Path, iterations: usize) {
    fs::write(
        path,
        format!(
            r#"(import :marlin/deck-runtime-compiled-policy-sample)

(display-marlin-deck-runtime-sample-compiled-policy-batch-metrics
 {iterations}
 "cargo test -p marlin-gerbil-scheme"
 "sub-agent")

(display-marlin-deck-runtime-sample-compiled-policy-index-batch-metrics
 {iterations}
 "cargo test -p marlin-gerbil-scheme"
 "sub-agent")
"#
        ),
    )
    .expect("write deck runtime compiled policy template batch example");
}

fn write_deck_runtime_complex_strategy_example(path: &Path) {
    fs::write(
        path,
        r#"(import :clan/poo/object
        :marlin/deck-runtime
        :marlin/deck-runtime-strategy)

(def policies
  (list
   (make-marlin-deck-runtime-model-route-policy
    "cheap-test-runner"
    "openai"
    "gpt-5-mini"
    (list "cargo test")
    (list "sub-agent" "hook")
    "forked-context"
    "workspace-isolated")
   (make-marlin-deck-runtime-model-route-policy
    "deep-customer-reviewer"
    "anthropic"
    "claude-opus-4-8"
    (list "gpt-5.5 customer-review" "cargo test")
    (list "sub-agent")
    "shared-context"
    "isolated-session")))

(defmarlin-deck-runtime-strategy-rule
  customer-release-subagent
  "customer-release-subagent"
  "deep-customer-reviewer"
  "release-session"
  (list "root-agent" "customer-agent")
  (list "real-gxi-ready" "org-memory-indexed")
  (list "hook-roadmap" "runtime-positioning")
  "customer-agent"
  "defer"
  "")

(def context
  (make-marlin-deck-runtime-strategy-context
   "release-session"
   (list "root-agent" "customer-agent" "review-agent")
   (list "real-gxi-ready" "org-memory-indexed" "dirty-docs-ok")
   (list "hook-roadmap" "runtime-positioning" "native-aot-benchmark")
   "customer-agent"))

(def selection
  (marlin-deck-runtime-strategy-selection
   policies
   (list customer-release-subagent)
   context
   "gpt-5.5 customer-review --session release-session"
   "sub-agent"))

(display (.get selection kind))
(newline)
(display (if (.get selection matched) "matched" "miss"))
(newline)
(display (.get selection strategy-rule))
(newline)
(display (.get selection hook-action))
(newline)
(write (.get selection matched-signals))
(newline)
(write (.get selection capabilities))
(newline)
(def selected-policy (.get selection policy))
(display (.get selected-policy name))
(newline)
(display (.get selected-policy provider))
(newline)
(display (.get selected-policy model))
(newline)
"#,
    )
    .expect("write deck runtime complex strategy example");
}
