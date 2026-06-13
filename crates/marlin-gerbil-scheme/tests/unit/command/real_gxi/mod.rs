pub(super) use super::support::{
    AGENT_SCENARIO_CONTRACT_SOURCE, MARLIN_REQUIRE_REAL_GXI_ENV, RICH_LOOP_GRAPH_SOURCE,
    WORKSPACE_PATCH_INTENT_SOURCE, WORKSPACE_SCHEMA_SOURCE, WORKSPACE_SOURCE_COMMIT_INTENT_SOURCE,
    assert_agent_scenario_contract_artifact, assert_rich_loop_graph_artifact,
    assert_workspace_patch_intent_artifact, assert_workspace_schema_artifact, local_gxi,
    real_gxi_command_adapter_batch_compiler, real_gxi_module_compiler,
};
use marlin_gerbil_scheme::{
    GerbilArtifactKind, GerbilCompileRequest, GerbilCompiledArtifact, GerbilSource,
    GerbilWorkspaceContractFacts,
};
use marlin_org_model::{
    OrgContract, OrgContractAssertion, OrgContractCompareOp, OrgContractDiagnostic,
    OrgContractDiagnosticSeverity, OrgContractElementCategory, OrgContractElementKind,
    OrgContractExpectation, OrgContractId, OrgContractKind, OrgContractQuery, OrgContractReference,
    OrgContractReferenceScope, OrgContractRegistry, OrgContractResolution,
    OrgContractResolutionReport, OrgContractScope, OrgContractSeverity, OrgContractSourceSpan,
    OrgContractTemplate, OrgContractTemplateEngine, OrgContractTemplateKind,
    OrgContractValidationReceipt, OrgContractValidationReport, OrgContractValidationStatus,
    OrgContractValidationTarget, OrgNodeId, OrgSourceSpan,
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

fn sample_contract_facts() -> GerbilWorkspaceContractFacts {
    let contract_id = OrgContractId::new("agent.task.v1");
    let target_node = OrgNodeId::new("memory.org:1:goal");
    let reference = OrgContractReference {
        raw: "agent.task.v1".to_string(),
        path: None,
        contract_id: Some(contract_id.clone()),
        scope: OrgContractReferenceScope::Subtree,
        target_node: Some(target_node.clone()),
        source: Some(OrgSourceSpan {
            document: "memory.org".to_string(),
            start_byte: 12,
            end_byte: 25,
            start_line: 2,
            end_line: 2,
        }),
    };

    GerbilWorkspaceContractFacts {
        registry: OrgContractRegistry {
            contracts: vec![OrgContract {
                id: contract_id.clone(),
                aliases: vec![OrgContractId::new("task.v1")],
                scope: OrgContractScope::new("Subtree"),
                kind: OrgContractKind::new("OrgElementsAssertions"),
                assertions: vec![OrgContractAssertion {
                    id: "task.has-goal".to_string(),
                    severity: OrgContractSeverity::new("Error"),
                    bindings: Vec::new(),
                    query: OrgContractQuery {
                        category: Some(OrgContractElementCategory::new("Section")),
                        kind: Some(OrgContractElementKind::new("headline")),
                        summary_equals: vec![("title".to_string(), "Goal".to_string())],
                        use_scope_outline_path: true,
                        ..Default::default()
                    },
                    expectation: OrgContractExpectation::Count {
                        op: OrgContractCompareOp::Ge,
                        expected: 1,
                    },
                    message: Some("Task must contain a Goal section.".to_string()),
                    fix: None,
                    templates: vec![OrgContractTemplate {
                        kind: OrgContractTemplateKind::Message,
                        engine: OrgContractTemplateEngine::new("jinja2"),
                        body: "Task `{{ scope.title }}` must contain a Goal section.".to_string(),
                        source: Some(OrgContractSourceSpan {
                            start_line: 20,
                            start_column: 1,
                            end_line: 22,
                            end_column: 1,
                            start_byte: 100,
                            end_byte: 160,
                        }),
                    }],
                    query_source: Some(OrgContractSourceSpan {
                        start_line: 9,
                        start_column: 1,
                        end_line: 14,
                        end_column: 1,
                        start_byte: 40,
                        end_byte: 90,
                    }),
                    expect_source: Some(OrgContractSourceSpan {
                        start_line: 16,
                        start_column: 1,
                        end_line: 18,
                        end_column: 1,
                        start_byte: 91,
                        end_byte: 99,
                    }),
                }],
            }],
        },
        resolutions: OrgContractResolutionReport {
            references: vec![OrgContractResolution {
                reference: reference.clone(),
                resolved_contract_id: Some(contract_id.clone()),
            }],
            diagnostics: Vec::new(),
        },
        validations: OrgContractValidationReport {
            receipts: vec![OrgContractValidationReceipt {
                contract_id,
                assertion_id: "task.has-goal".to_string(),
                target: OrgContractValidationTarget::Node(target_node.clone()),
                status: OrgContractValidationStatus::Failed,
                severity: OrgContractSeverity::new("Error"),
                message: Some("Task must contain a Goal section.".to_string()),
                matched_nodes: Vec::new(),
                skip_reason: None,
                source: Some(OrgSourceSpan {
                    document: "memory.org".to_string(),
                    start_byte: 0,
                    end_byte: 80,
                    start_line: 1,
                    end_line: 5,
                }),
            }],
            diagnostics: vec![OrgContractDiagnostic {
                code: "ORG-CONTRACT-TEST".to_string(),
                severity: OrgContractDiagnosticSeverity::Warning,
                message: "sample contract facts crossed the real gxi command boundary".to_string(),
                reference,
            }],
        },
    }
}

fn command_adapter_batch_requests() -> Vec<GerbilCompileRequest> {
    vec![
        GerbilCompileRequest::with_contract_facts(
            GerbilSource::new("audit/control-plane", RICH_LOOP_GRAPH_SOURCE),
            GerbilArtifactKind::LoopGraph,
            sample_contract_facts(),
        ),
        GerbilCompileRequest::new(
            GerbilSource::new("audit/workspace-schema", WORKSPACE_SCHEMA_SOURCE),
            GerbilArtifactKind::WorkspaceSchema,
        ),
        GerbilCompileRequest::with_contract_facts(
            GerbilSource::new(
                "audit/workspace-patch-intent",
                WORKSPACE_PATCH_INTENT_SOURCE,
            ),
            GerbilArtifactKind::WorkspacePatchIntent,
            sample_contract_facts(),
        ),
        GerbilCompileRequest::new(
            GerbilSource::new("audit/agent-scenario", AGENT_SCENARIO_CONTRACT_SOURCE),
            GerbilArtifactKind::AgentScenarioContract,
        ),
        GerbilCompileRequest::new(
            GerbilSource::new("audit/release-topology", RELEASE_TOPOLOGY_SOURCE),
            GerbilArtifactKind::ReleaseTopology,
        ),
        GerbilCompileRequest::new(
            GerbilSource::new(
                "audit/workspace-source-commit",
                WORKSPACE_SOURCE_COMMIT_INTENT_SOURCE,
            ),
            GerbilArtifactKind::WorkspacePatchIntent,
        ),
    ]
}

mod artifacts;
mod errors;
mod examples;
mod workflow;
