use marlin_agent_core::run_marlin_cli_from_args;
use serde_json::{Value, json};
use std::path::Path;

fn assert_package_owned_debug_loadpath(summary: &Value) {
    let loadpath = summary["loadpath"]
        .as_str()
        .expect("gerbil policy receipt loadpath");
    let entries = loadpath.split(':').collect::<Vec<_>>();
    assert!(
        entries.iter().any(|entry| *entry == "src"),
        "expected marlin package src in loadpath: {loadpath}"
    );
    assert!(
        entries.iter().any(|entry| *entry == ".gerbil/lib"),
        "expected gerbil.pkg dependency library in loadpath: {loadpath}"
    );
    assert!(
        entries.iter().any(|entry| *entry == "t"),
        "expected marlin package t in loadpath: {loadpath}"
    );
    assert!(
        !entries.iter().any(|entry| entry.ends_with("poo-flow/src")),
        "poo-flow must be resolved through gerbil.pkg/gxpkg, not a local checkout loadpath: {loadpath}"
    );
    assert!(
        !loadpath.contains("/Users/"),
        "default debug loadpath must not capture a developer-local checkout: {loadpath}"
    );
}

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
    assert_package_owned_debug_loadpath(&summary);
    assert_eq!(summary["status"], "ok");
    assert_eq!(summary["command"], "gerbil policy-receipt");
    assert_eq!(
        summary["call_expr"],
        "(emit-policy-receipt-gate-cli-report)"
    );
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
    assert_eq!(summary["policy_extension_managed_by"], "poo-flow.modules");
    assert_eq!(
        summary["policy_extension_projection_owner"],
        "poo-flow.scheme"
    );
    assert_eq!(summary["policy_extension_runtime_owner"], "rust");
    assert_eq!(summary["policy_module_kind"], "poo-flow-module");
    assert_eq!(summary["policy_module_id"], "debug-policy-extension-module");
    assert_eq!(summary["policy_module_family"], "subagent-policy-extension");
    assert_eq!(
        summary["policy_projection_target"],
        "extension-policy-receipt"
    );
    assert_eq!(
        summary["module_catalog_kind"],
        "poo-flow.modules.value-catalog.v1"
    );
    assert_eq!(summary["module_catalog_count"], 1);
    assert_eq!(
        summary["module_eval_result_kind"],
        "poo-flow.modules.eval-result.v1"
    );
    assert_eq!(
        summary["module_eval_workflow_kind"],
        "marlin.modules.policy-workflow.v1"
    );
    assert_eq!(
        summary["module_system_presentation_kind"],
        "poo-flow.modules.system-presentation.v1"
    );
    assert_eq!(
        summary["module_system_projection_chain_kind"],
        "marlin.modules.projection-chain.v1"
    );
    assert_eq!(summary["module_system_root_import_count"], 0);
    assert_eq!(summary["module_system_root_extension_count"], 1);
    assert_eq!(
        summary["module_system_root_policy_extension_object_count"],
        1
    );
    assert_eq!(
        summary["module_system_import_graph_owner"],
        "poo-flow.modules"
    );
    assert_eq!(
        summary["module_system_option_merge_owner"],
        "poo-flow.modules"
    );
    assert_eq!(
        summary["module_system_extension_composition_owner"],
        "poo-flow.modules"
    );
    assert_eq!(
        summary["module_system_native_projection_payload_owner"],
        "rust"
    );
    assert_eq!(summary["module_system_budget_receipt_owner"], "rust");
    assert_eq!(
        summary["module_system_catalog_resolution_receipt_owner"],
        "rust"
    );
    assert_eq!(summary["module_system_rust_parses_scheme_source"], false);
    assert_eq!(
        summary["module_system_scheme_manufactures_rust_handlers"],
        false
    );
    assert_eq!(summary["policy_pack_kind"], "marlin.modules.policy-pack.v1");
    assert_eq!(summary["policy_pack_id"], "debug-policy-prefab-pack");
    assert_eq!(
        summary["policy_pack_presentation_kind"],
        "marlin.modules.policy-pack-presentation.v1"
    );
    assert_eq!(
        summary["policy_pack_inventory_kind"],
        "marlin.modules.policy-pack-inventory.v1"
    );
    assert_eq!(
        summary["policy_pack_module_system_presentation_kind"],
        "poo-flow.modules.system-presentation.v1"
    );
    assert_eq!(summary["policy_pack_object_count"], 3);
    assert_eq!(summary["policy_pack_default_object_count"], 3);
    assert_eq!(summary["policy_pack_disabled_object_count"], 1);
    assert_eq!(
        summary["policy_pack_policy_families"],
        json!([
            "model-route-policy",
            "human-review-policy",
            "memory-trigger-policy"
        ])
    );
    assert_eq!(
        summary["policy_pack_policy_object_ids"],
        json!([
            "debug-fast-route",
            "debug-human-review",
            "debug-memory-trigger"
        ])
    );
    assert_eq!(
        summary["policy_pack_default_policy_object_ids"],
        json!([
            "debug-policy-extension",
            "debug-human-review",
            "debug-runtime-catalog-hook"
        ])
    );
    assert_eq!(
        summary["policy_pack_disabled_policy_object_ids"],
        json!(["debug-human-review"])
    );
    assert_eq!(summary["policy_pack_operation_count"], 4);
    assert_eq!(summary["policy_pack_surgery_receipt_count"], 4);
    assert_eq!(summary["policy_pack_conflict_surgery_receipt_count"], 0);
    assert_eq!(summary["policy_pack_duplicate_object_conflict_count"], 0);
    assert_eq!(summary["policy_pack_missing_target_conflict_count"], 0);
    assert_eq!(summary["policy_pack_disabled_target_conflict_count"], 0);
    assert_eq!(summary["policy_pack_invalid_replacement_conflict_count"], 0);
    assert_eq!(summary["policy_pack_add_count"], 1);
    assert_eq!(summary["policy_pack_remove_count"], 1);
    assert_eq!(summary["policy_pack_disable_count"], 1);
    assert_eq!(summary["policy_pack_replace_count"], 1);
    assert_eq!(summary["policy_pack_matched_surgery_receipt_count"], 4);
    assert_eq!(summary["policy_pack_allowed_hook_count"], 1);
    assert_eq!(
        summary["policy_pack_allowed_hook_ids"],
        json!(["debug-runtime-catalog-hook"])
    );
    assert_eq!(
        summary["policy_pack_import_graph_owner"],
        "poo-flow.modules"
    );
    assert_eq!(
        summary["policy_pack_option_merge_owner"],
        "poo-flow.modules"
    );
    assert_eq!(
        summary["policy_pack_extension_composition_owner"],
        "poo-flow.modules"
    );
    assert_eq!(
        summary["policy_pack_native_projection_payload_owner"],
        "rust"
    );
    assert_eq!(summary["policy_pack_rust_parses_scheme_source"], false);
    assert_eq!(summary["policy_pack_rust_handler_manufactured"], false);
    assert_eq!(
        summary["policy_projection_kind"],
        "marlin.modules.policy-projection.v1"
    );
    assert_eq!(
        summary["policy_projection_pack_id"],
        "debug-policy-prefab-pack"
    );
    assert_eq!(
        summary["policy_projection_chain_kind"],
        "marlin.modules.projection-chain.v1"
    );
    assert_eq!(
        summary["policy_projection_module_evaluation_receipt_kind"],
        "marlin.modules.policy-pack.module-evaluation-receipt.v1"
    );
    assert_eq!(
        summary["policy_projection_policy_projection_receipt_kind"],
        "marlin.modules.policy-projection.v1"
    );
    assert_eq!(
        summary["policy_projection_native_projection_payload_kind"],
        "marlin.modules.policy-pack-presentation.v1"
    );
    assert_eq!(
        summary["policy_projection_native_projection_payload_owner"],
        "rust"
    );
    assert_eq!(summary["policy_projection_budget_receipt_owner"], "rust");
    assert_eq!(
        summary["policy_projection_catalog_resolution_receipt_owner"],
        "rust"
    );
    assert_eq!(
        summary["policy_projection_import_graph_owner"],
        "poo-flow.modules"
    );
    assert_eq!(
        summary["policy_projection_option_merge_owner"],
        "poo-flow.modules"
    );
    assert_eq!(
        summary["policy_projection_extension_composition_owner"],
        "poo-flow.modules"
    );
    assert_eq!(
        summary["policy_projection_policy_composition_owner"],
        "poo-flow.scheme"
    );
    assert_eq!(summary["policy_projection_runtime_lifecycle_owner"], "rust");
    assert_eq!(
        summary["policy_projection_rust_parses_scheme_source"],
        false
    );
    assert_eq!(
        summary["policy_projection_rust_handler_manufactured"],
        false
    );
    assert_eq!(summary["policy_projection_replayable"], true);
    assert_eq!(
        summary["policy_projection_chain_receipt_kind"],
        "marlin.modules.policy-projection-chain-receipt.v1"
    );
    assert_eq!(
        summary["policy_projection_chain_receipt_pack_id"],
        "debug-policy-prefab-pack"
    );
    assert_eq!(
        summary["policy_projection_chain_module_evaluation_receipt_kind"],
        "marlin.modules.policy-pack.module-evaluation-receipt.v1"
    );
    assert_eq!(
        summary["policy_projection_chain_policy_projection_receipt_kind"],
        "marlin.modules.policy-projection.v1"
    );
    assert_eq!(
        summary["policy_projection_chain_native_projection_payload_kind"],
        "marlin.modules.policy-pack-presentation.v1"
    );
    assert_eq!(
        summary["policy_projection_chain_budget_receipt_kind"],
        "marlin.runtime.policy-budget-receipt.v1"
    );
    assert_eq!(
        summary["policy_projection_chain_catalog_resolution_receipt_kind"],
        "marlin.runtime.policy-catalog-resolution-receipt.v1"
    );
    assert_eq!(summary["policy_projection_chain_receipt_family_count"], 5);
    assert_eq!(
        summary["policy_projection_chain_receipt_family_ids"],
        json!([
            "module_evaluation_receipt",
            "policy_projection_receipt",
            "native_projection_payload",
            "budget_receipt",
            "catalog_resolution_receipt"
        ])
    );
    assert_eq!(
        summary["policy_projection_chain_module_evaluation_receipt_owner"],
        "poo-flow.modules"
    );
    assert_eq!(
        summary["policy_projection_chain_policy_projection_receipt_owner"],
        "poo-flow.scheme"
    );
    assert_eq!(
        summary["policy_projection_chain_native_projection_payload_owner"],
        "rust"
    );
    assert_eq!(
        summary["policy_projection_chain_budget_receipt_owner"],
        "rust"
    );
    assert_eq!(
        summary["policy_projection_chain_catalog_resolution_receipt_owner"],
        "rust"
    );
    assert_eq!(
        summary["policy_projection_chain_catalog_allowed_hook_count"],
        1
    );
    assert_eq!(summary["policy_projection_chain_replayable"], true);
    assert_eq!(
        summary["default_policy_delivery_kind"],
        "marlin.modules.prefabs.default-policy.delivery-receipt.v1"
    );
    assert_eq!(
        summary["default_policy_pack_id"],
        "marlin-default-policy-pack"
    );
    assert_eq!(summary["default_policy_pack_count"], 1);
    assert_eq!(
        summary["default_policy_pack_ids"],
        json!(["marlin-default-policy-pack"])
    );
    assert_eq!(summary["default_policy_object_count"], 18);
    assert_eq!(summary["default_policy_default_object_count"], 18);
    assert_eq!(summary["default_policy_allowed_hook_count"], 1);
    assert_eq!(
        summary["default_policy_allowed_hook_ids"],
        json!(["runtime-catalog-default-hook"])
    );
    assert_eq!(
        summary["default_policy_catalog_presentation_kind"],
        "marlin.modules.policy-pack-catalog-presentation.v1"
    );
    assert_eq!(
        summary["default_policy_projection_kind"],
        "marlin.modules.policy-projection.v1"
    );
    assert_eq!(
        summary["default_policy_projection_chain_receipt_kind"],
        "marlin.modules.policy-projection-chain-receipt.v1"
    );
    assert_eq!(
        summary["default_policy_budget_receipt_kind"],
        "marlin.runtime.policy-budget-receipt.v1"
    );
    assert_eq!(
        summary["default_policy_catalog_resolution_receipt_kind"],
        "marlin.runtime.policy-catalog-resolution-receipt.v1"
    );
    assert_eq!(summary["default_policy_replayable"], true);
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
        "poo-flow.modules.runtime-evaluation.v1"
    );
    assert_eq!(summary["policy_module_count"], 1);
    assert_eq!(summary["policy_extension_count"], 1);
    assert_eq!(summary["policy_extension_object_count"], 1);
    assert_eq!(summary["policy_script_count"], 0);
    assert_eq!(summary["policy_option_count"], 2);
    assert_eq!(summary["policy_validation_receipt_count"], 2);
    assert_eq!(summary["policy_substrate_gate_replayable"], true);
    assert_eq!(summary["scheme_policy_owner"], "poo-flow.scheme");
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

#[test]
fn debug_cli_gerbil_policy_receipt_loads_user_entrypoint_call() {
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
        "--entrypoint",
        "t/fixtures/custom-policy-receipt-entrypoint.ss",
        "--call",
        "(emit-user-authored-policy-receipt-gate-cli-report)",
        "--iterations",
        "3",
    ]);

    assert_eq!(result.status, 0, "{}", result.stderr);
    let summary: Value = serde_json::from_str(&result.stdout).expect("gerbil policy summary");
    assert_package_owned_debug_loadpath(&summary);
    assert_eq!(summary["status"], "ok");
    assert_eq!(
        summary["entrypoint"],
        "t/fixtures/custom-policy-receipt-entrypoint.ss"
    );
    assert_eq!(
        summary["call_expr"],
        "(emit-user-authored-policy-receipt-gate-cli-report)"
    );
    assert_eq!(
        summary["policy_projection_kind"],
        "marlin.modules.policy-projection.v1"
    );
    assert_eq!(
        summary["policy_projection_native_projection_payload_kind"],
        "marlin.modules.policy-pack-presentation.v1"
    );
    assert_eq!(
        summary["policy_projection_policy_composition_owner"],
        "poo-flow.scheme"
    );
    assert_eq!(summary["policy_projection_budget_receipt_owner"], "rust");
    assert_eq!(
        summary["policy_projection_catalog_resolution_receipt_owner"],
        "rust"
    );
    assert_eq!(summary["policy_projection_replayable"], true);
    assert_eq!(summary["iterations"], 3);
}
