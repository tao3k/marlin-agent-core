//! Contract projection and `CONTRACT_ORG` reference resolution.

use marlin_org_model::{
    OrgContract, OrgContractAssertion, OrgContractBinding, OrgContractDiagnostic,
    OrgContractDiagnosticSeverity, OrgContractElementCategory, OrgContractElementKind,
    OrgContractExpectation, OrgContractId, OrgContractKind, OrgContractQuery, OrgContractReference,
    OrgContractReferenceScope, OrgContractRegistry, OrgContractRelativeScope,
    OrgContractResolution, OrgContractResolutionReport, OrgContractScope, OrgContractSeverity,
    OrgContractSourceSpan, OrgContractTemplate, OrgContractTemplateEngine, OrgContractTemplateKind,
    OrgNodeId, OrgSourceSpan,
};
use orgize::ast::{Document, ParsedAnnotation, parse_contract_reference};

pub(crate) fn project_contract_registry(
    registry: orgize::ast::OrgContractRegistry,
) -> OrgContractRegistry {
    OrgContractRegistry {
        contracts: registry.contracts.iter().map(project_contract).collect(),
    }
}

pub(crate) fn document_contract_reference(
    document: &Document<ParsedAnnotation>,
    document_id: &str,
    document_text: &str,
) -> Option<OrgContractReference> {
    document
        .metadata
        .iter()
        .rev()
        .find(|keyword| keyword.key.eq_ignore_ascii_case("CONTRACT_ORG"))
        .map(|keyword| {
            contract_reference(
                keyword.value.as_str(),
                OrgContractReferenceScope::Document,
                None,
                Some(source_span(
                    document_id,
                    document_text,
                    text_size_offset(keyword.ann.range.start()),
                    text_size_offset(keyword.ann.range.end()),
                )),
            )
        })
}

pub(crate) fn contract_reference(
    value: &str,
    scope: OrgContractReferenceScope,
    target_node: Option<OrgNodeId>,
    source: Option<OrgSourceSpan>,
) -> OrgContractReference {
    let reference = parse_contract_reference(value);
    OrgContractReference {
        raw: reference.raw,
        path: reference.path,
        contract_id: reference.contract_id.map(OrgContractId::from),
        scope,
        target_node,
        source,
    }
}

pub(crate) fn resolve_contract_references(
    references: Vec<OrgContractReference>,
    registry: &OrgContractRegistry,
) -> OrgContractResolutionReport {
    let mut report = OrgContractResolutionReport::default();

    for reference in references {
        let resolved_contract_id = reference
            .contract_id
            .as_ref()
            .and_then(|contract_id| find_contract(registry, contract_id))
            .map(|contract| contract.id.clone());

        if reference.raw.trim().is_empty() {
            report.diagnostics.push(contract_reference_diagnostic(
                &reference,
                "CONTRACT_ORG is empty; load or choose an Org contract id",
            ));
        } else if resolved_contract_id.is_none() {
            report.diagnostics.push(contract_reference_diagnostic(
                &reference,
                format!(
                    "CONTRACT_ORG `{}` was not found in the loaded Org contract registry",
                    reference.raw
                ),
            ));
        }

        report.references.push(OrgContractResolution {
            reference,
            resolved_contract_id,
        });
    }

    report
}

fn find_contract<'a>(
    registry: &'a OrgContractRegistry,
    contract_id: &OrgContractId,
) -> Option<&'a OrgContract> {
    registry.contracts.iter().find(|contract| {
        &contract.id == contract_id || contract.aliases.iter().any(|alias| alias == contract_id)
    })
}

fn project_contract(contract: &orgize::ast::OrgContract) -> OrgContract {
    OrgContract {
        id: OrgContractId::from(contract.id.clone()),
        aliases: contract
            .aliases
            .iter()
            .cloned()
            .map(OrgContractId::from)
            .collect(),
        scope: OrgContractScope::new(debug_label(&contract.scope)),
        kind: OrgContractKind::new(debug_label(&contract.kind)),
        assertions: contract
            .assertions
            .iter()
            .map(project_contract_assertion)
            .collect(),
    }
}

fn project_contract_assertion(
    assertion: &orgize::ast::OrgContractAssertion,
) -> OrgContractAssertion {
    OrgContractAssertion {
        id: assertion.id.clone(),
        severity: OrgContractSeverity::new(debug_label(&assertion.severity)),
        bindings: assertion
            .bindings
            .iter()
            .map(|binding| OrgContractBinding {
                name: binding.name.clone(),
                query: project_contract_query(&binding.query),
            })
            .collect(),
        query: project_contract_query(&assertion.query),
        expectation: OrgContractExpectation::new(debug_label(&assertion.expectation)),
        message: assertion.message.clone(),
        fix: assertion.fix.clone(),
        templates: project_contract_templates(assertion),
        query_source: assertion
            .query_source
            .as_ref()
            .map(|source| OrgContractSourceSpan {
                start_line: source.start.line,
                start_column: source.start.column,
                end_line: source.end.line,
                end_column: source.end.column,
                start_byte: source.range_start as usize,
                end_byte: source.range_end as usize,
            }),
        expect_source: assertion
            .expect_source
            .as_ref()
            .map(|source| OrgContractSourceSpan {
                start_line: source.start.line,
                start_column: source.start.column,
                end_line: source.end.line,
                end_column: source.end.column,
                start_byte: source.range_start as usize,
                end_byte: source.range_end as usize,
            }),
    }
}

fn project_contract_templates(
    assertion: &orgize::ast::OrgContractAssertion,
) -> Vec<OrgContractTemplate> {
    let mut templates = Vec::new();

    if let Some(message) = &assertion.message {
        templates.push(OrgContractTemplate {
            kind: OrgContractTemplateKind::Message,
            engine: OrgContractTemplateEngine::new("jinja2"),
            body: message.clone(),
            source: None,
        });
    }

    if let Some(fix) = &assertion.fix {
        templates.push(OrgContractTemplate {
            kind: OrgContractTemplateKind::Fix,
            engine: OrgContractTemplateEngine::new("jinja2"),
            body: fix.clone(),
            source: None,
        });
    }

    templates
}

fn project_contract_query(query: &orgize::ast::OrgContractQuery) -> OrgContractQuery {
    OrgContractQuery {
        category: query
            .category
            .as_ref()
            .map(|category| OrgContractElementCategory::new(debug_label(category))),
        kind: query
            .kind
            .as_ref()
            .map(|kind| OrgContractElementKind::new(debug_label(kind))),
        affiliated_name: query.affiliated_name.clone(),
        context: query.context.clone(),
        outline_path_prefix: query.outline_path_prefix.clone(),
        outline_path_exact_len: query.outline_path_exact_len,
        property_equals: query.property_equals.clone(),
        property_contains: query.property_contains.clone(),
        summary_equals: query.summary_equals.clone(),
        summary_contains: query.summary_contains.clone(),
        limit: query.limit,
        use_scope_outline_path: query.use_scope_outline_path,
        has_outline_path_prefix: query.has_outline_path_prefix,
        scope_outline_depth: query.scope_outline_depth,
        relative_to: query
            .relative_to
            .as_ref()
            .map(|scope| OrgContractRelativeScope::new(debug_label(scope))),
    }
}

fn contract_reference_diagnostic(
    reference: &OrgContractReference,
    message: impl Into<String>,
) -> OrgContractDiagnostic {
    OrgContractDiagnostic {
        code: "ORG044".to_string(),
        severity: OrgContractDiagnosticSeverity::Warning,
        message: message.into(),
        reference: reference.clone(),
    }
}

fn debug_label(value: &impl std::fmt::Debug) -> String {
    format!("{value:?}")
}

fn source_span(
    document_id: &str,
    document_text: &str,
    start_byte: usize,
    end_byte: usize,
) -> OrgSourceSpan {
    OrgSourceSpan {
        document: document_id.to_string(),
        start_byte,
        end_byte,
        start_line: line_number_at_byte(document_text, start_byte),
        end_line: line_number_at_byte(document_text, end_byte.saturating_sub(1)),
    }
}

fn line_number_at_byte(document_text: &str, byte_offset: usize) -> usize {
    document_text
        .get(..byte_offset.min(document_text.len()))
        .unwrap_or_default()
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1
}

fn text_size_offset(offset: orgize::TextSize) -> usize {
    u32::from(offset) as usize
}
