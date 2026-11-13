// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Contract projection and `CONTRACT_ORG` reference resolution.

use marlin_context_org_model::{
    OrgContract, OrgContractAssertion, OrgContractBinding, OrgContractCompareOp,
    OrgContractDiagnostic, OrgContractDiagnosticSeverity, OrgContractDocumentPredicate,
    OrgContractElementCategory, OrgContractElementKind, OrgContractExpectation, OrgContractId,
    OrgContractKind, OrgContractPredicate, OrgContractQuery, OrgContractReference,
    OrgContractReferenceScope, OrgContractRegistry, OrgContractRelativeScope,
    OrgContractResolution, OrgContractResolutionReport, OrgContractScope, OrgContractSeverity,
    OrgContractSourceSpan, OrgContractTemplate, OrgContractTemplateEngine, OrgContractTemplateKind,
    OrgContractValue, OrgNodeId, OrgSourceSpan,
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
        expectation: project_contract_expectation(&assertion.expectation),
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

fn project_contract_expectation(
    expectation: &orgize::ast::OrgContractExpectation,
) -> OrgContractExpectation {
    match expectation {
        orgize::ast::OrgContractExpectation::Exists => OrgContractExpectation::Exists,
        orgize::ast::OrgContractExpectation::NotExists => OrgContractExpectation::NotExists,
        orgize::ast::OrgContractExpectation::Count(op, expected) => OrgContractExpectation::Count {
            op: project_contract_compare_op(*op),
            expected: *expected,
        },
        // These expectations need a binding context. Validation retains the
        // parser-owned contract and evaluates it through Orgize.
        orgize::ast::OrgContractExpectation::CountBinding(..)
        | orgize::ast::OrgContractExpectation::ValueSetEqual { .. } => {
            OrgContractExpectation::unsupported(expectation.expected_summary())
        }
    }
}

fn project_contract_compare_op(op: orgize::ast::OrgContractCompareOp) -> OrgContractCompareOp {
    match op {
        orgize::ast::OrgContractCompareOp::Eq => OrgContractCompareOp::Eq,
        orgize::ast::OrgContractCompareOp::Ne => OrgContractCompareOp::Ne,
        orgize::ast::OrgContractCompareOp::Lt => OrgContractCompareOp::Lt,
        orgize::ast::OrgContractCompareOp::Le => OrgContractCompareOp::Le,
        orgize::ast::OrgContractCompareOp::Gt => OrgContractCompareOp::Gt,
        orgize::ast::OrgContractCompareOp::Ge => OrgContractCompareOp::Ge,
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
        alternatives: query
            .alternatives
            .iter()
            .map(project_contract_query)
            .collect(),
        predicates: query
            .predicates
            .iter()
            .map(project_contract_predicate)
            .collect(),
        document_predicates: query
            .document_predicates
            .iter()
            .map(project_document_predicate)
            .collect(),
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

fn project_contract_predicate(
    predicate: &orgize::ast::OrgElementQueryPredicate,
) -> OrgContractPredicate {
    use orgize::ast::OrgElementQueryPredicate as Source;
    match predicate {
        Source::All(items) => {
            OrgContractPredicate::All(items.iter().map(project_contract_predicate).collect())
        }
        Source::Any(items) => {
            OrgContractPredicate::Any(items.iter().map(project_contract_predicate).collect())
        }
        Source::Not(item) => OrgContractPredicate::Not(Box::new(project_contract_predicate(item))),
        Source::Category(value) => {
            OrgContractPredicate::Category(OrgContractElementCategory::new(debug_label(value)))
        }
        Source::Kind(value) => {
            OrgContractPredicate::Kind(OrgContractElementKind::new(debug_label(value)))
        }
        Source::AffiliatedName(value) => OrgContractPredicate::AffiliatedName(value.clone()),
        Source::Context(value) => OrgContractPredicate::Context(value.clone()),
        Source::PropertyEquals(value) => OrgContractPredicate::PropertyEquals {
            key: value.key.clone(),
            value: project_contract_value(&value.value),
        },
        Source::PropertyContains(value) => OrgContractPredicate::PropertyContains {
            key: value.key.clone(),
            needle: value.needle.clone(),
        },
        Source::PropertyPositiveInteger(value) => {
            OrgContractPredicate::PropertyPositiveInteger(value.clone())
        }
        Source::SummaryEquals(value) => OrgContractPredicate::SummaryEquals {
            key: value.key.clone(),
            value: project_contract_value(&value.value),
        },
        Source::SummaryContains(value) => OrgContractPredicate::SummaryContains {
            key: value.key.clone(),
            needle: value.needle.clone(),
        },
        Source::SummaryPositiveInteger(value) => {
            OrgContractPredicate::SummaryPositiveInteger(value.clone())
        }
    }
}

fn project_contract_value(value: &orgize::ast::OrgElementsIndexSummaryValue) -> OrgContractValue {
    use orgize::ast::OrgElementsIndexSummaryValue as Source;
    match value {
        Source::Null => OrgContractValue::Null,
        Source::Bool(value) => OrgContractValue::Bool(*value),
        Source::Integer(value) => OrgContractValue::Integer(*value),
        Source::Text(value) => OrgContractValue::Text(value.clone()),
        Source::StringList(value) => OrgContractValue::StringList(value.clone()),
    }
}

fn project_document_predicate(
    predicate: &orgize::ast::OrgContractDocumentPredicate,
) -> OrgContractDocumentPredicate {
    use orgize::ast::OrgContractDocumentPredicate as Source;
    match predicate {
        Source::MetadataExists(value) => {
            OrgContractDocumentPredicate::MetadataExists(value.clone())
        }
        Source::SourcePathEquals(value) => {
            OrgContractDocumentPredicate::SourcePathEquals(value.clone())
        }
        Source::SourcePathContains(value) => {
            OrgContractDocumentPredicate::SourcePathContains(value.clone())
        }
        Source::SourceFilenameEquals(value) => {
            OrgContractDocumentPredicate::SourceFilenameEquals(value.clone())
        }
        Source::SourceFilenamePrefix(value) => {
            OrgContractDocumentPredicate::SourceFilenamePrefix(value.clone())
        }
        Source::SourceFilenameSuffix(value) => {
            OrgContractDocumentPredicate::SourceFilenameSuffix(value.clone())
        }
        Source::SourceFilenameStemUppercase(value) => {
            OrgContractDocumentPredicate::SourceFilenameStemUppercase(*value)
        }
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
