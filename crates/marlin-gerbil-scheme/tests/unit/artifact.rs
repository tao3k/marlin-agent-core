use marlin_agent_protocol::{
    AgentScenario, AgentScenarioContract, AgentScenarioStep, LoopEvidenceKind,
};
use marlin_gerbil_ir::{CompiledLoopGraph, ReleaseTopologySpec, WorkspacePatchIntentSpec};
use marlin_gerbil_ir::{ReleaseGateSpec, ReleaseVisibilitySpec};
use marlin_gerbil_scheme::{GerbilArtifactKind, GerbilCompiledArtifact};
use marlin_org_memory::MemoryOrgWorkspace;
use marlin_org_model::OrgNodeId;
use marlin_org_store::FileSystemReleaseStatusStore;
use marlin_workspace_patch::{WorkspacePatch, WorkspacePatchOp};
use marlin_workspace_protocol::{
    AgentWorkspace, ReleaseGateReceipt, ReleaseGateState, WorkspaceCtx, WorkspaceTarget,
};
use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

fn empty_loop_graph() -> CompiledLoopGraph {
    CompiledLoopGraph {
        graph_id: "audit-loop".to_string(),
        nodes: Vec::new(),
        edges: Vec::new(),
    }
}

#[test]
fn artifact_reports_its_kind() {
    let artifact = GerbilCompiledArtifact::LoopGraph(empty_loop_graph());

    assert_eq!(artifact.kind(), GerbilArtifactKind::LoopGraph);
}

#[test]
fn artifact_rejects_wrong_expected_kind() {
    let artifact = GerbilCompiledArtifact::LoopGraph(empty_loop_graph());
    let error = artifact
        .ensure_kind(GerbilArtifactKind::WorkspaceSchema)
        .unwrap_err();

    assert_eq!(error.expected, GerbilArtifactKind::WorkspaceSchema);
    assert_eq!(error.actual, GerbilArtifactKind::LoopGraph);
}

#[test]
fn artifact_reports_workspace_patch_intent_kind() {
    let mut patch = WorkspacePatch::new("gerbil intent");
    patch.ops.push(WorkspacePatchOp::MarkMemoryCandidate {
        node: OrgNodeId::new("memory.org:1:goal"),
        dispatch: "long-term".to_owned(),
    });
    let artifact = GerbilCompiledArtifact::WorkspacePatchIntent(WorkspacePatchIntentSpec {
        intent_id: "intent:memory".to_owned(),
        patch,
        dry_run_first: true,
    });

    assert_eq!(artifact.kind(), GerbilArtifactKind::WorkspacePatchIntent);
}

#[test]
fn artifact_reports_agent_scenario_contract_kind() {
    let scenario = AgentScenario::new("gerbil-scenario")
        .with_step(AgentScenarioStep::new("run").expecting_event_topic("kernel.execution"))
        .expecting_evidence(LoopEvidenceKind::Runtime);
    let artifact =
        GerbilCompiledArtifact::AgentScenarioContract(AgentScenarioContract::new(scenario));

    assert_eq!(artifact.kind(), GerbilArtifactKind::AgentScenarioContract);
}

#[test]
fn artifact_reports_release_topology_kind() {
    let artifact = GerbilCompiledArtifact::ReleaseTopology(ReleaseTopologySpec {
        topology_id: "gerbil-scheme-internal-release".to_owned(),
        crate_name: "marlin-gerbil-scheme".to_owned(),
        publish_enabled: false,
        asset_audit_command: "cargo package -p marlin-gerbil-scheme --list --allow-dirty"
            .to_owned(),
        package_assets: vec!["README.md".to_owned()],
        runtime_dependency_chain: vec!["marlin-gerbil-ir".to_owned()],
        workflow_dependency_chain: vec!["marlin-org-workflow".to_owned()],
        gates: Vec::new(),
    });

    assert_eq!(artifact.kind(), GerbilArtifactKind::ReleaseTopology);
    assert!(
        artifact
            .ensure_kind(GerbilArtifactKind::ReleaseTopology)
            .is_ok()
    );
}

#[test]
fn artifact_release_topology_projects_into_workspace_status() {
    let artifact = GerbilCompiledArtifact::ReleaseTopology(ReleaseTopologySpec {
        topology_id: "gerbil-scheme-internal-release".to_owned(),
        crate_name: "marlin-gerbil-scheme".to_owned(),
        publish_enabled: false,
        asset_audit_command: "cargo package -p marlin-gerbil-scheme --list --allow-dirty"
            .to_owned(),
        package_assets: vec!["fixtures/gerbil/build.ss".to_owned()],
        runtime_dependency_chain: vec!["marlin-gerbil-ir".to_owned()],
        workflow_dependency_chain: vec!["marlin-org-workflow".to_owned()],
        gates: vec![ReleaseGateSpec {
            gate_id: "package-assets".to_owned(),
            command: "cargo package -p marlin-gerbil-scheme --list --allow-dirty".to_owned(),
            requires_local_gerbil: false,
            required_artifacts: vec!["fixtures/gerbil/build.ss".to_owned()],
            visibility: vec![ReleaseVisibilitySpec {
                report_key: "package_asset_audit".to_owned(),
                evidence_keys: vec!["required_artifacts".to_owned()],
                artifact_paths: vec!["fixtures/gerbil/build.ss".to_owned()],
            }],
        }],
    });
    let workspace = MemoryOrgWorkspace::new();

    workspace
        .record_release_topology(
            artifact
                .release_topology()
                .expect("release topology artifact"),
        )
        .expect("release topology recorded");
    let report = futures_executor::block_on(
        workspace.status(WorkspaceTarget::Workspace, WorkspaceCtx::new("unit-test")),
    )
    .expect("workspace status");
    let release = report.release.expect("release status");

    assert_eq!(release.topology_id, "gerbil-scheme-internal-release");
    assert_eq!(release.crate_name, "marlin-gerbil-scheme");
    assert!(
        release
            .visibility_reports
            .iter()
            .any(|report| report.report_key == "package_asset_audit"
                && report.artifact_paths == ["fixtures/gerbil/build.ss"])
    );
}

#[test]
fn artifact_release_topology_persists_landing_status_sidecar() {
    let artifact = GerbilCompiledArtifact::ReleaseTopology(ReleaseTopologySpec {
        topology_id: "release:gerbil".to_owned(),
        crate_name: "marlin-gerbil-scheme".to_owned(),
        publish_enabled: false,
        asset_audit_command: "cargo package -p marlin-gerbil-scheme --list --allow-dirty"
            .to_owned(),
        package_assets: vec!["fixtures/gerbil/build.ss".to_owned()],
        runtime_dependency_chain: vec!["marlin-gerbil-ir".to_owned()],
        workflow_dependency_chain: vec!["marlin-org-workflow".to_owned()],
        gates: vec![ReleaseGateSpec {
            gate_id: "package-assets".to_owned(),
            command: "cargo package -p marlin-gerbil-scheme --list --allow-dirty".to_owned(),
            requires_local_gerbil: false,
            required_artifacts: vec!["fixtures/gerbil/build.ss".to_owned()],
            visibility: vec![ReleaseVisibilitySpec {
                report_key: "package_asset_audit".to_owned(),
                evidence_keys: vec!["required_artifacts".to_owned()],
                artifact_paths: vec!["fixtures/gerbil/build.ss".to_owned()],
            }],
        }],
    });
    let root = test_root("release-topology-status");
    let store = FileSystemReleaseStatusStore::new(&root);

    let pending = store
        .record_release_topology(
            artifact
                .release_topology()
                .expect("release topology artifact"),
        )
        .expect("release topology sidecar should be written");
    assert_eq!(pending.topology_id, "release:gerbil");
    assert_eq!(pending.gates[0].state, ReleaseGateState::Pending);
    assert!(store.path().is_file());

    assert!(
        store
            .record_release_gate_receipt(ReleaseGateReceipt::passed(
                "package-assets",
                vec!["required_artifacts".to_owned()],
                vec!["fixtures/gerbil/build.ss".to_owned()],
            ))
            .expect("release gate receipt should update persisted sidecar")
    );

    let reopened = FileSystemReleaseStatusStore::new(&root);
    let status = reopened
        .read_status()
        .expect("release status sidecar should be readable")
        .expect("release status should exist");

    assert_eq!(status.crate_name, "marlin-gerbil-scheme");
    assert_eq!(status.gates[0].state, ReleaseGateState::Passed);
    assert!(status.visibility_reports[0].observed);
    assert_eq!(
        status.gates[0]
            .last_receipt
            .as_ref()
            .expect("release gate receipt")
            .artifact_paths,
        ["fixtures/gerbil/build.ss"]
    );
    let landing = reopened
        .read_landing_report()
        .expect("landing report should be readable")
        .expect("landing report should exist");
    assert!(landing.landing_complete);
    assert_eq!(landing.crate_name, "marlin-gerbil-scheme");
    assert_eq!(landing.passed_gates, 1);
    assert_eq!(landing.observed_visibility_reports, 1);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn artifact_non_release_topology_has_no_release_topology_payload() {
    let artifact = GerbilCompiledArtifact::LoopGraph(empty_loop_graph());

    assert!(artifact.release_topology().is_none());
    assert!(artifact.into_release_topology().is_none());
}

fn test_root(name: &str) -> std::path::PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    std::env::temp_dir().join(format!(
        "marlin-gerbil-scheme-{name}-{}-{suffix}",
        std::process::id()
    ))
}
