pub(super) use super::support::{
    AGENT_SCENARIO_CONTRACT_SOURCE, RICH_LOOP_GRAPH_SOURCE, WORKSPACE_PATCH_INTENT_SOURCE,
    WORKSPACE_SCHEMA_SOURCE, WORKSPACE_SOURCE_COMMIT_INTENT_SOURCE,
    assert_agent_scenario_contract_artifact, assert_rich_loop_graph_artifact,
    assert_workspace_patch_intent_artifact, assert_workspace_schema_artifact, local_gxi,
    real_gxi_command_adapter_compiler, real_gxi_module_compiler,
};

mod artifacts;
mod errors;
mod examples;
mod workflow;
