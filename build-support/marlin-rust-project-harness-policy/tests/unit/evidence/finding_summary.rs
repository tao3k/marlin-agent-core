use marlin_rust_project_harness_policy::RustProjectHarnessFindingSummary;
use rust_lang_project_harness::{
    RustDiagnosticSeverity, RustHarnessFinding, RustHarnessReport, SourceLocation,
};
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn summary_retains_blocking_counts_and_deduplicated_rule_ids() {
    let findings = [
        ("RUST-INFO", RustDiagnosticSeverity::Info),
        ("RUST-WARN", RustDiagnosticSeverity::Warning),
        ("RUST-WARN", RustDiagnosticSeverity::Warning),
        ("RUST-ERROR", RustDiagnosticSeverity::Error),
    ]
    .into_iter()
    .map(|(rule_id, severity)| finding(rule_id, severity))
    .collect();
    let report = RustHarnessReport {
        modules: Vec::new(),
        findings,
        invariant_candidates: Vec::new(),
        root_paths: Vec::new(),
        blocking_severities: BTreeSet::from([
            RustDiagnosticSeverity::Warning,
            RustDiagnosticSeverity::Error,
        ]),
        project_scope: None,
        workspace_member_scopes: Vec::new(),
    };

    let summary = RustProjectHarnessFindingSummary::from_report(&report);

    assert_eq!(summary.total_count, 4);
    assert_eq!(summary.info_count, 1);
    assert_eq!(summary.warning_count, 2);
    assert_eq!(summary.error_count, 1);
    assert_eq!(summary.blocking_count, 3);
    assert_eq!(
        summary.blocking_rule_ids,
        vec!["RUST-ERROR".to_string(), "RUST-WARN".to_string()]
    );
}

fn finding(rule_id: &str, severity: RustDiagnosticSeverity) -> RustHarnessFinding {
    RustHarnessFinding {
        rule_id: rule_id.to_string(),
        pack_id: "test.pack".to_string(),
        severity,
        title: "test finding".to_string(),
        summary: "test finding summary".to_string(),
        location: SourceLocation {
            path: None,
            line: 1,
            column: 0,
        },
        requirement: "test requirement".to_string(),
        source_line: None,
        label: "test label".to_string(),
        labels: BTreeMap::new(),
    }
}
