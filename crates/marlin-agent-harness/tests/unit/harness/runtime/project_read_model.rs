use marlin_agent_harness::{AgentHarness, AgentHarnessEvidenceKind};
use marlin_agent_protocol::{
    GraphQueryContext, GraphQueryFamily, GraphQueryRequest, ProjectMemoryRecallIntent,
    ProjectMemoryRecallRequest, ProjectRuntimeMemoryCitation, TurnContextItemKind,
    TurnContextItemViewReceipt,
};
use marlin_agent_test_support::project_runtime_read_model_replay_artifact_fixture;
use marlin_org_memory::{
    MemoryOrgWorkspace, PROJECT_MEMORY_CONTRACT_VALIDATED_PROPERTY, PROJECT_MEMORY_ID_PROPERTY,
    PROJECT_MEMORY_PROJECT_ID_PROPERTY, PROJECT_MEMORY_RECALL_QUERY_PROPERTY,
    PROJECT_MEMORY_ROOT_SESSION_ID_PROPERTY, PROJECT_MEMORY_WORKTREE_ID_PROPERTY,
    TOOL_CAPABILITY_BACKEND_REQUIREMENTS_PROPERTY, TOOL_CAPABILITY_CONTRACT_VALIDATED_PROPERTY,
    TOOL_CAPABILITY_ID_PROPERTY, TOOL_CAPABILITY_ISOLATION_REQUIREMENTS_PROPERTY,
    TOOL_CAPABILITY_PROJECT_ID_PROPERTY, TOOL_CAPABILITY_REQUIRED_CAPABILITIES_PROPERTY,
    TOOL_CAPABILITY_REQUIRED_RECEIPTS_PROPERTY, TOOL_CAPABILITY_WORKSPACE_ID_PROPERTY,
};
use marlin_org_model::{OrgNode, OrgNodeId};

#[test]
fn harness_accepts_project_runtime_read_model_replay_artifact() {
    let artifact = project_runtime_read_model_replay_artifact_fixture();
    let report =
        AgentHarness::evaluate_contract(artifact.contract(), &[], artifact.replay_evidence());

    assert!(report.is_success(), "{:?}", report.diagnostics);
    assert_eq!(report.scenario_id, "project-runtime-read-model-replay");
    assert_eq!(
        report
            .evidence
            .iter()
            .filter(|evidence| evidence.kind == AgentHarnessEvidenceKind::Visibility)
            .count(),
        2
    );
    assert_eq!(
        report
            .evidence
            .iter()
            .filter(|evidence| evidence.kind == AgentHarnessEvidenceKind::Runtime)
            .count(),
        2
    );
    assert_eq!(
        report
            .evidence
            .iter()
            .filter(|evidence| evidence.kind == AgentHarnessEvidenceKind::Tool)
            .count(),
        2
    );
    assert_eq!(
        report
            .evidence
            .iter()
            .filter(|evidence| evidence.kind == AgentHarnessEvidenceKind::Content)
            .count(),
        5
    );
    assert!(detail_contains(
        &report.evidence,
        "families=[Memory,Tool,Session,Content]"
    ));
    assert!(detail_contains(&report.evidence, "live_llm=false"));
    assert!(detail_contains(&report.evidence, "sandbox_execution=false"));
    assert!(detail_contains(
        &report.evidence,
        "unsupported_org_fixture=true"
    ));
    assert!(detail_contains(&report.evidence, "turn_id=turn:review-8"));
    assert!(detail_contains(&report.evidence, "status=Committed"));
    assert!(detail_contains(&report.evidence, "candidate_score=8700"));
    assert!(detail_contains(
        &report.evidence,
        "steering_receipt_id=receipt:steer-review"
    ));
    assert!(detail_contains(
        &report.evidence,
        "memory_citation_ids=[citation:memory-runtime-1]"
    ));
    assert!(detail_contains(
        &report.evidence,
        "memory_citation_id=citation:memory-runtime-1"
    ));
    assert!(detail_contains(
        &report.evidence,
        "context_pack_id=context-pack:loaded-node-memory"
    ));
    assert!(detail_contains(
        &report.evidence,
        "receipt_id=receipt:item-view"
    ));
    assert!(detail_contains(
        &report.evidence,
        "item_kinds=[ProjectMemory,ToolCapability]"
    ));
    assert!(detail_contains(&report.evidence, "source_span_absent=true"));
    assert!(detail_contains(
        &report.evidence,
        "explicit_shard_plumbing=false"
    ));
    assert!(detail_contains(
        &report.evidence,
        "missing_source_span_regression=covered"
    ));
    assert!(detail_contains(
        &report.evidence,
        "backend_requirement_ids=[backend:process-sandbox,backend:macos-compatible]"
    ));
    assert!(detail_contains(
        &report.evidence,
        "sandbox_backend_selection=false"
    ));
    assert!(detail_contains(
        &report.evidence,
        "whole_memory_shard=false"
    ));
    assert!(detail_contains(&report.evidence, "raw_transcript=false"));
}

#[test]
fn harness_runtime_read_smoke_consumes_loaded_node_project_memory_context_pack() {
    let artifact = project_runtime_read_model_replay_artifact_fixture();
    let report =
        AgentHarness::evaluate_contract(artifact.contract(), &[], artifact.replay_evidence());
    assert!(report.is_success(), "{:?}", report.diagnostics);

    let workspace = MemoryOrgWorkspace::from_nodes([loaded_memory_node_without_source(
        "memory-node:missing-source",
        "memory:missing-source",
        "Loaded node memory context pack",
        "loaded-node memory recall",
    )]);
    let request = ProjectMemoryRecallRequest::new(
        GraphQueryContext::new("project-alpha")
            .with_worktree("worktree-a")
            .with_root_session("root-session-1"),
        ProjectMemoryRecallIntent::ContinueWork,
    )
    .with_query_term("loaded-node")
    .with_query_term("memory")
    .with_query_term("recall")
    .with_limit(5);

    let pack = workspace
        .recall_project_memory_from_loaded_nodes(
            "context-pack:loaded-node-memory",
            "receipt:loaded-node-memory-query",
            request,
        )
        .expect("loaded-node recall should produce a context pack");

    assert_eq!(
        pack.context_pack_id.as_str(),
        "context-pack:loaded-node-memory"
    );
    assert_eq!(pack.source_receipts.len(), 1);
    assert_eq!(
        pack.source_receipts[0].as_str(),
        "receipt:loaded-node-memory-query"
    );
    assert_eq!(pack.facts.len(), 1);
    let fact = &pack.facts[0];
    assert_eq!(fact.claim, "Loaded node memory context pack");
    assert!(fact.source_span.is_none());
    assert_eq!(
        fact.graph_match
            .source_anchor_id
            .as_ref()
            .expect("source anchor")
            .as_str(),
        "memory-node:missing-source"
    );
    assert_eq!(
        fact.graph_match
            .memory_id
            .as_ref()
            .expect("memory id")
            .as_str(),
        "memory:missing-source"
    );
    assert_eq!(fact.evidence_ids[0].as_str(), "memory:missing-source");
    assert!(detail_contains(
        &report.evidence,
        "explicit_shard_plumbing=false"
    ));
}

#[test]
fn harness_runtime_read_smoke_consumes_tool_capability_card_requirements() {
    let artifact = project_runtime_read_model_replay_artifact_fixture();
    let report =
        AgentHarness::evaluate_contract(artifact.contract(), &[], artifact.replay_evidence());
    assert!(report.is_success(), "{:?}", report.diagnostics);

    let workspace = MemoryOrgWorkspace::from_nodes([tool_capability_node(
        "tool-node:rustfmt-card",
        "tool:rustfmt",
        "Rust formatter card",
    )]);
    let request = GraphQueryRequest::new(
        GraphQueryContext::new("project-alpha").with_workspace("workspace-a"),
        GraphQueryFamily::Tool,
        "rust formatter card",
    )
    .with_tool_capability("tool:rustfmt")
    .with_limit(5);

    let cards = workspace
        .query_tool_capability_cards("receipt:tool-card-query", request)
        .expect("tool card query should succeed");

    assert_eq!(cards.len(), 1);
    let card = &cards[0];
    assert_eq!(
        card.graph_match
            .tool_capability_id
            .as_ref()
            .expect("tool capability")
            .as_str(),
        "tool:rustfmt"
    );
    assert_eq!(
        card.required_receipt_ids[0].as_str(),
        "receipt:format-check"
    );
    assert_eq!(
        card.required_capability_ids[0].as_str(),
        "tool:workspace-status"
    );
    assert_eq!(
        card.isolation_requirement_ids[0].as_str(),
        "isolation:write-worktree"
    );
    assert_eq!(
        card.backend_requirement_ids[0].as_str(),
        "backend:process-sandbox"
    );
    assert!(detail_contains(
        &report.evidence,
        "sandbox_backend_selection=false"
    ));
}

#[test]
fn harness_runtime_read_smoke_consumes_turn_context_item_view_receipt() {
    let artifact = project_runtime_read_model_replay_artifact_fixture();
    let report =
        AgentHarness::evaluate_contract(artifact.contract(), &[], artifact.replay_evidence());
    assert!(report.is_success(), "{:?}", report.diagnostics);

    let workspace = MemoryOrgWorkspace::from_nodes([
        loaded_memory_node_without_source(
            "memory-node:missing-source",
            "memory:missing-source",
            "Loaded node memory context pack",
            "loaded-node memory recall",
        ),
        tool_capability_node(
            "tool-node:rustfmt-card",
            "tool:rustfmt",
            "Rust formatter card",
        ),
    ]);
    let memory_request = ProjectMemoryRecallRequest::new(
        GraphQueryContext::new("project-alpha")
            .with_worktree("worktree-a")
            .with_root_session("root-session-1"),
        ProjectMemoryRecallIntent::ContinueWork,
    )
    .with_query_term("loaded-node")
    .with_query_term("memory")
    .with_query_term("recall")
    .with_limit(5);
    let memory_pack = workspace
        .recall_project_memory_from_loaded_nodes(
            "context-pack:loaded-node-memory",
            "receipt:loaded-node-memory-query",
            memory_request,
        )
        .expect("loaded-node recall should produce a context pack");
    let tool_request = GraphQueryRequest::new(
        GraphQueryContext::new("project-alpha").with_workspace("workspace-a"),
        GraphQueryFamily::Tool,
        "rust formatter card",
    )
    .with_tool_capability("tool:rustfmt")
    .with_limit(5);
    let tool_cards = workspace
        .query_tool_capability_cards("receipt:tool-card-query", tool_request)
        .expect("tool card query should produce a capability card");
    let memory_citation =
        ProjectRuntimeMemoryCitation::new("citation:memory-runtime-1", "memory:missing-source")
            .with_source_anchor("memory-node:missing-source")
            .with_graph_query_receipt("receipt:loaded-node-memory-query");

    let item_view_receipt = TurnContextItemViewReceipt::from_memory_context_pack(
        "receipt:item-view",
        &memory_pack,
        &[memory_citation],
    )
    .with_steering_receipt("receipt:steer-review")
    .with_tool_capability_card(&tool_cards[0]);

    assert_eq!(item_view_receipt.item_views.len(), 2);
    assert_eq!(
        item_view_receipt.item_views[0].kind,
        TurnContextItemKind::ProjectMemory
    );
    assert_eq!(
        item_view_receipt.item_views[0]
            .memory_citation_id
            .as_ref()
            .expect("memory citation")
            .as_str(),
        "citation:memory-runtime-1"
    );
    assert_eq!(
        item_view_receipt.item_views[1].kind,
        TurnContextItemKind::ToolCapability
    );
    assert_eq!(
        item_view_receipt.item_views[1].required_receipt_ids[0].as_str(),
        "receipt:format-check"
    );
    assert_eq!(
        item_view_receipt.item_views[1].backend_requirement_ids[1].as_str(),
        "backend:macos-compatible"
    );
    assert_eq!(
        item_view_receipt.source_anchor_ids[0].as_str(),
        "memory-node:missing-source"
    );
    assert_eq!(
        item_view_receipt.source_anchor_ids[1].as_str(),
        "tool-node:rustfmt-card"
    );
    assert!(detail_contains(
        &report.evidence,
        "receipt_id=receipt:item-view"
    ));
    assert!(detail_contains(
        &report.evidence,
        "item_kinds=[ProjectMemory,ToolCapability]"
    ));
    assert!(detail_contains(
        &report.evidence,
        "sandbox_backend_selection=false"
    ));
}

fn loaded_memory_node_without_source(
    node_id: &str,
    memory_id: &str,
    title: &str,
    recall_query: &str,
) -> OrgNode {
    let mut node = OrgNode::heading(OrgNodeId::from(node_id), title);
    node.properties.insert(
        PROJECT_MEMORY_ID_PROPERTY.to_string(),
        memory_id.to_string(),
    );
    node.properties.insert(
        PROJECT_MEMORY_PROJECT_ID_PROPERTY.to_string(),
        "project-alpha".to_string(),
    );
    node.properties.insert(
        PROJECT_MEMORY_WORKTREE_ID_PROPERTY.to_string(),
        "worktree-a".to_string(),
    );
    node.properties.insert(
        PROJECT_MEMORY_ROOT_SESSION_ID_PROPERTY.to_string(),
        "root-session-1".to_string(),
    );
    node.properties.insert(
        PROJECT_MEMORY_RECALL_QUERY_PROPERTY.to_string(),
        recall_query.to_string(),
    );
    node.properties.insert(
        PROJECT_MEMORY_CONTRACT_VALIDATED_PROPERTY.to_string(),
        "true".to_string(),
    );
    node
}

fn tool_capability_node(node_id: &str, capability_id: &str, title: &str) -> OrgNode {
    let mut node = OrgNode::heading(OrgNodeId::from(node_id), title);
    node.properties.insert(
        TOOL_CAPABILITY_ID_PROPERTY.to_string(),
        capability_id.to_string(),
    );
    node.properties.insert(
        TOOL_CAPABILITY_PROJECT_ID_PROPERTY.to_string(),
        "project-alpha".to_string(),
    );
    node.properties.insert(
        TOOL_CAPABILITY_WORKSPACE_ID_PROPERTY.to_string(),
        "workspace-a".to_string(),
    );
    node.properties.insert(
        TOOL_CAPABILITY_REQUIRED_RECEIPTS_PROPERTY.to_string(),
        "receipt:format-check receipt:workspace-clean".to_string(),
    );
    node.properties.insert(
        TOOL_CAPABILITY_REQUIRED_CAPABILITIES_PROPERTY.to_string(),
        "tool:workspace-status".to_string(),
    );
    node.properties.insert(
        TOOL_CAPABILITY_ISOLATION_REQUIREMENTS_PROPERTY.to_string(),
        "isolation:write-worktree".to_string(),
    );
    node.properties.insert(
        TOOL_CAPABILITY_BACKEND_REQUIREMENTS_PROPERTY.to_string(),
        "backend:process-sandbox backend:macos-compatible".to_string(),
    );
    node.properties.insert(
        TOOL_CAPABILITY_CONTRACT_VALIDATED_PROPERTY.to_string(),
        "true".to_string(),
    );
    node
}

fn detail_contains(evidence: &[marlin_agent_harness::AgentHarnessEvidence], needle: &str) -> bool {
    evidence
        .iter()
        .filter_map(|evidence| evidence.detail.as_deref())
        .any(|detail| detail.contains(needle))
}
