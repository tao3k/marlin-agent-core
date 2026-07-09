use serde::Deserialize;
use std::collections::BTreeSet;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct EvidencePacketDocument {
    packet: EvidencePacket,
}

#[derive(Debug, Deserialize)]
struct EvidencePacket {
    id: String,
    phase1: EvidencePhase,
    phase2: EvidencePhase,
    gates: Vec<EvidenceGate>,
}

#[derive(Debug, Deserialize)]
struct EvidencePhase {
    status: String,
    receipt: String,
    #[serde(default)]
    verification: Vec<String>,
    #[serde(default)]
    focus: Vec<String>,
    #[serde(default)]
    evidence: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct EvidenceGate {
    phase: String,
    kind: String,
    receipt: String,
}

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn rfcdb_evidence_packet_covers_p0_to_p6_receipts() {
    let packet =
        toml::from_str::<EvidencePacketDocument>(include_str!("evidence/rfcdb_packet.toml"))
            .expect("RFCDB evidence packet fixture must be valid TOML")
            .packet;

    assert_eq!(packet.id, "rfcdb-storage-p0-p6");
    assert_eq!(packet.phase1.status, "complete");
    assert_eq!(packet.phase2.status, "intake");
    assert!(
        !packet.phase1.verification.is_empty(),
        "phase 1 completion must keep verification commands"
    );
    assert!(
        packet
            .phase2
            .focus
            .iter()
            .any(|focus| focus == "ifc-policy-pack-evidence-projection"),
        "phase 2 must include the IFC policy-pack evidence projection"
    );
    assert!(
        packet
            .phase2
            .evidence
            .iter()
            .any(|receipt| receipt == "tests/ifc_policy_pack_storage_gate.rs"),
        "phase 2 must record the IFC policy-pack storage gate"
    );

    let root = crate_root();
    assert!(
        root.join(&packet.phase1.receipt).exists(),
        "missing RFCDB phase1 completion receipt at {}",
        packet.phase1.receipt
    );
    assert!(
        root.join(&packet.phase2.receipt).exists(),
        "missing RFCDB phase2 intake receipt at {}",
        packet.phase2.receipt
    );
    for receipt in &packet.phase2.evidence {
        assert!(
            root.join(receipt).exists(),
            "missing RFCDB phase2 evidence receipt at {receipt}"
        );
    }

    let phases = packet
        .gates
        .iter()
        .map(|gate| gate.phase.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        phases,
        ["P0", "P1", "P2", "P3", "P4", "P5", "P6"]
            .into_iter()
            .collect::<BTreeSet<_>>()
    );

    for gate in &packet.gates {
        assert!(
            root.join(&gate.receipt).exists(),
            "missing RFCDB {} {} receipt at {}",
            gate.phase,
            gate.kind,
            gate.receipt
        );
    }
}
