//! Typed no-live-LLM test run evidence receipts.

use marlin_agent_harness_types::{AgentHarnessEvidence, AgentHarnessEvidenceKind};

use crate::graph_policy::{
    accepted_graph_policy_proposal_fixture, rejected_graph_policy_proposal_fixture,
};

/// Schema version for test run evidence receipts.
pub const TEST_RUN_EVIDENCE_SCHEMA_VERSION: u32 = 1;

/// Test layer used to separate deterministic tests from live external gates.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TestRunLayer {
    NonLiveUnit,
    NonLiveIntegration,
    LiveExternal,
}

impl TestRunLayer {
    /// Stable layer identifier for diagnostics and evidence graphs.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NonLiveUnit => "non_live_unit",
            Self::NonLiveIntegration => "non_live_integration",
            Self::LiveExternal => "live_external",
        }
    }

    /// Returns true when this layer must not call a live provider.
    pub fn is_non_live(self) -> bool {
        matches!(self, Self::NonLiveUnit | Self::NonLiveIntegration)
    }
}

/// Outcome for one recorded test case.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TestRunCaseStatus {
    Passed,
    Failed,
    Ignored,
}

/// One test case projected into the test run evidence graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TestRunCaseRecord {
    pub package_name: String,
    pub test_name: String,
    pub layer: TestRunLayer,
    pub status: TestRunCaseStatus,
    pub ignored_reason: Option<String>,
}

impl TestRunCaseRecord {
    /// Records a passing test case.
    pub fn passed(
        package_name: impl Into<String>,
        test_name: impl Into<String>,
        layer: TestRunLayer,
    ) -> Self {
        Self {
            package_name: package_name.into(),
            test_name: test_name.into(),
            layer,
            status: TestRunCaseStatus::Passed,
            ignored_reason: None,
        }
    }

    /// Records a failing test case.
    pub fn failed(
        package_name: impl Into<String>,
        test_name: impl Into<String>,
        layer: TestRunLayer,
    ) -> Self {
        Self {
            package_name: package_name.into(),
            test_name: test_name.into(),
            layer,
            status: TestRunCaseStatus::Failed,
            ignored_reason: None,
        }
    }

    /// Records an ignored external test with a machine-readable reason.
    pub fn ignored(
        package_name: impl Into<String>,
        test_name: impl Into<String>,
        layer: TestRunLayer,
        ignored_reason: impl Into<String>,
    ) -> Self {
        Self {
            package_name: package_name.into(),
            test_name: test_name.into(),
            layer,
            status: TestRunCaseStatus::Ignored,
            ignored_reason: Some(ignored_reason.into()),
        }
    }
}

/// Per-layer summary for one test run receipt.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TestRunLayerSummary {
    pub layer: TestRunLayer,
    pub passed: usize,
    pub failed: usize,
    pub ignored: usize,
}

impl TestRunLayerSummary {
    /// Returns true when this layer has at least one recorded test case.
    pub fn is_present(self) -> bool {
        self.passed + self.failed + self.ignored > 0
    }

    /// Compact summary for diagnostics.
    pub fn render(self) -> String {
        format!(
            "{}:passed={} failed={} ignored={}",
            self.layer.as_str(),
            self.passed,
            self.failed,
            self.ignored
        )
    }
}

/// Workspace-level test evidence receipt used by deterministic harness tests.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TestRunEvidenceReceipt {
    pub schema_version: u32,
    pub cases: Vec<TestRunCaseRecord>,
    pub evidence: Vec<AgentHarnessEvidence>,
}

impl TestRunEvidenceReceipt {
    /// Creates a receipt from case records.
    pub fn new(cases: Vec<TestRunCaseRecord>) -> Self {
        Self {
            schema_version: TEST_RUN_EVIDENCE_SCHEMA_VERSION,
            cases,
            evidence: Vec::new(),
        }
    }

    /// Adds typed harness evidence facts to this test-run receipt.
    pub fn with_evidence(
        mut self,
        evidence: impl IntoIterator<Item = AgentHarnessEvidence>,
    ) -> Self {
        self.evidence.extend(evidence);
        self
    }

    /// Number of recorded test cases.
    pub fn case_count(&self) -> usize {
        self.cases.len()
    }

    /// Number of evidence facts attached to this receipt.
    pub fn evidence_count(&self) -> usize {
        self.evidence.len()
    }

    /// Number of evidence facts recorded with this kind.
    pub fn evidence_count_by_kind(&self, kind: AgentHarnessEvidenceKind) -> usize {
        self.evidence
            .iter()
            .filter(|evidence| evidence.kind == kind)
            .count()
    }

    /// Number of passing test cases.
    pub fn passed_count(&self) -> usize {
        self.count_by_status(TestRunCaseStatus::Passed)
    }

    /// Number of failing test cases.
    pub fn failed_count(&self) -> usize {
        self.count_by_status(TestRunCaseStatus::Failed)
    }

    /// Number of ignored test cases.
    pub fn ignored_count(&self) -> usize {
        self.count_by_status(TestRunCaseStatus::Ignored)
    }

    /// Number of failed deterministic no-live tests.
    pub fn non_live_failed_count(&self) -> usize {
        self.cases
            .iter()
            .filter(|case| case.layer.is_non_live() && case.status == TestRunCaseStatus::Failed)
            .count()
    }

    /// Number of recorded deterministic no-live tests.
    pub fn non_live_case_count(&self) -> usize {
        self.cases
            .iter()
            .filter(|case| case.layer.is_non_live())
            .count()
    }

    /// Number of ignored live external tests.
    pub fn ignored_live_external_count(&self) -> usize {
        self.cases
            .iter()
            .filter(|case| {
                case.layer == TestRunLayer::LiveExternal
                    && case.status == TestRunCaseStatus::Ignored
            })
            .count()
    }

    /// Summary for a single layer.
    pub fn summary_for_layer(&self, layer: TestRunLayer) -> TestRunLayerSummary {
        let mut summary = TestRunLayerSummary {
            layer,
            passed: 0,
            failed: 0,
            ignored: 0,
        };

        for case in self.cases.iter().filter(|case| case.layer == layer) {
            match case.status {
                TestRunCaseStatus::Passed => summary.passed += 1,
                TestRunCaseStatus::Failed => summary.failed += 1,
                TestRunCaseStatus::Ignored => summary.ignored += 1,
            }
        }

        summary
    }

    /// Ordered layer summaries.
    pub fn layer_summaries(&self) -> Vec<TestRunLayerSummary> {
        [
            TestRunLayer::NonLiveUnit,
            TestRunLayer::NonLiveIntegration,
            TestRunLayer::LiveExternal,
        ]
        .into_iter()
        .map(|layer| self.summary_for_layer(layer))
        .collect()
    }

    /// Returns true when deterministic test layers have coverage and no failures.
    pub fn is_non_live_success(&self) -> bool {
        self.non_live_case_count() > 0
            && self.non_live_failed_count() == 0
            && self
                .summary_for_layer(TestRunLayer::NonLiveUnit)
                .is_present()
            && self
                .summary_for_layer(TestRunLayer::NonLiveIntegration)
                .is_present()
    }

    /// Compact test run summary for harness diagnostics.
    pub fn render_summary(&self) -> String {
        let layers = self
            .layer_summaries()
            .into_iter()
            .map(TestRunLayerSummary::render)
            .collect::<Vec<_>>()
            .join(", ");

        format!(
            "schema={} total={} passed={} failed={} ignored={} non_live_failed={} evidence={} layers=[{}]",
            self.schema_version,
            self.case_count(),
            self.passed_count(),
            self.failed_count(),
            self.ignored_count(),
            self.non_live_failed_count(),
            self.evidence_count(),
            layers
        )
    }

    fn count_by_status(&self, status: TestRunCaseStatus) -> usize {
        self.cases
            .iter()
            .filter(|case| case.status == status)
            .count()
    }
}

/// Deterministic fixture representing the current no-live test system shape.
pub fn deterministic_test_run_evidence_fixture() -> TestRunEvidenceReceipt {
    TestRunEvidenceReceipt::new(vec![
        TestRunCaseRecord::passed(
            "marlin-agent-runtime",
            "runtime_session::sub_agent_session_context_isolated_from_parent",
            TestRunLayer::NonLiveUnit,
        ),
        TestRunCaseRecord::passed(
            "marlin-agent-stream",
            "stream::chunk_gate_releases_chunks_in_order",
            TestRunLayer::NonLiveUnit,
        ),
        TestRunCaseRecord::passed(
            "marlin-agent-test-support",
            "three_layer::test_support_three_layer_testing_system_covers_workspace_packages_without_live_llm",
            TestRunLayer::NonLiveIntegration,
        ),
        TestRunCaseRecord::passed(
            "marlin-agent-harness",
            "harness::three_layer::harness_consumes_test_support_three_layer_package_coverage",
            TestRunLayer::NonLiveIntegration,
        ),
        TestRunCaseRecord::ignored(
            "marlin-gerbil-scheme",
            "command::real_gxi::artifacts::command_compiler_can_call_real_gxi_workspace_schema",
            TestRunLayer::LiveExternal,
            "requires a local Gerbil gxi executable",
        ),
    ])
    .with_evidence(graph_policy_visibility_evidence())
}

/// Asserts the deterministic no-live test run evidence contract.
pub fn assert_deterministic_test_run_evidence() -> TestRunEvidenceReceipt {
    let receipt = deterministic_test_run_evidence_fixture();
    assert!(
        receipt.is_non_live_success(),
        "deterministic no-live test run failed quality contract: {}",
        receipt.render_summary(),
    );
    assert!(
        receipt.ignored_live_external_count() > 0,
        "test run evidence should preserve ignored live external gates: {}",
        receipt.render_summary(),
    );
    receipt
}

/// Fixture connecting the Marlin user-interface loop smoke to the live LLM gate.
pub fn user_interface_loop_live_llm_gate_evidence_fixture() -> TestRunEvidenceReceipt {
    TestRunEvidenceReceipt::new(vec![
        TestRunCaseRecord::passed(
            "marlin-agent-stream",
            "stream::no_live_http_fixture_denies_stream_provider_posts",
            TestRunLayer::NonLiveUnit,
        ),
        TestRunCaseRecord::passed(
            "marlin-gerbil-scheme",
            "command::real_gxi::examples::user_interface_worker_space::command_compiler_real_gxtest_runs_user_interface_module_config_example",
            TestRunLayer::NonLiveIntegration,
        ),
        TestRunCaseRecord::ignored(
            "marlin-agent-stream",
            "stream::live_litellm_stream_gateway_completes_provider_neutral_request",
            TestRunLayer::LiveExternal,
            "requires MARLIN_LIVE_LLM_GATE=1 and live LiteLLM provider credentials",
        ),
    ])
    .with_evidence(user_interface_loop_live_llm_gate_evidence())
}

/// Asserts that the user-interface loop has a no-live baseline and live gate.
pub fn assert_user_interface_loop_live_llm_gate_evidence() -> TestRunEvidenceReceipt {
    let receipt = user_interface_loop_live_llm_gate_evidence_fixture();
    assert!(
        receipt.is_non_live_success(),
        "user-interface loop live LLM gate lacks no-live baseline: {}",
        receipt.render_summary(),
    );
    assert_eq!(
        receipt.ignored_live_external_count(),
        1,
        "user-interface loop live LLM gate should preserve exactly one live external test: {}",
        receipt.render_summary(),
    );
    receipt
}

fn graph_policy_visibility_evidence() -> Vec<AgentHarnessEvidence> {
    vec![
        accepted_graph_policy_proposal_fixture().visibility_evidence(),
        rejected_graph_policy_proposal_fixture().visibility_evidence(),
    ]
}

fn user_interface_loop_live_llm_gate_evidence() -> Vec<AgentHarnessEvidence> {
    vec![
        AgentHarnessEvidence::present(
            AgentHarnessEvidenceKind::Runtime,
            "user-interface-loop:no-live-manifest-handoff",
        )
        .with_detail(
            "loop_engine_capabilities=[+manifest-handoff,+l1-receipts] runtime_executed=false",
        ),
        AgentHarnessEvidence::present(
            AgentHarnessEvidenceKind::Runtime,
            "user-interface-loop:live-litellm-gate",
        )
        .with_detail(
            "env_gate=MARLIN_LIVE_LLM_GATE provider_env=MARLIN_LIVE_LLM_PROVIDER model_env=MARLIN_LIVE_LLM_MODEL",
        ),
    ]
}
