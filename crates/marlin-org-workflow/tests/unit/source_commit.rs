use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use marlin_org_model::{CheckboxState, OrgNodeId, TodoState};
use marlin_org_store::{FileSystemOrgSourceStore, OrgSourceWritePolicy};
use marlin_org_workflow::{OrgWorkspaceSourceCommit, OrgWorkspaceSourceCommitter};
use marlin_workspace_patch::{WorkspacePatch, WorkspacePatchOp};

#[test]
fn filesystem_workflow_loads_plans_and_commits_workspace_patch() {
    let root = test_root("workspace-facade");
    fs::create_dir_all(&root).expect("create temp root");
    let original = "* TODO Goal\n:PROPERTIES:\n:OWNER: old-owner\n:END:\n- [ ] verify\n";
    fs::write(root.join("memory.org"), original).expect("seed document");

    let node = OrgNodeId::new("memory.org:1:goal");
    let mut patch = WorkspacePatch::new("complete goal");
    patch.ops.push(WorkspacePatchOp::SetTodo {
        node: node.clone(),
        state: TodoState::Done,
    });
    patch.ops.push(WorkspacePatchOp::SetProperty {
        node: node.clone(),
        key: "OWNER".to_owned(),
        value: "marlin-org-workflow".to_owned(),
    });
    patch.ops.push(WorkspacePatchOp::MarkCheckbox {
        node,
        index: 0,
        state: CheckboxState::Checked,
    });
    let request = OrgWorkspaceSourceCommit::new("memory.org", patch, OrgSourceWritePolicy::write());
    let mut store = FileSystemOrgSourceStore::new(&root);

    let receipt = OrgWorkspaceSourceCommitter::commit_document(&mut store, &request);

    assert!(receipt.source.accepted());
    assert_eq!(receipt.plan.edits.len(), 3);
    assert_eq!(receipt.source.planned_edits.len(), 3);
    assert_eq!(
        fs::read_to_string(root.join("memory.org")).expect("read committed document"),
        "* DONE Goal\n:PROPERTIES:\n:OWNER: marlin-org-workflow\n:END:\n- [X] verify\n"
    );
    let _ = fs::remove_dir_all(root);
}

fn test_root(name: &str) -> std::path::PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    std::env::temp_dir().join(format!(
        "marlin-org-workflow-{name}-{}-{suffix}",
        std::process::id()
    ))
}
