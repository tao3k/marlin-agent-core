//! Release visibility evidence bridge for `Gerbil` release topology gates.

use marlin_agent_protocol::{LoopEvidence, LoopEvidenceKind};
use marlin_gerbil_ir::{ReleaseGateSpec, ReleaseTopologySpec, ReleaseVisibilitySpec};

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

/// Convert all release gate visibility declarations in a topology into harness evidence.
pub fn release_topology_visibility_evidence(topology: &ReleaseTopologySpec) -> Vec<LoopEvidence> {
    topology
        .gates
        .iter()
        .flat_map(|gate| release_gate_visibility_evidence(topology, gate))
        .collect()
}
