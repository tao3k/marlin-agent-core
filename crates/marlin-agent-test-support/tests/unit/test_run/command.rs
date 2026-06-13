use marlin_agent_test_support::{
    LibtestCommandSpec, LibtestTextImportConfig, capture_libtest_command_output,
    capture_workspace_libtest_commands,
};

#[cfg(unix)]
#[test]
fn imports_stdout_libtest_output() {
    let script = r#"printf '%s\n' 'running 2 tests' 'test runtime::session::sub_agent_session_context_isolated_from_parent ... ok' 'test command::real_gxi::artifacts::command_compiler_can_call_real_gxi_workspace_schema ... ignored, requires a local Gerbil gxi executable' '' 'test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s'"#;
    let spec = libtest_shell_command(
        LibtestTextImportConfig::new("marlin-agent-test-support")
            .with_live_external_prefix("command::real_gxi::"),
        script,
    );

    let report = capture_libtest_command_output(&spec).expect("capture libtest command output");

    assert_eq!(report.capture.status_code, Some(0));
    assert!(
        report.import.is_consistent(),
        "{:?}",
        report.import.diagnostics
    );
    assert!(report.is_consistent(), "{:?}", report.diagnostics);
    assert_eq!(report.import.receipt.case_count(), 2);
    assert_eq!(report.import.receipt.passed_count(), 1);
    assert_eq!(report.import.receipt.ignored_live_external_count(), 1);
}

#[cfg(unix)]
#[test]
fn records_nonzero_status_but_keeps_failed_test_receipt() {
    let script = r#"printf '%s\n' 'running 1 tests' 'test runtime::session::sub_agent_session_context_isolated_from_parent ... FAILED' '' 'test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s'; exit 1"#;
    let spec = libtest_shell_command(LibtestTextImportConfig::new("marlin-agent-runtime"), script);

    let report = capture_libtest_command_output(&spec).expect("capture libtest command output");

    assert_eq!(report.capture.status_code, Some(1));
    assert!(!report.command_succeeded());
    assert!(
        report.import.is_consistent(),
        "{:?}",
        report.import.diagnostics
    );
    assert!(!report.is_consistent());
    assert!(
        report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic == "command_status=1")
    );
    assert_eq!(report.import.receipt.failed_count(), 1);
    assert_eq!(report.import.receipt.non_live_failed_count(), 1);
}

#[cfg(unix)]
#[test]
fn workspace_command_capture_merges_captured_packages() {
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

    assert!(report.is_consistent(), "{:?}", report.diagnostics);
    assert_eq!(report.package_count(), 2);
    assert_eq!(report.receipt.case_count(), 3);
    assert_eq!(report.receipt.passed_count(), 2);
    assert_eq!(report.receipt.non_live_failed_count(), 0);
    assert_eq!(report.receipt.ignored_live_external_count(), 1);
}

#[cfg(unix)]
fn libtest_shell_command(config: LibtestTextImportConfig, script: &str) -> LibtestCommandSpec {
    LibtestCommandSpec::new(config, "/bin/sh").with_args(["-c", script])
}
