//! In-memory application of planned `Org` text edits.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{OrgPatchDiagnostic, OrgPatchPlan, OrgTextEdit};

/// Applies source edit plans to caller-owned document text.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OrgPatchApplier;

impl OrgPatchApplier {
    /// Applies a plan to an in-memory document map after validating all edits.
    pub fn apply_to_documents(
        plan: &OrgPatchPlan,
        documents: &mut BTreeMap<String, String>,
    ) -> OrgPatchApplyReport {
        let mut by_document = group_edits_by_document(&plan.edits);
        let diagnostics = blocking_diagnostics(plan, &mut by_document, documents);
        if !diagnostics.is_empty() {
            return OrgPatchApplyReport {
                applied_edits: 0,
                changed_documents: Vec::new(),
                diagnostics,
            };
        }

        let before_hashes = document_hashes(&by_document, documents);
        let applied_edits = apply_validated_edits(by_document, documents);
        OrgPatchApplyReport {
            applied_edits,
            changed_documents: changed_documents(&before_hashes, documents),
            diagnostics: Vec::new(),
        }
    }
}

/// Result of applying a planned source patch to in-memory documents.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrgPatchApplyReport {
    pub applied_edits: usize,
    pub changed_documents: Vec<OrgPatchDocumentChange>,
    pub diagnostics: Vec<OrgPatchDiagnostic>,
}

/// Before/after proof for a changed in-memory `Org` document.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrgPatchDocumentChange {
    pub document: String,
    pub before_hash: String,
    pub after_hash: String,
}

/// Stable portable hash for `Org` document text.
pub fn org_text_hash(text: &str) -> String {
    stable_text_hash(text)
}

fn group_edits_by_document(edits: &[OrgTextEdit]) -> BTreeMap<&str, Vec<&OrgTextEdit>> {
    edits.iter().fold(BTreeMap::new(), |mut by_document, edit| {
        by_document
            .entry(edit.document.as_str())
            .or_default()
            .push(edit);
        by_document
    })
}

fn blocking_diagnostics(
    plan: &OrgPatchPlan,
    by_document: &mut BTreeMap<&str, Vec<&OrgTextEdit>>,
    documents: &BTreeMap<String, String>,
) -> Vec<OrgPatchDiagnostic> {
    if !plan.diagnostics.is_empty() {
        return plan.diagnostics.clone();
    }

    by_document
        .iter_mut()
        .flat_map(|(document, edits)| validate_document_edits(document, edits, documents))
        .collect()
}

fn document_hashes(
    by_document: &BTreeMap<&str, Vec<&OrgTextEdit>>,
    documents: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    by_document
        .keys()
        .filter_map(|document| {
            documents
                .get(*document)
                .map(|text| ((*document).to_owned(), stable_text_hash(text)))
        })
        .collect()
}

fn changed_documents(
    before_hashes: &BTreeMap<String, String>,
    documents: &BTreeMap<String, String>,
) -> Vec<OrgPatchDocumentChange> {
    before_hashes
        .iter()
        .filter_map(|(document, before_hash)| {
            documents.get(document).map(|text| OrgPatchDocumentChange {
                document: document.clone(),
                before_hash: before_hash.clone(),
                after_hash: stable_text_hash(text),
            })
        })
        .filter(|change| change.before_hash != change.after_hash)
        .collect()
}

fn apply_validated_edits(
    by_document: BTreeMap<&str, Vec<&OrgTextEdit>>,
    documents: &mut BTreeMap<String, String>,
) -> usize {
    by_document
        .into_iter()
        .filter_map(|(document, edits)| {
            documents
                .get_mut(document)
                .map(|text| apply_document_edits(text, &edits))
        })
        .sum()
}

fn validate_document_edits(
    document: &str,
    edits: &mut Vec<&OrgTextEdit>,
    documents: &BTreeMap<String, String>,
) -> Vec<OrgPatchDiagnostic> {
    edits.sort_by_key(|edit| (edit.start_byte, edit.end_byte));

    let Some(text) = documents.get(document) else {
        return vec![apply_diagnostic(format!("missing document: {document}"))];
    };

    let mut previous_end = 0;
    edits
        .iter()
        .filter_map(|edit| {
            if edit.start_byte > edit.end_byte || edit.end_byte > text.len() {
                Some(apply_diagnostic(format!(
                    "invalid byte range {}..{} for {}",
                    edit.start_byte, edit.end_byte, edit.document
                )))
            } else if !text.is_char_boundary(edit.start_byte)
                || !text.is_char_boundary(edit.end_byte)
            {
                Some(apply_diagnostic(format!(
                    "byte range {}..{} is not a UTF-8 character boundary in {}",
                    edit.start_byte, edit.end_byte, edit.document
                )))
            } else if edit.start_byte < previous_end {
                Some(apply_diagnostic(format!(
                    "overlapping edit range {}..{} in {}",
                    edit.start_byte, edit.end_byte, edit.document
                )))
            } else {
                previous_end = edit.end_byte;
                None
            }
        })
        .collect()
}

fn apply_document_edits(text: &mut String, edits: &[&OrgTextEdit]) -> usize {
    edits
        .iter()
        .rev()
        .map(|edit| {
            text.replace_range(edit.start_byte..edit.end_byte, &edit.replacement);
            1
        })
        .sum()
}

fn apply_diagnostic(message: String) -> OrgPatchDiagnostic {
    OrgPatchDiagnostic {
        node: None,
        operation: "apply-plan".to_owned(),
        message,
    }
}

fn stable_text_hash(text: &str) -> String {
    let hash = text
        .as_bytes()
        .iter()
        .fold(0xcbf29ce484222325, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
        });
    format!("fnv1a64:{hash:016x}")
}
