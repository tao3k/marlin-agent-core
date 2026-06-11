pub(super) use super::support::{
    AGENT_SCENARIO_CONTRACT_SOURCE, RICH_LOOP_GRAPH_SOURCE, WORKSPACE_PATCH_INTENT_SOURCE,
    WORKSPACE_SCHEMA_SOURCE, WORKSPACE_SOURCE_COMMIT_INTENT_SOURCE,
    assert_agent_scenario_contract_artifact, assert_rich_loop_graph_artifact,
    assert_workspace_patch_intent_artifact, assert_workspace_schema_artifact, local_gxi,
    real_gxi_command_adapter_batch_compiler, real_gxi_module_compiler,
};
use marlin_gerbil_scheme::{
    GerbilArtifactKind, GerbilCompileRequest, GerbilCompiledArtifact, GerbilSource,
    GerbilWorkspaceContractFacts,
};
use marlin_org_model::{
    OrgContractRegistry, OrgContractResolutionReport, OrgContractValidationReport,
};
use std::sync::OnceLock;

pub(super) const RELEASE_TOPOLOGY_SOURCE: &str = super::support::RELEASE_TOPOLOGY_SOURCE;

pub(super) fn assert_release_topology_artifact(artifact: GerbilCompiledArtifact) {
    super::support::assert_release_topology_artifact(artifact);
}

static COMMAND_ADAPTER_BATCH_ARTIFACTS: OnceLock<Option<Vec<GerbilCompiledArtifact>>> =
    OnceLock::new();

pub(super) fn command_adapter_batch_artifacts() -> Option<&'static [GerbilCompiledArtifact]> {
    COMMAND_ADAPTER_BATCH_ARTIFACTS
        .get_or_init(|| {
            let compiler = real_gxi_command_adapter_batch_compiler()?;
            Some(
                compiler
                    .compile_requests(command_adapter_batch_requests())
                    .expect("real gxi command adapter batch should compile artifacts"),
            )
        })
        .as_deref()
}

fn empty_contract_facts() -> GerbilWorkspaceContractFacts {
    GerbilWorkspaceContractFacts {
        registry: OrgContractRegistry::default(),
        resolutions: OrgContractResolutionReport::default(),
        validations: OrgContractValidationReport::default(),
    }
}

fn command_adapter_batch_requests() -> Vec<GerbilCompileRequest> {
    vec![
        GerbilCompileRequest {
            source: GerbilSource::new("audit/control-plane", RICH_LOOP_GRAPH_SOURCE),
            expected: GerbilArtifactKind::LoopGraph,
            contract_facts: Some(empty_contract_facts()),
        },
        GerbilCompileRequest {
            source: GerbilSource::new("audit/workspace-schema", WORKSPACE_SCHEMA_SOURCE),
            expected: GerbilArtifactKind::WorkspaceSchema,
            contract_facts: None,
        },
        GerbilCompileRequest {
            source: GerbilSource::new(
                "audit/workspace-patch-intent",
                WORKSPACE_PATCH_INTENT_SOURCE,
            ),
            expected: GerbilArtifactKind::WorkspacePatchIntent,
            contract_facts: None,
        },
        GerbilCompileRequest {
            source: GerbilSource::new("audit/agent-scenario", AGENT_SCENARIO_CONTRACT_SOURCE),
            expected: GerbilArtifactKind::AgentScenarioContract,
            contract_facts: None,
        },
        GerbilCompileRequest {
            source: GerbilSource::new("audit/release-topology", RELEASE_TOPOLOGY_SOURCE),
            expected: GerbilArtifactKind::ReleaseTopology,
            contract_facts: None,
        },
    ]
}

mod artifacts;
mod errors;
mod examples;
mod workflow;
