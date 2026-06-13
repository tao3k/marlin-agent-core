use marlin_agent_test_support::{
    LibtestTextImportConfig, TestRunLayer, WorkspaceLibtestTextImportInput,
    import_libtest_text_output, import_workspace_libtest_text_outputs,
};

#[test]
fn parses_libtest_text_into_layered_receipt() {
    let output = r#"
running 3 tests
test runtime::session::sub_agent_session_context_isolated_from_parent ... ok
test three_layer::test_support_three_layer_testing_system_covers_workspace_packages_without_live_llm ... ok
test command::real_gxi::artifacts::command_compiler_can_call_real_gxi_workspace_schema ... ignored, requires a local Gerbil gxi executable

test result: ok. 2 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s
"#;
    let config = LibtestTextImportConfig::new("marlin-agent-test-support")
        .with_integration_prefix("three_layer::")
        .with_live_external_prefix("command::real_gxi::");

    let report = import_libtest_text_output(&config, output);

    assert!(report.is_consistent(), "{:?}", report.diagnostics);
    assert_eq!(report.declared_test_count, Some(3));
    assert_eq!(report.receipt.case_count(), 3);
    assert_eq!(report.receipt.passed_count(), 2);
    assert_eq!(report.receipt.ignored_live_external_count(), 1);
    assert_eq!(
        report
            .receipt
            .summary_for_layer(TestRunLayer::NonLiveIntegration)
            .passed,
        1
    );
}

#[test]
fn reports_summary_mismatches() {
    let output = r#"
running 2 tests
test runtime::session::sub_agent_session_context_isolated_from_parent ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
"#;
    let config = LibtestTextImportConfig::new("marlin-agent-test-support");

    let report = import_libtest_text_output(&config, output);

    assert!(!report.is_consistent());
    assert_eq!(report.receipt.case_count(), 1);
    assert!(
        report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic == "declared_test_count=2 parsed_case_count=1")
    );
    assert!(
        report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic == "passed_summary=2 parsed_passed=1")
    );
}

#[test]
fn marks_failed_non_live_cases_as_quality_failures() {
    let output = r#"
running 1 tests
test runtime::session::sub_agent_session_context_isolated_from_parent ... FAILED

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
"#;
    let config = LibtestTextImportConfig::new("marlin-agent-runtime");

    let report = import_libtest_text_output(&config, output);

    assert!(report.is_consistent(), "{:?}", report.diagnostics);
    assert_eq!(report.receipt.failed_count(), 1);
    assert_eq!(report.receipt.non_live_failed_count(), 1);
    assert!(!report.receipt.is_non_live_success());
}

#[test]
fn workspace_importer_merges_package_libtest_reports() {
    let runtime_output = r#"
running 1 tests
test runtime_session::sub_agent_session_context_isolated_from_parent ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
"#;
    let test_support_output = r#"
running 2 tests
test three_layer::test_support_three_layer_testing_system_covers_workspace_packages_without_live_llm ... ok
test command::real_gxi::artifacts::command_compiler_can_call_real_gxi_workspace_schema ... ignored, requires a local Gerbil gxi executable

test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s
"#;

    let report = import_workspace_libtest_text_outputs([
        WorkspaceLibtestTextImportInput::new(
            LibtestTextImportConfig::new("marlin-agent-runtime"),
            runtime_output,
        ),
        WorkspaceLibtestTextImportInput::new(
            LibtestTextImportConfig::new("marlin-agent-test-support")
                .with_integration_prefix("three_layer::")
                .with_live_external_prefix("command::real_gxi::"),
            test_support_output,
        ),
    ]);

    assert!(report.is_consistent(), "{:?}", report.diagnostics);
    assert_eq!(report.package_count(), 2);
    assert_eq!(report.receipt.case_count(), 3);
    assert_eq!(report.receipt.passed_count(), 2);
    assert_eq!(report.receipt.ignored_live_external_count(), 1);
    assert_eq!(report.receipt.non_live_failed_count(), 0);
}

#[test]
fn workspace_importer_prefixes_package_diagnostics() {
    let output = r#"
running 2 tests
test runtime_session::sub_agent_session_context_isolated_from_parent ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
"#;

    let report = import_workspace_libtest_text_outputs([WorkspaceLibtestTextImportInput::new(
        LibtestTextImportConfig::new("marlin-agent-runtime"),
        output,
    )]);

    assert!(!report.is_consistent());
    assert_eq!(report.package_count(), 1);
    assert!(
        report.diagnostics.iter().any(|diagnostic| diagnostic
            == "marlin-agent-runtime:declared_test_count=2 parsed_case_count=1")
    );
    assert!(
        report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic == "marlin-agent-runtime:passed_summary=2 parsed_passed=1")
    );
}
