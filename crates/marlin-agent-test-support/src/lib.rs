//! Shared test support for Marlin agent runtime and stream contracts.

mod graph_policy;
mod hook;
mod replay;
mod runtime_environment;
mod stability;
mod stream;
mod sub_agent_scenario;
mod sub_agent_session;
mod test_run;
mod three_layer;

pub use graph_policy::{
    DeterministicGraphPolicyProposalFixture, accepted_gerbil_ir_graph_policy_proposal_fixture,
    accepted_graph_policy_proposal_fixture,
    assert_accepted_gerbil_ir_graph_policy_proposal_fixture,
    assert_accepted_graph_policy_proposal_fixture, assert_budgeted_graph_policy_execution_request,
    assert_rejected_graph_policy_proposal_fixture, budgeted_graph_policy_execution_request_fixture,
    complex_gerbil_graph_policy_replay_fixture, graph_native_abi_readiness_receipt_fixture,
    graph_native_abi_requirement_fixture, rejected_graph_policy_proposal_fixture,
};
pub use hook::{
    assert_complex_gerbil_hook_policy_receipt, assert_custom_hook_policy_receipt,
    assert_custom_sub_agent_start_hook_summary, assert_sub_agent_hook_dispatch_selection,
    complex_gerbil_hook_policy_decision_context_fixture,
    complex_gerbil_hook_policy_receipt_fixture,
    complex_gerbil_hook_policy_receipt_with_decision_context, custom_hook_policy_receipt_fixture,
    custom_sub_agent_start_hook_summary_fixture, hook_dispatch_replay_evidence,
    sub_agent_hook_dispatch_selection_fixture,
};
pub use replay::{
    NO_LLM_RUNTIME_REPLAY_ARTIFACT_ID, NO_LLM_RUNTIME_REPLAY_CONTRACT_JSON,
    NoLlmRuntimeReplayArtifact, NoLlmRuntimeReplayArtifactLoadError,
    load_no_llm_runtime_replay_artifact, no_llm_runtime_replay_artifact_fixture,
};
pub use runtime_environment::{
    DirenvActivationFixture, RuntimeEnvironmentFixture, ScriptedDirenvCommandRunner,
    assert_custom_sub_agent_environment, assert_direnv_activation_fixture,
    assert_hook_environment_uses_root_home, custom_home_runtime_environment_fixture,
    direnv_activation_fixtures,
};
pub use stability::{
    RuntimeStabilityEvidenceInput, runtime_stability_budget_diagnostics,
    runtime_stability_budget_evidence,
};
pub use stream::{
    NO_LIVE_LLM_GATE_DENIAL_MESSAGE, NoLiveLlmModelGateway, ScriptedChunkGate,
    ScriptedChunkGatePermit, ScriptedGatewayRequestReceipt, ScriptedModelGateway,
    ScriptedModelStream, ScriptedModelStreamChunk, ScriptedModelStreamEvent, ScriptedStreamReceipt,
    no_live_llm_gateway_denial_evidence, scripted_stream_gate_evidence,
};
pub use sub_agent_scenario::{
    DeterministicRoutedSubAgentExecutionReceipt, DeterministicSubAgentScenarioFixture,
    assert_deterministic_reviewer_applied_environment_activation_receipt,
    assert_deterministic_reviewer_environment_activation_receipt,
    assert_deterministic_routed_sub_agent_execution, assert_deterministic_routed_sub_agent_session,
    assert_deterministic_sub_agent_gateway_request, assert_deterministic_sub_agent_route_decision,
    assert_deterministic_sub_agent_scenario_fixture,
    deterministic_reviewer_applied_environment_activation_receipt_fixture,
    deterministic_reviewer_routed_receipt_family_evidence,
    deterministic_reviewer_sub_agent_scenario_fixture,
    deterministic_reviewer_sub_agent_spawn_config,
};
pub use sub_agent_session::{
    SubAgentMemoryExpectation, SubAgentMemorySessionFixture,
    assert_sub_agent_memory_session_fixture, sub_agent_memory_allowed_fixture,
    sub_agent_memory_allowed_fixture_with_config, sub_agent_memory_denied_fixture,
    sub_agent_memory_session_replay_evidence, sub_agent_memory_session_visibility_evidence,
};
pub use test_run::{
    LibtestCommandCapture, LibtestCommandImportReport, LibtestCommandSpec, LibtestTextImportConfig,
    LibtestTextImportReport, LibtestTextResultSummary, TEST_RUN_EVIDENCE_SCHEMA_VERSION,
    TestRunCaseRecord, TestRunCaseStatus, TestRunEvidenceReceipt, TestRunLayer,
    TestRunLayerSummary, WorkspaceLibtestCommandImportReport, WorkspaceLibtestCommandPackageReport,
    WorkspaceLibtestTextImportInput, WorkspaceLibtestTextImportReport,
    WorkspaceLibtestTextPackageReport, assert_deterministic_test_run_evidence,
    capture_libtest_command_output, capture_workspace_libtest_commands,
    deterministic_test_run_evidence_fixture, import_libtest_text_output,
    import_workspace_libtest_text_outputs,
};
pub use three_layer::{
    DEFAULT_THREE_LAYER_PACKAGES, DeterministicAgentRuntimeScenarioReceipt,
    DeterministicGatewayReceipt, PackageThreeLayerReceipt, ThreeLayerCoverageReport,
    assert_deterministic_agent_runtime_scenario_layer, assert_deterministic_gateway_layer,
    assert_package_three_layer_coverage, assert_three_layer_testing_system_for_workspace,
    workspace_crate_dirs,
};
