use marlin_org_model::{OrgNode, OrgNodeKind};

#[test]
fn heading_node_starts_as_structured_workspace_record() {
    let node = OrgNode::heading("goal:workspace", "Implement workspace protocol");

    assert_eq!(node.id.as_str(), "goal:workspace");
    assert_eq!(node.kind, OrgNodeKind::Heading);
    assert_eq!(node.title.as_deref(), Some("Implement workspace protocol"));
    assert!(node.properties.is_empty());
    assert!(node.children.is_empty());
}
