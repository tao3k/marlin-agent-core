use marlin_gerbil_ir::{WorkspacePatchIntentSpec, WorkspaceViewPolicySpec};
use marlin_org_model::{OrgNodeId, TodoState};
use marlin_workspace_patch::{WorkspacePatch, WorkspacePatchOp};
use marlin_workspace_view::{RenderMode, WorkspaceViewSpec};

#[test]
fn gerbil_view_policy_carries_typed_rust_view_spec() {
    let policy = WorkspaceViewPolicySpec {
        policy_id: "active-goal-view".to_string(),
        view: WorkspaceViewSpec::compact(vec!["goal:workspace".into()]),
    };

    assert_eq!(policy.policy_id, "active-goal-view");
    assert_eq!(policy.view.render_mode, RenderMode::AgentCompact);
}

#[test]
fn gerbil_patch_intent_carries_typed_workspace_patch() {
    let node = OrgNodeId::new("memory.org:1:goal");
    let mut patch = WorkspacePatch::new("advance goal from gerbil");
    patch.source_agent = Some("gerbil:test".to_string());
    patch.ops.push(WorkspacePatchOp::SetTodo {
        node: node.clone(),
        state: TodoState::Next,
    });

    let intent = WorkspacePatchIntentSpec {
        intent_id: "intent:advance-goal".to_string(),
        patch,
        dry_run_first: true,
    };

    assert_eq!(intent.intent_id, "intent:advance-goal");
    assert!(intent.dry_run_first);
    assert_eq!(intent.patch.source_agent.as_deref(), Some("gerbil:test"));
    assert_eq!(intent.patch.ops.len(), 1);
    assert!(matches!(
        &intent.patch.ops[0],
        WorkspacePatchOp::SetTodo {
            node: actual,
            state: TodoState::Next,
        } if actual == &node
    ));
}
