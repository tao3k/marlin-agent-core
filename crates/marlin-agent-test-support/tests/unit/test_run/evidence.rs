use marlin_agent_harness_types::HarnessEvidenceKind;
use marlin_agent_test_support::{
    TestRunCaseRecord, TestRunCaseStatus, TestRunEvidenceReceipt, TestRunLayer,
    assert_deterministic_test_run_evidence,
};

#[test]
fn counts_no_live_layers_and_ignored_external_gates() {
    let receipt = assert_deterministic_test_run_evidence();

    assert_eq!(receipt.schema_version, 1);
    assert_eq!(receipt.case_count(), 5);
    assert_eq!(receipt.evidence_count(), 2);
    assert_eq!(
        receipt.evidence_count_by_kind(HarnessEvidenceKind::Visibility),
        2
    );
    assert_eq!(receipt.failed_count(), 0);
    assert_eq!(receipt.ignored_count(), 1);
    assert_eq!(receipt.non_live_failed_count(), 0);
    assert_eq!(receipt.ignored_live_external_count(), 1);
    assert!(
        receipt
            .summary_for_layer(TestRunLayer::NonLiveUnit)
            .is_present()
    );
    assert!(
        receipt
            .summary_for_layer(TestRunLayer::NonLiveIntegration)
            .is_present()
    );
    assert!(receipt.render_summary().contains("evidence=2"));
}

#[test]
fn marks_non_live_failures_as_quality_failures() {
    let receipt = TestRunEvidenceReceipt::new(vec![
        TestRunCaseRecord::passed(
            "marlin-agent-runtime",
            "runtime_session::sub_agent_session_context_isolated_from_parent",
            TestRunLayer::NonLiveUnit,
        ),
        TestRunCaseRecord::failed(
            "marlin-agent-harness",
            "harness::three_layer::harness_consumes_test_support_three_layer_package_coverage",
            TestRunLayer::NonLiveIntegration,
        ),
    ]);

    assert_eq!(receipt.failed_count(), 1);
    assert_eq!(receipt.non_live_failed_count(), 1);
    assert!(!receipt.is_non_live_success());
    assert!(receipt.render_summary().contains("non_live_failed=1"));
}

#[test]
fn keeps_live_external_ignored_reason_out_of_non_live_failure_count() {
    let receipt = TestRunEvidenceReceipt::new(vec![TestRunCaseRecord::ignored(
        "marlin-gerbil-scheme",
        "command::real_gxi::artifacts::command_compiler_can_call_real_gxi_workspace_schema",
        TestRunLayer::LiveExternal,
        "requires a local Gerbil gxi executable",
    )]);

    assert_eq!(receipt.failed_count(), 0);
    assert_eq!(receipt.non_live_failed_count(), 0);
    assert_eq!(receipt.ignored_live_external_count(), 1);
    assert!(!receipt.is_non_live_success());
    assert_eq!(receipt.cases[0].status, TestRunCaseStatus::Ignored);
    assert_eq!(
        receipt.cases[0].ignored_reason.as_deref(),
        Some("requires a local Gerbil gxi executable")
    );
}
