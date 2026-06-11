use marlin_agent_protocol::LoopEvidenceKind;
use marlin_org_workflow::{
    GerbilWorkspacePatchIntentDryRunner, gerbil_workspace_patch_receipt_evidence,
};
use marlin_workspace_patch::{PatchId, ValidationSeverity, WorkspacePatchExecutionMode};

use super::support::{failed_contract_facts, workspace_patch_intent};

#[test]
fn gerbil_patch_intent_dry_run_returns_receipt_without_workspace_write() {
    let intent = workspace_patch_intent(true);

    let receipt = GerbilWorkspacePatchIntentDryRunner::dry_run(&intent);

    assert_eq!(receipt.patch_id, PatchId::new("intent:memory"));
    assert_eq!(receipt.affected_nodes.len(), 1);
    assert!(receipt.affected_sources.is_empty());
    assert_eq!(receipt.before_hash, "dry-run:no-workspace-read");
    assert_eq!(receipt.after_hash, "dry-run:no-workspace-write");
    assert!(receipt.validation.accepted);
    assert_eq!(receipt.execution.mode, WorkspacePatchExecutionMode::DryRun);
    assert!(receipt.execution.policy.accepted);
    assert!(receipt.validation.diagnostics.is_empty());
    assert_eq!(receipt.memory_dispatch.len(), 1);
    assert_eq!(receipt.memory_dispatch[0].target, "long-term");
    assert!(!receipt.memory_dispatch[0].accepted);
    assert_eq!(
        receipt.memory_dispatch[0].reason.as_deref(),
        Some("dry-run only: memory dispatch not executed")
    );

    let evidence = gerbil_workspace_patch_receipt_evidence(&receipt);
    assert_eq!(evidence.kind, LoopEvidenceKind::Workflow);
    assert_eq!(evidence.subject, "workspace-patch:intent:memory");
    assert!(evidence.present);
    assert_eq!(
        evidence.detail.as_deref(),
        Some(
            "accepted=true mode=DryRun policy_accepted=true affected_nodes=1 affected_sources=0 memory_dispatch=1 diagnostics=0"
        )
    );
}

#[test]
fn gerbil_patch_intent_dry_run_rejects_missing_dry_run_first() {
    let intent = workspace_patch_intent(false);

    let receipt = GerbilWorkspacePatchIntentDryRunner::dry_run(&intent);

    assert!(!receipt.validation.accepted);
    assert_eq!(receipt.execution.mode, WorkspacePatchExecutionMode::DryRun);
    assert!(!receipt.execution.policy.accepted);
    assert_eq!(receipt.validation.diagnostics.len(), 1);
    assert_eq!(
        receipt.validation.diagnostics[0].severity,
        ValidationSeverity::Error
    );
    assert!(
        receipt.validation.diagnostics[0]
            .message
            .contains("requires dry_run_first")
    );
    assert!(receipt.memory_dispatch.is_empty());

    let evidence = gerbil_workspace_patch_receipt_evidence(&receipt);
    assert_eq!(evidence.kind, LoopEvidenceKind::Workflow);
    assert_eq!(evidence.subject, "workspace-patch:intent:memory");
    assert!(!evidence.present);
    assert_eq!(
        evidence.detail.as_deref(),
        Some(
            "accepted=false mode=DryRun policy_accepted=false affected_nodes=1 affected_sources=0 memory_dispatch=0 diagnostics=1"
        )
    );
}

#[test]
fn gerbil_patch_intent_dry_run_rejects_failed_contract_facts() {
    let intent = workspace_patch_intent(true);

    let receipt = GerbilWorkspacePatchIntentDryRunner::dry_run_with_contract_facts(
        &intent,
        &failed_contract_facts(),
    );

    assert!(!receipt.validation.accepted);
    assert_eq!(receipt.validation.diagnostics.len(), 1);
    assert!(
        receipt.validation.diagnostics[0]
            .message
            .contains("org contract assertion failed")
    );
    assert!(receipt.memory_dispatch.is_empty());
    assert!(!receipt.execution.policy.accepted);

    let evidence = gerbil_workspace_patch_receipt_evidence(&receipt);
    assert!(!evidence.present);
    assert_eq!(
        evidence.detail.as_deref(),
        Some(
            "accepted=false mode=DryRun policy_accepted=false affected_nodes=1 affected_sources=0 memory_dispatch=0 diagnostics=1"
        )
    );
}
