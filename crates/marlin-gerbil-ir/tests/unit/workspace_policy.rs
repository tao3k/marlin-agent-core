use marlin_gerbil_ir::{
    ReleaseGateSpec, ReleaseTopologySpec, ReleaseVisibilitySpec, WorkspacePatchIntentSpec,
    WorkspaceViewPolicySpec,
};
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

#[test]
fn gerbil_release_topology_carries_package_gates_and_dependency_chains() {
    let topology = ReleaseTopologySpec {
        topology_id: "gerbil-scheme-internal-release".to_string(),
        crate_name: "marlin-gerbil-scheme".to_string(),
        publish_enabled: false,
        asset_audit_command: "cargo package -p marlin-gerbil-scheme --list --allow-dirty"
            .to_string(),
        package_assets: vec![
            "README.md".to_string(),
            "examples/workspace-patch-intent-workflow.rs".to_string(),
            "fixtures/gerbil/marlin/protocol.ss".to_string(),
        ],
        runtime_dependency_chain: vec![
            "marlin-gerbil-ir".to_string(),
            "marlin-org-model".to_string(),
            "marlin-agent-protocol".to_string(),
        ],
        workflow_dependency_chain: vec![
            "marlin-workspace-patch".to_string(),
            "marlin-org-workflow".to_string(),
        ],
        gates: vec![ReleaseGateSpec {
            gate_id: "package-assets".to_string(),
            command: "cargo package -p marlin-gerbil-scheme --list --allow-dirty".to_string(),
            requires_local_gerbil: false,
            required_artifacts: vec!["fixtures/gerbil/marlin/protocol.ss".to_string()],
            visibility: vec![ReleaseVisibilitySpec {
                report_key: "package_asset_audit".to_string(),
                evidence_keys: vec!["required_artifacts".to_string()],
                artifact_paths: vec!["fixtures/gerbil/marlin/protocol.ss".to_string()],
            }],
        }],
    };

    assert_eq!(topology.crate_name, "marlin-gerbil-scheme");
    assert!(!topology.publish_enabled);
    assert!(
        topology
            .package_assets
            .iter()
            .any(|asset| asset == "README.md")
    );
    assert!(
        topology
            .runtime_dependency_chain
            .iter()
            .any(|crate_name| crate_name == "marlin-agent-protocol")
    );
    assert_eq!(topology.gates[0].gate_id, "package-assets");
    assert!(!topology.gates[0].requires_local_gerbil);
    assert_eq!(
        topology.gates[0].visibility[0].report_key,
        "package_asset_audit"
    );
    assert!(
        topology.gates[0].visibility[0]
            .evidence_keys
            .iter()
            .any(|key| key == "required_artifacts")
    );
    assert_eq!(
        topology.gates[0].visibility[0].artifact_paths,
        topology.gates[0].required_artifacts
    );
}
