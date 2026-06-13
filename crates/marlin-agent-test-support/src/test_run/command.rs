//! Command capture helpers for stable libtest text output.

use std::{
    ffi::OsString,
    io,
    path::PathBuf,
    process::{Command, Output},
};

use super::{
    import::{LibtestTextImportConfig, LibtestTextImportReport, import_libtest_text_output},
    model::TestRunEvidenceReceipt,
};

/// Command specification for a libtest-compatible process.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LibtestCommandSpec {
    pub import_config: LibtestTextImportConfig,
    pub program: PathBuf,
    pub args: Vec<OsString>,
    pub cwd: Option<PathBuf>,
}

impl LibtestCommandSpec {
    /// Creates a command spec for one package test stream.
    pub fn new(import_config: LibtestTextImportConfig, program: impl Into<PathBuf>) -> Self {
        Self {
            import_config,
            program: program.into(),
            args: Vec::new(),
            cwd: None,
        }
    }

    /// Appends one command argument.
    pub fn with_arg(mut self, arg: impl Into<OsString>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Appends many command arguments.
    pub fn with_args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    /// Sets the command working directory.
    pub fn with_cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }
}

/// Captured stdout/stderr and exit status for a libtest command.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LibtestCommandCapture {
    pub status_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

impl LibtestCommandCapture {
    /// Combined text stream used by the libtest importer.
    pub fn combined_output(&self) -> String {
        match (self.stdout.is_empty(), self.stderr.is_empty()) {
            (true, true) => String::new(),
            (false, true) => self.stdout.clone(),
            (true, false) => self.stderr.clone(),
            (false, false) => format!("{}\n{}", self.stdout, self.stderr),
        }
    }

    fn from_output(output: Output) -> Self {
        Self {
            status_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }
}

/// Imported command report for one libtest-compatible process.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LibtestCommandImportReport {
    pub capture: LibtestCommandCapture,
    pub import: LibtestTextImportReport,
    pub diagnostics: Vec<String>,
}

impl LibtestCommandImportReport {
    /// Returns true when the command exited with status code zero.
    pub fn command_succeeded(&self) -> bool {
        self.capture.status_code == Some(0)
    }

    /// Returns true when command status and text import diagnostics are clean.
    pub fn is_consistent(&self) -> bool {
        self.diagnostics.is_empty()
    }

    /// Returns true when the command and deterministic no-live layers passed.
    pub fn is_non_live_success(&self) -> bool {
        self.is_consistent() && self.import.receipt.is_non_live_success()
    }
}

/// Imported command report for one package in a workspace run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceLibtestCommandPackageReport {
    pub package_name: String,
    pub command: LibtestCommandImportReport,
}

impl WorkspaceLibtestCommandPackageReport {
    /// Returns true when this package command and import are consistent.
    pub fn is_consistent(&self) -> bool {
        self.command.is_consistent()
    }
}

/// Workspace-level command import report across many package test streams.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceLibtestCommandImportReport {
    pub receipt: TestRunEvidenceReceipt,
    pub package_reports: Vec<WorkspaceLibtestCommandPackageReport>,
    pub diagnostics: Vec<String>,
}

impl WorkspaceLibtestCommandImportReport {
    /// Returns true when every package command and import is internally consistent.
    pub fn is_consistent(&self) -> bool {
        self.diagnostics.is_empty()
    }

    /// Returns true when workspace deterministic no-live test layers passed.
    pub fn is_non_live_success(&self) -> bool {
        self.is_consistent() && self.receipt.is_non_live_success()
    }

    /// Number of imported package command streams.
    pub fn package_count(&self) -> usize {
        self.package_reports.len()
    }
}

/// Runs one libtest-compatible command and imports its captured text output.
pub fn capture_libtest_command_output(
    spec: &LibtestCommandSpec,
) -> io::Result<LibtestCommandImportReport> {
    let mut command = Command::new(&spec.program);
    command.args(&spec.args);
    if let Some(cwd) = &spec.cwd {
        command.current_dir(cwd);
    }

    let capture = LibtestCommandCapture::from_output(command.output()?);
    let import = import_libtest_text_output(&spec.import_config, &capture.combined_output());
    let diagnostics = command_import_diagnostics(&capture, &import);

    Ok(LibtestCommandImportReport {
        capture,
        import,
        diagnostics,
    })
}

/// Runs many package libtest-compatible commands and imports a workspace receipt.
pub fn capture_workspace_libtest_commands(
    specs: impl IntoIterator<Item = LibtestCommandSpec>,
) -> io::Result<WorkspaceLibtestCommandImportReport> {
    let package_reports = specs
        .into_iter()
        .map(capture_workspace_libtest_command)
        .collect::<io::Result<Vec<_>>>()?;
    let receipt = TestRunEvidenceReceipt::new(
        package_reports
            .iter()
            .flat_map(|package_report| package_report.command.import.receipt.cases.iter().cloned())
            .collect(),
    );
    let diagnostics = package_reports
        .iter()
        .flat_map(workspace_command_package_diagnostics)
        .collect();

    Ok(WorkspaceLibtestCommandImportReport {
        receipt,
        package_reports,
        diagnostics,
    })
}

fn capture_workspace_libtest_command(
    spec: LibtestCommandSpec,
) -> io::Result<WorkspaceLibtestCommandPackageReport> {
    let package_name = spec.import_config.package_name.clone();
    let command = capture_libtest_command_output(&spec)?;
    Ok(WorkspaceLibtestCommandPackageReport {
        package_name,
        command,
    })
}

fn command_import_diagnostics(
    capture: &LibtestCommandCapture,
    import: &LibtestTextImportReport,
) -> Vec<String> {
    command_status_diagnostics(capture)
        .into_iter()
        .chain(
            import
                .diagnostics
                .iter()
                .map(|diagnostic| format!("import:{diagnostic}")),
        )
        .collect()
}

fn command_status_diagnostics(capture: &LibtestCommandCapture) -> Vec<String> {
    match capture.status_code {
        Some(0) => Vec::new(),
        Some(status_code) => vec![format!("command_status={status_code}")],
        None => vec!["command_status=unknown".to_string()],
    }
}

fn workspace_command_package_diagnostics(
    package_report: &WorkspaceLibtestCommandPackageReport,
) -> impl Iterator<Item = String> + '_ {
    package_report
        .command
        .diagnostics
        .iter()
        .map(|diagnostic| format!("{}:{diagnostic}", package_report.package_name))
}
