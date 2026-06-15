use super::support::loop_graph_artifact;
use marlin_agent_protocol::{AGENT_SCENARIO_CONTRACT_SCHEMA_ID, AgentEventTopic, AgentSpanName};
use marlin_gerbil_scheme::{
    GerbilArtifactKind, GerbilCompileRequest, GerbilCompiledArtifact, GerbilSource,
    GerbilWorkspaceContractFacts,
};
use marlin_org_model::{
    OrgContract, OrgContractAssertion, OrgContractCompareOp, OrgContractExpectation, OrgContractId,
    OrgContractKind, OrgContractQuery, OrgContractRegistry, OrgContractResolutionReport,
    OrgContractScope, OrgContractSeverity, OrgContractValidationReceipt,
    OrgContractValidationReport, OrgContractValidationSkipReason, OrgContractValidationStatus,
    OrgContractValidationTarget,
};
use marlin_workspace_status::ContractStatus;
use marlin_workspace_view::{RenderedContractFacts, RenderedContractFactsInput};
use serde_json::json;

#[test]
fn rust_compile_request_round_trips_contract_facts() {
    let request = GerbilCompileRequest {
        source: GerbilSource::new("audit/control-plane", "(module audit/control-plane)"),
        expected: GerbilArtifactKind::LoopGraph,
        contract_facts: Some(GerbilWorkspaceContractFacts {
            registry: contract_registry(),
            resolutions: OrgContractResolutionReport::default(),
            validations: OrgContractValidationReport::default(),
        }),
    };

    let encoded = serde_json::to_string(&request).expect("request should encode");
    assert!(encoded.contains("\"expected\":\"LoopGraph\""));
    assert!(encoded.contains("\"kind\":\"count\""));
    assert!(encoded.contains("\"op\":\"Ge\""));
    assert!(encoded.contains("\"expected\":1"));

    let decoded: GerbilCompileRequest =
        serde_json::from_str(&encoded).expect("request should decode");

    assert_eq!(decoded, request);
    assert!(decoded.contract_facts.is_some());
}

#[test]
fn rust_compile_request_preserves_view_status_contract_facts() {
    let rendered = RenderedContractFacts::from_input(RenderedContractFactsInput {
        registry: contract_registry(),
        validations: validation_report_with_skip_reason(),
        ..Default::default()
    });
    let status = ContractStatus {
        resolved_references: rendered.summary.resolved_references,
        unresolved_references: rendered.summary.unresolved_references,
        diagnostics: rendered.summary.diagnostics,
        templates: rendered.summary.templates,
        contract_assertions: rendered.summary.contract_assertions,
        validation_receipts: rendered.summary.validation_receipts,
        validation_passed: rendered.summary.validation_passed,
        validation_failed: rendered.summary.validation_failed,
        validation_skipped: rendered.summary.validation_skipped,
        validation_matched_nodes: rendered.summary.validation_matched_nodes,
        validation_matched_node_ids: rendered.summary.validation_matched_node_ids.clone(),
        validation_skip_reasons: rendered.summary.validation_skip_reasons.clone(),
        reference_resolutions: rendered.resolutions.clone(),
        diagnostic_records: rendered.diagnostics.clone(),
        template_records: rendered.templates.clone(),
        registry: rendered.registry.clone(),
        validation_report: rendered.validations.clone(),
        contract_expectation_summaries: rendered.summary.contract_expectation_summaries.clone(),
        rendered_summary: rendered.rendered_lines.clone(),
    };

    assert_eq!(
        status.contract_expectation_summaries,
        ["agent.task.v1/task.has-goal: count >= 1"]
    );
    assert_eq!(
        status.validation_skip_reasons,
        ["agent.task.v1/task.legacy-check: unsupported expectation: legacy predicate"]
    );
    assert!(
        status.rendered_summary.iter().any(|line| line
            == "contract.validation.expectation: agent.task.v1/task.has-goal: count >= 1")
    );

    let request = GerbilCompileRequest {
        source: GerbilSource::new("audit/control-plane", "(module audit/control-plane)"),
        expected: GerbilArtifactKind::LoopGraph,
        contract_facts: Some(GerbilWorkspaceContractFacts {
            registry: status.registry.clone(),
            resolutions: OrgContractResolutionReport {
                references: status.reference_resolutions.clone(),
                diagnostics: status.diagnostic_records.clone(),
            },
            validations: status.validation_report.clone(),
        }),
    };

    let encoded = serde_json::to_value(&request).expect("request should encode");
    assert_eq!(
        encoded["contract_facts"]["registry"]["contracts"][0]["assertions"][0]["expectation"],
        json!({
            "kind": "count",
            "op": "Ge",
            "expected": 1
        })
    );
    assert_eq!(
        encoded["contract_facts"]["validations"]["receipts"][0]["skip_reason"],
        json!({
            "kind": "unsupported_expectation",
            "expectation": "legacy predicate"
        })
    );

    let decoded: GerbilCompileRequest =
        serde_json::from_value(encoded).expect("request should decode");
    let contract_facts = decoded.contract_facts.expect("contract facts");
    assert_eq!(contract_facts.registry, status.registry);
    assert_eq!(contract_facts.validations, status.validation_report);
}

#[test]
fn compiled_artifact_decodes_agent_scenario_contract_and_ensures_kind() {
    let artifact: GerbilCompiledArtifact = serde_json::from_str(
        r#"{"AgentScenarioContract":{"schema_id":"marlin.agent.scenario.v1","scenario":{"id":"gerbil-scenario","description":"from gerbil","steps":[{"name":"run","input":{"path":"LOOP.org"},"expected_event_topics":["kernel.execution"],"expected_span_names":["harness.execution"]}],"expected_evidence":["Runtime"]}}}"#,
    )
    .expect("agent scenario contract artifact should decode");

    let artifact = artifact
        .ensure_kind(GerbilArtifactKind::AgentScenarioContract)
        .expect("agent scenario contract artifact should match requested kind");

    match artifact {
        GerbilCompiledArtifact::AgentScenarioContract(contract) => {
            assert_eq!(contract.schema_id, AGENT_SCENARIO_CONTRACT_SCHEMA_ID);
            assert!(contract.is_supported_schema());
            assert_eq!(contract.scenario.id, "gerbil-scenario");
            assert_eq!(
                contract.scenario.description.as_deref(),
                Some("from gerbil")
            );
            assert_eq!(
                contract.scenario.steps[0]
                    .input
                    .get("path")
                    .map(String::as_str),
                Some("LOOP.org")
            );
            assert_eq!(
                contract.scenario.steps[0].expected_event_topics,
                vec![AgentEventTopic::new("kernel.execution")]
            );
            assert_eq!(
                contract.scenario.steps[0].expected_span_names,
                vec![AgentSpanName::new("harness.execution")]
            );
        }
        other => panic!("expected agent scenario contract artifact, got {other:?}"),
    }
}

#[test]
fn compiled_artifact_carries_typed_kind() {
    let artifact = loop_graph_artifact("response-loop");

    assert_eq!(artifact.kind(), GerbilArtifactKind::LoopGraph);
}

#[test]
fn compiled_artifact_decodes_workspace_schema_and_ensures_kind() {
    let artifact: GerbilCompiledArtifact = serde_json::from_str(
        r#"{"WorkspaceSchema":{"schema_id":"workspace-record","required_properties":["ID","TITLE"],"todo_states":["TODO","DONE"]}}"#,
    )
    .expect("workspace schema artifact should decode");

    let artifact = artifact
        .ensure_kind(GerbilArtifactKind::WorkspaceSchema)
        .expect("workspace schema artifact should match requested kind");

    match artifact {
        GerbilCompiledArtifact::WorkspaceSchema(schema) => {
            assert_eq!(schema.schema_id, "workspace-record");
            assert_eq!(schema.required_properties, ["ID", "TITLE"]);
            assert_eq!(schema.todo_states, ["TODO", "DONE"]);
        }
        other => panic!("expected workspace schema artifact, got {other:?}"),
    }
}

fn contract_registry() -> OrgContractRegistry {
    OrgContractRegistry {
        contracts: vec![OrgContract {
            id: OrgContractId::new("agent.task.v1"),
            aliases: Vec::new(),
            scope: OrgContractScope::new("Subtree"),
            kind: OrgContractKind::new("OrgElementsAssertions"),
            assertions: vec![OrgContractAssertion {
                id: "task.has-goal".to_string(),
                severity: OrgContractSeverity::new("Error"),
                bindings: Vec::new(),
                query: OrgContractQuery::default(),
                expectation: OrgContractExpectation::Count {
                    op: OrgContractCompareOp::Ge,
                    expected: 1,
                },
                message: None,
                fix: None,
                templates: Vec::new(),
                query_source: None,
                expect_source: None,
            }],
        }],
    }
}

fn validation_report_with_skip_reason() -> OrgContractValidationReport {
    OrgContractValidationReport {
        receipts: vec![OrgContractValidationReceipt {
            contract_id: OrgContractId::new("agent.task.v1"),
            assertion_id: "task.legacy-check".to_string(),
            target: OrgContractValidationTarget::Document,
            status: OrgContractValidationStatus::Skipped,
            severity: OrgContractSeverity::new("Warning"),
            message: None,
            matched_nodes: Vec::new(),
            skip_reason: Some(OrgContractValidationSkipReason::UnsupportedExpectation {
                expectation: "legacy predicate".to_string(),
            }),
            source: None,
        }],
        diagnostics: Vec::new(),
    }
}
