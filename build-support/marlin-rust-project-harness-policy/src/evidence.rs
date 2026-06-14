//! Writes `rust-lang-project-harness` evidence artifacts from Cargo build scripts.

use std::{
    env, fs,
    path::{Path, PathBuf},
};

use serde::Serialize;

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

use crate::gerbil_runtime_assets::{
    GerbilRuntimeAssetManifestReceipt, GerbilRuntimeAssetManifestStatus,
    inspect_gerbil_runtime_assets,
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
    /// Path to the agent-actionable quality findings JSON artifact.
    pub quality_findings_path: PathBuf,
    /// Path to the Gerbil runtime asset manifest receipt JSON artifact.
    pub gerbil_runtime_assets_path: PathBuf,
    /// Path to the compact text summary artifact.
    pub summary_path: PathBuf,
    /// Summary copied from the emitted evidence graph.
    pub evidence_graph_summary: RustEvidenceGraphSummary,
    /// Package-level performance and stability gate receipt.
    pub gate_receipt: RustProjectHarnessGateReceipt,
    /// Agent-actionable quality findings derived from Marlin's package gate.
    pub quality_findings_receipt: RustProjectHarnessQualityFindingsReceipt,
    /// Gerbil runtime asset manifest observed at build time.
    pub gerbil_runtime_assets_receipt: GerbilRuntimeAssetManifestReceipt,
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

/// Severity taxonomy for agent-actionable quality findings.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustProjectHarnessFindingSeverity {
    HardError,
    Warning,
    Advice,
}

/// One structured finding that an agent can reason over without parsing prose.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RustProjectHarnessQualityFinding {
    pub finding_id: String,
    pub severity: RustProjectHarnessFindingSeverity,
    pub rule_id: String,
    pub owner: String,
    pub evidence: Vec<String>,
    pub why: String,
    pub agent_next_action: String,
    pub verification_command: String,
    pub source_authority: String,
}

/// JSON receipt containing hard errors, warnings, and advice for the package gate.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RustProjectHarnessQualityFindingsReceipt {
    pub schema_id: String,
    pub schema_version: String,
    pub package_name: String,
    pub findings: Vec<RustProjectHarnessQualityFinding>,
}

/// Named input for evaluating package-level gate state into structured findings.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustProjectHarnessQualityFindingsInput {
    pub package_name: String,
    pub gate_receipt: RustProjectHarnessGateReceipt,
    pub evidence_paths: RustProjectHarnessQualityFindingEvidencePaths,
}

/// Artifact paths exposed to agents as structured evidence handles.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustProjectHarnessQualityFindingEvidencePaths {
    evidence_graph_path: PathBuf,
    verification_plan_path: PathBuf,
    task_index_path: PathBuf,
}

impl RustProjectHarnessQualityFindingEvidencePaths {
    pub fn new(
        evidence_graph_path: impl Into<PathBuf>,
        verification_plan_path: impl Into<PathBuf>,
        task_index_path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            evidence_graph_path: evidence_graph_path.into(),
            verification_plan_path: verification_plan_path.into(),
            task_index_path: task_index_path.into(),
        }
    }

    pub fn agent_evidence(&self) -> Vec<String> {
        vec![
            self.evidence_graph_path.display().to_string(),
            self.verification_plan_path.display().to_string(),
            self.task_index_path.display().to_string(),
        ]
    }
}

impl RustProjectHarnessQualityFindingsReceipt {
    pub fn hard_error_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|finding| finding.severity == RustProjectHarnessFindingSeverity::HardError)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|finding| finding.severity == RustProjectHarnessFindingSeverity::Warning)
            .count()
    }

    pub fn advice_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|finding| finding.severity == RustProjectHarnessFindingSeverity::Advice)
            .count()
    }
}

/// Evaluates package-level gate state into structured findings for agent repair.
pub fn evaluate_quality_findings_for_gate(
    input: RustProjectHarnessQualityFindingsInput,
) -> RustProjectHarnessQualityFindingsReceipt {
    let RustProjectHarnessQualityFindingsInput {
        package_name,
        gate_receipt,
        evidence_paths,
    } = input;
    let mut findings = Vec::new();

    push_gate_finding(
        &mut findings,
        &package_name,
        gate_receipt.performance_gate,
        "MARLIN-QUALITY-GATE-PERF",
        "performance-gate",
        "active performance verification task",
        "add or enable a performance verification task for this package policy",
    );
    push_gate_finding(
        &mut findings,
        &package_name,
        gate_receipt.stability_gate,
        "MARLIN-QUALITY-GATE-STABILITY",
        "stability-gate",
        "active stability verification task",
        "add or enable a stability verification task for this package policy",
    );
    push_gate_finding(
        &mut findings,
        &package_name,
        gate_receipt.performance_report_obligation,
        "MARLIN-QUALITY-REPORT-PERF",
        "performance-report-obligation",
        "performance_index_json report obligation",
        "add performance_index_json to the package verification report obligations",
    );
    push_gate_finding(
        &mut findings,
        &package_name,
        gate_receipt.stability_report_obligation,
        "MARLIN-QUALITY-REPORT-STABILITY",
        "stability-report-obligation",
        "stability_index_json report obligation",
        "add stability_index_json to the package verification report obligations",
    );

    findings.push(RustProjectHarnessQualityFinding {
        finding_id: format!("{package_name}:agent-read-evidence"),
        severity: RustProjectHarnessFindingSeverity::Advice,
        rule_id: "MARLIN-QUALITY-AGENT-EVIDENCE".to_owned(),
        owner: package_name.clone(),
        evidence: evidence_paths.agent_evidence(),
        why: "agent repair should reason from structured evidence before editing package code"
            .to_owned(),
        agent_next_action:
            "read evidence-graph.json, verification_plan.json, and task_index.json before selecting an edit boundary"
                .to_owned(),
        verification_command: "cargo test --workspace --no-fail-fast --quiet".to_owned(),
        source_authority: "marlin-rust-project-harness-policy".to_owned(),
    });

    RustProjectHarnessQualityFindingsReceipt {
        schema_id: "marlin.rust-project-harness.quality-findings".to_owned(),
        schema_version: "1".to_owned(),
        package_name,
        findings,
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
    let gerbil_runtime_assets_receipt = inspect_gerbil_runtime_assets(&build_env.project_root);

    let gate_receipt = evaluate_performance_and_stability_gate(
        &artifacts.verification_plan,
        &build_env.package_name,
    );
    let quality_findings_receipt = build_quality_findings_receipt(
        &paths,
        &build_env,
        &artifacts,
        &gate_receipt,
        &gerbil_runtime_assets_receipt,
    );
    write_evidence_artifacts(
        &paths,
        &build_env,
        &artifacts,
        &quality_findings_receipt,
        &gerbil_runtime_assets_receipt,
    );
    assert_gerbil_runtime_asset_manifest_receipt(&gerbil_runtime_assets_receipt);
    assert_performance_and_stability_gate_receipt(&gate_receipt);

    paths.into_receipt(
        artifacts.evidence_graph_summary,
        gate_receipt,
        quality_findings_receipt,
        gerbil_runtime_assets_receipt,
    )
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

fn assert_performance_and_stability_gate_receipt(receipt: &RustProjectHarnessGateReceipt) {
    let package_name = receipt.package_name.as_str();
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
    quality_findings_path: PathBuf,
    gerbil_runtime_assets_path: PathBuf,
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
            quality_findings_path: evidence_dir.join("quality_findings.json"),
            gerbil_runtime_assets_path: evidence_dir.join("gerbil_runtime_assets.json"),
            summary_path: evidence_dir.join("summary.txt"),
            evidence_dir,
        }
    }

    fn into_receipt(
        self,
        evidence_graph_summary: RustEvidenceGraphSummary,
        gate_receipt: RustProjectHarnessGateReceipt,
        quality_findings_receipt: RustProjectHarnessQualityFindingsReceipt,
        gerbil_runtime_assets_receipt: GerbilRuntimeAssetManifestReceipt,
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
            quality_findings_path: self.quality_findings_path,
            gerbil_runtime_assets_path: self.gerbil_runtime_assets_path,
            summary_path: self.summary_path,
            evidence_graph_summary,
            gate_receipt,
            quality_findings_receipt,
            gerbil_runtime_assets_receipt,
        }
    }
}

fn write_evidence_artifacts(
    paths: &HarnessEvidencePaths,
    build_env: &HarnessEvidenceBuildEnv,
    artifacts: &HarnessEvidenceArtifacts,
    quality_findings_receipt: &RustProjectHarnessQualityFindingsReceipt,
    gerbil_runtime_assets_receipt: &GerbilRuntimeAssetManifestReceipt,
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
    write_artifact(
        &paths.quality_findings_path,
        serde_json::to_string_pretty(quality_findings_receipt)
            .unwrap_or_else(|error| panic!("failed to render quality findings receipt: {error}")),
    );
    write_artifact(
        &paths.gerbil_runtime_assets_path,
        serde_json::to_string_pretty(gerbil_runtime_assets_receipt).unwrap_or_else(|error| {
            panic!("failed to render Gerbil runtime asset receipt: {error}")
        }),
    );
    write_artifact(
        &paths.summary_path,
        render_summary(
            build_env,
            artifacts,
            quality_findings_receipt,
            gerbil_runtime_assets_receipt,
        ),
    );
}

fn build_quality_findings_receipt(
    paths: &HarnessEvidencePaths,
    build_env: &HarnessEvidenceBuildEnv,
    artifacts: &HarnessEvidenceArtifacts,
    gate_receipt: &RustProjectHarnessGateReceipt,
    gerbil_runtime_assets_receipt: &GerbilRuntimeAssetManifestReceipt,
) -> RustProjectHarnessQualityFindingsReceipt {
    let package_name = build_env.package_name.as_str();
    let mut receipt = evaluate_quality_findings_for_gate(RustProjectHarnessQualityFindingsInput {
        package_name: package_name.to_owned(),
        gate_receipt: gate_receipt.clone(),
        evidence_paths: RustProjectHarnessQualityFindingEvidencePaths::new(
            paths.evidence_graph_path.clone(),
            paths.verification_plan_path.clone(),
            paths.task_index_path.clone(),
        ),
    });

    append_quality_findings_for_artifacts(&mut receipt, package_name, artifacts, Some(paths));
    append_quality_findings_for_gerbil_runtime_assets(
        &mut receipt,
        package_name,
        gerbil_runtime_assets_receipt,
        &paths.gerbil_runtime_assets_path,
    );

    receipt
}

fn append_quality_findings_for_artifacts(
    receipt: &mut RustProjectHarnessQualityFindingsReceipt,
    package_name: &str,
    artifacts: &HarnessEvidenceArtifacts,
    paths: Option<&HarnessEvidencePaths>,
) {
    if artifacts.evidence_graph_summary.nodes == 0 {
        let evidence = paths
            .map(|paths| paths.evidence_graph_path.display().to_string())
            .unwrap_or_else(|| "evidence-graph".to_owned());
        receipt.findings.push(RustProjectHarnessQualityFinding {
            finding_id: format!("{package_name}:evidence-graph-empty"),
            severity: RustProjectHarnessFindingSeverity::Warning,
            rule_id: "MARLIN-QUALITY-EVIDENCE-GRAPH".to_owned(),
            owner: package_name.to_owned(),
            evidence: vec![evidence],
            why: "the emitted evidence graph has no nodes for the agent to inspect".to_owned(),
            agent_next_action:
                "inspect upstream rust-harness graph inputs before editing Marlin policy"
                    .to_owned(),
            verification_command: "cargo test -p marlin-rust-project-harness-policy --quiet"
                .to_owned(),
            source_authority: "marlin-rust-project-harness-policy".to_owned(),
        });
    }
    if artifacts.determinism_observation_count == 0 {
        let evidence = paths
            .map(|paths| paths.determinism_readiness_path.display().to_string())
            .unwrap_or_else(|| "determinism-readiness".to_owned());
        receipt.findings.push(RustProjectHarnessQualityFinding {
            finding_id: format!("{package_name}:determinism-observations-empty"),
            severity: RustProjectHarnessFindingSeverity::Warning,
            rule_id: "MARLIN-QUALITY-DETERMINISM".to_owned(),
            owner: package_name.to_owned(),
            evidence: vec![evidence],
            why: "the determinism readiness packet contains no observations".to_owned(),
            agent_next_action:
                "inspect language harness determinism inputs and package ownership boundaries"
                    .to_owned(),
            verification_command: "cargo test -p marlin-rust-project-harness-policy --quiet"
                .to_owned(),
            source_authority: "marlin-rust-project-harness-policy".to_owned(),
        });
    }
}

fn append_quality_findings_for_gerbil_runtime_assets(
    receipt: &mut RustProjectHarnessQualityFindingsReceipt,
    package_name: &str,
    gerbil_runtime_assets_receipt: &GerbilRuntimeAssetManifestReceipt,
    gerbil_runtime_assets_path: &Path,
) {
    if !matches!(
        gerbil_runtime_assets_receipt.status,
        GerbilRuntimeAssetManifestStatus::MissingRequiredAssets
    ) {
        return;
    }

    receipt.findings.push(RustProjectHarnessQualityFinding {
        finding_id: format!("{package_name}:gerbil-runtime-assets-missing"),
        severity: RustProjectHarnessFindingSeverity::HardError,
        rule_id: "MARLIN-GERBIL-RUNTIME-ASSETS".to_owned(),
        owner: package_name.to_owned(),
        evidence: vec![gerbil_runtime_assets_path.display().to_string()],
        why: format!(
            "the crate owns a gerbil/ runtime root but misses required assets: {}",
            gerbil_runtime_assets_receipt
                .missing_required_assets
                .join(", ")
        ),
        agent_next_action:
            "restore the missing Gerbil runtime files before relying on generated loadpath assets"
                .to_owned(),
        verification_command: "cargo test -p marlin-rust-project-harness-policy --quiet".to_owned(),
        source_authority: "marlin-rust-project-harness-policy".to_owned(),
    });
}

fn assert_gerbil_runtime_asset_manifest_receipt(receipt: &GerbilRuntimeAssetManifestReceipt) {
    if receipt.is_success() {
        return;
    }

    panic!(
        "Gerbil runtime asset manifest gate failed for {}: missing {}",
        receipt.project_root.display(),
        receipt.missing_required_assets.join(", ")
    );
}

fn push_gate_finding(
    findings: &mut Vec<RustProjectHarnessQualityFinding>,
    package_name: &str,
    gate_present: bool,
    rule_id: &str,
    finding_suffix: &str,
    evidence_label: &str,
    agent_next_action: &str,
) {
    if gate_present {
        return;
    }

    findings.push(RustProjectHarnessQualityFinding {
        finding_id: format!("{package_name}:{finding_suffix}"),
        severity: RustProjectHarnessFindingSeverity::HardError,
        rule_id: rule_id.to_owned(),
        owner: package_name.to_owned(),
        evidence: vec![evidence_label.to_owned()],
        why: format!("package quality gate is missing {evidence_label}"),
        agent_next_action: agent_next_action.to_owned(),
        verification_command: "cargo test -p marlin-rust-project-harness-policy --quiet".to_owned(),
        source_authority: "marlin-rust-project-harness-policy".to_owned(),
    });
}

fn render_summary(
    build_env: &HarnessEvidenceBuildEnv,
    artifacts: &HarnessEvidenceArtifacts,
    quality_findings_receipt: &RustProjectHarnessQualityFindingsReceipt,
    gerbil_runtime_assets_receipt: &GerbilRuntimeAssetManifestReceipt,
) -> String {
    format!(
        "package={}\nmodules={}\ndeterminism_observations={}\nevidence_graph_nodes={}\nverification_tasks={}\nactive_verification_tasks={}\nreport_obligations={}\ntask_index_records={}\nperformance_records={}\nstability_records={}\nstability_picture_records={}\ngerbil_runtime_asset_status={:?}\ngerbil_runtime_assets={}\ngerbil_runtime_missing_assets={}\nquality_findings={}\nquality_hard_errors={}\nquality_warnings={}\nquality_advice={}\n",
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
            .map_or(0, |picture| picture.records.len()),
        gerbil_runtime_assets_receipt.status,
        gerbil_runtime_assets_receipt.asset_count,
        gerbil_runtime_assets_receipt.missing_required_assets.len(),
        quality_findings_receipt.findings.len(),
        quality_findings_receipt.hard_error_count(),
        quality_findings_receipt.warning_count(),
        quality_findings_receipt.advice_count()
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
