// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use marlin_gerbil_ir::{ReleaseGateSpec, ReleaseTopologySpec, ReleaseVisibilitySpec};

#[test]
fn gerbil_release_topology_carries_package_gates_and_dependency_chains() {
    let topology = ReleaseTopologySpec {
        topology_id: "gerbil-scheme-internal-release".to_string(),
        crate_name: "marlin-gerbil-scheme".to_string(),
        publish_enabled: false,
        asset_audit_command: "cargo package -p marlin-gerbil-scheme --list --allow-dirty"
            .to_string(),
        package_assets: vec![
            "README.org".to_string(),
            "src/lib.rs".to_string(),
            "fixtures/gerbil/marlin/protocol.ss".to_string(),
        ],
        runtime_dependency_chain: vec![
            "marlin-gerbil-ir".to_string(),
            "marlin-context-org-model".to_string(),
            "marlin-agent-protocol".to_string(),
        ],
        workflow_dependency_chain: vec![
            "marlin-agent-gitops".to_string(),
            "marlin-agent-context".to_string(),
        ],
        gates: vec![ReleaseGateSpec {
            gate_id: "package-assets".to_string(),
            command: "cargo package -p marlin-gerbil-scheme --list --allow-dirty".to_string(),
            requires_local_gerbil: false,
            required_artifacts: vec!["fixtures/gerbil/marlin/protocol.ss".to_string()],
            visibility: vec![ReleaseVisibilitySpec {
                report_key: "package_asset_audit".to_string(),
                evidence_keys: vec!["required_artifacts".to_string()],
                artifact_paths: vec!["fixtures/gerbil/marlin/protocol.ss".to_string()],
            }],
        }],
    };

    assert_eq!(topology.crate_name, "marlin-gerbil-scheme");
    assert!(!topology.publish_enabled);
    assert!(
        topology
            .package_assets
            .iter()
            .any(|asset| asset == "README.org")
    );
    assert!(
        topology
            .runtime_dependency_chain
            .iter()
            .any(|crate_name| crate_name == "marlin-agent-protocol")
    );
    assert_eq!(topology.gates[0].gate_id, "package-assets");
    assert!(!topology.gates[0].requires_local_gerbil);
    assert_eq!(
        topology.gates[0].visibility[0].report_key,
        "package_asset_audit"
    );
    assert!(
        topology.gates[0].visibility[0]
            .evidence_keys
            .iter()
            .any(|key| key == "required_artifacts")
    );
    assert_eq!(
        topology.gates[0].visibility[0].artifact_paths,
        topology.gates[0].required_artifacts
    );
}
