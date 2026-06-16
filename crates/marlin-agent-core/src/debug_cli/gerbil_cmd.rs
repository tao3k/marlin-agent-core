//! `marlin gerbil ...` command implementations.

use std::{collections::BTreeMap, path::PathBuf, process::Command, time::Instant};

use serde::Serialize;

use super::{
    MarlinCliResult,
    args::{ArgCursor, GerbilPolicyReceiptOptions},
    gerbil_usage,
};

fn policy_receipt_probe(iterations: u64) -> String {
    format!(
        r#"
(begin
  (import :clan/poo/object
          :marlin/deck-runtime-debug-policy-extension)
  (def (emit key value)
    (display key)
    (display "\t")
    (display value)
    (newline))
  (def iterations {iterations})
  (def extension marlin-deck-runtime-debug-policy-extension)
  (def catalog (marlin-deck-runtime-debug-policy-extension-catalog))
  (def scheme-policy-loop-started (time->seconds (current-time)))
  (def receipt
    (marlin-deck-runtime-debug-policy-extension-receipt-loop iterations))
  (def scheme-policy-loop-elapsed-micros
    (inexact->exact
     (floor
      (* 1000000
         (- (time->seconds (current-time)) scheme-policy-loop-started)))))
  (def action (.get receipt dynamic-hook-action))
  (def selection (.get receipt dynamic-hook-selection))
  (emit "extension_kind" (.get extension kind))
  (emit "extension_id" (.get extension id))
  (emit "extension_source" marlin-deck-runtime-debug-policy-extension-source)
  (emit "extension_surface" "poo-extension-object")
  (emit "extension_capability_count" (length (.get extension capabilities)))
  (emit "catalog_kind" (.get catalog kind))
  (emit "scheme_catalog_role" "extension-object-selection")
  (emit "runtime_catalog_owner" "rust")
  (emit "catalog_resolved_by_scheme" #f)
  (emit "iterations" iterations)
  (emit "scheme_policy_loop_elapsed_micros" scheme-policy-loop-elapsed-micros)
  (emit "avg_scheme_policy_micros_per_iteration"
        (quotient scheme-policy-loop-elapsed-micros iterations))
  (emit "receipt_kind" (.get receipt kind))
  (emit "matched" (.get receipt matched))
  (emit "policy_engine" (.get receipt policy-engine))
  (emit "extension_receipt_id" (.get receipt extension-id))
  (emit "dynamic_hook_action" (.get action action))
  (emit "dynamic_hook_hook_id" (.get action hook-id))
  (emit "dynamic_hook_registration" (.get action registration))
  (emit "dynamic_hook_selection_source" (.get selection source))
  (emit "dynamic_hook_selection_selector" (.get selection selector)))
"#
    )
}

#[derive(Clone, Debug, Serialize)]
struct GerbilPolicyReceiptDebugSummary {
    status: &'static str,
    command: &'static str,
    gxi: PathBuf,
    package_root: PathBuf,
    loadpath: String,
    extension_kind: String,
    extension_id: String,
    extension_source: String,
    extension_surface: String,
    extension_capability_count: u64,
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
    let probe = policy_receipt_probe(options.iterations);
    let started_at = Instant::now();
    let output = Command::new(&options.gxi)
        .arg("-e")
        .arg(probe)
        .current_dir(&options.package_root)
        .env("GERBIL_LOADPATH", &loadpath)
        .output()
        .map_err(|error| {
            format!(
                "failed to run gxi `{}` in `{}`: {error}",
                options.gxi.display(),
                options.package_root.display()
            )
        })?;
    let process_elapsed_micros = duration_micros_u64(started_at.elapsed());

    if !output.status.success() {
        return Err(format!(
            "gerbil policy receipt probe failed with status {}: stdout:\n{}\nstderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let facts = parse_policy_receipt_probe(&String::from_utf8_lossy(&output.stdout))?;
    let iterations = required_u64_fact(&facts, "iterations")?;
    Ok(GerbilPolicyReceiptDebugSummary {
        status: "ok",
        command: "gerbil policy-receipt",
        gxi: options.gxi,
        package_root: options.package_root,
        loadpath,
        extension_kind: required_fact(&facts, "extension_kind")?,
        extension_id: required_fact(&facts, "extension_id")?,
        extension_source: required_fact(&facts, "extension_source")?,
        extension_surface: required_fact(&facts, "extension_surface")?,
        extension_capability_count: required_u64_fact(&facts, "extension_capability_count")?,
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

fn parse_policy_receipt_probe(output: &str) -> Result<BTreeMap<String, String>, String> {
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

fn required_fact(facts: &BTreeMap<String, String>, key: &str) -> Result<String, String> {
    facts
        .get(key)
        .cloned()
        .ok_or_else(|| format!("missing gerbil policy receipt fact `{key}`"))
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
