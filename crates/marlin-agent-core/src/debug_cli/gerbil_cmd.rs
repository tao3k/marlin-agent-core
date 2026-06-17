//! `marlin gerbil ...` command implementations.

use std::{collections::BTreeMap, path::PathBuf, process::Command, time::Instant};

use serde::Serialize;

use super::{
    MarlinCliResult,
    args::{ArgCursor, GerbilPolicyReceiptOptions},
    gerbil_usage,
};

#[derive(Clone, Debug, Serialize)]
struct GerbilPolicyReceiptDebugSummary {
    status: &'static str,
    command: &'static str,
    call_expr: String,
    entrypoint: PathBuf,
    gxi: PathBuf,
    package_root: PathBuf,
    loadpath: String,
    extension_kind: String,
    extension_id: String,
    extension_source: String,
    extension_surface: String,
    extension_capability_count: u64,
    policy_extension_object_kind: String,
    policy_extension_object: bool,
    policy_extension_source: String,
    policy_extension_managed_by: String,
    policy_extension_projection_owner: String,
    policy_extension_runtime_owner: String,
    policy_module_kind: String,
    policy_module_id: String,
    policy_module_family: String,
    policy_projection_target: String,
    module_catalog_kind: String,
    module_catalog_count: u64,
    module_eval_result_kind: String,
    module_eval_workflow_kind: String,
    module_system_presentation_kind: String,
    module_system_projection_chain_kind: String,
    module_system_root_import_count: u64,
    module_system_root_extension_count: u64,
    module_system_root_policy_extension_object_count: u64,
    module_system_import_graph_owner: String,
    module_system_option_merge_owner: String,
    module_system_extension_composition_owner: String,
    module_system_native_projection_payload_owner: String,
    module_system_budget_receipt_owner: String,
    module_system_catalog_resolution_receipt_owner: String,
    module_system_rust_parses_scheme_source: bool,
    module_system_scheme_manufactures_rust_handlers: bool,
    policy_pack_kind: String,
    policy_pack_id: String,
    policy_pack_presentation_kind: String,
    policy_pack_inventory_kind: String,
    policy_pack_module_system_presentation_kind: String,
    policy_pack_object_count: u64,
    policy_pack_default_object_count: u64,
    policy_pack_disabled_object_count: u64,
    policy_pack_policy_families: Vec<String>,
    policy_pack_policy_object_ids: Vec<String>,
    policy_pack_default_policy_object_ids: Vec<String>,
    policy_pack_disabled_policy_object_ids: Vec<String>,
    policy_pack_operation_count: u64,
    policy_pack_surgery_receipt_count: u64,
    policy_pack_conflict_surgery_receipt_count: u64,
    policy_pack_duplicate_object_conflict_count: u64,
    policy_pack_missing_target_conflict_count: u64,
    policy_pack_disabled_target_conflict_count: u64,
    policy_pack_invalid_replacement_conflict_count: u64,
    policy_pack_add_count: u64,
    policy_pack_remove_count: u64,
    policy_pack_disable_count: u64,
    policy_pack_replace_count: u64,
    policy_pack_matched_surgery_receipt_count: u64,
    policy_pack_allowed_hook_count: u64,
    policy_pack_allowed_hook_ids: Vec<String>,
    policy_pack_import_graph_owner: String,
    policy_pack_option_merge_owner: String,
    policy_pack_extension_composition_owner: String,
    policy_pack_native_projection_payload_owner: String,
    policy_pack_rust_parses_scheme_source: bool,
    policy_pack_rust_handler_manufactured: bool,
    policy_projection_kind: String,
    policy_projection_pack_id: String,
    policy_projection_chain_kind: String,
    policy_projection_module_evaluation_receipt_kind: String,
    policy_projection_policy_projection_receipt_kind: String,
    policy_projection_native_projection_payload_kind: String,
    policy_projection_native_projection_payload_owner: String,
    policy_projection_budget_receipt_owner: String,
    policy_projection_catalog_resolution_receipt_owner: String,
    policy_projection_import_graph_owner: String,
    policy_projection_option_merge_owner: String,
    policy_projection_extension_composition_owner: String,
    policy_projection_policy_composition_owner: String,
    policy_projection_runtime_lifecycle_owner: String,
    policy_projection_rust_parses_scheme_source: bool,
    policy_projection_rust_handler_manufactured: bool,
    policy_projection_replayable: bool,
    policy_projection_chain_receipt_kind: String,
    policy_projection_chain_receipt_pack_id: String,
    policy_projection_chain_module_evaluation_receipt_kind: String,
    policy_projection_chain_policy_projection_receipt_kind: String,
    policy_projection_chain_native_projection_payload_kind: String,
    policy_projection_chain_budget_receipt_kind: String,
    policy_projection_chain_catalog_resolution_receipt_kind: String,
    policy_projection_chain_replayable: bool,
    default_policy_delivery_kind: String,
    default_policy_pack_id: String,
    default_policy_pack_count: u64,
    default_policy_pack_ids: Vec<String>,
    default_policy_object_count: u64,
    default_policy_default_object_count: u64,
    default_policy_allowed_hook_count: u64,
    default_policy_allowed_hook_ids: Vec<String>,
    default_policy_catalog_presentation_kind: String,
    default_policy_projection_kind: String,
    default_policy_projection_chain_receipt_kind: String,
    default_policy_budget_receipt_kind: String,
    default_policy_catalog_resolution_receipt_kind: String,
    default_policy_replayable: bool,
    policy_substrate_gate_kind: String,
    policy_substrate_gate_profile: String,
    policy_substrate_gate_receipt_kind: String,
    policy_module_evaluation_kind: String,
    policy_module_count: u64,
    policy_extension_count: u64,
    policy_extension_object_count: u64,
    policy_script_count: u64,
    policy_option_count: u64,
    policy_validation_receipt_count: u64,
    policy_substrate_gate_replayable: bool,
    scheme_policy_owner: String,
    rust_kernel_owner: String,
    catalog_kind: String,
    scheme_catalog_role: String,
    runtime_catalog_owner: String,
    catalog_resolved_by_scheme: bool,
    iterations: u64,
    timing_scope: &'static str,
    process_elapsed_micros: u64,
    avg_process_micros_per_iteration: u64,
    scheme_policy_loop_elapsed_micros: u64,
    avg_scheme_policy_micros_per_iteration: u64,
    receipt_kind: String,
    matched: bool,
    policy_engine: String,
    extension_receipt_id: String,
    dynamic_hook_action: String,
    dynamic_hook_hook_id: String,
    dynamic_hook_registration: String,
    dynamic_hook_selection_source: String,
    dynamic_hook_selection_selector: String,
}

pub(super) fn dispatch_gerbil(cursor: &mut ArgCursor) -> Result<MarlinCliResult, String> {
    let Some(command) = cursor.next() else {
        return Err(format!("missing gerbil command\n{}", gerbil_usage()));
    };

    match command.as_str() {
        "policy-receipt" => {
            let options = GerbilPolicyReceiptOptions::parse(cursor)?;
            let summary = run_policy_receipt(options)?;
            Ok(MarlinCliResult::success_json(&summary))
        }
        "-h" | "--help" | "help" => Ok(MarlinCliResult::success_text(gerbil_usage())),
        unknown => Err(format!(
            "unknown gerbil command `{unknown}`\n{}",
            gerbil_usage()
        )),
    }
}

fn run_policy_receipt(
    options: GerbilPolicyReceiptOptions,
) -> Result<GerbilPolicyReceiptDebugSummary, String> {
    let loadpath = options.loadpath.unwrap_or_else(|| "src:t".to_owned());
    let iterations = options.iterations.to_string();
    let entrypoint_load_expr = format!(
        "(load {})",
        scheme_string_literal(options.entrypoint.to_string_lossy().as_ref())
    );
    let started_at = Instant::now();
    let output = Command::new(&options.gxi)
        .arg("-e")
        .arg(&entrypoint_load_expr)
        .arg("-e")
        .arg(&options.call_expr)
        .current_dir(&options.package_root)
        .env("GERBIL_LOADPATH", &loadpath)
        .env("MARLIN_POLICY_RECEIPT_ITERATIONS", iterations)
        .output()
        .map_err(|error| {
            format!(
                "failed to run gxi `{}` entrypoint `{}` in `{}`: {error}",
                options.gxi.display(),
                options.entrypoint.display(),
                options.package_root.display()
            )
        })?;
    let process_elapsed_micros = duration_micros_u64(started_at.elapsed());

    if !output.status.success() {
        return Err(format!(
            "gerbil policy receipt entrypoint failed with status {}: stdout:\n{}\nstderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let facts = parse_policy_receipt_facts(&String::from_utf8_lossy(&output.stdout))?;
    let iterations = required_u64_fact(&facts, "iterations")?;
    Ok(GerbilPolicyReceiptDebugSummary {
        status: "ok",
        command: "gerbil policy-receipt",
        call_expr: options.call_expr,
        entrypoint: options.entrypoint,
        gxi: options.gxi,
        package_root: options.package_root,
        loadpath,
        extension_kind: required_fact(&facts, "extension_kind")?,
        extension_id: required_fact(&facts, "extension_id")?,
        extension_source: required_fact(&facts, "extension_source")?,
        extension_surface: required_fact(&facts, "extension_surface")?,
        extension_capability_count: required_u64_fact(&facts, "extension_capability_count")?,
        policy_extension_object_kind: required_fact(&facts, "policy_extension_object_kind")?,
        policy_extension_object: required_bool_fact(&facts, "policy_extension_object")?,
        policy_extension_source: required_fact(&facts, "policy_extension_source")?,
        policy_extension_managed_by: required_fact(&facts, "policy_extension_managed_by")?,
        policy_extension_projection_owner: required_fact(
            &facts,
            "policy_extension_projection_owner",
        )?,
        policy_extension_runtime_owner: required_fact(&facts, "policy_extension_runtime_owner")?,
        policy_module_kind: required_fact(&facts, "policy_module_kind")?,
        policy_module_id: required_fact(&facts, "policy_module_id")?,
        policy_module_family: required_fact(&facts, "policy_module_family")?,
        policy_projection_target: required_fact(&facts, "policy_projection_target")?,
        module_catalog_kind: required_fact(&facts, "module_catalog_kind")?,
        module_catalog_count: required_u64_fact(&facts, "module_catalog_count")?,
        module_eval_result_kind: required_fact(&facts, "module_eval_result_kind")?,
        module_eval_workflow_kind: required_fact(&facts, "module_eval_workflow_kind")?,
        module_system_presentation_kind: required_fact(&facts, "module_system_presentation_kind")?,
        module_system_projection_chain_kind: required_fact(
            &facts,
            "module_system_projection_chain_kind",
        )?,
        module_system_root_import_count: required_u64_fact(
            &facts,
            "module_system_root_import_count",
        )?,
        module_system_root_extension_count: required_u64_fact(
            &facts,
            "module_system_root_extension_count",
        )?,
        module_system_root_policy_extension_object_count: required_u64_fact(
            &facts,
            "module_system_root_policy_extension_object_count",
        )?,
        module_system_import_graph_owner: required_fact(
            &facts,
            "module_system_import_graph_owner",
        )?,
        module_system_option_merge_owner: required_fact(
            &facts,
            "module_system_option_merge_owner",
        )?,
        module_system_extension_composition_owner: required_fact(
            &facts,
            "module_system_extension_composition_owner",
        )?,
        module_system_native_projection_payload_owner: required_fact(
            &facts,
            "module_system_native_projection_payload_owner",
        )?,
        module_system_budget_receipt_owner: required_fact(
            &facts,
            "module_system_budget_receipt_owner",
        )?,
        module_system_catalog_resolution_receipt_owner: required_fact(
            &facts,
            "module_system_catalog_resolution_receipt_owner",
        )?,
        module_system_rust_parses_scheme_source: required_bool_fact(
            &facts,
            "module_system_rust_parses_scheme_source",
        )?,
        module_system_scheme_manufactures_rust_handlers: required_bool_fact(
            &facts,
            "module_system_scheme_manufactures_rust_handlers",
        )?,
        policy_pack_kind: required_fact(&facts, "policy_pack_kind")?,
        policy_pack_id: required_fact(&facts, "policy_pack_id")?,
        policy_pack_presentation_kind: required_fact(&facts, "policy_pack_presentation_kind")?,
        policy_pack_inventory_kind: required_fact(&facts, "policy_pack_inventory_kind")?,
        policy_pack_module_system_presentation_kind: required_fact(
            &facts,
            "policy_pack_module_system_presentation_kind",
        )?,
        policy_pack_object_count: required_u64_fact(&facts, "policy_pack_object_count")?,
        policy_pack_default_object_count: required_u64_fact(
            &facts,
            "policy_pack_default_object_count",
        )?,
        policy_pack_disabled_object_count: required_u64_fact(
            &facts,
            "policy_pack_disabled_object_count",
        )?,
        policy_pack_policy_families: required_csv_fact(&facts, "policy_pack_policy_families")?,
        policy_pack_policy_object_ids: required_csv_fact(&facts, "policy_pack_policy_object_ids")?,
        policy_pack_default_policy_object_ids: required_csv_fact(
            &facts,
            "policy_pack_default_policy_object_ids",
        )?,
        policy_pack_disabled_policy_object_ids: required_csv_fact(
            &facts,
            "policy_pack_disabled_policy_object_ids",
        )?,
        policy_pack_operation_count: required_u64_fact(&facts, "policy_pack_operation_count")?,
        policy_pack_surgery_receipt_count: required_u64_fact(
            &facts,
            "policy_pack_surgery_receipt_count",
        )?,
        policy_pack_conflict_surgery_receipt_count: required_u64_fact(
            &facts,
            "policy_pack_conflict_surgery_receipt_count",
        )?,
        policy_pack_duplicate_object_conflict_count: required_u64_fact(
            &facts,
            "policy_pack_duplicate_object_conflict_count",
        )?,
        policy_pack_missing_target_conflict_count: required_u64_fact(
            &facts,
            "policy_pack_missing_target_conflict_count",
        )?,
        policy_pack_disabled_target_conflict_count: required_u64_fact(
            &facts,
            "policy_pack_disabled_target_conflict_count",
        )?,
        policy_pack_invalid_replacement_conflict_count: required_u64_fact(
            &facts,
            "policy_pack_invalid_replacement_conflict_count",
        )?,
        policy_pack_add_count: required_u64_fact(&facts, "policy_pack_add_count")?,
        policy_pack_remove_count: required_u64_fact(&facts, "policy_pack_remove_count")?,
        policy_pack_disable_count: required_u64_fact(&facts, "policy_pack_disable_count")?,
        policy_pack_replace_count: required_u64_fact(&facts, "policy_pack_replace_count")?,
        policy_pack_matched_surgery_receipt_count: required_u64_fact(
            &facts,
            "policy_pack_matched_surgery_receipt_count",
        )?,
        policy_pack_allowed_hook_count: required_u64_fact(
            &facts,
            "policy_pack_allowed_hook_count",
        )?,
        policy_pack_allowed_hook_ids: required_csv_fact(&facts, "policy_pack_allowed_hook_ids")?,
        policy_pack_import_graph_owner: required_fact(&facts, "policy_pack_import_graph_owner")?,
        policy_pack_option_merge_owner: required_fact(&facts, "policy_pack_option_merge_owner")?,
        policy_pack_extension_composition_owner: required_fact(
            &facts,
            "policy_pack_extension_composition_owner",
        )?,
        policy_pack_native_projection_payload_owner: required_fact(
            &facts,
            "policy_pack_native_projection_payload_owner",
        )?,
        policy_pack_rust_parses_scheme_source: required_bool_fact(
            &facts,
            "policy_pack_rust_parses_scheme_source",
        )?,
        policy_pack_rust_handler_manufactured: required_bool_fact(
            &facts,
            "policy_pack_rust_handler_manufactured",
        )?,
        policy_projection_kind: required_fact(&facts, "policy_projection_kind")?,
        policy_projection_pack_id: required_fact(&facts, "policy_projection_pack_id")?,
        policy_projection_chain_kind: required_fact(&facts, "policy_projection_chain_kind")?,
        policy_projection_module_evaluation_receipt_kind: required_fact(
            &facts,
            "policy_projection_module_evaluation_receipt_kind",
        )?,
        policy_projection_policy_projection_receipt_kind: required_fact(
            &facts,
            "policy_projection_policy_projection_receipt_kind",
        )?,
        policy_projection_native_projection_payload_kind: required_fact(
            &facts,
            "policy_projection_native_projection_payload_kind",
        )?,
        policy_projection_native_projection_payload_owner: required_fact(
            &facts,
            "policy_projection_native_projection_payload_owner",
        )?,
        policy_projection_budget_receipt_owner: required_fact(
            &facts,
            "policy_projection_budget_receipt_owner",
        )?,
        policy_projection_catalog_resolution_receipt_owner: required_fact(
            &facts,
            "policy_projection_catalog_resolution_receipt_owner",
        )?,
        policy_projection_import_graph_owner: required_fact(
            &facts,
            "policy_projection_import_graph_owner",
        )?,
        policy_projection_option_merge_owner: required_fact(
            &facts,
            "policy_projection_option_merge_owner",
        )?,
        policy_projection_extension_composition_owner: required_fact(
            &facts,
            "policy_projection_extension_composition_owner",
        )?,
        policy_projection_policy_composition_owner: required_fact(
            &facts,
            "policy_projection_policy_composition_owner",
        )?,
        policy_projection_runtime_lifecycle_owner: required_fact(
            &facts,
            "policy_projection_runtime_lifecycle_owner",
        )?,
        policy_projection_rust_parses_scheme_source: required_bool_fact(
            &facts,
            "policy_projection_rust_parses_scheme_source",
        )?,
        policy_projection_rust_handler_manufactured: required_bool_fact(
            &facts,
            "policy_projection_rust_handler_manufactured",
        )?,
        policy_projection_replayable: required_bool_fact(&facts, "policy_projection_replayable")?,
        policy_projection_chain_receipt_kind: required_fact(
            &facts,
            "policy_projection_chain_receipt_kind",
        )?,
        policy_projection_chain_receipt_pack_id: required_fact(
            &facts,
            "policy_projection_chain_receipt_pack_id",
        )?,
        policy_projection_chain_module_evaluation_receipt_kind: required_fact(
            &facts,
            "policy_projection_chain_module_evaluation_receipt_kind",
        )?,
        policy_projection_chain_policy_projection_receipt_kind: required_fact(
            &facts,
            "policy_projection_chain_policy_projection_receipt_kind",
        )?,
        policy_projection_chain_native_projection_payload_kind: required_fact(
            &facts,
            "policy_projection_chain_native_projection_payload_kind",
        )?,
        policy_projection_chain_budget_receipt_kind: required_fact(
            &facts,
            "policy_projection_chain_budget_receipt_kind",
        )?,
        policy_projection_chain_catalog_resolution_receipt_kind: required_fact(
            &facts,
            "policy_projection_chain_catalog_resolution_receipt_kind",
        )?,
        policy_projection_chain_replayable: required_bool_fact(
            &facts,
            "policy_projection_chain_replayable",
        )?,
        default_policy_delivery_kind: required_fact(&facts, "default_policy_delivery_kind")?,
        default_policy_pack_id: required_fact(&facts, "default_policy_pack_id")?,
        default_policy_pack_count: required_u64_fact(&facts, "default_policy_pack_count")?,
        default_policy_pack_ids: required_csv_fact(&facts, "default_policy_pack_ids")?,
        default_policy_object_count: required_u64_fact(&facts, "default_policy_object_count")?,
        default_policy_default_object_count: required_u64_fact(
            &facts,
            "default_policy_default_object_count",
        )?,
        default_policy_allowed_hook_count: required_u64_fact(
            &facts,
            "default_policy_allowed_hook_count",
        )?,
        default_policy_allowed_hook_ids: required_csv_fact(
            &facts,
            "default_policy_allowed_hook_ids",
        )?,
        default_policy_catalog_presentation_kind: required_fact(
            &facts,
            "default_policy_catalog_presentation_kind",
        )?,
        default_policy_projection_kind: required_fact(&facts, "default_policy_projection_kind")?,
        default_policy_projection_chain_receipt_kind: required_fact(
            &facts,
            "default_policy_projection_chain_receipt_kind",
        )?,
        default_policy_budget_receipt_kind: required_fact(
            &facts,
            "default_policy_budget_receipt_kind",
        )?,
        default_policy_catalog_resolution_receipt_kind: required_fact(
            &facts,
            "default_policy_catalog_resolution_receipt_kind",
        )?,
        default_policy_replayable: required_bool_fact(&facts, "default_policy_replayable")?,
        policy_substrate_gate_kind: required_fact(&facts, "policy_substrate_gate_kind")?,
        policy_substrate_gate_profile: required_fact(&facts, "policy_substrate_gate_profile")?,
        policy_substrate_gate_receipt_kind: required_fact(
            &facts,
            "policy_substrate_gate_receipt_kind",
        )?,
        policy_module_evaluation_kind: required_fact(&facts, "policy_module_evaluation_kind")?,
        policy_module_count: required_u64_fact(&facts, "policy_module_count")?,
        policy_extension_count: required_u64_fact(&facts, "policy_extension_count")?,
        policy_extension_object_count: required_u64_fact(&facts, "policy_extension_object_count")?,
        policy_script_count: required_u64_fact(&facts, "policy_script_count")?,
        policy_option_count: required_u64_fact(&facts, "policy_option_count")?,
        policy_validation_receipt_count: required_u64_fact(
            &facts,
            "policy_validation_receipt_count",
        )?,
        policy_substrate_gate_replayable: required_bool_fact(
            &facts,
            "policy_substrate_gate_replayable",
        )?,
        scheme_policy_owner: required_fact(&facts, "scheme_policy_owner")?,
        rust_kernel_owner: required_fact(&facts, "rust_kernel_owner")?,
        catalog_kind: required_fact(&facts, "catalog_kind")?,
        scheme_catalog_role: required_fact(&facts, "scheme_catalog_role")?,
        runtime_catalog_owner: required_fact(&facts, "runtime_catalog_owner")?,
        catalog_resolved_by_scheme: required_bool_fact(&facts, "catalog_resolved_by_scheme")?,
        iterations,
        timing_scope: "single-gxi-process-wall-clock-includes-startup",
        process_elapsed_micros,
        avg_process_micros_per_iteration: process_elapsed_micros / iterations,
        scheme_policy_loop_elapsed_micros: required_u64_fact(
            &facts,
            "scheme_policy_loop_elapsed_micros",
        )?,
        avg_scheme_policy_micros_per_iteration: required_u64_fact(
            &facts,
            "avg_scheme_policy_micros_per_iteration",
        )?,
        receipt_kind: required_fact(&facts, "receipt_kind")?,
        matched: required_bool_fact(&facts, "matched")?,
        policy_engine: required_fact(&facts, "policy_engine")?,
        extension_receipt_id: required_fact(&facts, "extension_receipt_id")?,
        dynamic_hook_action: required_fact(&facts, "dynamic_hook_action")?,
        dynamic_hook_hook_id: required_fact(&facts, "dynamic_hook_hook_id")?,
        dynamic_hook_registration: required_fact(&facts, "dynamic_hook_registration")?,
        dynamic_hook_selection_source: required_fact(&facts, "dynamic_hook_selection_source")?,
        dynamic_hook_selection_selector: required_fact(&facts, "dynamic_hook_selection_selector")?,
    })
}

fn parse_policy_receipt_facts(output: &str) -> Result<BTreeMap<String, String>, String> {
    let mut facts = BTreeMap::new();
    for line in output.lines().filter(|line| !line.trim().is_empty()) {
        let (key, value) = line
            .split_once('\t')
            .ok_or_else(|| format!("unexpected gerbil policy receipt line `{line}`"))?;
        facts.insert(key.to_owned(), value.to_owned());
    }
    Ok(facts)
}

fn duration_micros_u64(duration: std::time::Duration) -> u64 {
    duration.as_micros().try_into().unwrap_or(u64::MAX)
}

fn scheme_string_literal(value: &str) -> String {
    let mut escaped = String::from("\"");
    for character in value.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            other => escaped.push(other),
        }
    }
    escaped.push('"');
    escaped
}

fn required_fact(facts: &BTreeMap<String, String>, key: &str) -> Result<String, String> {
    facts
        .get(key)
        .cloned()
        .ok_or_else(|| format!("missing gerbil policy receipt fact `{key}`"))
}

fn required_csv_fact(facts: &BTreeMap<String, String>, key: &str) -> Result<Vec<String>, String> {
    let value = required_fact(facts, key)?;
    if value.is_empty() {
        return Ok(Vec::new());
    }

    Ok(value.split(',').map(ToOwned::to_owned).collect())
}

fn required_bool_fact(facts: &BTreeMap<String, String>, key: &str) -> Result<bool, String> {
    match required_fact(facts, key)?.as_str() {
        "#t" => Ok(true),
        "#f" => Ok(false),
        value => Err(format!(
            "expected boolean gerbil policy receipt fact `{key}`, got `{value}`"
        )),
    }
}

fn required_u64_fact(facts: &BTreeMap<String, String>, key: &str) -> Result<u64, String> {
    required_fact(facts, key)?
        .parse::<u64>()
        .map_err(|error| format!("expected unsigned gerbil policy receipt fact `{key}`: {error}"))
}
