// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use marlin_agent_context::org::MemoryOrgWorkspace;
use marlin_context_org_model::{
    OrgContract, OrgContractId, OrgContractKind, OrgContractRegistry, OrgContractScope,
    OrgContractSeverity, OrgContractValidationReceipt, OrgContractValidationReport,
    OrgContractValidationStatus, OrgContractValidationTarget, OrgNode, OrgNodeId,
};
use marlin_workspace_status::WorkspaceTarget;
use marlin_workspace_view::{RenderedContractFacts, RenderedContractFactsInput};

// The producer owns parsing and evaluation. This case tests only host fact
// accumulation, including preservation of a failed producer receipt.
#[test]
fn sequential_projections_accumulate_contract_facts_without_reevaluation() {
    let workspace = MemoryOrgWorkspace::new();
    for (id, status) in [
        ("agent.one.v1", OrgContractValidationStatus::Passed),
        ("agent.two.v1", OrgContractValidationStatus::Failed),
    ] {
        let contract_id = OrgContractId::new(id);
        let facts = RenderedContractFacts::from_input(RenderedContractFactsInput {
            registry: OrgContractRegistry {
                contracts: vec![OrgContract {
                    id: contract_id.clone(),
                    aliases: vec![],
                    scope: OrgContractScope::new("document"),
                    kind: OrgContractKind::new("org-elements"),
                    assertions: vec![],
                }],
            },
            validations: OrgContractValidationReport {
                receipts: vec![OrgContractValidationReceipt {
                    contract_id,
                    assertion_id: "producer-check".into(),
                    target: OrgContractValidationTarget::Document,
                    status,
                    severity: OrgContractSeverity::new("error"),
                    message: None,
                    matched_nodes: vec![],
                    skip_reason: None,
                    source: None,
                }],
                diagnostics: vec![],
            },
            ..Default::default()
        });
        let node = OrgNode::heading(OrgNodeId::new(id), id);
        assert_eq!(workspace.apply_projection([node], facts).unwrap().len(), 1);
    }
    let status = workspace.status(WorkspaceTarget::Workspace).unwrap();
    let contracts = status.contracts.unwrap();
    assert_eq!(contracts.registry.contracts.len(), 2);
    assert_eq!(contracts.validation_receipts, 2);
    assert_eq!(contracts.validation_passed, 1);
    assert_eq!(contracts.validation_failed, 1);
}
