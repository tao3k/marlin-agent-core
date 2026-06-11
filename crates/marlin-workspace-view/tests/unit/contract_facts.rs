use marlin_org_model::{
    OrgContract, OrgContractAssertion, OrgContractCompareOp, OrgContractExpectation, OrgContractId,
    OrgContractKind, OrgContractQuery, OrgContractRegistry, OrgContractScope, OrgContractSeverity,
    OrgContractValidationReceipt, OrgContractValidationReport, OrgContractValidationSkipReason,
    OrgContractValidationStatus, OrgContractValidationTarget,
};
use marlin_workspace_view::{RenderedContractFacts, RenderedContractFactsInput};
use serde_json::json;

#[test]
fn rendered_contract_facts_json_carries_typed_registry_expectations() {
    let facts = RenderedContractFacts::from_input(RenderedContractFactsInput {
        registry: contract_registry(),
        validations: validation_report_with_skip_reason(),
        ..Default::default()
    });

    let encoded = serde_json::to_value(&facts).expect("contract facts encode");

    assert_eq!(
        encoded["registry"]["contracts"][0]["assertions"][0]["expectation"],
        json!({
            "kind": "count",
            "op": "Ge",
            "expected": 1
        })
    );
    assert_eq!(encoded["summary"]["contract_assertions"], json!(1));
    assert_eq!(
        encoded["summary"]["contract_expectation_summaries"],
        json!(["agent.task.v1/task.has-goal: count >= 1"])
    );
    assert_eq!(
        encoded["validations"]["receipts"][0]["skip_reason"],
        json!({
            "kind": "unsupported_expectation",
            "expectation": "legacy predicate"
        })
    );
    assert_eq!(
        encoded["summary"]["validation_skip_reasons"],
        json!(["agent.task.v1/task.legacy-check: unsupported expectation: legacy predicate"])
    );
    assert!(
        encoded["rendered_lines"]
            .as_array()
            .expect("rendered lines")
            .iter()
            .any(|line| line.as_str()
                == Some(
                    "contract.validation.expectation: agent.task.v1/task.has-goal: count >= 1"
                ))
    );
    assert!(
        encoded["rendered_lines"]
            .as_array()
            .expect("rendered lines")
            .iter()
            .any(|line| line.as_str()
                == Some(
                    "contract.validation.skip_reason: agent.task.v1/task.legacy-check: unsupported expectation: legacy predicate"
                ))
    );

    let decoded: RenderedContractFacts =
        serde_json::from_value(encoded).expect("contract facts decode");
    assert_eq!(decoded, facts);
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
