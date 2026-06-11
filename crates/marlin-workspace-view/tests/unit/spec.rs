use marlin_org_model::OrgNodeId;
use marlin_workspace_view::{RenderMode, WorkspaceField, WorkspaceViewSpec};

#[test]
fn compact_view_reads_selected_fields_not_raw_org() {
    let spec = WorkspaceViewSpec::compact(vec![OrgNodeId::from("goal:workspace")]);

    assert_eq!(spec.max_tokens, 1_800);
    assert_eq!(spec.render_mode, RenderMode::AgentCompact);
    assert!(spec.include.contains(&WorkspaceField::SourceSpan));
    assert!(spec.include.contains(&WorkspaceField::OpenCheckboxes));
    assert!(spec.include.contains(&WorkspaceField::EvidenceLinks));
    assert!(spec.include.contains(&WorkspaceField::ContractFacts));
    assert!(spec.exclude.contains(&WorkspaceField::RawBlockOutput));
    assert!(spec.exclude.contains(&WorkspaceField::Archived));
}
