//! Source commit workflow for parser-owned `Org` workspace patches.

use marlin_org_model::{OrgNode, OrgNodeId};
use marlin_org_patch::{OrgPatchPlan, OrgPatchPlanner};
use marlin_org_store::{
    OrgSourceCommit, OrgSourceCommitReceipt, OrgSourceCommitter, OrgSourceDiagnostic,
    OrgSourceDiagnosticKind, OrgSourceDocumentHash, OrgSourceStore, OrgSourceWritePolicy,
};
use marlin_org_workspace::{OrgDocument, OrgDocumentLoader};
use marlin_workspace_patch::{AffectedNodeSource, WorkspacePatch, WorkspacePatchOp};
use serde::{Deserialize, Serialize};

/// Request to apply a typed workspace patch to one durable `Org` document.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OrgWorkspaceSourceCommit {
    pub document: String,
    pub patch: WorkspacePatch,
    pub policy: OrgSourceWritePolicy,
}

impl OrgWorkspaceSourceCommit {
    pub fn new(
        document: impl Into<String>,
        patch: WorkspacePatch,
        policy: OrgSourceWritePolicy,
    ) -> Self {
        Self {
            document: document.into(),
            patch,
            policy,
        }
    }
}

/// Result returned by the workspace-source workflow.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct OrgWorkspaceSourceCommitReceipt {
    pub loaded_nodes: Vec<OrgNodeId>,
    pub plan: OrgPatchPlan,
    pub source: OrgSourceCommitReceipt,
}

/// Loads a document, plans a patch from parser-owned spans, and commits it.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OrgWorkspaceSourceCommitter;

impl OrgWorkspaceSourceCommitter {
    pub fn commit_document<S: OrgSourceStore>(
        store: &mut S,
        request: &OrgWorkspaceSourceCommit,
    ) -> OrgWorkspaceSourceCommitReceipt {
        let Some(text) = store.read_document(&request.document) else {
            return blocked_workspace_receipt(
                OrgSourceDiagnosticKind::MissingDocument,
                format!("missing document: {}", request.document),
            );
        };

        let document = OrgDocument::new(request.document.clone(), text.clone());
        let nodes = match OrgDocumentLoader::load(&document) {
            Ok(nodes) => nodes,
            Err(error) => {
                return blocked_workspace_receipt(
                    OrgSourceDiagnosticKind::WorkspaceLoadFailed,
                    error.to_string(),
                );
            }
        };
        let sources = affected_sources(&nodes, &request.patch);
        let plan = OrgPatchPlanner::plan(&request.patch, &sources);
        let mut source_commit = OrgSourceCommit::new(plan.clone(), request.policy.clone());
        source_commit
            .expected_documents
            .push(OrgSourceDocumentHash::from_text(&request.document, &text));
        let source = OrgSourceCommitter::commit(store, &source_commit);

        OrgWorkspaceSourceCommitReceipt {
            loaded_nodes: nodes.into_iter().map(|node| node.id).collect(),
            plan,
            source,
        }
    }
}

fn affected_sources(nodes: &[OrgNode], patch: &WorkspacePatch) -> Vec<AffectedNodeSource> {
    patch
        .ops
        .iter()
        .filter_map(|op| {
            let target = op_node(op);
            nodes
                .iter()
                .find(|node| &node.id == target)
                .and_then(|node| {
                    node.source.clone().map(|source| AffectedNodeSource {
                        node: node.id.clone(),
                        source,
                        tokens: node.tokens.clone(),
                    })
                })
        })
        .collect()
}

fn op_node(op: &WorkspacePatchOp) -> &OrgNodeId {
    match op {
        WorkspacePatchOp::SetTodo { node, .. }
        | WorkspacePatchOp::SetProperty { node, .. }
        | WorkspacePatchOp::AddCheckbox { node, .. }
        | WorkspacePatchOp::MarkCheckbox { node, .. }
        | WorkspacePatchOp::AppendSection { node, .. }
        | WorkspacePatchOp::AddLink { node, .. }
        | WorkspacePatchOp::AddEvidenceRef { node, .. }
        | WorkspacePatchOp::AddMetricPoint { node, .. }
        | WorkspacePatchOp::AddDecision { node, .. }
        | WorkspacePatchOp::AddTraceEvent { node, .. }
        | WorkspacePatchOp::MarkMemoryCandidate { node, .. } => node,
    }
}

fn blocked_workspace_receipt(
    kind: OrgSourceDiagnosticKind,
    message: String,
) -> OrgWorkspaceSourceCommitReceipt {
    OrgWorkspaceSourceCommitReceipt {
        loaded_nodes: Vec::new(),
        plan: OrgPatchPlan::default(),
        source: OrgSourceCommitReceipt {
            applied_edits: 0,
            planned_edits: Vec::new(),
            changed_documents: Vec::new(),
            diagnostics: vec![OrgSourceDiagnostic {
                document: None,
                kind,
                message,
            }],
            conflicts: Vec::new(),
            wrote_documents: false,
        },
    }
}
