//! Reusable three-layer testing system support for agent runtime behavior.

use std::{
    fs,
    path::{Path, PathBuf},
};

use marlin_agent_harness_types::HarnessEvidenceKind;
use marlin_agent_protocol::{
    ModelEndpoint, ModelGateway, ModelGatewayError, ModelGatewayRequest, ModelGatewayTransport,
    user_gateway_message,
};

use crate::{
    ScriptedModelGateway, ScriptedModelStream, SubAgentMemoryExpectation, TestRunEvidenceReceipt,
    accepted_graph_policy_proposal_fixture, assert_accepted_graph_policy_proposal_fixture,
    assert_deterministic_sub_agent_gateway_request,
    assert_deterministic_sub_agent_scenario_fixture, assert_deterministic_test_run_evidence,
    deterministic_reviewer_sub_agent_scenario_fixture,
};

/// Agent packages expected to participate in the core three-layer test contract.
pub const DEFAULT_THREE_LAYER_PACKAGES: &[&str] = &[
    "marlin-agent-core",
    "marlin-agent-harness",
    "marlin-agent-hooks",
    "marlin-agent-runtime",
    "marlin-agent-sessions",
    "marlin-agent-stream",
    "marlin-agent-test-support",
];

/// Deterministic model gateway receipt proving the test did not call a live LLM.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeterministicGatewayReceipt {
    pub litellm_model_id: String,
    pub message_count: usize,
    pub transport: ModelGatewayTransport,
}

/// Deterministic agent-runtime scenario receipt for the no-LLM behavior layer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeterministicAgentRuntimeScenarioReceipt {
    pub graph_policy_visibility_subject: String,
    pub sub_agent_litellm_model_id: String,
    pub sub_agent_gateway_transport: ModelGatewayTransport,
    pub hook_selected_count: usize,
    pub hook_policy_evaluated_count: usize,
    pub memory_visibility_granted: bool,
    pub stream_chunk_count: usize,
}

impl DeterministicAgentRuntimeScenarioReceipt {
    /// Returns true when the agent-runtime layer exercised every no-LLM boundary.
    pub fn is_success(&self) -> bool {
        self.graph_policy_visibility_subject
            .contains("test-support-scheme-loop-ranker")
            && self.sub_agent_litellm_model_id == "anthropic/claude-opus-4-8"
            && self.sub_agent_gateway_transport == ModelGatewayTransport::Sse
            && self.hook_selected_count > 0
            && self.hook_policy_evaluated_count > 0
            && self.memory_visibility_granted
            && self.stream_chunk_count > 0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DeterministicSubAgentRuntimeBoundaryReceipt {
    litellm_model_id: String,
    gateway_transport: ModelGatewayTransport,
    hook_selected_count: usize,
    hook_policy_evaluated_count: usize,
    memory_visibility_granted: bool,
}

/// Package participation receipt for the agent-core workspace surface.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageThreeLayerReceipt {
    pub package_name: String,
    pub manifest_present: bool,
    pub library_root_present: bool,
    pub agent_core_package: bool,
}

impl PackageThreeLayerReceipt {
    /// Returns true when the package participates in the agent workspace surface.
    pub fn is_success(&self) -> bool {
        self.manifest_present && self.library_root_present
    }

    /// Renders compact diagnostics for a missing package receipt.
    pub fn render_missing(&self) -> String {
        format!(
            "{}:manifest_present={} library_root_present={} agent_core_package={}",
            self.package_name,
            self.manifest_present,
            self.library_root_present,
            self.agent_core_package,
        )
    }
}

/// Full three-layer coverage report for the agent test system.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThreeLayerCoverageReport {
    pub gateway_request: DeterministicGatewayReceipt,
    pub test_run: TestRunEvidenceReceipt,
    pub agent_runtime: DeterministicAgentRuntimeScenarioReceipt,
    pub packages: Vec<PackageThreeLayerReceipt>,
}

impl ThreeLayerCoverageReport {
    /// Creates a coverage report from the three agent test-support layers.
    pub fn new(
        gateway_request: DeterministicGatewayReceipt,
        test_run: TestRunEvidenceReceipt,
        agent_runtime: DeterministicAgentRuntimeScenarioReceipt,
        packages: Vec<PackageThreeLayerReceipt>,
    ) -> Self {
        Self {
            gateway_request,
            test_run,
            agent_runtime,
            packages,
        }
    }

    /// Number of covered packages.
    pub fn package_count(&self) -> usize {
        self.packages.len()
    }

    /// Returns true when a package receipt is present.
    pub fn covers_package(&self, package_name: &str) -> bool {
        self.package_receipt(package_name).is_some()
    }

    /// Returns the receipt for one package.
    pub fn package_receipt(&self, package_name: &str) -> Option<&PackageThreeLayerReceipt> {
        self.packages
            .iter()
            .find(|receipt| receipt.package_name == package_name)
    }

    /// Package receipts missing the agent workspace surface.
    pub fn missing_agent_package_coverage(&self) -> Vec<&PackageThreeLayerReceipt> {
        self.packages
            .iter()
            .filter(|receipt| !receipt.is_success())
            .collect()
    }

    /// Compact diagnostics for missing package receipts.
    pub fn render_missing_agent_package_coverage(&self) -> String {
        self.missing_agent_package_coverage()
            .into_iter()
            .map(PackageThreeLayerReceipt::render_missing)
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Asserts Marlin's default agent workspace expectations.
    pub fn assert_default_expectations(&self, expected_package_count: usize) {
        assert_eq!(self.package_count(), expected_package_count);
        assert!(
            self.gateway_request.litellm_model_id == "openai/gpt-5-test-double"
                && self.gateway_request.message_count == 1
                && self.gateway_request.transport == ModelGatewayTransport::Sse,
            "deterministic gateway layer should record the no-live-LLM request boundary",
        );
        assert!(
            self.test_run.is_non_live_success(),
            "deterministic test run layer should report no-live unit and integration success: {}",
            self.test_run.render_summary(),
        );
        assert!(
            self.agent_runtime.is_success(),
            "deterministic agent runtime layer should cover graph policy, sub-agent, hook, session, gateway, and stream boundaries",
        );
        assert!(
            self.missing_agent_package_coverage().is_empty(),
            "workspace packages missing agent test coverage surface: {}",
            self.render_missing_agent_package_coverage(),
        );
        for expected_package in DEFAULT_THREE_LAYER_PACKAGES {
            assert!(
                self.covers_package(expected_package),
                "three-layer package coverage missing {expected_package}",
            );
        }
    }
}

/// Runs and asserts the full three-layer test system for a workspace.
pub async fn assert_three_layer_testing_system_for_workspace(
    workspace_root: impl AsRef<Path>,
) -> ThreeLayerCoverageReport {
    let workspace_root = workspace_root.as_ref().to_path_buf();
    let gateway_receipt = assert_deterministic_gateway_layer().await;
    let test_run_receipt = assert_deterministic_test_run_evidence();
    let agent_runtime_receipt = assert_deterministic_agent_runtime_scenario_layer().await;
    let crates = workspace_crate_dirs(&workspace_root);

    assert!(
        crates.len() >= 20,
        "three-layer package coverage expected at least 20 crates, got {}",
        crates.len(),
    );

    let package_receipts = crates
        .iter()
        .map(assert_package_three_layer_coverage)
        .collect::<Vec<_>>();
    let report = ThreeLayerCoverageReport::new(
        gateway_receipt,
        test_run_receipt,
        agent_runtime_receipt,
        package_receipts,
    );
    report.assert_default_expectations(crates.len());
    report
}

/// Asserts the deterministic gateway layer without a live LLM call.
pub async fn assert_deterministic_gateway_layer() -> DeterministicGatewayReceipt {
    let gateway = ScriptedModelGateway::completion_failure("scripted no-live-llm");
    let request = ModelGatewayRequest::new(
        ModelEndpoint::new("openai", "gpt-5-test-double"),
        vec![user_gateway_message(
            "prove the deterministic gateway layer",
        )],
    )
    .with_transport(ModelGatewayTransport::Sse);

    let result = gateway.complete(request).await;

    assert!(
        matches!(result, Err(ModelGatewayError::Completion(message)) if message == "scripted no-live-llm"),
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

/// Asserts the deterministic agent-runtime layer without a live LLM call.
pub async fn assert_deterministic_agent_runtime_scenario_layer()
-> DeterministicAgentRuntimeScenarioReceipt {
    let graph_policy_visibility_subject = assert_graph_policy_runtime_boundary();
    let sub_agent = assert_sub_agent_runtime_boundary().await;
    let stream_chunk_count = assert_scripted_stream_boundary().await;

    DeterministicAgentRuntimeScenarioReceipt {
        graph_policy_visibility_subject,
        sub_agent_litellm_model_id: sub_agent.litellm_model_id,
        sub_agent_gateway_transport: sub_agent.gateway_transport,
        hook_selected_count: sub_agent.hook_selected_count,
        hook_policy_evaluated_count: sub_agent.hook_policy_evaluated_count,
        memory_visibility_granted: sub_agent.memory_visibility_granted,
        stream_chunk_count,
    }
}

fn assert_graph_policy_runtime_boundary() -> String {
    let graph_policy = accepted_graph_policy_proposal_fixture();
    assert_accepted_graph_policy_proposal_fixture(&graph_policy);
    let graph_policy_evidence = graph_policy.visibility_evidence();
    assert_eq!(graph_policy_evidence.kind, HarnessEvidenceKind::Visibility);

    graph_policy_evidence.subject
}

async fn assert_sub_agent_runtime_boundary() -> DeterministicSubAgentRuntimeBoundaryReceipt {
    let sub_agent = deterministic_reviewer_sub_agent_scenario_fixture();
    assert_deterministic_sub_agent_scenario_fixture(&sub_agent);

    let gateway = ScriptedModelGateway::completion_failure("scripted sub-agent no-live-llm");
    let result = gateway
        .complete(sub_agent.model_request("prove routed sub-agent gateway boundary"))
        .await;
    assert!(
        matches!(result, Err(ModelGatewayError::Completion(message)) if message == "scripted sub-agent no-live-llm"),
        "scripted sub-agent gateway should fail before any live LLM call",
    );
    let sub_agent_gateway_request = gateway
        .requests()
        .into_iter()
        .next()
        .expect("scripted sub-agent gateway request receipt should exist");
    assert_deterministic_sub_agent_gateway_request(&sub_agent, &sub_agent_gateway_request);

    DeterministicSubAgentRuntimeBoundaryReceipt {
        litellm_model_id: sub_agent_gateway_request.litellm_model_id,
        gateway_transport: sub_agent_gateway_request.transport,
        hook_selected_count: sub_agent.hook_selection().selected_count,
        hook_policy_evaluated_count: sub_agent.hook_policy().evaluated_count,
        memory_visibility_granted: sub_agent.session_fixture().expectation()
            == SubAgentMemoryExpectation::Granted,
    }
}

async fn assert_scripted_stream_boundary() -> usize {
    let stream_receipt = ScriptedModelStream::single_text_delta("agent-runtime-scenario")
        .collect()
        .await;
    assert!(stream_receipt.completed);
    assert!(!stream_receipt.failed);
    assert_eq!(stream_receipt.chunk_count, 1);

    stream_receipt.chunk_count
}

/// Asserts agent workspace package participation for one package.
pub fn assert_package_three_layer_coverage(
    crate_dir: impl AsRef<Path>,
) -> PackageThreeLayerReceipt {
    let crate_dir = crate_dir.as_ref();
    let package_name = crate_dir
        .file_name()
        .and_then(|name| name.to_str())
        .expect("workspace crate should have a utf-8 directory name")
        .to_owned();
    let manifest_present = crate_dir.join("Cargo.toml").is_file();
    let library_root_present = crate_dir.join("src/lib.rs").is_file();

    assert!(
        manifest_present,
        "{package_name} should expose a Cargo manifest",
    );
    assert!(
        library_root_present,
        "{package_name} should expose a library root for agent workspace coverage",
    );

    PackageThreeLayerReceipt {
        agent_core_package: package_name.starts_with("marlin-agent-"),
        package_name,
        manifest_present,
        library_root_present,
    }
}

/// Discovers workspace crates with library roots.
pub fn workspace_crate_dirs(workspace_root: impl AsRef<Path>) -> Vec<PathBuf> {
    let workspace_root = workspace_root.as_ref();
    let crates_dir = workspace_root.join("crates");
    let mut crates = fs::read_dir(&crates_dir)
        .unwrap_or_else(|error| panic!("read workspace crates dir {crates_dir:?}: {error}"))
        .map(|entry| entry.expect("workspace crate entry").path())
        .filter(|path| path.join("Cargo.toml").is_file() && path.join("src/lib.rs").is_file())
        .collect::<Vec<_>>();

    crates.sort();
    crates
}
