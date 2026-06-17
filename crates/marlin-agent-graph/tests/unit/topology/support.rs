use marlin_agent_graph::{
    AgentCapability, AgentCoordinationEvidenceKind, AgentCoordinationEvidenceRef, AgentEdgeId,
    AgentEvidenceId, AgentGraphId, AgentNode, AgentNodeId, AgentRole, GraphLoopEntryRef,
    GraphLoopGraphRef, GraphLoopNodeRef,
};

pub(super) fn node(id: &str, role_name: &str, loop_graph: &str, entry_node: &str) -> AgentNode {
    AgentNode {
        node_id: node_id(id),
        role: role(role_name),
        capabilities: vec![capability(role_name)],
        loop_entry: GraphLoopEntryRef {
            graph: GraphLoopGraphRef::new(loop_graph).unwrap(),
            entry_node: GraphLoopNodeRef::new(entry_node).unwrap(),
        },
        memory_scope: None,
        policy_scope: None,
    }
}

pub(super) fn evidence(
    kind: AgentCoordinationEvidenceKind,
    id: &str,
) -> AgentCoordinationEvidenceRef {
    AgentCoordinationEvidenceRef {
        kind,
        evidence_id: AgentEvidenceId::new(id).unwrap(),
    }
}

pub(super) fn graph_id(value: &str) -> AgentGraphId {
    AgentGraphId::new(value).unwrap()
}

pub(super) fn node_id(value: &str) -> AgentNodeId {
    AgentNodeId::new(value).unwrap()
}

pub(super) fn edge_id(value: &str) -> AgentEdgeId {
    AgentEdgeId::new(value).unwrap()
}

pub(super) fn role(value: &str) -> AgentRole {
    AgentRole::new(value).unwrap()
}

pub(super) fn capability(value: &str) -> AgentCapability {
    AgentCapability::new(value).unwrap()
}
