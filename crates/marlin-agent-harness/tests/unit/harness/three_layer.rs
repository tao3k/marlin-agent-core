use std::{
    fs,
    path::{Path, PathBuf},
    thread,
};

use marlin_agent_protocol::ModelEndpoint;
use marlin_agent_stream::{
    LiteLlmModelClientError, ModelStreamGateway, ModelStreamRequest, ModelStreamTransport,
    user_message,
};
use marlin_agent_test_support::ScriptedModelGateway;
use marlin_rust_project_harness_policy::{
    RustProjectHarnessGateReceipt, evaluate_performance_and_stability_gate,
    rust_project_harness_config_for_project,
};
use rust_lang_project_harness::{
    RustDeterminismReadinessInput, RustEvidenceGraphInput, RustReviewPacketInput,
    build_rust_determinism_readiness, build_rust_evidence_graph, build_rust_review_packet,
    plan_rust_project_verification_with_config, run_rust_project_harness_with_config,
};

const EXPECTED_LAYER_PACKAGES: &[&str] = &[
    "marlin-agent-core",
    "marlin-agent-harness",
    "marlin-agent-hooks",
    "marlin-agent-runtime",
    "marlin-agent-sessions",
    "marlin-agent-stream",
    "marlin-agent-test-support",
];

#[tokio::test]
async fn three_layer_testing_system_covers_workspace_packages_without_live_llm() {
    let gateway_receipt = assert_deterministic_gateway_layer().await;
    let crates = workspace_crates();

    assert!(
        crates.len() >= 20,
        "three-layer package coverage expected at least 20 crates, got {}",
        crates.len(),
    );

    let handles = crates
        .iter()
        .cloned()
        .map(|crate_dir| thread::spawn(move || assert_package_three_layer_coverage(&crate_dir)));
    let package_receipts = handles
        .map(|handle| {
            handle
                .join()
                .expect("three-layer package coverage worker should finish")
        })
        .collect::<Vec<_>>();
    let report = ThreeLayerCoverageReport::new(gateway_receipt, package_receipts);

    assert_eq!(report.package_count(), crates.len());
    assert!(
        report.gateway_request.litellm_model_id == "openai/gpt-5-test-double"
            && report.gateway_request.message_count == 1
            && report.gateway_request.transport == ModelStreamTransport::Sse,
        "deterministic gateway layer should record the no-live-LLM request boundary",
    );
    assert!(
        report.missing_package_quality_gates().is_empty(),
        "workspace packages missing three-layer quality gates: {}",
        report.render_missing_package_quality_gates(),
    );
    for expected_package in EXPECTED_LAYER_PACKAGES {
        assert!(
            report.covers_package(expected_package),
            "three-layer package coverage missing {expected_package}",
        );
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DeterministicGatewayReceipt {
    litellm_model_id: String,
    message_count: usize,
    transport: ModelStreamTransport,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PackageThreeLayerReceipt {
    package_name: String,
    harness_module_count: usize,
    evidence_graph_node_count: usize,
    gate_receipt: RustProjectHarnessGateReceipt,
}

impl PackageThreeLayerReceipt {
    fn is_success(&self) -> bool {
        self.harness_module_count > 0
            && self.evidence_graph_node_count > 0
            && self.gate_receipt.is_success()
    }

    fn render_missing(&self) -> String {
        format!(
            "{}:modules={} graph_nodes={} perf={} stability={} perf_report={} stability_report={}",
            self.package_name,
            self.harness_module_count,
            self.evidence_graph_node_count,
            self.gate_receipt.performance_gate,
            self.gate_receipt.stability_gate,
            self.gate_receipt.performance_report_obligation,
            self.gate_receipt.stability_report_obligation,
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ThreeLayerCoverageReport {
    gateway_request: DeterministicGatewayReceipt,
    packages: Vec<PackageThreeLayerReceipt>,
}

impl ThreeLayerCoverageReport {
    fn new(
        gateway_request: DeterministicGatewayReceipt,
        packages: Vec<PackageThreeLayerReceipt>,
    ) -> Self {
        Self {
            gateway_request,
            packages,
        }
    }

    fn package_count(&self) -> usize {
        self.packages.len()
    }

    fn covers_package(&self, package_name: &str) -> bool {
        self.packages
            .iter()
            .any(|receipt| receipt.package_name == package_name)
    }

    fn missing_package_quality_gates(&self) -> Vec<&PackageThreeLayerReceipt> {
        self.packages
            .iter()
            .filter(|receipt| !receipt.is_success())
            .collect()
    }

    fn render_missing_package_quality_gates(&self) -> String {
        self.missing_package_quality_gates()
            .into_iter()
            .map(PackageThreeLayerReceipt::render_missing)
            .collect::<Vec<_>>()
            .join(", ")
    }
}

async fn assert_deterministic_gateway_layer() -> DeterministicGatewayReceipt {
    let gateway = ScriptedModelGateway::completion_failure("scripted no-live-llm");
    let request = ModelStreamRequest::new(
        ModelEndpoint::new("openai", "gpt-5-test-double"),
        vec![user_message("prove the deterministic gateway layer")],
    )
    .with_transport(ModelStreamTransport::Sse);

    let result = gateway.complete(request).await;

    assert!(
        matches!(result, Err(LiteLlmModelClientError::Completion(message)) if message == "scripted no-live-llm"),
        "scripted gateway should fail deterministically before any live LLM call",
    );

    let requests = gateway.requests();
    assert_eq!(
        requests.len(),
        1,
        "scripted gateway should record exactly one model request",
    );
    let request = requests
        .into_iter()
        .next()
        .expect("scripted gateway request receipt should exist");

    DeterministicGatewayReceipt {
        litellm_model_id: request.litellm_model_id,
        message_count: request.message_count,
        transport: request.transport,
    }
}

fn assert_package_three_layer_coverage(crate_dir: &Path) -> PackageThreeLayerReceipt {
    let package_name = crate_dir
        .file_name()
        .and_then(|name| name.to_str())
        .expect("workspace crate should have a utf-8 directory name")
        .to_owned();
    let config = rust_project_harness_config_for_project(crate_dir);
    let harness_report = run_rust_project_harness_with_config(crate_dir, &config)
        .unwrap_or_else(|error| panic!("{package_name} rust harness report: {error}"));
    let harness_module_count = harness_report.modules.len();
    let determinism_readiness = build_rust_determinism_readiness(RustDeterminismReadinessInput {
        project_root: crate_dir.to_path_buf(),
        include_tests: config.include_tests,
    })
    .unwrap_or_else(|error| panic!("{package_name} determinism readiness: {error}"));
    let review_packet = build_rust_review_packet(RustReviewPacketInput {
        project_root: crate_dir.to_path_buf(),
        report: harness_report,
        receipts: Vec::new(),
        behavior_snapshots: Vec::new(),
        determinism_readiness: vec![determinism_readiness],
        proof_pilots: Vec::new(),
        waivers: Vec::new(),
    });
    let evidence_graph = build_rust_evidence_graph(RustEvidenceGraphInput {
        project_root: crate_dir.to_path_buf(),
        review_packets: vec![review_packet],
    });
    let plan = plan_rust_project_verification_with_config(crate_dir, &config)
        .unwrap_or_else(|error| panic!("{package_name} verification plan: {error}"));
    let gate_receipt = evaluate_performance_and_stability_gate(&plan, package_name.clone());

    PackageThreeLayerReceipt {
        package_name,
        harness_module_count,
        evidence_graph_node_count: evidence_graph.summary.nodes,
        gate_receipt,
    }
}

fn workspace_crates() -> Vec<PathBuf> {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("harness crate should live under workspace/crates");
    let crates_dir = workspace_root.join("crates");
    let mut crates = fs::read_dir(&crates_dir)
        .unwrap_or_else(|error| panic!("read workspace crates dir {crates_dir:?}: {error}"))
        .map(|entry| entry.expect("workspace crate entry").path())
        .filter(|path| path.join("Cargo.toml").is_file() && path.join("src/lib.rs").is_file())
        .collect::<Vec<_>>();

    crates.sort();
    crates
}
