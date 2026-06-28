//! Contract fact accumulation helpers for `MemoryOrgWorkspace`.

use std::collections::BTreeSet;

use marlin_org_model::{
    OrgContractRegistry, OrgContractResolutionReport, OrgContractTemplate,
    OrgContractValidationReport,
};
use marlin_org_workspace::OrgDocumentWorkspace;
use marlin_workspace_view::{RenderedContractFacts, RenderedContractFactsInput};

pub(super) fn merge_document_workspace(
    target: &mut OrgDocumentWorkspace,
    workspace: OrgDocumentWorkspace,
) {
    append_contracts_unique(&mut target.contracts, workspace.contracts);
    target.nodes.extend(workspace.nodes);
    target
        .contract_resolutions
        .references
        .extend(workspace.contract_resolutions.references);
    target
        .contract_resolutions
        .diagnostics
        .extend(workspace.contract_resolutions.diagnostics);
    target
        .contract_validations
        .receipts
        .extend(workspace.contract_validations.receipts);
    target
        .contract_validations
        .diagnostics
        .extend(workspace.contract_validations.diagnostics);
}

pub(super) fn contract_facts_from_workspace(
    registry: OrgContractRegistry,
    resolutions: OrgContractResolutionReport,
    validations: OrgContractValidationReport,
) -> RenderedContractFacts {
    let templates = contract_templates_from_registry(&registry);

    RenderedContractFacts::from_input(RenderedContractFactsInput {
        registry,
        resolutions: resolutions.references,
        diagnostics: resolutions.diagnostics,
        templates,
        validations,
    })
}

pub(super) fn merge_contract_facts(
    target: &mut RenderedContractFacts,
    incoming: RenderedContractFacts,
) {
    append_contracts_unique(&mut target.registry, incoming.registry);
    target.resolutions.extend(incoming.resolutions);
    target.diagnostics.extend(incoming.diagnostics);
    target.templates = contract_templates_from_registry(&target.registry);
    target
        .validations
        .receipts
        .extend(incoming.validations.receipts);
    target
        .validations
        .diagnostics
        .extend(incoming.validations.diagnostics);
    target.refresh_summary();
}

fn append_contracts_unique(target: &mut OrgContractRegistry, incoming: OrgContractRegistry) {
    let mut existing_contract_ids = target
        .contracts
        .iter()
        .map(|contract| contract.id.clone())
        .collect::<BTreeSet<_>>();
    for contract in incoming.contracts {
        if existing_contract_ids.insert(contract.id.clone()) {
            target.contracts.push(contract);
        }
    }
}

fn contract_templates_from_registry(registry: &OrgContractRegistry) -> Vec<OrgContractTemplate> {
    registry
        .contracts
        .iter()
        .flat_map(|contract| contract.assertions.iter())
        .flat_map(|assertion| assertion.templates.iter().cloned())
        .collect()
}
