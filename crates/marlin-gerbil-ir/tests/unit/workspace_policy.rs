use marlin_gerbil_ir::WorkspaceViewPolicySpec;
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
