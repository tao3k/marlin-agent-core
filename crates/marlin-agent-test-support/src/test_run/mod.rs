//! Typed no-live-LLM test run evidence facade.

mod command;
mod import;
mod model;

pub use command::{
    LibtestCommandCapture, LibtestCommandImportReport, LibtestCommandSpec,
    WorkspaceLibtestCommandImportReport, WorkspaceLibtestCommandPackageReport,
    capture_libtest_command_output, capture_workspace_libtest_commands,
};
pub use import::{
    LibtestTextImportConfig, LibtestTextImportReport, LibtestTextResultSummary,
    WorkspaceLibtestTextImportInput, WorkspaceLibtestTextImportReport,
    WorkspaceLibtestTextPackageReport, import_libtest_text_output,
    import_workspace_libtest_text_outputs,
};
pub use model::{
    TEST_RUN_EVIDENCE_SCHEMA_VERSION, TestRunCaseRecord, TestRunCaseStatus, TestRunEvidenceReceipt,
    TestRunLayer, TestRunLayerSummary, assert_deterministic_test_run_evidence,
    assert_user_interface_loop_live_llm_gate_evidence, deterministic_test_run_evidence_fixture,
    user_interface_loop_live_llm_gate_evidence_fixture,
};
