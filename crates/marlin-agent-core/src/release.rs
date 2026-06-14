//! Release receipt bridges exposed by the core facade.

use marlin_agent_harness::{
    ReleaseGateExecutionReceipt, ReleaseGateExecutionStatus, release_gate_execution_receipt,
};
use marlin_gerbil_ir::{ReleaseGateSpec, ReleaseTopologySpec};
use marlin_org_store::{FileSystemReleaseStatusStore, OrgSourceStoreResult};
use marlin_org_workflow::{
    GerbilReleaseStatusCommit, GerbilReleaseStatusCommitReceipt, GerbilReleaseStatusCommitter,
};
use marlin_workspace_protocol::{ReleaseGateReceipt, ReleaseGateState};
use std::process::Command;

/// Result of executing one release gate command.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseGateCommandOutput {
    pub success: bool,
    pub status_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

impl ReleaseGateCommandOutput {
    /// Creates a successful command output fixture or adapter result.
    pub fn passed() -> Self {
        Self {
            success: true,
            status_code: Some(0),
            stdout: String::new(),
            stderr: String::new(),
        }
    }

    /// Creates a failed command output fixture or adapter result.
    pub fn failed(status_code: Option<i32>, stderr: impl Into<String>) -> Self {
        Self {
            success: false,
            status_code,
            stdout: String::new(),
            stderr: stderr.into(),
        }
    }
}

/// Runner abstraction used by release gate execution.
pub trait ReleaseGateCommandRunner {
    fn run_release_gate_command(&self, command: &str) -> ReleaseGateCommandOutput;
}

/// Process-backed release gate runner.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProcessReleaseGateCommandRunner;

impl ReleaseGateCommandRunner for ProcessReleaseGateCommandRunner {
    fn run_release_gate_command(&self, command: &str) -> ReleaseGateCommandOutput {
        match Command::new("sh").arg("-lc").arg(command).output() {
            Ok(output) => ReleaseGateCommandOutput {
                success: output.status.success(),
                status_code: output.status.code(),
                stdout: String::from_utf8_lossy(&output.stdout).trim().to_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
            },
            Err(error) => ReleaseGateCommandOutput::failed(None, error.to_string()),
        }
    }
}

/// Controls whether runner-owned execution may invoke external Gerbil/Gambit gates.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ReleaseGateRunOptions {
    pub run_local_gerbil_gates: bool,
}

impl ReleaseGateRunOptions {
    /// Creates options that allow manually requested local Gerbil/Gambit gates.
    pub fn with_local_gerbil_gates() -> Self {
        Self {
            run_local_gerbil_gates: true,
        }
    }
}

/// Request used to execute and persist one release gate.
#[derive(Clone, Copy, Debug)]
pub struct ReleaseGateRecordRequest<'a> {
    pub store: &'a FileSystemReleaseStatusStore,
    pub topology: &'a ReleaseTopologySpec,
    pub gate: &'a ReleaseGateSpec,
    pub options: ReleaseGateRunOptions,
}

impl<'a> ReleaseGateRecordRequest<'a> {
    /// Creates a request for one release gate execution and status-store write.
    pub fn new(
        store: &'a FileSystemReleaseStatusStore,
        topology: &'a ReleaseTopologySpec,
        gate: &'a ReleaseGateSpec,
    ) -> Self {
        Self {
            store,
            topology,
            gate,
            options: ReleaseGateRunOptions::default(),
        }
    }

    /// Sets runner options for this release gate execution.
    pub fn with_options(mut self, options: ReleaseGateRunOptions) -> Self {
        self.options = options;
        self
    }
}

/// Persist a Gerbil release status commit from harness execution receipts.
pub fn commit_release_gate_execution_receipts(
    store: &FileSystemReleaseStatusStore,
    topology: ReleaseTopologySpec,
    receipts: &[ReleaseGateExecutionReceipt],
) -> OrgSourceStoreResult<GerbilReleaseStatusCommitReceipt> {
    let commit = gerbil_release_status_commit_from_execution_receipts(topology, receipts);
    GerbilReleaseStatusCommitter::commit(store, &commit)
}

/// Build a Gerbil release workflow commit from harness execution receipts.
pub fn gerbil_release_status_commit_from_execution_receipts(
    topology: ReleaseTopologySpec,
    receipts: &[ReleaseGateExecutionReceipt],
) -> GerbilReleaseStatusCommit {
    receipts.iter().fold(
        GerbilReleaseStatusCommit::new(topology),
        |commit, receipt| commit.with_gate_receipt(release_gate_status_receipt(receipt)),
    )
}

/// Record a harness release gate execution receipt in a file-backed workspace status sidecar.
pub fn record_release_gate_execution_receipt(
    store: &FileSystemReleaseStatusStore,
    receipt: &ReleaseGateExecutionReceipt,
) -> OrgSourceStoreResult<bool> {
    store.record_release_gate_receipt(release_gate_status_receipt(receipt))
}

/// Execute one release gate and return a typed harness receipt.
pub fn execute_release_gate_with_runner<R>(
    topology: &ReleaseTopologySpec,
    gate: &ReleaseGateSpec,
    options: ReleaseGateRunOptions,
    runner: &R,
) -> ReleaseGateExecutionReceipt
where
    R: ReleaseGateCommandRunner,
{
    if gate.requires_local_gerbil && !options.run_local_gerbil_gates {
        return release_gate_execution_receipt(topology, gate, ReleaseGateExecutionStatus::Skipped)
            .with_diagnostics([
                "release_gate.skipped".to_owned(),
                "release_gate.requires_local_gerbil=true".to_owned(),
                "release_gate.local_gerbil_not_requested".to_owned(),
            ]);
    }

    release_gate_execution_receipt_from_output(
        topology,
        gate,
        runner.run_release_gate_command(&gate.command),
    )
}

/// Execute one release gate with the default process runner.
pub fn execute_release_gate(
    topology: &ReleaseTopologySpec,
    gate: &ReleaseGateSpec,
    options: ReleaseGateRunOptions,
) -> ReleaseGateExecutionReceipt {
    execute_release_gate_with_runner(topology, gate, options, &ProcessReleaseGateCommandRunner)
}

/// Execute and persist one release gate receipt in the release status store.
pub fn execute_and_record_release_gate_with_runner<R>(
    request: ReleaseGateRecordRequest<'_>,
    runner: &R,
) -> OrgSourceStoreResult<ReleaseGateExecutionReceipt>
where
    R: ReleaseGateCommandRunner,
{
    let receipt =
        execute_release_gate_with_runner(request.topology, request.gate, request.options, runner);
    record_release_gate_execution_receipt(request.store, &receipt)?;
    Ok(receipt)
}

/// Convert one command result into a release gate execution receipt.
pub fn release_gate_execution_receipt_from_output(
    topology: &ReleaseTopologySpec,
    gate: &ReleaseGateSpec,
    output: ReleaseGateCommandOutput,
) -> ReleaseGateExecutionReceipt {
    let status = if output.success {
        ReleaseGateExecutionStatus::Passed
    } else {
        ReleaseGateExecutionStatus::Failed
    };
    let mut diagnostics = vec![format!(
        "release_gate.status_code={}",
        output
            .status_code
            .map(|code| code.to_string())
            .unwrap_or_else(|| "none".to_owned())
    )];
    if !output.stdout.is_empty() {
        diagnostics.push(format!("release_gate.stdout={}", output.stdout));
    }
    if !output.stderr.is_empty() {
        diagnostics.push(format!("release_gate.stderr={}", output.stderr));
    }

    release_gate_execution_receipt(topology, gate, status).with_diagnostics(diagnostics)
}

/// Convert a harness release gate execution receipt into a workspace status receipt.
pub fn release_gate_status_receipt(receipt: &ReleaseGateExecutionReceipt) -> ReleaseGateReceipt {
    ReleaseGateReceipt {
        gate_id: receipt.gate_id.clone(),
        state: release_gate_state_from_execution(receipt),
        evidence_keys: receipt.evidence_keys.clone(),
        artifact_paths: receipt.artifact_paths.clone(),
        diagnostics: receipt.diagnostics.clone(),
    }
}

/// Project harness execution status into the workspace release gate state model.
pub fn release_gate_state_from_execution(
    receipt: &ReleaseGateExecutionReceipt,
) -> ReleaseGateState {
    match receipt.status {
        ReleaseGateExecutionStatus::Expected if receipt.requires_local_gerbil => {
            ReleaseGateState::RequiresLocalGerbil
        }
        ReleaseGateExecutionStatus::Expected => ReleaseGateState::Pending,
        ReleaseGateExecutionStatus::Passed => ReleaseGateState::Passed,
        ReleaseGateExecutionStatus::Failed => ReleaseGateState::Failed,
        ReleaseGateExecutionStatus::Skipped => ReleaseGateState::Skipped,
    }
}
