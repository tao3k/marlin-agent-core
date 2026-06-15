use marlin_agent_harness::HarnessEvidenceKind;
use marlin_agent_test_support::{
    LibtestCommandSpec, LibtestTextImportConfig, TestRunLayer,
    assert_deterministic_test_run_evidence, capture_workspace_libtest_commands,
    deterministic_test_run_evidence_fixture,
};

#[test]
fn harness_consumes_test_support_test_run_evidence_receipt() {
    let receipt = assert_deterministic_test_run_evidence();

    assert!(receipt.is_non_live_success());
    assert_eq!(receipt.non_live_failed_count(), 0);
    assert_eq!(receipt.ignored_live_external_count(), 1);
    assert_eq!(
        receipt.evidence_count_by_kind(HarnessEvidenceKind::Visibility),
        2
    );
    assert!(receipt.render_summary().contains("non_live_integration"));
}

#[test]
fn harness_can_project_test_run_receipt_into_layer_summaries() {
    let receipt = deterministic_test_run_evidence_fixture();
    let live_external = receipt.summary_for_layer(TestRunLayer::LiveExternal);

    assert_eq!(live_external.passed, 0);
    assert_eq!(live_external.failed, 0);
    assert_eq!(live_external.ignored, 1);
}

#[cfg(unix)]
#[test]
fn harness_consumes_command_capture_workspace_receipt() {
    let runtime_script = r#"printf '%s\n' 'running 1 tests' 'test runtime_session::sub_agent_session_context_isolated_from_parent ... ok' '' 'test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s'"#;
    let test_support_script = r#"printf '%s\n' 'running 2 tests' 'test three_layer::test_support_three_layer_testing_system_covers_workspace_packages_without_live_llm ... ok' 'test command::real_gxi::artifacts::command_compiler_can_call_real_gxi_workspace_schema ... ignored, requires a local Gerbil gxi executable' '' 'test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s'"#;

    let report = capture_workspace_libtest_commands([
        libtest_shell_command(
            LibtestTextImportConfig::new("marlin-agent-runtime"),
            runtime_script,
        ),
        libtest_shell_command(
            LibtestTextImportConfig::new("marlin-agent-test-support")
                .with_integration_prefix("three_layer::")
                .with_live_external_prefix("command::real_gxi::"),
            test_support_script,
        ),
    ])
    .expect("capture workspace libtest commands");

    let non_live_integration = report
        .receipt
        .summary_for_layer(TestRunLayer::NonLiveIntegration);

    assert!(report.is_consistent(), "{:?}", report.diagnostics);
    assert!(report.is_non_live_success());
    assert_eq!(report.package_count(), 2);
    assert_eq!(non_live_integration.passed, 1);
    assert_eq!(report.receipt.non_live_failed_count(), 0);
    assert_eq!(report.receipt.ignored_live_external_count(), 1);
    assert!(
        report
            .receipt
            .render_summary()
            .contains("non_live_integration")
    );
}

#[cfg(unix)]
fn libtest_shell_command(config: LibtestTextImportConfig, script: &str) -> LibtestCommandSpec {
    LibtestCommandSpec::new(config, "/bin/sh").with_args(["-c", script])
}
