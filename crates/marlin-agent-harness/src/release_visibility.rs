//! Release visibility evidence bridge for `Gerbil` release topology gates.

use marlin_agent_protocol::{LoopEvidence, LoopEvidenceKind};
use marlin_gerbil_ir::{ReleaseGateSpec, ReleaseTopologySpec, ReleaseVisibilitySpec};

/// Execution status captured for one release gate receipt.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReleaseGateExecutionStatus {
    Expected,
    Passed,
    Failed,
    Skipped,
}

/// Typed receipt for one release gate execution boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseGateExecutionReceipt {
    pub topology_id: String,
    pub crate_name: String,
    pub gate_id: String,
    pub command: String,
    pub requires_local_gerbil: bool,
    pub status: ReleaseGateExecutionStatus,
    pub required_artifacts: Vec<String>,
    pub evidence_keys: Vec<String>,
    pub artifact_paths: Vec<String>,
    pub diagnostics: Vec<String>,
    pub visibility_evidence: Vec<LoopEvidence>,
}

impl ReleaseGateExecutionReceipt {
    /// Attach diagnostics captured while evaluating this release gate.
    pub fn with_diagnostics(mut self, diagnostics: impl IntoIterator<Item = String>) -> Self {
        self.diagnostics = diagnostics.into_iter().collect();
        self
    }
}

/// Convert one release visibility declaration into harness evidence.
pub fn release_visibility_evidence(
    topology: &ReleaseTopologySpec,
    gate: &ReleaseGateSpec,
    visibility: &ReleaseVisibilitySpec,
) -> LoopEvidence {
    let detail = format!(
        "topology_id={} crate_name={} gate_id={} report_key={} evidence_keys=[{}] artifact_paths=[{}]",
        topology.topology_id,
        topology.crate_name,
        gate.gate_id,
        visibility.report_key,
        visibility.evidence_keys.join(","),
        visibility.artifact_paths.join(","),
    );

    LoopEvidence::present(
        LoopEvidenceKind::Visibility,
        format!(
            "release-visibility:{}:{}:{}",
            topology.topology_id, gate.gate_id, visibility.report_key
        ),
    )
    .with_detail(detail)
}

/// Convert all visibility declarations from one release gate into harness evidence.
pub fn release_gate_visibility_evidence(
    topology: &ReleaseTopologySpec,
    gate: &ReleaseGateSpec,
) -> Vec<LoopEvidence> {
    gate.visibility
        .iter()
        .map(|visibility| release_visibility_evidence(topology, gate, visibility))
        .collect()
}

/// Convert one release gate into a typed execution receipt plus visibility evidence.
pub fn release_gate_execution_receipt(
    topology: &ReleaseTopologySpec,
    gate: &ReleaseGateSpec,
    status: ReleaseGateExecutionStatus,
) -> ReleaseGateExecutionReceipt {
    let evidence_keys = gate
        .visibility
        .iter()
        .flat_map(|visibility| visibility.evidence_keys.iter().cloned())
        .collect();
    let artifact_paths = gate
        .visibility
        .iter()
        .flat_map(|visibility| visibility.artifact_paths.iter().cloned())
        .collect();

    ReleaseGateExecutionReceipt {
        topology_id: topology.topology_id.clone(),
        crate_name: topology.crate_name.clone(),
        gate_id: gate.gate_id.clone(),
        command: gate.command.clone(),
        requires_local_gerbil: gate.requires_local_gerbil,
        status,
        required_artifacts: gate.required_artifacts.clone(),
        evidence_keys,
        artifact_paths,
        diagnostics: Vec::new(),
        visibility_evidence: release_gate_visibility_evidence(topology, gate),
    }
}

/// Convert all release gate visibility declarations in a topology into harness evidence.
pub fn release_topology_visibility_evidence(topology: &ReleaseTopologySpec) -> Vec<LoopEvidence> {
    topology
        .gates
        .iter()
        .flat_map(|gate| release_gate_visibility_evidence(topology, gate))
        .collect()
}

/// Convert all release gates in a topology into typed execution receipts.
pub fn release_topology_execution_receipts(
    topology: &ReleaseTopologySpec,
    status: ReleaseGateExecutionStatus,
) -> Vec<ReleaseGateExecutionReceipt> {
    topology
        .gates
        .iter()
        .map(|gate| release_gate_execution_receipt(topology, gate, status))
        .collect()
}
