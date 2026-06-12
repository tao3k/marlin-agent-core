use std::{
    fs,
    path::{Path, PathBuf},
};

use marlin_agent_harness::{AgentHarness, HarnessRuntime};
use marlin_agent_protocol::{AgentScenario, LoopEvidence, LoopEvidenceKind};
use rust_lang_project_harness::{
    RustDeterminismReadiness, RustDeterminismReadinessInput, RustEvidenceGraph,
    RustEvidenceGraphInput, RustHarnessReport, RustReviewPacketInput,
    build_rust_determinism_readiness, build_rust_evidence_graph, build_rust_review_packet,
    render_rust_determinism_readiness_json, render_rust_evidence_graph_json,
    render_rust_review_packet_json, run_rust_project_harness_with_config,
    rust_harness_config_for_project,
};

#[test]
fn rust_project_harness_policy_produces_agent_visibility_evidence() {
    let tempdir = tempfile::tempdir().expect("tempdir should be created");
    write_fixture_crate(tempdir.path());

    let artifacts = build_rust_project_harness_policy_artifacts(tempdir.path())
        .expect("rust project harness policy artifacts should build");

    assert_eq!(artifacts.project_root, tempdir.path());
    assert!(!artifacts.harness_report.modules.is_empty());
    assert!(!artifacts.determinism_readiness.observations.is_empty());
    assert!(
        artifacts
            .determinism_readiness_json
            .contains("determinism-readiness")
    );
    assert!(artifacts.review_packet_json.contains("review-packet"));
    assert!(artifacts.evidence_graph.summary.nodes > 0);
    assert!(artifacts.evidence_graph_json.contains("evidence-graph"));

    let evidence = rust_project_harness_policy_visibility_evidence(&artifacts);
    assert_eq!(evidence.len(), 4);
    assert!(
        evidence
            .iter()
            .all(|evidence| evidence.kind == LoopEvidenceKind::Visibility)
    );
    assert!(
        evidence
            .iter()
            .any(|evidence| evidence.subject.ends_with(":harness-report"))
    );
    assert!(
        evidence
            .iter()
            .any(|evidence| evidence.subject.ends_with(":determinism-readiness"))
    );
    assert!(
        evidence
            .iter()
            .any(|evidence| evidence.subject.ends_with(":review-packet"))
    );
    assert!(
        evidence
            .iter()
            .any(|evidence| evidence.subject.ends_with(":evidence-graph"))
    );
    assert!(
        evidence.iter().any(|evidence| evidence
            .detail
            .as_deref()
            .is_some_and(|detail| detail.contains("artifact=evidence_graph")
                && detail.contains("node_count=")
                && detail.contains("json_bytes=")))
    );

    let scenario = AgentScenario::new("rust-project-harness-policy")
        .expecting_evidence(LoopEvidenceKind::Visibility);
    let mut runtime = HarnessRuntime::new(8);
    for evidence in rust_project_harness_policy_visibility_evidence(&artifacts) {
        runtime.record_evidence(evidence);
    }

    let report = AgentHarness::evaluate(&scenario, &[], runtime.evidence());
    assert!(report.is_success());
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RustProjectHarnessPolicyArtifacts {
    project_root: PathBuf,
    harness_report: RustHarnessReport,
    determinism_readiness: RustDeterminismReadiness,
    evidence_graph: RustEvidenceGraph,
    determinism_readiness_json: String,
    review_packet_json: String,
    evidence_graph_json: String,
}

fn build_rust_project_harness_policy_artifacts(
    project_root: impl AsRef<Path>,
) -> Result<RustProjectHarnessPolicyArtifacts, String> {
    let project_root = project_root.as_ref().to_path_buf();
    let config = rust_harness_config_for_project(&project_root);
    let harness_report = run_rust_project_harness_with_config(&project_root, &config)?;
    let determinism_readiness = build_rust_determinism_readiness(RustDeterminismReadinessInput {
        project_root: project_root.clone(),
        include_tests: config.include_tests,
    })?;
    let review_packet = build_rust_review_packet(RustReviewPacketInput {
        project_root: project_root.clone(),
        report: harness_report.clone(),
        receipts: Vec::new(),
        behavior_snapshots: Vec::new(),
        determinism_readiness: vec![determinism_readiness.clone()],
        proof_pilots: Vec::new(),
        waivers: Vec::new(),
    });
    let evidence_graph = build_rust_evidence_graph(RustEvidenceGraphInput {
        project_root: project_root.clone(),
        review_packets: vec![review_packet.clone()],
    });
    let determinism_readiness_json = render_rust_determinism_readiness_json(&determinism_readiness)
        .map_err(|error| error.to_string())?;
    let review_packet_json = render_rust_review_packet_json(&review_packet)?;
    let evidence_graph_json =
        render_rust_evidence_graph_json(&evidence_graph).map_err(|error| error.to_string())?;

    Ok(RustProjectHarnessPolicyArtifacts {
        project_root,
        harness_report,
        determinism_readiness,
        evidence_graph,
        determinism_readiness_json,
        review_packet_json,
        evidence_graph_json,
    })
}

fn rust_project_harness_policy_visibility_evidence(
    artifacts: &RustProjectHarnessPolicyArtifacts,
) -> Vec<LoopEvidence> {
    vec![
        rust_project_harness_policy_artifact_evidence(
            artifacts,
            "harness-report",
            format!(
                "project_root={} artifact=harness_report module_count={}",
                artifacts.project_root.display(),
                artifacts.harness_report.modules.len()
            ),
        ),
        rust_project_harness_policy_artifact_evidence(
            artifacts,
            "determinism-readiness",
            format!(
                "project_root={} artifact=determinism_readiness observation_count={} json_bytes={}",
                artifacts.project_root.display(),
                artifacts.determinism_readiness.observations.len(),
                artifacts.determinism_readiness_json.len()
            ),
        ),
        rust_project_harness_policy_artifact_evidence(
            artifacts,
            "review-packet",
            format!(
                "project_root={} artifact=review_packet json_bytes={}",
                artifacts.project_root.display(),
                artifacts.review_packet_json.len()
            ),
        ),
        rust_project_harness_policy_artifact_evidence(
            artifacts,
            "evidence-graph",
            format!(
                "project_root={} artifact=evidence_graph node_count={} json_bytes={}",
                artifacts.project_root.display(),
                artifacts.evidence_graph.summary.nodes,
                artifacts.evidence_graph_json.len()
            ),
        ),
    ]
}

fn rust_project_harness_policy_artifact_evidence(
    artifacts: &RustProjectHarnessPolicyArtifacts,
    artifact: &str,
    detail: String,
) -> LoopEvidence {
    LoopEvidence::present(
        LoopEvidenceKind::Visibility,
        format!(
            "rust-project-harness-policy:{}:{}",
            artifacts.project_root.display(),
            artifact
        ),
    )
    .with_detail(detail)
}

fn write_fixture_crate(root: &Path) {
    fs::create_dir_all(root.join("src")).expect("fixture src dir should be created");
    fs::write(
        root.join("Cargo.toml"),
        r#"[package]
name = "fixture"
version = "0.1.0"
edition = "2024"
"#,
    )
    .expect("fixture manifest should be written");
    fs::write(
        root.join("src/lib.rs"),
        r#"pub fn nondeterministic_timestamp() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos()
}
"#,
    )
    .expect("fixture lib should be written");
}
