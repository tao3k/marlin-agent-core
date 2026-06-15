//! Runtime-independent Org contract validation receipts.

use marlin_org_model::{
    OrgContractId, OrgContractReference, OrgContractReferenceScope, OrgContractResolutionReport,
    OrgContractSeverity, OrgContractValidationReceipt, OrgContractValidationReport,
    OrgContractValidationStatus, OrgContractValidationTarget, OrgNode, OrgNodeId, OrgSourceSpan,
};
use orgize::ast::{
    Document, OrgContract, OrgContractAssertionEvaluation, OrgContractAssertionStatus,
    OrgContractEvaluationScope, OrgContractRegistry, ParsedAnnotation, evaluate_org_contract,
};

pub(crate) fn validate_contract_references(
    document: &Document<ParsedAnnotation>,
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
        let scope = evaluation_scope(nodes, &target);
        let evaluation = evaluate_org_contract(document, contract, scope);

        for assertion in evaluation.assertions {
            report.receipts.push(contract_validation_receipt(
                contract.id.as_str(),
                assertion,
                &target,
                &scoped_nodes,
                nodes,
                document,
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
        contract.id == contract_id.as_str()
            || contract
                .aliases
                .iter()
                .any(|alias| alias == contract_id.as_str())
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

fn contract_validation_receipt(
    contract_id: &str,
    assertion: OrgContractAssertionEvaluation,
    target: &OrgContractValidationTarget,
    scoped_nodes: &[&OrgNode],
    nodes: &[OrgNode],
    document: &Document<ParsedAnnotation>,
) -> OrgContractValidationReceipt {
    OrgContractValidationReceipt {
        contract_id: OrgContractId::from(contract_id.to_owned()),
        assertion_id: assertion.assertion_id,
        target: target.clone(),
        status: validation_status(assertion.status),
        severity: OrgContractSeverity::new(format!("{:?}", assertion.severity)),
        message: assertion.message_template,
        matched_nodes: matched_node_ids(nodes, document, &assertion.matched_ids),
        skip_reason: None,
        source: validation_target_source(target, scoped_nodes),
    }
}

fn validation_status(status: OrgContractAssertionStatus) -> OrgContractValidationStatus {
    match status {
        OrgContractAssertionStatus::Passed => OrgContractValidationStatus::Passed,
        OrgContractAssertionStatus::Failed => OrgContractValidationStatus::Failed,
    }
}

fn matched_node_ids(
    nodes: &[OrgNode],
    document: &Document<ParsedAnnotation>,
    matched_ids: &[orgize::ast::OrgElementId],
) -> Vec<OrgNodeId> {
    let graph = document.org_elements_graph();
    matched_ids
        .iter()
        .filter_map(|matched_id| graph.record(*matched_id))
        .filter_map(|record| {
            let start_byte = u32::from(record.ann.range.start()) as usize;
            let end_byte = u32::from(record.ann.range.end()) as usize;
            nodes
                .iter()
                .find(|node| {
                    node.source.as_ref().is_some_and(|source| {
                        source.start_byte == start_byte && source.end_byte == end_byte
                    })
                })
                .map(|node| node.id.clone())
        })
        .collect()
}

fn evaluation_scope(
    nodes: &[OrgNode],
    target: &OrgContractValidationTarget,
) -> OrgContractEvaluationScope {
    match target {
        OrgContractValidationTarget::Document => OrgContractEvaluationScope::document(),
        OrgContractValidationTarget::Node(root) => nodes
            .iter()
            .find(|node| node.id == *root)
            .and_then(|node| {
                let source = node.source.as_ref()?;
                Some(OrgContractEvaluationScope::section(
                    node.title.clone().unwrap_or_default(),
                    outline_path_for_node(nodes, root),
                    orgize::TextRange::new(
                        (source.start_byte as u32).into(),
                        (source.end_byte as u32).into(),
                    ),
                ))
            })
            .unwrap_or_else(OrgContractEvaluationScope::document),
    }
}

fn outline_path_for_node(nodes: &[OrgNode], root: &OrgNodeId) -> Vec<String> {
    let mut path = Vec::new();
    for node in root_nodes(nodes) {
        if collect_outline_path_from(nodes, &node.id, root, &mut path) {
            return path;
        }
    }
    Vec::new()
}

fn root_nodes(nodes: &[OrgNode]) -> Vec<&OrgNode> {
    nodes
        .iter()
        .filter(|node| {
            !nodes
                .iter()
                .any(|candidate| candidate.children.iter().any(|child| child == &node.id))
        })
        .collect()
}

fn collect_outline_path_from(
    nodes: &[OrgNode],
    current: &OrgNodeId,
    target: &OrgNodeId,
    path: &mut Vec<String>,
) -> bool {
    let Some(node) = nodes.iter().find(|node| node.id == *current) else {
        return false;
    };
    path.push(node.title.clone().unwrap_or_default());
    if node.id == *target {
        return true;
    }
    if node
        .children
        .iter()
        .any(|child| collect_outline_path_from(nodes, child, target, path))
    {
        return true;
    }
    path.pop();
    false
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
