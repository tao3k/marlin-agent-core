//! Dry-run workflow for Gerbil-emitted workspace patch intents.

use marlin_agent_harness_types::{AgentHarnessEvidence, AgentHarnessEvidenceKind};
use marlin_gerbil_ir::{GerbilWorkspaceContractFacts, WorkspacePatchIntentSpec};
use marlin_org_model::{OrgContractDiagnosticSeverity, OrgContractValidationStatus, OrgNodeId};
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
pub fn gerbil_workspace_patch_receipt_evidence(
    receipt: &WorkspacePatchReceipt,
) -> AgentHarnessEvidence {
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
        AgentHarnessEvidence::present(AgentHarnessEvidenceKind::Workflow, subject)
            .with_detail(detail)
    } else {
        AgentHarnessEvidence::missing(AgentHarnessEvidenceKind::Workflow, subject)
            .with_detail(detail)
    }
}

/// Consumes a Gerbil patch intent without executing durable workspace mutation.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GerbilWorkspacePatchIntentDryRunner;

impl GerbilWorkspacePatchIntentDryRunner {
    pub fn dry_run(intent: &WorkspacePatchIntentSpec) -> WorkspacePatchReceipt {
        Self::dry_run_with_optional_contract_facts(intent, None)
    }

    pub fn dry_run_with_contract_facts(
        intent: &WorkspacePatchIntentSpec,
        contract_facts: &GerbilWorkspaceContractFacts,
    ) -> WorkspacePatchReceipt {
        Self::dry_run_with_optional_contract_facts(intent, Some(contract_facts))
    }

    fn dry_run_with_optional_contract_facts(
        intent: &WorkspacePatchIntentSpec,
        contract_facts: Option<&GerbilWorkspaceContractFacts>,
    ) -> WorkspacePatchReceipt {
        let validation = validate_intent_with_contract_facts(intent, contract_facts);
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract_facts: Option<GerbilWorkspaceContractFacts>,
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
            contract_facts: None,
        }
    }

    pub fn with_contract_facts(mut self, contract_facts: GerbilWorkspaceContractFacts) -> Self {
        self.contract_facts = Some(contract_facts);
        self
    }
}

pub(crate) fn validate_intent_with_contract_facts(
    intent: &WorkspacePatchIntentSpec,
    contract_facts: Option<&GerbilWorkspaceContractFacts>,
) -> WorkspaceValidationReport {
    let mut diagnostics = Vec::new();

    if !intent.dry_run_first {
        diagnostics.push(ValidationDiagnostic {
            severity: ValidationSeverity::Error,
            message:
                "gerbil workspace patch intent requires dry_run_first before workflow consumption"
                    .to_owned(),
        });
    }

    if let Some(contract_facts) = contract_facts {
        diagnostics.extend(contract_fact_diagnostics(contract_facts));
    }

    WorkspaceValidationReport {
        accepted: diagnostics.is_empty(),
        diagnostics,
    }
}

fn contract_fact_diagnostics(
    contract_facts: &GerbilWorkspaceContractFacts,
) -> Vec<ValidationDiagnostic> {
    let resolution_errors = contract_facts
        .resolutions
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == OrgContractDiagnosticSeverity::Error)
        .map(|diagnostic| ValidationDiagnostic {
            severity: ValidationSeverity::Error,
            message: format!("org contract resolution failed: {}", diagnostic.message),
        });
    let validation_errors = contract_facts
        .validations
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == OrgContractDiagnosticSeverity::Error)
        .map(|diagnostic| ValidationDiagnostic {
            severity: ValidationSeverity::Error,
            message: format!("org contract validation failed: {}", diagnostic.message),
        });
    let failed_assertions = contract_facts
        .validations
        .receipts
        .iter()
        .filter(|receipt| receipt.status == OrgContractValidationStatus::Failed)
        .map(|receipt| ValidationDiagnostic {
            severity: ValidationSeverity::Error,
            message: format!(
                "org contract assertion failed: {}#{}{}",
                receipt.contract_id.as_str(),
                receipt.assertion_id,
                receipt
                    .message
                    .as_ref()
                    .map(|message| format!(": {message}"))
                    .unwrap_or_default()
            ),
        });

    resolution_errors
        .chain(validation_errors)
        .chain(failed_assertions)
        .collect()
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
