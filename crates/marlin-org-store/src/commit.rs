//! Commit orchestration for source-aware `Org` patch plans.

use std::collections::{BTreeMap, BTreeSet};

use marlin_org_patch::{
    OrgPatchApplier, OrgPatchDiagnostic, OrgPatchDocumentChange, OrgPatchPlan, OrgTextEdit,
    org_text_hash,
};
use serde::{Deserialize, Serialize};

use crate::OrgSourceStore;

/// Source persistence request.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrgSourceCommit {
    pub plan: OrgPatchPlan,
    pub expected_documents: Vec<OrgSourceDocumentHash>,
    pub policy: OrgSourceWritePolicy,
}

impl OrgSourceCommit {
    pub fn new(plan: OrgPatchPlan, policy: OrgSourceWritePolicy) -> Self {
        Self {
            plan,
            expected_documents: Vec::new(),
            policy,
        }
    }
}

/// Expected durable document hash before a commit.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrgSourceDocumentHash {
    pub document: String,
    pub hash: String,
}

impl OrgSourceDocumentHash {
    pub fn from_text(document: impl Into<String>, text: &str) -> Self {
        Self {
            document: document.into(),
            hash: org_text_hash(text),
        }
    }
}

/// Write mode for a source commit.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum OrgSourceWriteMode {
    DryRun,
    Write,
}

/// Caller-selected source write policy.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrgSourceWritePolicy {
    pub mode: OrgSourceWriteMode,
    pub require_clean: bool,
    pub multi_document: OrgSourceMultiDocumentPolicy,
}

impl OrgSourceWritePolicy {
    pub fn dry_run() -> Self {
        Self {
            mode: OrgSourceWriteMode::DryRun,
            require_clean: false,
            multi_document: OrgSourceMultiDocumentPolicy::Reject,
        }
    }

    pub fn write() -> Self {
        Self {
            mode: OrgSourceWriteMode::Write,
            require_clean: false,
            multi_document: OrgSourceMultiDocumentPolicy::Reject,
        }
    }

    pub fn write_require_clean() -> Self {
        Self {
            mode: OrgSourceWriteMode::Write,
            require_clean: true,
            multi_document: OrgSourceMultiDocumentPolicy::Reject,
        }
    }

    pub fn allow_best_effort_multi_document(mut self) -> Self {
        self.multi_document = OrgSourceMultiDocumentPolicy::AllowBestEffort;
        self
    }
}

/// Multi-document write policy for stores without rollback support.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum OrgSourceMultiDocumentPolicy {
    #[default]
    Reject,
    AllowBestEffort,
}

impl Default for OrgSourceWritePolicy {
    fn default() -> Self {
        Self {
            mode: OrgSourceWriteMode::DryRun,
            require_clean: false,
            multi_document: OrgSourceMultiDocumentPolicy::Reject,
        }
    }
}

/// Source persistence result.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrgSourceCommitReceipt {
    pub applied_edits: usize,
    pub planned_edits: Vec<OrgTextEdit>,
    pub changed_documents: Vec<OrgPatchDocumentChange>,
    pub diagnostics: Vec<OrgSourceDiagnostic>,
    pub conflicts: Vec<OrgSourceConflict>,
    pub wrote_documents: bool,
}

impl OrgSourceCommitReceipt {
    pub fn accepted(&self) -> bool {
        self.diagnostics.is_empty() && self.conflicts.is_empty()
    }
}

/// Stale source conflict detected before applying a plan.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrgSourceConflict {
    pub document: String,
    pub expected_hash: String,
    pub actual_hash: String,
}

/// Source commit diagnostic.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrgSourceDiagnostic {
    pub document: Option<String>,
    pub kind: OrgSourceDiagnosticKind,
    pub message: String,
}

/// Diagnostic category for source persistence failures.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum OrgSourceDiagnosticKind {
    DirtyStore,
    MissingDocument,
    MissingExpectedHash,
    MultiDocumentWriteUnsupported,
    PatchDiagnostic,
    StaleDocument,
    StoreWriteFailed,
    WorkspaceLoadFailed,
}

/// Applies a source commit through an explicit store adapter.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OrgSourceCommitter;

impl OrgSourceCommitter {
    pub fn commit<S: OrgSourceStore>(
        store: &mut S,
        commit: &OrgSourceCommit,
    ) -> OrgSourceCommitReceipt {
        let target_documents = target_documents(&commit.plan);
        let expected_hashes = expected_hashes(&commit.expected_documents);
        let loaded_documents = load_documents(store, &target_documents);
        let diagnostics =
            preflight_diagnostics(store, commit, &target_documents, &loaded_documents);
        let conflicts = stale_conflicts(&loaded_documents, &expected_hashes);

        if !diagnostics.is_empty() || !conflicts.is_empty() {
            return blocked_receipt(&commit.plan, diagnostics, conflicts);
        }

        apply_commit(store, commit, loaded_documents)
    }
}

fn target_documents(plan: &OrgPatchPlan) -> Vec<String> {
    plan.edits
        .iter()
        .map(|edit| edit.document.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn expected_hashes(expected_documents: &[OrgSourceDocumentHash]) -> BTreeMap<String, String> {
    expected_documents
        .iter()
        .map(|expected| (expected.document.clone(), expected.hash.clone()))
        .collect()
}

fn load_documents<S: OrgSourceStore>(
    store: &S,
    documents: &[String],
) -> BTreeMap<String, Option<String>> {
    documents
        .iter()
        .map(|document| (document.clone(), store.read_document(document)))
        .collect()
}

fn preflight_diagnostics<S: OrgSourceStore>(
    store: &S,
    commit: &OrgSourceCommit,
    target_documents: &[String],
    loaded_documents: &BTreeMap<String, Option<String>>,
) -> Vec<OrgSourceDiagnostic> {
    dirty_store_diagnostic(store, &commit.policy)
        .into_iter()
        .chain(patch_diagnostics(&commit.plan))
        .chain(missing_document_diagnostics(loaded_documents))
        .chain(missing_expected_hash_diagnostics(
            target_documents,
            &expected_hashes(&commit.expected_documents),
        ))
        .chain(multi_document_diagnostics(&commit.policy, target_documents))
        .collect()
}

fn dirty_store_diagnostic<S: OrgSourceStore>(
    store: &S,
    policy: &OrgSourceWritePolicy,
) -> Option<OrgSourceDiagnostic> {
    (policy.require_clean && !store.is_clean()).then(|| OrgSourceDiagnostic {
        document: None,
        kind: OrgSourceDiagnosticKind::DirtyStore,
        message: "source store is not clean".to_owned(),
    })
}

fn patch_diagnostics(plan: &OrgPatchPlan) -> Vec<OrgSourceDiagnostic> {
    plan.diagnostics
        .iter()
        .map(source_patch_diagnostic)
        .collect()
}

fn source_patch_diagnostic(diagnostic: &OrgPatchDiagnostic) -> OrgSourceDiagnostic {
    OrgSourceDiagnostic {
        document: None,
        kind: OrgSourceDiagnosticKind::PatchDiagnostic,
        message: format!("{}: {}", diagnostic.operation, diagnostic.message),
    }
}

fn missing_document_diagnostics(
    loaded_documents: &BTreeMap<String, Option<String>>,
) -> Vec<OrgSourceDiagnostic> {
    loaded_documents
        .iter()
        .filter(|(_, text)| text.is_none())
        .map(|(document, _)| OrgSourceDiagnostic {
            document: Some(document.clone()),
            kind: OrgSourceDiagnosticKind::MissingDocument,
            message: format!("missing document: {document}"),
        })
        .collect()
}

fn missing_expected_hash_diagnostics(
    target_documents: &[String],
    expected_hashes: &BTreeMap<String, String>,
) -> Vec<OrgSourceDiagnostic> {
    target_documents
        .iter()
        .filter(|document| !expected_hashes.contains_key(*document))
        .map(|document| OrgSourceDiagnostic {
            document: Some(document.clone()),
            kind: OrgSourceDiagnosticKind::MissingExpectedHash,
            message: format!("missing expected hash for document: {document}"),
        })
        .collect()
}

fn multi_document_diagnostics(
    policy: &OrgSourceWritePolicy,
    target_documents: &[String],
) -> Vec<OrgSourceDiagnostic> {
    (policy.multi_document == OrgSourceMultiDocumentPolicy::Reject && target_documents.len() > 1)
        .then(|| OrgSourceDiagnostic {
            document: None,
            kind: OrgSourceDiagnosticKind::MultiDocumentWriteUnsupported,
            message: "multi-document source commits are rejected by the write policy".to_owned(),
        })
        .into_iter()
        .collect()
}

fn stale_conflicts(
    loaded_documents: &BTreeMap<String, Option<String>>,
    expected_hashes: &BTreeMap<String, String>,
) -> Vec<OrgSourceConflict> {
    loaded_documents
        .iter()
        .filter_map(|(document, text)| {
            text.as_ref()
                .zip(expected_hashes.get(document))
                .map(|(text, expected_hash)| {
                    (document.clone(), expected_hash.clone(), org_text_hash(text))
                })
        })
        .filter(|(_, expected_hash, actual_hash)| expected_hash != actual_hash)
        .map(|(document, expected_hash, actual_hash)| OrgSourceConflict {
            document,
            expected_hash,
            actual_hash,
        })
        .collect()
}

fn blocked_receipt(
    plan: &OrgPatchPlan,
    diagnostics: Vec<OrgSourceDiagnostic>,
    conflicts: Vec<OrgSourceConflict>,
) -> OrgSourceCommitReceipt {
    OrgSourceCommitReceipt {
        applied_edits: 0,
        planned_edits: plan.edits.clone(),
        changed_documents: Vec::new(),
        diagnostics,
        conflicts,
        wrote_documents: false,
    }
}

fn apply_commit<S: OrgSourceStore>(
    store: &mut S,
    commit: &OrgSourceCommit,
    loaded_documents: BTreeMap<String, Option<String>>,
) -> OrgSourceCommitReceipt {
    let mut documents = present_documents(loaded_documents);
    let apply_report = OrgPatchApplier::apply_to_documents(&commit.plan, &mut documents);
    let diagnostics = patch_apply_diagnostics(&apply_report.diagnostics);

    if !diagnostics.is_empty() {
        return blocked_receipt(&commit.plan, diagnostics, Vec::new());
    }

    if commit.policy.mode == OrgSourceWriteMode::DryRun {
        return apply_receipt(
            &commit.plan,
            apply_report.applied_edits,
            apply_report.changed_documents,
            false,
        );
    }

    let changed_text = changed_document_text(&apply_report.changed_documents, &documents);
    match store.write_documents(changed_text) {
        Ok(()) => apply_receipt(
            &commit.plan,
            apply_report.applied_edits,
            apply_report.changed_documents,
            true,
        ),
        Err(error) => blocked_receipt(
            &commit.plan,
            vec![OrgSourceDiagnostic {
                document: None,
                kind: OrgSourceDiagnosticKind::StoreWriteFailed,
                message: error.message,
            }],
            Vec::new(),
        ),
    }
}

fn present_documents(
    loaded_documents: BTreeMap<String, Option<String>>,
) -> BTreeMap<String, String> {
    loaded_documents
        .into_iter()
        .filter_map(|(document, text)| text.map(|text| (document, text)))
        .collect()
}

fn patch_apply_diagnostics(diagnostics: &[OrgPatchDiagnostic]) -> Vec<OrgSourceDiagnostic> {
    diagnostics.iter().map(source_patch_diagnostic).collect()
}

fn changed_document_text(
    changed_documents: &[OrgPatchDocumentChange],
    documents: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    changed_documents
        .iter()
        .filter_map(|change| {
            documents
                .get(&change.document)
                .map(|text| (change.document.clone(), text.clone()))
        })
        .collect()
}

fn apply_receipt(
    plan: &OrgPatchPlan,
    applied_edits: usize,
    changed_documents: Vec<OrgPatchDocumentChange>,
    wrote_documents: bool,
) -> OrgSourceCommitReceipt {
    OrgSourceCommitReceipt {
        applied_edits,
        planned_edits: plan.edits.clone(),
        changed_documents,
        diagnostics: Vec::new(),
        conflicts: Vec::new(),
        wrote_documents,
    }
}
