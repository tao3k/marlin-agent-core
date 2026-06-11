//! Release receipt bridges exposed by the core facade.

use marlin_agent_harness::{ReleaseGateExecutionReceipt, ReleaseGateExecutionStatus};
use marlin_gerbil_ir::ReleaseTopologySpec;
use marlin_org_store::{FileSystemReleaseStatusStore, OrgSourceStoreResult};
use marlin_org_workflow::{
    GerbilReleaseStatusCommit, GerbilReleaseStatusCommitReceipt, GerbilReleaseStatusCommitter,
};
use marlin_workspace_protocol::{ReleaseGateReceipt, ReleaseGateState};

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
