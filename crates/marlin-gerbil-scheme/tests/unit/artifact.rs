use marlin_agent_protocol::{
    AgentScenario, AgentScenarioContract, AgentScenarioStep, LoopEvidenceKind,
};
use marlin_gerbil_ir::{CompiledLoopGraph, WorkspacePatchIntentSpec};
use marlin_gerbil_scheme::{GerbilArtifactKind, GerbilCompiledArtifact};
use marlin_org_model::OrgNodeId;
use marlin_workspace_patch::{WorkspacePatch, WorkspacePatchOp};

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
