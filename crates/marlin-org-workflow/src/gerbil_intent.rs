//! Dry-run workflow for Gerbil-emitted workspace patch intents.

use marlin_agent_protocol::{LoopEvidence, LoopEvidenceKind};
use marlin_gerbil_ir::WorkspacePatchIntentSpec;
use marlin_org_model::OrgNodeId;
use marlin_org_store::OrgSourceWritePolicy;
use marlin_workspace_patch::{
    MemoryDispatchReceipt, PatchId, ValidationDiagnostic, ValidationSeverity,
    WorkspacePatchExecutionMode, WorkspacePatchExecutionReceipt, WorkspacePatchReceipt,
    WorkspaceValidationReport,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use crate::patch_ops::workspace_patch_op_node;

const DRY_RUN_BEFORE_HASH: &str = "dry-run:no-workspace-read";
const DRY_RUN_AFTER_HASH: &str = "dry-run:no-workspace-write";

/// Projects a Gerbil workspace patch receipt into harness-visible workflow evidence.
pub fn gerbil_workspace_patch_receipt_evidence(receipt: &WorkspacePatchReceipt) -> LoopEvidence {
    let subject = format!("workspace-patch:{}", receipt.patch_id.as_str());
    let detail = format!(
        "accepted={} mode={:?} policy_accepted={} affected_nodes={} affected_sources={} memory_dispatch={} diagnostics={}",
        receipt.validation.accepted,
        receipt.execution.mode,
        receipt.execution.policy.accepted,
        receipt.affected_nodes.len(),
        receipt.affected_sources.len(),
        receipt.memory_dispatch.len(),
        receipt.validation.diagnostics.len()
    );

    if receipt.validation.accepted {
        LoopEvidence::present(LoopEvidenceKind::Workflow, subject).with_detail(detail)
    } else {
        LoopEvidence::missing(LoopEvidenceKind::Workflow, subject).with_detail(detail)
    }
}

/// Consumes a Gerbil patch intent without executing durable workspace mutation.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GerbilWorkspacePatchIntentDryRunner;

impl GerbilWorkspacePatchIntentDryRunner {
    pub fn dry_run(intent: &WorkspacePatchIntentSpec) -> WorkspacePatchReceipt {
        let validation = validate_intent(intent);
        let accepted = validation.accepted;

        WorkspacePatchReceipt {
            patch_id: PatchId::new(intent.intent_id.clone()),
            affected_nodes: affected_nodes(intent),
            affected_sources: Vec::new(),
            before_hash: DRY_RUN_BEFORE_HASH.to_owned(),
            after_hash: DRY_RUN_AFTER_HASH.to_owned(),
            execution: dry_run_execution(&validation),
            validation,
            memory_dispatch: if accepted {
                memory_dispatch_dry_run_receipts(intent)
            } else {
                Vec::new()
            },
        }
    }
}

fn dry_run_execution(validation: &WorkspaceValidationReport) -> WorkspacePatchExecutionReceipt {
    if validation.accepted {
        WorkspacePatchExecutionReceipt::dry_run_accepted(
            "gerbil intent validated without workspace write",
        )
    } else {
        WorkspacePatchExecutionReceipt::rejected(
            WorkspacePatchExecutionMode::DryRun,
            "gerbil intent rejected before workspace write",
        )
    }
}

/// Explicit request to persist a Gerbil-emitted workspace patch intent.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GerbilWorkspacePatchIntentCommit {
    pub document: String,
    pub intent: WorkspacePatchIntentSpec,
    pub policy: OrgSourceWritePolicy,
}

impl GerbilWorkspacePatchIntentCommit {
    pub fn new(
        document: impl Into<String>,
        intent: WorkspacePatchIntentSpec,
        policy: OrgSourceWritePolicy,
    ) -> Self {
        Self {
            document: document.into(),
            intent,
            policy,
        }
    }
}

pub(crate) fn validate_intent(intent: &WorkspacePatchIntentSpec) -> WorkspaceValidationReport {
    if intent.dry_run_first {
        WorkspaceValidationReport::accepted()
    } else {
        WorkspaceValidationReport {
            accepted: false,
            diagnostics: vec![ValidationDiagnostic {
                severity: ValidationSeverity::Error,
                message: "gerbil workspace patch intent requires dry_run_first before workflow consumption"
                    .to_owned(),
            }],
        }
    }
}

fn affected_nodes(intent: &WorkspacePatchIntentSpec) -> Vec<OrgNodeId> {
    intent
        .patch
        .ops
        .iter()
        .map(|op| workspace_patch_op_node(op).clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn memory_dispatch_dry_run_receipts(
    intent: &WorkspacePatchIntentSpec,
) -> Vec<MemoryDispatchReceipt> {
    intent
        .patch
        .ops
        .iter()
        .filter_map(|op| match op {
            marlin_workspace_patch::WorkspacePatchOp::MarkMemoryCandidate { dispatch, .. } => {
                Some(MemoryDispatchReceipt {
                    target: dispatch.clone(),
                    accepted: false,
                    reason: Some("dry-run only: memory dispatch not executed".to_owned()),
                })
            }
            _ => None,
        })
        .collect()
}
