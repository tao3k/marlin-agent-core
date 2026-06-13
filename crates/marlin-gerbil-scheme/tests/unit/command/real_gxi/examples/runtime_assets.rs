use super::support::{
    assert_workspace_patch_intent_artifact, local_gxi, test_root,
    write_deck_runtime_handshake_example,
};
use marlin_gerbil_scheme::{
    GERBIL_LOADPATH_ENV, GERBIL_POO_MOP_MODULE, GERBIL_POO_OBJECT_MODULE, GERBIL_POO_PACKAGE_NAME,
    GERBIL_POO_PROTO_MODULE, GerbilCompileResponse, gerbil_runtime_loadpath,
    write_gerbil_runtime_assets,
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

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
    let response: GerbilCompileResponse =
        serde_json::from_slice(&output.stdout).expect("decode example response");
    assert_workspace_patch_intent_artifact(response.artifact);
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
    let response: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("decode deck runtime handshake");
    assert_eq!(response["package"], "marlin-deck-runtime");
    assert_eq!(response["module"], ":marlin/deck-runtime");
    assert_eq!(
        response["poo_dependency"],
        "git.cons.io/mighty-gerbils/gerbil-poo"
    );
    assert_eq!(response["poo_package"], GERBIL_POO_PACKAGE_NAME);
    let poo_modules = response["poo_modules"]
        .as_array()
        .expect("deck runtime poo modules are an array");
    assert!(
        poo_modules
            .iter()
            .any(|value| value.as_str() == Some(GERBIL_POO_OBJECT_MODULE))
    );
    assert!(
        poo_modules
            .iter()
            .any(|value| value.as_str() == Some(GERBIL_POO_MOP_MODULE))
    );
    assert!(
        poo_modules
            .iter()
            .any(|value| value.as_str() == Some(GERBIL_POO_PROTO_MODULE))
    );
    let poo_forms = response["poo_forms"]
        .as_array()
        .expect("deck runtime poo forms are an array");
    assert!(poo_forms.iter().any(|value| value.as_str() == Some(".o")));
    assert!(
        poo_forms
            .iter()
            .any(|value| value.as_str() == Some(".defgeneric"))
    );
    assert!(
        poo_forms
            .iter()
            .any(|value| value.as_str() == Some("defmethod"))
    );
    let capabilities = response["capabilities"]
        .as_array()
        .expect("deck runtime capabilities are an array");
    assert!(
        capabilities
            .iter()
            .any(|value| value.as_str() == Some("rust-bridge"))
    );
    assert!(
        capabilities
            .iter()
            .any(|value| value.as_str() == Some("poo-object-system"))
    );
    let rust_contracts = response["rust_contracts"]
        .as_array()
        .expect("deck runtime rust contracts are an array");
    assert!(
        rust_contracts
            .iter()
            .any(|value| value.as_str() == Some("real-gxi"))
    );
    assert!(
        rust_contracts
            .iter()
            .any(|value| value.as_str() == Some("json-handshake"))
    );
}

#[test]
#[ignore = "requires a local Gerbil gxi executable and installed gerbil-poo dependency"]
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
    let response: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("decode deck runtime policy selection");
    assert_eq!(
        response["schema_id"],
        "marlin-deck-runtime.model-route-selection.v1"
    );
    assert_eq!(response["matched"], true);
    assert_eq!(
        response["policy"]["kind"],
        "marlin-deck-runtime.model-route-policy.v1"
    );
    assert_eq!(response["policy"]["name"], "cheap-test-runner");
    assert_eq!(response["policy"]["provider"], "openai");
    assert_eq!(response["policy"]["model"], "gpt-5-mini");
    assert_eq!(response["policy"]["context_mode"], "forked-context");
    assert_eq!(response["policy"]["isolation_mode"], "workspace-isolated");
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
    let response: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("decode deck runtime strategy selection");
    assert_eq!(
        response["schema_id"],
        "marlin-deck-runtime.strategy-selection.v1"
    );
    assert_eq!(response["matched"], true);
    assert_eq!(response["strategy_rule"], "customer-release-subagent");
    assert_eq!(response["hook_action"], "defer");
    assert_eq!(response["rewrite_command"], serde_json::Value::Null);
    assert_eq!(response["policy"]["name"], "deep-customer-reviewer");
    assert_eq!(response["policy"]["provider"], "anthropic");
    assert_eq!(response["policy"]["model"], "claude-opus-4-8");

    let signals = response["matched_signals"]
        .as_array()
        .expect("matched signals are an array");
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
            signals
                .iter()
                .any(|signal| signal.as_str() == Some(expected)),
            "missing strategy signal {expected}"
        );
    }

    let capabilities = response["capabilities"]
        .as_array()
        .expect("strategy capabilities are an array");
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
            capabilities
                .iter()
                .any(|capability| capability.as_str() == Some(expected)),
            "missing strategy capability {expected}"
        );
    }
}

fn write_deck_runtime_model_route_policy_example(path: &Path) {
    fs::write(
        path,
        r#"(import :marlin/deck-runtime)

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

(display-marlin-deck-runtime-model-route-selection-json
 policies
 "cargo test -p marlin-gerbil-scheme --test unit_test"
 "sub-agent")
(newline)
"#,
    )
    .expect("write deck runtime model route policy example");
}

fn write_deck_runtime_complex_strategy_example(path: &Path) {
    fs::write(
        path,
        r#"(import :marlin/deck-runtime
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
    (list "codex customer-review" "cargo test")
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

(display-marlin-deck-runtime-strategy-selection-json
 policies
 (list customer-release-subagent)
 context
 "codex customer-review --session release-session"
 "sub-agent")
(newline)
"#,
    )
    .expect("write deck runtime complex strategy example");
}
