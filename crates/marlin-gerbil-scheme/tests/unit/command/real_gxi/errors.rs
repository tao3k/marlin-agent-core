use super::{
    RICH_LOOP_GRAPH_SOURCE, WORKSPACE_SCHEMA_SOURCE, real_gxi_command_adapter_batch_compiler,
};
use marlin_gerbil_scheme::{
    GerbilArtifactKind, GerbilCompileRequest, GerbilCompiledArtifact, GerbilSource,
};
use std::sync::OnceLock;

static COMMAND_ADAPTER_BATCH_ERRORS: OnceLock<Option<Vec<Result<GerbilCompiledArtifact, String>>>> =
    OnceLock::new();

fn command_adapter_batch_errors() -> Option<&'static [Result<GerbilCompiledArtifact, String>]> {
    COMMAND_ADAPTER_BATCH_ERRORS
        .get_or_init(|| {
            let compiler = real_gxi_command_adapter_batch_compiler()?;
            Some(
                compiler
                    .compile_request_results(command_adapter_error_requests())
                    .expect("real gxi command adapter batch should return request results"),
            )
        })
        .as_deref()
}

fn command_adapter_batch_error(index: usize) -> Option<&'static String> {
    let result = command_adapter_batch_errors()?
        .get(index)
        .expect("batch error result should exist");
    Some(result.as_ref().expect_err("request should fail"))
}

fn command_adapter_error_requests() -> Vec<GerbilCompileRequest> {
    vec![
        GerbilCompileRequest {
            source: GerbilSource::new("audit/control-plane", RICH_LOOP_GRAPH_SOURCE),
            expected: GerbilArtifactKind::WorkspaceSchema,
            contract_facts: None,
        },
        GerbilCompileRequest {
            source: GerbilSource::new("audit/workspace-schema", WORKSPACE_SCHEMA_SOURCE),
            expected: GerbilArtifactKind::LoopGraph,
            contract_facts: None,
        },
        GerbilCompileRequest {
            source: GerbilSource::new("audit/control-plane", RICH_LOOP_GRAPH_SOURCE),
            expected: GerbilArtifactKind::WorkspaceViewPolicy,
            contract_facts: None,
        },
        GerbilCompileRequest {
            source: GerbilSource::new("audit/control-plane", RICH_LOOP_GRAPH_SOURCE),
            expected: GerbilArtifactKind::WorkspacePatchIntent,
            contract_facts: None,
        },
        GerbilCompileRequest {
            source: GerbilSource::new("audit/control-plane", RICH_LOOP_GRAPH_SOURCE),
            expected: GerbilArtifactKind::AgentScenarioContract,
            contract_facts: None,
        },
    ]
}

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn command_compiler_real_gxi_workspace_schema_rejects_loop_graph_source() {
    let Some(error) = command_adapter_batch_error(0) else {
        return;
    };

    assert!(error.contains("gerbil compiler command failed"));
    assert!(error.contains("expected workspace-schema form"));
    assert!(error.contains("loop-graph"));
}

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn command_compiler_real_gxi_loop_graph_rejects_workspace_schema_source() {
    let Some(error) = command_adapter_batch_error(1) else {
        return;
    };

    assert!(error.contains("gerbil compiler command failed"));
    assert!(error.contains("expected loop-graph form"));
    assert!(error.contains("workspace-schema"));
}

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn command_compiler_real_gxi_module_entry_rejects_unsupported_expected_kind() {
    let Some(error) = command_adapter_batch_error(2) else {
        return;
    };

    assert!(error.contains("gerbil compiler command failed"));
    assert!(error.contains("unsupported artifact kind"));
    assert!(error.contains("LoopGraph"));
    assert!(error.contains("WorkspaceSchema"));
    assert!(error.contains("WorkspacePatchIntent"));
    assert!(error.contains("AgentScenarioContract"));
    assert!(error.contains("WorkspaceViewPolicy"));
}

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn command_compiler_real_gxi_workspace_patch_intent_rejects_loop_graph_source() {
    let Some(error) = command_adapter_batch_error(3) else {
        return;
    };

    assert!(error.contains("gerbil compiler command failed"));
    assert!(error.contains("expected workspace-patch-intent form"));
    assert!(error.contains("loop-graph"));
}

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn command_compiler_real_gxi_agent_scenario_contract_rejects_loop_graph_source() {
    let Some(error) = command_adapter_batch_error(4) else {
        return;
    };

    assert!(error.contains("gerbil compiler command failed"));
    assert!(error.contains("expected agent-scenario-contract form"));
    assert!(error.contains("loop-graph"));
}
