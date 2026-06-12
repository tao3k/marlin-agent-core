//! Writes `rust-lang-project-harness` evidence artifacts from Cargo build scripts.

use std::{
    env, fs,
    path::{Path, PathBuf},
};

use rust_lang_project_harness::{
    RustDeterminismReadiness, RustDeterminismReadinessInput, RustEvidenceGraph,
    RustEvidenceGraphInput, RustEvidenceGraphSummary, RustHarnessConfig, RustHarnessReport,
    RustReviewPacket, RustReviewPacketInput, RustVerificationPerformanceIndex,
    RustVerificationPlan, RustVerificationStabilityIndex, RustVerificationStabilityPicture,
    RustVerificationTaskIndex, RustVerificationTaskKind, build_rust_determinism_readiness,
    build_rust_evidence_graph, build_rust_review_packet, build_rust_verification_performance_index,
    build_rust_verification_stability_index, build_rust_verification_stability_picture_from_index,
    build_rust_verification_task_index, plan_rust_project_verification_with_config,
    render_rust_determinism_readiness_json, render_rust_evidence_graph_json,
    render_rust_review_packet_json, render_rust_verification_performance_index_json,
    render_rust_verification_plan_json, render_rust_verification_stability_index_json,
    render_rust_verification_stability_picture_json, render_rust_verification_task_index_json,
};

/// Receipt for `rust-lang-project-harness` evidence emitted from `build.rs`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustProjectHarnessEvidenceReceipt {
    /// Directory under `OUT_DIR` containing all emitted evidence files.
    pub evidence_dir: PathBuf,
    /// Path to the determinism-readiness JSON artifact.
    pub determinism_readiness_path: PathBuf,
    /// Path to the review-packet JSON artifact.
    pub review_packet_path: PathBuf,
    /// Path to the evidence-graph JSON artifact.
    pub evidence_graph_path: PathBuf,
    /// Path to the verification-plan JSON artifact.
    pub verification_plan_path: PathBuf,
    /// Path to the configured verification task-index JSON artifact.
    pub task_index_path: PathBuf,
    /// Path to the performance-index JSON artifact.
    pub performance_index_path: PathBuf,
    /// Path to the stability-index JSON artifact.
    pub stability_index_path: PathBuf,
    /// Path to the stability-picture JSON artifact, when configured.
    pub stability_picture_path: Option<PathBuf>,
    /// Path to the compact text summary artifact.
    pub summary_path: PathBuf,
    /// Summary copied from the emitted evidence graph.
    pub evidence_graph_summary: RustEvidenceGraphSummary,
    /// Package-level performance and stability gate receipt.
    pub gate_receipt: RustProjectHarnessGateReceipt,
}

/// Structured package-level gate receipt for required Rust harness evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustProjectHarnessGateReceipt {
    /// Cargo package name evaluated by the build gate.
    pub package_name: String,
    /// Whether an active performance verification task is configured.
    pub performance_gate: bool,
    /// Whether an active stability verification task is configured.
    pub stability_gate: bool,
    /// Whether the performance report obligation is configured.
    pub performance_report_obligation: bool,
    /// Whether the stability report obligation is configured.
    pub stability_report_obligation: bool,
}

impl RustProjectHarnessGateReceipt {
    /// Returns true when all required package-level quality gates are present.
    pub fn is_success(&self) -> bool {
        self.performance_gate
            && self.stability_gate
            && self.performance_report_obligation
            && self.stability_report_obligation
    }
}

/// Writes `rust-lang-project-harness` evidence artifacts using Cargo build env vars.
pub fn write_evidence_graph_from_env(
    config: &RustHarnessConfig,
    harness_report: RustHarnessReport,
) -> RustProjectHarnessEvidenceReceipt {
    let build_env = HarnessEvidenceBuildEnv::from_process();
    let artifacts = build_evidence_artifacts(config, harness_report, &build_env.project_root);
    let paths = HarnessEvidencePaths::create(&build_env.out_dir);

    let gate_receipt = assert_performance_and_stability_gate(
        &artifacts.verification_plan,
        &build_env.package_name,
    );
    write_evidence_artifacts(&paths, &build_env, &artifacts);

    paths.into_receipt(artifacts.evidence_graph_summary, gate_receipt)
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct HarnessEvidenceBuildEnv {
    project_root: PathBuf,
    out_dir: PathBuf,
    package_name: String,
}

impl HarnessEvidenceBuildEnv {
    fn from_process() -> Self {
        Self {
            project_root: cargo_path_env("CARGO_MANIFEST_DIR"),
            out_dir: cargo_path_env("OUT_DIR"),
            package_name: cargo_string_env("CARGO_PKG_NAME"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct HarnessEvidenceArtifacts {
    determinism_readiness: RustDeterminismReadiness,
    review_packet: RustReviewPacket,
    evidence_graph: RustEvidenceGraph,
    verification_plan: RustVerificationPlan,
    task_index: RustVerificationTaskIndex,
    performance_index: RustVerificationPerformanceIndex,
    stability_index: RustVerificationStabilityIndex,
    stability_picture: Option<RustVerificationStabilityPicture>,
    evidence_graph_summary: RustEvidenceGraphSummary,
    module_count: usize,
    determinism_observation_count: usize,
    active_verification_task_count: usize,
}

fn build_evidence_artifacts(
    config: &RustHarnessConfig,
    harness_report: RustHarnessReport,
    project_root: &Path,
) -> HarnessEvidenceArtifacts {
    let module_count = harness_report.modules.len();
    let determinism_readiness = build_determinism_readiness(config, project_root);
    let determinism_observation_count = determinism_readiness.observations.len();
    let review_packet = build_review_packet(project_root, harness_report, &determinism_readiness);
    let evidence_graph = build_evidence_graph(project_root, &review_packet);
    let evidence_graph_summary = evidence_graph.summary;
    let verification_plan = plan_rust_project_verification_with_config(project_root, config)
        .unwrap_or_else(|error| panic!("rust verification plan failed: {error}"));
    let active_verification_task_count = verification_plan.active_tasks().len();
    let task_index = build_rust_verification_task_index(&verification_plan);
    let performance_index = build_rust_verification_performance_index(&verification_plan);
    let stability_index = build_rust_verification_stability_index(&verification_plan);
    let stability_picture =
        config
            .verification_policy
            .stability_picture
            .as_ref()
            .map(|picture_config| {
                build_rust_verification_stability_picture_from_index(
                    &stability_index,
                    picture_config,
                )
            });

    HarnessEvidenceArtifacts {
        determinism_readiness,
        review_packet,
        evidence_graph,
        verification_plan,
        task_index,
        performance_index,
        stability_index,
        stability_picture,
        evidence_graph_summary,
        module_count,
        determinism_observation_count,
        active_verification_task_count,
    }
}

fn build_determinism_readiness(
    config: &RustHarnessConfig,
    project_root: &Path,
) -> RustDeterminismReadiness {
    build_rust_determinism_readiness(RustDeterminismReadinessInput {
        project_root: project_root.to_path_buf(),
        include_tests: config.include_tests,
    })
    .unwrap_or_else(|error| panic!("rust determinism readiness failed: {error}"))
}

fn build_review_packet(
    project_root: &Path,
    harness_report: RustHarnessReport,
    determinism_readiness: &RustDeterminismReadiness,
) -> RustReviewPacket {
    build_rust_review_packet(RustReviewPacketInput {
        project_root: project_root.to_path_buf(),
        report: harness_report,
        receipts: Vec::new(),
        behavior_snapshots: Vec::new(),
        determinism_readiness: vec![determinism_readiness.clone()],
        proof_pilots: Vec::new(),
        waivers: Vec::new(),
    })
}

fn build_evidence_graph(
    project_root: &Path,
    review_packet: &RustReviewPacket,
) -> RustEvidenceGraph {
    build_rust_evidence_graph(RustEvidenceGraphInput {
        project_root: project_root.to_path_buf(),
        review_packets: vec![review_packet.clone()],
    })
}

/// Evaluates required package-level performance and stability gates.
pub fn evaluate_performance_and_stability_gate(
    plan: &RustVerificationPlan,
    package_name: impl Into<String>,
) -> RustProjectHarnessGateReceipt {
    let package_name = package_name.into();
    RustProjectHarnessGateReceipt {
        package_name,
        performance_gate: has_active_verification_task(plan, RustVerificationTaskKind::Performance),
        stability_gate: has_active_verification_task(plan, RustVerificationTaskKind::Stability),
        performance_report_obligation: has_report_obligation(plan, "performance_index_json"),
        stability_report_obligation: has_report_obligation(plan, "stability_index_json"),
    }
}

fn assert_performance_and_stability_gate(
    plan: &RustVerificationPlan,
    package_name: &str,
) -> RustProjectHarnessGateReceipt {
    let receipt = evaluate_performance_and_stability_gate(plan, package_name.to_owned());
    assert_verification_gate(receipt.performance_gate, package_name, "performance");
    assert_verification_report_obligation(
        receipt.performance_report_obligation,
        package_name,
        "performance_index_json",
    );
    assert_verification_gate(receipt.stability_gate, package_name, "stability");
    assert_verification_report_obligation(
        receipt.stability_report_obligation,
        package_name,
        "stability_index_json",
    );
    receipt
}

fn has_active_verification_task(
    plan: &RustVerificationPlan,
    task_kind: RustVerificationTaskKind,
) -> bool {
    plan.tasks
        .iter()
        .any(|task| task.kind == task_kind && task.is_active())
}

fn has_report_obligation(plan: &RustVerificationPlan, report_obligation_key: &str) -> bool {
    plan.report_obligations
        .iter()
        .any(|obligation| obligation.key == report_obligation_key)
}

fn assert_verification_gate(gate_present: bool, package_name: &str, task_label: &str) {
    assert!(
        gate_present,
        "{package_name} build gate must configure active {task_label} verification tasks",
    );
}

fn assert_verification_report_obligation(
    obligation_present: bool,
    package_name: &str,
    report_obligation_key: &str,
) {
    assert!(
        obligation_present,
        "{package_name} build gate must require {report_obligation_key}",
    );
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct HarnessEvidencePaths {
    evidence_dir: PathBuf,
    determinism_readiness_path: PathBuf,
    review_packet_path: PathBuf,
    evidence_graph_path: PathBuf,
    verification_plan_path: PathBuf,
    task_index_path: PathBuf,
    performance_index_path: PathBuf,
    stability_index_path: PathBuf,
    stability_picture_path: PathBuf,
    summary_path: PathBuf,
}

impl HarnessEvidencePaths {
    fn create(out_dir: &Path) -> Self {
        let evidence_dir = out_dir.join("marlin-rust-project-harness");
        fs::create_dir_all(&evidence_dir).unwrap_or_else(|error| {
            panic!(
                "failed to create rust project harness evidence dir {}: {error}",
                evidence_dir.display()
            )
        });

        Self {
            determinism_readiness_path: evidence_dir.join("determinism-readiness.json"),
            review_packet_path: evidence_dir.join("review-packet.json"),
            evidence_graph_path: evidence_dir.join("evidence-graph.json"),
            verification_plan_path: evidence_dir.join("verification_plan.json"),
            task_index_path: evidence_dir.join("task_index.json"),
            performance_index_path: evidence_dir.join("performance_index.json"),
            stability_index_path: evidence_dir.join("stability_index.json"),
            stability_picture_path: evidence_dir.join("stability_picture.json"),
            summary_path: evidence_dir.join("summary.txt"),
            evidence_dir,
        }
    }

    fn into_receipt(
        self,
        evidence_graph_summary: RustEvidenceGraphSummary,
        gate_receipt: RustProjectHarnessGateReceipt,
    ) -> RustProjectHarnessEvidenceReceipt {
        let stability_picture_path = self
            .stability_picture_path
            .is_file()
            .then_some(self.stability_picture_path);

        RustProjectHarnessEvidenceReceipt {
            evidence_dir: self.evidence_dir,
            determinism_readiness_path: self.determinism_readiness_path,
            review_packet_path: self.review_packet_path,
            evidence_graph_path: self.evidence_graph_path,
            verification_plan_path: self.verification_plan_path,
            task_index_path: self.task_index_path,
            performance_index_path: self.performance_index_path,
            stability_index_path: self.stability_index_path,
            stability_picture_path,
            summary_path: self.summary_path,
            evidence_graph_summary,
            gate_receipt,
        }
    }
}

fn write_evidence_artifacts(
    paths: &HarnessEvidencePaths,
    build_env: &HarnessEvidenceBuildEnv,
    artifacts: &HarnessEvidenceArtifacts,
) {
    write_artifact(
        &paths.determinism_readiness_path,
        render_rust_determinism_readiness_json(&artifacts.determinism_readiness)
            .unwrap_or_else(|error| panic!("failed to render determinism readiness: {error}")),
    );
    write_artifact(
        &paths.review_packet_path,
        render_rust_review_packet_json(&artifacts.review_packet)
            .unwrap_or_else(|error| panic!("failed to render review packet: {error}")),
    );
    write_artifact(
        &paths.evidence_graph_path,
        render_rust_evidence_graph_json(&artifacts.evidence_graph)
            .unwrap_or_else(|error| panic!("failed to render evidence graph: {error}")),
    );
    write_artifact(
        &paths.verification_plan_path,
        render_rust_verification_plan_json(&artifacts.verification_plan)
            .unwrap_or_else(|error| panic!("failed to render verification plan: {error}")),
    );
    write_artifact(
        &paths.task_index_path,
        render_rust_verification_task_index_json(&artifacts.task_index)
            .unwrap_or_else(|error| panic!("failed to render verification task index: {error}")),
    );
    write_artifact(
        &paths.performance_index_path,
        render_rust_verification_performance_index_json(&artifacts.performance_index)
            .unwrap_or_else(|error| panic!("failed to render performance index: {error}")),
    );
    write_artifact(
        &paths.stability_index_path,
        render_rust_verification_stability_index_json(&artifacts.stability_index)
            .unwrap_or_else(|error| panic!("failed to render stability index: {error}")),
    );
    if let Some(stability_picture) = &artifacts.stability_picture {
        write_artifact(
            &paths.stability_picture_path,
            render_rust_verification_stability_picture_json(stability_picture)
                .unwrap_or_else(|error| panic!("failed to render stability picture: {error}")),
        );
    }
    write_artifact(&paths.summary_path, render_summary(build_env, artifacts));
}

fn render_summary(
    build_env: &HarnessEvidenceBuildEnv,
    artifacts: &HarnessEvidenceArtifacts,
) -> String {
    format!(
        "package={}\nmodules={}\ndeterminism_observations={}\nevidence_graph_nodes={}\nverification_tasks={}\nactive_verification_tasks={}\nreport_obligations={}\ntask_index_records={}\nperformance_records={}\nstability_records={}\nstability_picture_records={}\n",
        build_env.package_name,
        artifacts.module_count,
        artifacts.determinism_observation_count,
        artifacts.evidence_graph_summary.nodes,
        artifacts.verification_plan.tasks.len(),
        artifacts.active_verification_task_count,
        artifacts.verification_plan.report_obligations.len(),
        artifacts.task_index.records.len(),
        artifacts.performance_index.records.len(),
        artifacts.stability_index.records.len(),
        artifacts
            .stability_picture
            .as_ref()
            .map_or(0, |picture| picture.records.len())
    )
}

fn write_artifact(path: &Path, contents: String) {
    fs::write(path, contents).unwrap_or_else(|error| {
        panic!(
            "failed to write harness artifact {}: {error}",
            path.display()
        )
    });
}

fn cargo_path_env(key: &str) -> PathBuf {
    PathBuf::from(cargo_string_env(key))
}

fn cargo_string_env(key: &str) -> String {
    env::var(key).unwrap_or_else(|_| panic!("{key} should be set for build.rs"))
}
