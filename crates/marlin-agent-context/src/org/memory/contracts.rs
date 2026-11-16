// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Contract fact accumulation helpers for `MemoryOrgWorkspace`.

use std::collections::BTreeSet;

use marlin_context_org_model::{OrgContractRegistry, OrgContractTemplate};
use marlin_workspace_view::RenderedContractFacts;

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
