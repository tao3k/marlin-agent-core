//! Import helpers for stable libtest text output.

use super::model::{TestRunCaseRecord, TestRunEvidenceReceipt, TestRunLayer};

/// Configuration for importing one libtest text stream into test evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LibtestTextImportConfig {
    pub package_name: String,
    pub default_layer: TestRunLayer,
    pub integration_prefixes: Vec<String>,
    pub live_external_prefixes: Vec<String>,
}

impl LibtestTextImportConfig {
    /// Builds a config for one package, defaulting tests to the no-live unit layer.
    pub fn new(package_name: impl Into<String>) -> Self {
        Self {
            package_name: package_name.into(),
            default_layer: TestRunLayer::NonLiveUnit,
            integration_prefixes: Vec::new(),
            live_external_prefixes: Vec::new(),
        }
    }

    /// Sets the fallback layer for unclassified tests.
    pub fn with_default_layer(mut self, layer: TestRunLayer) -> Self {
        self.default_layer = layer;
        self
    }

    /// Marks tests with this name prefix as no-live integration tests.
    pub fn with_integration_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.integration_prefixes.push(prefix.into());
        self
    }

    /// Marks tests with this name prefix as live external tests.
    pub fn with_live_external_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.live_external_prefixes.push(prefix.into());
        self
    }

    fn layer_for_test(&self, test_name: &str) -> TestRunLayer {
        if self
            .live_external_prefixes
            .iter()
            .any(|prefix| test_name.starts_with(prefix))
        {
            return TestRunLayer::LiveExternal;
        }
        if self
            .integration_prefixes
            .iter()
            .any(|prefix| test_name.starts_with(prefix))
        {
            return TestRunLayer::NonLiveIntegration;
        }
        self.default_layer
    }
}

/// Counts parsed from a libtest `test result:` summary line.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LibtestTextResultSummary {
    pub passed: usize,
    pub failed: usize,
    pub ignored: usize,
    pub measured: usize,
    pub filtered_out: usize,
}

/// Result of importing one libtest text stream.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LibtestTextImportReport {
    pub receipt: TestRunEvidenceReceipt,
    pub declared_test_count: Option<usize>,
    pub result_summary: Option<LibtestTextResultSummary>,
    pub diagnostics: Vec<String>,
}

impl LibtestTextImportReport {
    /// Returns true when the parsed stream has no importer diagnostics.
    pub fn is_consistent(&self) -> bool {
        self.diagnostics.is_empty()
    }
}

/// Input for importing one package's libtest text stream into a workspace report.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceLibtestTextImportInput {
    pub config: LibtestTextImportConfig,
    pub output: String,
}

impl WorkspaceLibtestTextImportInput {
    /// Creates a workspace import input for one package.
    pub fn new(config: LibtestTextImportConfig, output: impl Into<String>) -> Self {
        Self {
            config,
            output: output.into(),
        }
    }
}

/// Imported report for one package inside a workspace test run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceLibtestTextPackageReport {
    pub package_name: String,
    pub report: LibtestTextImportReport,
}

impl WorkspaceLibtestTextPackageReport {
    /// Returns true when this package import has no diagnostics.
    pub fn is_consistent(&self) -> bool {
        self.report.is_consistent()
    }
}

/// Workspace-level import report across many package libtest streams.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceLibtestTextImportReport {
    pub receipt: TestRunEvidenceReceipt,
    pub package_reports: Vec<WorkspaceLibtestTextPackageReport>,
    pub diagnostics: Vec<String>,
}

impl WorkspaceLibtestTextImportReport {
    /// Returns true when every package import is internally consistent.
    pub fn is_consistent(&self) -> bool {
        self.diagnostics.is_empty()
    }

    /// Number of imported package streams.
    pub fn package_count(&self) -> usize {
        self.package_reports.len()
    }
}

/// Imports stable libtest text output into a typed test run evidence receipt.
pub fn import_libtest_text_output(
    config: &LibtestTextImportConfig,
    output: &str,
) -> LibtestTextImportReport {
    let parsed_lines = parse_libtest_lines(config, output);
    let receipt = TestRunEvidenceReceipt::new(parsed_lines.cases);
    let diagnostics = import_diagnostics(
        &receipt,
        parsed_lines.declared_test_count,
        parsed_lines.result_summary,
    );

    LibtestTextImportReport {
        receipt,
        declared_test_count: parsed_lines.declared_test_count,
        result_summary: parsed_lines.result_summary,
        diagnostics,
    }
}

/// Imports many package libtest text streams into a workspace test evidence receipt.
pub fn import_workspace_libtest_text_outputs(
    inputs: impl IntoIterator<Item = WorkspaceLibtestTextImportInput>,
) -> WorkspaceLibtestTextImportReport {
    let package_reports = inputs
        .into_iter()
        .map(import_workspace_package_libtest_text_output)
        .collect::<Vec<_>>();
    let receipt = TestRunEvidenceReceipt::new(
        package_reports
            .iter()
            .flat_map(|package_report| package_report.report.receipt.cases.iter().cloned())
            .collect(),
    );
    let diagnostics = package_reports
        .iter()
        .flat_map(workspace_package_diagnostics)
        .collect();

    WorkspaceLibtestTextImportReport {
        receipt,
        package_reports,
        diagnostics,
    }
}

fn import_workspace_package_libtest_text_output(
    input: WorkspaceLibtestTextImportInput,
) -> WorkspaceLibtestTextPackageReport {
    let report = import_libtest_text_output(&input.config, &input.output);
    WorkspaceLibtestTextPackageReport {
        package_name: input.config.package_name,
        report,
    }
}

fn workspace_package_diagnostics(
    package_report: &WorkspaceLibtestTextPackageReport,
) -> impl Iterator<Item = String> + '_ {
    package_report
        .report
        .diagnostics
        .iter()
        .map(|diagnostic| format!("{}:{diagnostic}", package_report.package_name))
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct LibtestTextParsedLines {
    cases: Vec<TestRunCaseRecord>,
    declared_test_count: Option<usize>,
    result_summary: Option<LibtestTextResultSummary>,
}

fn parse_libtest_lines(config: &LibtestTextImportConfig, output: &str) -> LibtestTextParsedLines {
    output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .fold(
            LibtestTextParsedLines::default(),
            |mut parsed_lines, line| {
                if let Some(count) = parse_running_test_count(line) {
                    parsed_lines.declared_test_count = Some(count);
                } else if let Some(summary) = parse_result_summary(line) {
                    parsed_lines.result_summary = Some(summary);
                } else if let Some(case) = parse_test_case_line(config, line) {
                    parsed_lines.cases.push(case);
                }
                parsed_lines
            },
        )
}

fn import_diagnostics(
    receipt: &TestRunEvidenceReceipt,
    declared_test_count: Option<usize>,
    result_summary: Option<LibtestTextResultSummary>,
) -> Vec<String> {
    let mut diagnostics = Vec::new();
    if let Some(count) = declared_test_count
        && count != receipt.case_count()
    {
        diagnostics.push(format!(
            "declared_test_count={} parsed_case_count={}",
            count,
            receipt.case_count()
        ));
    }
    if let Some(summary) = result_summary {
        append_summary_diagnostics(receipt, summary, &mut diagnostics);
    }

    diagnostics
}

fn parse_running_test_count(line: &str) -> Option<usize> {
    let rest = line.strip_prefix("running ")?;
    let count = rest.strip_suffix(" tests")?.trim();
    count.parse::<usize>().ok()
}

fn parse_test_case_line(config: &LibtestTextImportConfig, line: &str) -> Option<TestRunCaseRecord> {
    let rest = line.strip_prefix("test ")?;
    let (test_name, status_text) = rest.rsplit_once(" ... ")?;
    let layer = config.layer_for_test(test_name);
    let package_name = config.package_name.clone();

    if status_text == "ok" {
        return Some(TestRunCaseRecord::passed(package_name, test_name, layer));
    }
    if status_text == "FAILED" {
        return Some(TestRunCaseRecord::failed(package_name, test_name, layer));
    }
    if let Some(reason) = status_text.strip_prefix("ignored") {
        let reason = reason.trim_start_matches(',').trim();
        return Some(TestRunCaseRecord::ignored(
            package_name,
            test_name,
            layer,
            reason,
        ));
    }

    None
}

fn parse_result_summary(line: &str) -> Option<LibtestTextResultSummary> {
    let rest = line.strip_prefix("test result: ")?;
    let (_, counts) = rest.split_once(". ")?;
    Some(LibtestTextResultSummary {
        passed: parse_summary_count(counts, "passed")?,
        failed: parse_summary_count(counts, "failed")?,
        ignored: parse_summary_count(counts, "ignored")?,
        measured: parse_summary_count(counts, "measured")?,
        filtered_out: parse_summary_count(counts, "filtered out")?,
    })
}

fn parse_summary_count(counts: &str, label: &str) -> Option<usize> {
    counts
        .split(';')
        .map(str::trim)
        .find_map(|part| part.strip_suffix(label).map(str::trim))
        .and_then(|count| count.parse::<usize>().ok())
}

fn append_summary_diagnostics(
    receipt: &TestRunEvidenceReceipt,
    summary: LibtestTextResultSummary,
    diagnostics: &mut Vec<String>,
) {
    append_count_diagnostic(
        "passed",
        summary.passed,
        receipt.passed_count(),
        diagnostics,
    );
    append_count_diagnostic(
        "failed",
        summary.failed,
        receipt.failed_count(),
        diagnostics,
    );
    append_count_diagnostic(
        "ignored",
        summary.ignored,
        receipt.ignored_count(),
        diagnostics,
    );
}

fn append_count_diagnostic(
    label: &str,
    expected: usize,
    actual: usize,
    diagnostics: &mut Vec<String>,
) {
    if expected != actual {
        diagnostics.push(format!(
            "{label}_summary={expected} parsed_{label}={actual}"
        ));
    }
}
