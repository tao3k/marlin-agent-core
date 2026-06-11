//! Runtime-independent Org contract validation receipts.

use marlin_org_model::{
    OrgContract, OrgContractAssertion, OrgContractExpectation, OrgContractId, OrgContractQuery,
    OrgContractReference, OrgContractReferenceScope, OrgContractRegistry,
    OrgContractResolutionReport, OrgContractValidationReceipt, OrgContractValidationReport,
    OrgContractValidationStatus, OrgContractValidationTarget, OrgNode, OrgSourceSpan, TodoState,
};

pub(crate) fn validate_contract_references(
    nodes: &[OrgNode],
    contracts: &OrgContractRegistry,
    resolutions: &OrgContractResolutionReport,
) -> OrgContractValidationReport {
    let mut report = OrgContractValidationReport {
        receipts: Vec::new(),
        diagnostics: resolutions.diagnostics.clone(),
    };

    for resolution in &resolutions.references {
        let Some(contract_id) = resolution.resolved_contract_id.as_ref() else {
            continue;
        };
        let Some(contract) = contract_by_id(contracts, contract_id) else {
            continue;
        };
        let target = validation_target(&resolution.reference);
        let scoped_nodes = scoped_validation_nodes(nodes, &target);

        for assertion in &contract.assertions {
            report.receipts.push(validate_contract_assertion(
                contract,
                assertion,
                &target,
                &scoped_nodes,
            ));
        }
    }

    report
}

fn contract_by_id<'a>(
    contracts: &'a OrgContractRegistry,
    contract_id: &OrgContractId,
) -> Option<&'a OrgContract> {
    contracts.contracts.iter().find(|contract| {
        &contract.id == contract_id || contract.aliases.iter().any(|alias| alias == contract_id)
    })
}

fn validation_target(reference: &OrgContractReference) -> OrgContractValidationTarget {
    match reference.scope {
        OrgContractReferenceScope::Document => OrgContractValidationTarget::Document,
        OrgContractReferenceScope::Subtree => reference
            .target_node
            .clone()
            .map(OrgContractValidationTarget::Node)
            .unwrap_or(OrgContractValidationTarget::Document),
    }
}

fn scoped_validation_nodes<'a>(
    nodes: &'a [OrgNode],
    target: &OrgContractValidationTarget,
) -> Vec<&'a OrgNode> {
    match target {
        OrgContractValidationTarget::Document => nodes.iter().collect(),
        OrgContractValidationTarget::Node(root) => {
            let mut scoped = Vec::new();
            let mut pending = vec![root.clone()];
            while let Some(node_id) = pending.pop() {
                if let Some(node) = nodes.iter().find(|node| node.id == node_id) {
                    pending.extend(node.children.iter().cloned());
                    scoped.push(node);
                }
            }
            scoped
        }
    }
}

fn validate_contract_assertion(
    contract: &OrgContract,
    assertion: &OrgContractAssertion,
    target: &OrgContractValidationTarget,
    scoped_nodes: &[&OrgNode],
) -> OrgContractValidationReceipt {
    let matched_nodes = scoped_nodes
        .iter()
        .filter(|node| node_matches_contract_query(node, &assertion.query))
        .map(|node| node.id.clone())
        .collect::<Vec<_>>();
    let matched = matched_nodes.len();
    let status = count_at_least(&assertion.expectation)
        .map(|minimum| {
            if matched >= minimum {
                OrgContractValidationStatus::Passed
            } else {
                OrgContractValidationStatus::Failed
            }
        })
        .unwrap_or(OrgContractValidationStatus::Skipped);

    OrgContractValidationReceipt {
        contract_id: contract.id.clone(),
        assertion_id: assertion.id.clone(),
        target: target.clone(),
        status,
        severity: assertion.severity.clone(),
        message: assertion.message.clone(),
        matched_nodes,
        source: validation_target_source(target, scoped_nodes),
    }
}

fn count_at_least(expectation: &OrgContractExpectation) -> Option<usize> {
    let label = expectation.as_str();
    let lower = label.to_ascii_lowercase();
    if !lower.contains("count") {
        return None;
    }

    let start = label.find(|character: char| character.is_ascii_digit())?;
    let value = label[start..]
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .collect::<String>();
    value.parse().ok()
}

fn node_matches_contract_query(node: &OrgNode, query: &OrgContractQuery) -> bool {
    query
        .summary_equals
        .iter()
        .all(|(key, value)| node_summary_value(node, key).is_some_and(|actual| actual == value))
        && query.summary_contains.iter().all(|(key, value)| {
            node_summary_value(node, key).is_some_and(|actual| actual.contains(value))
        })
        && query.property_equals.iter().all(|(key, value)| {
            node.properties
                .get(key)
                .is_some_and(|actual| actual == value)
        })
        && query.property_contains.iter().all(|(key, value)| {
            node.properties
                .get(key)
                .is_some_and(|actual| actual.contains(value))
        })
}

fn node_summary_value<'a>(node: &'a OrgNode, key: &str) -> Option<&'a str> {
    match key {
        "title" => node.title.as_deref(),
        "todo" => node.todo.as_ref().map(todo_state_value),
        _ => None,
    }
}

fn todo_state_value(state: &TodoState) -> &str {
    match state {
        TodoState::Todo => "TODO",
        TodoState::Next => "NEXT",
        TodoState::Wait => "WAIT",
        TodoState::Blocked => "BLOCKED",
        TodoState::Done => "DONE",
        TodoState::Custom(value) => value.as_str(),
    }
}

fn validation_target_source(
    target: &OrgContractValidationTarget,
    scoped_nodes: &[&OrgNode],
) -> Option<OrgSourceSpan> {
    match target {
        OrgContractValidationTarget::Document => scoped_nodes.first().and_then(|node| {
            let first = node.source.as_ref()?;
            let last = scoped_nodes
                .iter()
                .filter_map(|node| node.source.as_ref())
                .max_by_key(|source| source.end_byte)
                .unwrap_or(first);
            Some(OrgSourceSpan {
                document: first.document.clone(),
                start_byte: first.start_byte,
                end_byte: last.end_byte,
                start_line: first.start_line,
                end_line: last.end_line,
            })
        }),
        OrgContractValidationTarget::Node(root) => scoped_nodes
            .iter()
            .find(|node| node.id == *root)
            .and_then(|node| node.source.clone()),
    }
}
