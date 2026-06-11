//! Workflow for persisting Gerbil release topology status sidecars.

use marlin_gerbil_ir::ReleaseTopologySpec;
use marlin_org_store::{FileSystemReleaseStatusStore, OrgSourceStoreResult};
use marlin_workspace_status::{ReleaseGateReceipt, ReleaseStatus};
use serde::{Deserialize, Serialize};

/// File-backed release status commit emitted from the Gerbil release workflow.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilReleaseStatusCommit {
    pub topology: ReleaseTopologySpec,
    pub gate_receipts: Vec<ReleaseGateReceipt>,
}

impl GerbilReleaseStatusCommit {
    /// Create a release status commit from a Gerbil release topology.
    pub fn new(topology: ReleaseTopologySpec) -> Self {
        Self {
            topology,
            gate_receipts: Vec::new(),
        }
    }

    /// Add one gate receipt to persist with this commit.
    pub fn with_gate_receipt(mut self, receipt: ReleaseGateReceipt) -> Self {
        self.gate_receipts.push(receipt);
        self
    }
}

/// Receipt returned after persisting a Gerbil release status commit.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilReleaseStatusCommitReceipt {
    pub status: ReleaseStatus,
    pub recorded_gate_receipts: usize,
    pub missing_gate_receipts: Vec<String>,
}

impl GerbilReleaseStatusCommitReceipt {
    /// Returns true when every provided gate receipt matched the topology.
    pub fn accepted(&self) -> bool {
        self.missing_gate_receipts.is_empty()
    }
}

/// Persists Gerbil release topology and gate receipts into a file-backed sidecar.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GerbilReleaseStatusCommitter;

impl GerbilReleaseStatusCommitter {
    /// Persist the release topology and matching gate receipts into the sidecar store.
    pub fn commit(
        store: &FileSystemReleaseStatusStore,
        commit: &GerbilReleaseStatusCommit,
    ) -> OrgSourceStoreResult<GerbilReleaseStatusCommitReceipt> {
        let mut status = ReleaseStatus::pending_from_topology(&commit.topology);
        let receipt_summary = apply_gate_receipts(&mut status, &commit.gate_receipts);

        store.write_status(&status)?;
        Ok(GerbilReleaseStatusCommitReceipt {
            status,
            recorded_gate_receipts: receipt_summary.recorded,
            missing_gate_receipts: receipt_summary.missing,
        })
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct GateReceiptApplySummary {
    recorded: usize,
    missing: Vec<String>,
}

fn apply_gate_receipts(
    status: &mut ReleaseStatus,
    receipts: &[ReleaseGateReceipt],
) -> GateReceiptApplySummary {
    let receipt_results = receipts
        .iter()
        .map(|receipt| {
            status
                .record_gate_receipt(receipt.clone())
                .then_some(())
                .ok_or_else(|| receipt.gate_id.clone())
        })
        .collect::<Vec<_>>();

    GateReceiptApplySummary {
        recorded: receipt_results
            .iter()
            .filter(|result| result.is_ok())
            .count(),
        missing: receipt_results
            .into_iter()
            .filter_map(Result::err)
            .collect(),
    }
}
