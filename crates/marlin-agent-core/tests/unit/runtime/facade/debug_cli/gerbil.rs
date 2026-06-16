use marlin_agent_core::run_marlin_cli_from_args;
use serde_json::Value;
use std::path::Path;

#[test]
fn debug_cli_gerbil_policy_receipt_runs_scheme_policy_engine() {
    let gxi = Path::new("/usr/local/bin/gxi");
    let package_root = Path::new("crates/marlin-gerbil-scheme/gerbil");
    if !gxi.exists() || !package_root.exists() {
        return;
    }

    let result = run_marlin_cli_from_args([
        "gerbil",
        "policy-receipt",
        "--gxi",
        gxi.to_str().expect("gxi path"),
        "--package-root",
        package_root.to_str().expect("package root"),
        "--iterations",
        "7",
    ]);

    assert_eq!(result.status, 0, "{}", result.stderr);
    let summary: Value = serde_json::from_str(&result.stdout).expect("gerbil policy summary");
    assert_eq!(summary["status"], "ok");
    assert_eq!(summary["command"], "gerbil policy-receipt");
    assert_eq!(
        summary["entrypoint"],
        "src/marlin/deck-runtime-policy-receipt-gate-cli.ss"
    );
    assert_eq!(
        summary["extension_kind"],
        "marlin-deck-runtime.extension.v1"
    );
    assert_eq!(summary["extension_id"], "debug-policy-extension");
    assert_eq!(
        summary["extension_source"],
        ":marlin/deck-runtime-debug-policy-extension"
    );
    assert_eq!(summary["extension_surface"], "poo-extension-object");
    assert_eq!(summary["extension_capability_count"], 4);
    assert_eq!(
        summary["policy_extension_object_kind"],
        "marlin.modules.policy-extension-object.v1"
    );
    assert_eq!(summary["policy_extension_object"], true);
    assert_eq!(
        summary["policy_extension_source"],
        ":marlin/deck-runtime-debug-policy-extension"
    );
    assert_eq!(
        summary["policy_extension_managed_by"],
        "gerbil-module-system"
    );
    assert_eq!(summary["policy_extension_projection_owner"], "gerbil-poo");
    assert_eq!(summary["policy_extension_runtime_owner"], "rust");
    assert_eq!(
        summary["policy_module_kind"],
        "marlin.modules.policy-module.v1"
    );
    assert_eq!(summary["policy_module_id"], "debug-policy-extension-module");
    assert_eq!(summary["policy_module_family"], "subagent-policy-extension");
    assert_eq!(
        summary["policy_projection_target"],
        "extension-policy-receipt"
    );
    assert_eq!(summary["module_catalog_kind"], "marlin.modules.catalog.v1");
    assert_eq!(summary["module_catalog_count"], 1);
    assert_eq!(
        summary["module_eval_result_kind"],
        "marlin.modules.eval-result.v1"
    );
    assert_eq!(
        summary["module_eval_workflow_kind"],
        "marlin.modules.policy-workflow.v1"
    );
    assert_eq!(
        summary["policy_substrate_gate_kind"],
        "marlin.modules.policy-substrate-gate.v1"
    );
    assert_eq!(summary["policy_substrate_gate_profile"], "policy-substrate");
    assert_eq!(
        summary["policy_substrate_gate_receipt_kind"],
        "marlin-deck-runtime.extension-receipt.v1"
    );
    assert_eq!(
        summary["policy_module_evaluation_kind"],
        "marlin-deck-runtime.user-module-evaluation.v1"
    );
    assert_eq!(summary["policy_module_count"], 1);
    assert_eq!(summary["policy_extension_count"], 1);
    assert_eq!(summary["policy_extension_object_count"], 1);
    assert_eq!(summary["policy_script_count"], 0);
    assert_eq!(summary["policy_option_count"], 2);
    assert_eq!(summary["policy_validation_receipt_count"], 2);
    assert_eq!(summary["policy_substrate_gate_replayable"], true);
    assert_eq!(summary["scheme_policy_owner"], "gerbil-poo");
    assert_eq!(summary["rust_kernel_owner"], "rust");
    assert_eq!(
        summary["catalog_kind"],
        "marlin-deck-runtime.extension-catalog.v1"
    );
    assert_eq!(summary["scheme_catalog_role"], "extension-object-selection");
    assert_eq!(summary["runtime_catalog_owner"], "rust");
    assert_eq!(summary["catalog_resolved_by_scheme"], false);
    assert_eq!(summary["iterations"], 7);
    assert_eq!(
        summary["timing_scope"],
        "single-gxi-process-wall-clock-includes-startup"
    );
    assert!(
        summary["process_elapsed_micros"]
            .as_u64()
            .expect("elapsed micros")
            > 0,
        "{summary}"
    );
    assert!(
        summary["avg_process_micros_per_iteration"]
            .as_u64()
            .expect("avg micros")
            > 0,
        "{summary}"
    );
    assert!(
        summary["scheme_policy_loop_elapsed_micros"]
            .as_u64()
            .is_some(),
        "{summary}"
    );
    assert!(
        summary["avg_scheme_policy_micros_per_iteration"]
            .as_u64()
            .is_some(),
        "{summary}"
    );
    assert_eq!(
        summary["receipt_kind"],
        "marlin-deck-runtime.extension-receipt.v1"
    );
    assert_eq!(summary["matched"], true);
    assert_eq!(summary["policy_engine"], "scheme-poo-extension");
    assert_eq!(summary["extension_receipt_id"], "debug-policy-extension");
    assert_eq!(summary["dynamic_hook_action"], "register");
    assert_eq!(
        summary["dynamic_hook_hook_id"],
        "debug-runtime-catalog-hook"
    );
    assert_eq!(
        summary["dynamic_hook_registration"],
        "debug-runtime-catalog-hook"
    );
    assert_eq!(summary["dynamic_hook_selection_source"], "extension-action");
    assert_eq!(summary["dynamic_hook_selection_selector"], "#f");
}
