//! Workspace-wide `scenario_benchmark` aggregation and improvement receipts.

use std::{
    fs,
    path::{Path, PathBuf},
};

use rust_lang_project_harness::validate_required_rust_scenario_benchmarks;

use super::{
    RustScenarioBenchmarkStatus, RustScenarioBenchmarkSuiteReceipt,
    RustScenarioPerformanceCrateName, RustScenarioPerformanceOptimizationFinding,
    RustScenarioPerformanceOptimizationPriority, RustScenarioWorkspaceBenchmarkCrateReceipt,
    RustScenarioWorkspaceBenchmarkGateReceipt, RustScenarioWorkspaceBenchmarkImprovementKind,
    RustScenarioWorkspaceBenchmarkImprovementSuggestion, RustScenarioWorkspaceBenchmarkStatus,
    optimization_receipt_from_benchmark_receipt, paths::scenario_root,
    validate_rust_scenario_benchmark,
};

/// Build the workspace-wide gate receipt for all first-party crates under `crates/`.
pub fn validate_workspace_required_scenario_benchmarks(
    workspace_root: &Path,
) -> RustScenarioWorkspaceBenchmarkGateReceipt {
    let workspace_root = workspace_root.to_path_buf();
    let mut crate_receipts = workspace_crate_members(&workspace_root)
        .into_iter()
        .map(|crate_root| validate_workspace_crate_scenario_benchmark(&workspace_root, crate_root))
        .collect::<Vec<_>>();

    crate_receipts.sort_by(|left, right| left.crate_name.cmp(&right.crate_name));

    let improvements = crate_receipts
        .iter()
        .flat_map(|receipt| receipt.improvements.iter().cloned())
        .collect::<Vec<_>>();
    let failed_crate_count = crate_receipts
        .iter()
        .filter(|receipt| receipt.status == RustScenarioWorkspaceBenchmarkStatus::Fail)
        .count();
    let passed_crate_count = crate_receipts.len().saturating_sub(failed_crate_count);
    let status = if failed_crate_count == 0 && improvements.is_empty() {
        RustScenarioWorkspaceBenchmarkStatus::Pass
    } else {
        RustScenarioWorkspaceBenchmarkStatus::Fail
    };

    RustScenarioWorkspaceBenchmarkGateReceipt {
        schema_id: "marlin.rust-project-harness.workspace-scenario-benchmark-gate".to_owned(),
        schema_version: "1".to_owned(),
        workspace_root,
        status,
        crate_count: crate_receipts.len(),
        passed_crate_count,
        failed_crate_count,
        improvement_count: improvements.len(),
        crates: crate_receipts,
        improvements,
    }
}

/// Alias for callers that use the gate as a reflection/optimization receipt.
pub fn workspace_scenario_performance_optimization_receipt(
    workspace_root: &Path,
) -> RustScenarioWorkspaceBenchmarkGateReceipt {
    validate_workspace_required_scenario_benchmarks(workspace_root)
}

/// Assert that every first-party crate uses the upstream scenario benchmark schema.
pub fn assert_workspace_scenario_performance_contract_gate_accepts_all_crates(
    workspace_root: &Path,
) {
    let receipt = validate_workspace_required_scenario_benchmarks(workspace_root);
    assert_ne!(receipt.crate_count, 0, "{receipt:?}");
    assert_eq!(
        receipt.status,
        RustScenarioWorkspaceBenchmarkStatus::Pass,
        "{receipt:?}"
    );
    assert!(receipt.improvements.is_empty(), "{receipt:?}");
}

fn scenario_schema_violation_messages(receipt: &RustScenarioBenchmarkSuiteReceipt) -> Vec<String> {
    receipt
        .receipts
        .iter()
        .flat_map(|scenario_receipt| {
            scenario_receipt.violations.iter().map(move |violation| {
                format!(
                    "{}: {:?} {}: {}",
                    scenario_receipt.root.display(),
                    violation.kind,
                    violation.field,
                    violation.message
                )
            })
        })
        .collect()
}

fn validate_workspace_crate_scenario_benchmark(
    workspace_root: &Path,
    crate_root: PathBuf,
) -> RustScenarioWorkspaceBenchmarkCrateReceipt {
    let crate_name = workspace_crate_name(&crate_root);
    match validate_required_rust_scenario_benchmarks(&crate_root) {
        Ok(required_receipt) => {
            let mut improvements = Vec::new();
            let scenario_schema_violations = scenario_schema_violation_messages(&required_receipt);
            let violation_count =
                required_receipt.violations.len() + scenario_schema_violations.len();
            if required_receipt.status != RustScenarioBenchmarkStatus::Pass || violation_count != 0
            {
                let schema_error = if !required_receipt.violations.is_empty() {
                    format!("{:?}", required_receipt.violations)
                } else if !scenario_schema_violations.is_empty() {
                    scenario_schema_violations.join("; ")
                } else {
                    format!(
                        "status={:?}; required_scenario_count={}",
                        required_receipt.status,
                        required_receipt.requirements.len()
                    )
                };
                improvements.push(invalid_schema_suggestion(
                    &crate_root,
                    crate_name.as_str(),
                    schema_error,
                ));
            }

            let optimization_receipt = validate_rust_scenario_benchmark(scenario_root(&crate_root))
                .ok()
                .map(|receipt| {
                    optimization_receipt_from_benchmark_receipt(crate_name.as_str(), &receipt)
                });

            if let Some(optimization_receipt) = &optimization_receipt {
                improvements.extend(
                    optimization_receipt
                        .findings
                        .iter()
                        .map(|finding| optimization_suggestion_from_finding(&crate_root, finding)),
                );
            }

            let status = if required_receipt.status == RustScenarioBenchmarkStatus::Pass
                && required_receipt.violations.is_empty()
                && improvements.is_empty()
            {
                RustScenarioWorkspaceBenchmarkStatus::Pass
            } else {
                RustScenarioWorkspaceBenchmarkStatus::Fail
            };

            RustScenarioWorkspaceBenchmarkCrateReceipt {
                crate_name: RustScenarioPerformanceCrateName::new(crate_name),
                crate_root: crate_root
                    .strip_prefix(workspace_root)
                    .unwrap_or(crate_root.as_path())
                    .to_path_buf(),
                status,
                required_scenario_count: required_receipt.requirements.len(),
                violation_count,
                optimization_status: optimization_receipt.map(|receipt| receipt.status),
                optimization_finding_count: improvements.len(),
                error: None,
                improvements,
            }
        }
        Err(error) => {
            let suggestion = invalid_schema_suggestion(&crate_root, crate_name.as_str(), error);
            RustScenarioWorkspaceBenchmarkCrateReceipt {
                crate_name: RustScenarioPerformanceCrateName::new(crate_name),
                crate_root: crate_root
                    .strip_prefix(workspace_root)
                    .unwrap_or(crate_root.as_path())
                    .to_path_buf(),
                status: RustScenarioWorkspaceBenchmarkStatus::Fail,
                required_scenario_count: 0,
                violation_count: 1,
                optimization_status: None,
                optimization_finding_count: 0,
                error: Some(suggestion.problem.clone()),
                improvements: vec![suggestion],
            }
        }
    }
}

fn workspace_crate_members(workspace_root: &Path) -> Vec<PathBuf> {
    let manifest = read_toml(workspace_root.join("Cargo.toml"));
    let Some(members) = manifest
        .get("workspace")
        .and_then(|workspace| workspace.get("members"))
        .and_then(|members| members.as_array())
    else {
        return Vec::new();
    };

    let mut crate_roots = members
        .iter()
        .filter_map(|member| member.as_str())
        .flat_map(|member| resolve_workspace_member(workspace_root, member))
        .filter(|path| {
            path.strip_prefix(workspace_root)
                .is_ok_and(is_product_crate_member)
        })
        .filter(|path| path.join("Cargo.toml").is_file())
        .collect::<Vec<_>>();
    crate_roots.sort();
    crate_roots.dedup();
    crate_roots
}

fn resolve_workspace_member(workspace_root: &Path, member: &str) -> Vec<PathBuf> {
    if let Some(parent) = member.strip_suffix("/*") {
        let parent = workspace_root.join(parent);
        return fs::read_dir(parent)
            .ok()
            .into_iter()
            .flat_map(|entries| entries.filter_map(Result::ok))
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .collect();
    }

    vec![workspace_root.join(member)]
}

fn is_product_crate_member(path: &Path) -> bool {
    path.starts_with("crates")
}

fn workspace_crate_name(crate_root: &Path) -> String {
    read_toml(crate_root.join("Cargo.toml"))
        .get("package")
        .and_then(|package| package.get("name"))
        .and_then(|name| name.as_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| {
            panic!(
                "crate Cargo.toml at {} must define [package].name",
                crate_root.display()
            )
        })
}

fn read_toml(path: PathBuf) -> toml::Value {
    let content = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    toml::from_str::<toml::Value>(&content)
        .unwrap_or_else(|error| panic!("parse {}: {error}", path.display()))
}

fn invalid_schema_suggestion(
    crate_root: &Path,
    crate_name: &str,
    error: impl std::fmt::Display,
) -> RustScenarioWorkspaceBenchmarkImprovementSuggestion {
    RustScenarioWorkspaceBenchmarkImprovementSuggestion {
        kind: RustScenarioWorkspaceBenchmarkImprovementKind::InvalidScenarioBenchmarkSchema,
        priority: RustScenarioPerformanceOptimizationPriority::Critical,
        crate_name: RustScenarioPerformanceCrateName::new(crate_name),
        crate_root: crate_root.to_path_buf(),
        scenario_id: None,
        problem: format!("crate scenario benchmark does not satisfy upstream schema: {error}"),
        next_action:
            "migrate benchmark.toml to the upstream rust-lang-project-harness scenario schema"
                .to_owned(),
        verification_command: format!(
            "cargo test -p {crate_name} scenario_performance_contract_gate_accepts_crate_scenarios"
        ),
        evidence: vec![
            scenario_root(crate_root)
                .join("benchmark.toml")
                .display()
                .to_string(),
        ],
    }
}

fn optimization_suggestion_from_finding(
    crate_root: &Path,
    finding: &RustScenarioPerformanceOptimizationFinding,
) -> RustScenarioWorkspaceBenchmarkImprovementSuggestion {
    let crate_name = workspace_crate_name(crate_root);
    RustScenarioWorkspaceBenchmarkImprovementSuggestion {
        kind: RustScenarioWorkspaceBenchmarkImprovementKind::ScenarioPerformanceOptimization,
        priority: finding.priority,
        crate_name: RustScenarioPerformanceCrateName::new(crate_name.clone()),
        crate_root: crate_root.to_path_buf(),
        scenario_id: Some(finding.scenario_id.clone()),
        problem: finding.problem.clone(),
        next_action: finding.next_action.clone(),
        verification_command: format!(
            "cargo test -p {crate_name} scenario_performance_first_batch_optimization_frontier_is_clear"
        ),
        evidence: vec![
            scenario_root(crate_root)
                .join("benchmark.toml")
                .display()
                .to_string(),
        ],
    }
}
