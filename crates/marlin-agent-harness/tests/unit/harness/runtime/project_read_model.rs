use marlin_agent_harness::{AgentHarness, AgentHarnessEvidenceKind};
use marlin_agent_test_support::project_runtime_read_model_replay_artifact_fixture;

#[test]
fn harness_accepts_project_runtime_read_model_replay_artifact() {
    let artifact = project_runtime_read_model_replay_artifact_fixture();
    let report =
        AgentHarness::evaluate_contract(artifact.contract(), &[], artifact.replay_evidence());

    assert!(report.is_success(), "{:?}", report.diagnostics);
    assert_eq!(report.scenario_id, "project-runtime-read-model-replay");
    assert_eq!(
        report
            .evidence
            .iter()
            .filter(|evidence| evidence.kind == AgentHarnessEvidenceKind::Visibility)
            .count(),
        2
    );
    assert_eq!(
        report
            .evidence
            .iter()
            .filter(|evidence| evidence.kind == AgentHarnessEvidenceKind::Runtime)
            .count(),
        2
    );
    assert_eq!(
        report
            .evidence
            .iter()
            .filter(|evidence| evidence.kind == AgentHarnessEvidenceKind::Tool)
            .count(),
        1
    );
    assert_eq!(
        report
            .evidence
            .iter()
            .filter(|evidence| evidence.kind == AgentHarnessEvidenceKind::Content)
            .count(),
        1
    );
    assert!(detail_contains(
        &report.evidence,
        "families=[Memory,Tool,Session,Content]"
    ));
    assert!(detail_contains(&report.evidence, "live_llm=false"));
    assert!(detail_contains(&report.evidence, "sandbox_execution=false"));
    assert!(detail_contains(
        &report.evidence,
        "unsupported_org_fixture=true"
    ));
}

fn detail_contains(evidence: &[marlin_agent_harness::AgentHarnessEvidence], needle: &str) -> bool {
    evidence
        .iter()
        .filter_map(|evidence| evidence.detail.as_deref())
        .any(|detail| detail.contains(needle))
}
