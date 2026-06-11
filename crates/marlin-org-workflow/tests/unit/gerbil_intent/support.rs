use std::time::{SystemTime, UNIX_EPOCH};

use marlin_gerbil_ir::{GerbilWorkspaceContractFacts, WorkspacePatchIntentSpec};
use marlin_org_model::{
    OrgContractId, OrgContractSeverity, OrgContractValidationReceipt, OrgContractValidationReport,
    OrgContractValidationStatus, OrgContractValidationTarget, OrgNodeId,
};
use marlin_workspace_patch::{WorkspacePatch, WorkspacePatchOp};

pub fn workspace_patch_intent(dry_run_first: bool) -> WorkspacePatchIntentSpec {
    let node = OrgNodeId::new("memory.org:1:goal");
    let mut patch = WorkspacePatch::new("remember gerbil workspace intent");
    patch.source_agent = Some("gerbil:test".to_string());
    patch.ops.push(WorkspacePatchOp::MarkMemoryCandidate {
        node,
        dispatch: "long-term".to_string(),
    });
    WorkspacePatchIntentSpec {
        intent_id: "intent:memory".to_string(),
        patch,
        dry_run_first,
    }
}

pub fn workspace_patch_source_intent(dry_run_first: bool) -> WorkspacePatchIntentSpec {
    let node = OrgNodeId::new("memory.org:1:goal");
    let mut patch = WorkspacePatch::new("append gerbil verification task");
    patch.source_agent = Some("gerbil:test".to_string());
    patch.ops.push(WorkspacePatchOp::AddCheckbox {
        node,
        text: "verify via gerbil".to_string(),
        state: marlin_org_model::CheckboxState::Open,
    });
    WorkspacePatchIntentSpec {
        intent_id: "intent:source".to_string(),
        patch,
        dry_run_first,
    }
}

pub fn failed_contract_facts() -> GerbilWorkspaceContractFacts {
    GerbilWorkspaceContractFacts {
        validations: OrgContractValidationReport {
            receipts: vec![OrgContractValidationReceipt {
                contract_id: OrgContractId::new("agent.task.v1"),
                assertion_id: "task.has-goal".to_string(),
                target: OrgContractValidationTarget::Node(OrgNodeId::new("memory.org:1:goal")),
                status: OrgContractValidationStatus::Failed,
                severity: OrgContractSeverity::new("error"),
                message: Some("Goal section is required".to_string()),
                source: None,
            }],
            diagnostics: Vec::new(),
        },
        ..Default::default()
    }
}

pub fn test_root(name: &str) -> std::path::PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    std::env::temp_dir().join(format!(
        "marlin-org-workflow-{name}-{}-{suffix}",
        std::process::id()
    ))
}
