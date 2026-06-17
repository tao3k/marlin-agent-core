use marlin_agent_protocol::{
    AgentContentCompressionState, AgentContentNode, AgentContentNodeInput, AgentContentRole,
    AgentSessionFact, ContentCompressionReceipt, ContentCompressionStatus, ContentUsageKind,
    ContentUsageReceipt, ContentUsageReceiptInput, ContextPackReceipt, GraphQueryContext,
    GraphQueryMatch, MemoryTriggerReceipt, MemoryTriggerStatus, ProjectMemoryContextFact,
    ProjectMemoryContextPack, ProjectMemoryRecallIntent, ProjectMemoryRecallRequest,
    ProjectRuntimeContentBodyRef, ProjectRuntimeContentId, ProjectRuntimeContextPackId,
    ProjectRuntimeMemoryCitation, ProjectRuntimeProjectId, ProjectRuntimeReceiptId,
    ProjectRuntimeRootSessionId, ProjectRuntimeSessionId, ProjectRuntimeToolCapabilityCard,
    ProjectRuntimeTurnId, TurnContextItemKind, TurnContextItemViewReceipt,
    TurnContextOmissionReason, TurnContextSteeringReceipt, TurnContextSteeringReceiptInput,
};

#[test]
fn session_fact_round_trips_child_content_fork_and_context_pack() {
    let session = AgentSessionFact::child(
        "project-alpha",
        "root-session-1",
        "child-session-7",
        "parent-session-1",
    )
    .with_content_fork("content-node-42")
    .with_context_pack("context-pack-1")
    .with_history_limit(8);

    let value = serde_json::to_value(&session).expect("session fact should serialize");
    assert_eq!(value["kind"], "Child");
    assert_eq!(value["project_id"], "project-alpha");
    assert_eq!(value["root_session_id"], "root-session-1");
    assert_eq!(value["session_id"], "child-session-7");
    assert_eq!(value["parent_session_id"], "parent-session-1");
    assert_eq!(value["forked_from_content_id"], "content-node-42");
    assert_eq!(value["context_pack_id"], "context-pack-1");
    assert_eq!(value["history_limit"], 8);

    let decoded: AgentSessionFact =
        serde_json::from_value(value).expect("session fact should deserialize");
    assert_eq!(decoded, session);
}

#[test]
fn content_node_records_body_ref_without_raw_body() {
    let content = AgentContentNode::from_input(AgentContentNodeInput {
        project_id: ProjectRuntimeProjectId::new("project-alpha"),
        content_id: ProjectRuntimeContentId::new("content-node-42"),
        root_session_id: ProjectRuntimeRootSessionId::new("root-session-1"),
        role: AgentContentRole::Assistant,
        body_ref: ProjectRuntimeContentBodyRef::new("org://session/root-session-1/content-node-42"),
        token_count: 1_024.into(),
    })
    .with_workspace("workspace-main")
    .with_session("child-session-7")
    .with_parent_content("content-node-41")
    .with_content_fork("content-node-40")
    .with_compression_state(AgentContentCompressionState::Packed)
    .with_source_receipt("receipt-query-1");

    let value = serde_json::to_value(&content).expect("content node should serialize");
    assert_eq!(value["content_id"], "content-node-42");
    assert_eq!(value["role"], "Assistant");
    assert_eq!(
        value["body_ref"],
        "org://session/root-session-1/content-node-42"
    );
    assert_eq!(value["token_count"], 1_024);
    assert_eq!(value["compression_state"], "Packed");
    assert!(value.get("body").is_none());
    assert!(value.get("raw_body").is_none());

    let decoded: AgentContentNode =
        serde_json::from_value(value).expect("content node should deserialize");
    assert_eq!(decoded, content);
}

#[test]
fn content_and_memory_receipts_round_trip_typed_boundaries() {
    let usage = ContentUsageReceipt::from_input(ContentUsageReceiptInput {
        receipt_id: ProjectRuntimeReceiptId::new("receipt-usage-1"),
        project_id: ProjectRuntimeProjectId::new("project-alpha"),
        content_id: ProjectRuntimeContentId::new("content-node-42"),
        usage_kind: ContentUsageKind::Prompt,
        token_count: 1_024.into(),
    })
    .with_session("child-session-7")
    .with_source_receipt("receipt-query-1");
    let compression = ContentCompressionReceipt::new(
        "receipt-compress-1",
        "content-node-42",
        "content-node-42-summary",
        ContentCompressionStatus::Completed,
    )
    .with_input_tokens(1_024)
    .with_output_tokens(256)
    .with_source_receipt("receipt-usage-1");
    let context_pack = ContextPackReceipt::new(
        "receipt-pack-1",
        "context-pack-1",
        "root-session-1",
        "child-session-7",
    )
    .with_content("content-node-42-summary")
    .with_token_budget(2_048)
    .with_source_receipt("receipt-compress-1");
    let memory_trigger = MemoryTriggerReceipt::new(
        "receipt-memory-1",
        "content-node-42-summary",
        MemoryTriggerStatus::Committed,
    )
    .with_memory("memory-runtime-1")
    .with_turn("turn-review-8")
    .with_context_pack("context-pack-1")
    .with_steering_receipt("receipt-steer-1")
    .with_memory_citation("citation-memory-1")
    .with_source_anchor("anchor-memory-1")
    .with_graph_query_receipt("receipt-memory-query-1")
    .with_candidate_score(8_700)
    .with_source_receipt("receipt-pack-1");

    let value = serde_json::json!({
        "usage": usage,
        "compression": compression,
        "context_pack": context_pack,
        "memory_trigger": memory_trigger
    });

    assert_eq!(value["usage"]["token_count"], 1_024);
    assert_eq!(value["compression"]["status"], "Completed");
    assert_eq!(value["compression"]["output_token_count"], 256);
    assert_eq!(value["context_pack"]["token_budget"], 2_048);
    assert_eq!(value["memory_trigger"]["status"], "Committed");
    assert_eq!(value["memory_trigger"]["candidate_score"], 8_700);
    assert_eq!(value["memory_trigger"]["turn_id"], "turn-review-8");
    assert_eq!(value["memory_trigger"]["context_pack_id"], "context-pack-1");
    assert_eq!(
        value["memory_trigger"]["steering_receipt_id"],
        "receipt-steer-1"
    );
    assert_eq!(
        value["memory_trigger"]["memory_citation_id"],
        "citation-memory-1"
    );
    assert_eq!(
        value["memory_trigger"]["source_anchor_ids"][0],
        "anchor-memory-1"
    );
    assert_eq!(
        value["memory_trigger"]["graph_query_receipt_ids"][0],
        "receipt-memory-query-1"
    );
    assert!(value["memory_trigger"].get("memory_shard_body").is_none());
    assert!(value["memory_trigger"].get("raw_transcript").is_none());

    let decoded_usage: ContentUsageReceipt =
        serde_json::from_value(value["usage"].clone()).expect("usage should deserialize");
    let decoded_compression: ContentCompressionReceipt =
        serde_json::from_value(value["compression"].clone())
            .expect("compression should deserialize");
    let decoded_pack: ContextPackReceipt =
        serde_json::from_value(value["context_pack"].clone()).expect("pack should deserialize");
    let decoded_memory: MemoryTriggerReceipt =
        serde_json::from_value(value["memory_trigger"].clone())
            .expect("memory trigger should deserialize");

    assert_eq!(decoded_usage, usage);
    assert_eq!(decoded_compression, compression);
    assert_eq!(decoded_pack, context_pack);
    assert_eq!(decoded_memory, memory_trigger);
}

#[test]
fn turn_context_steering_receipt_records_selected_items_and_citations() {
    let steering = TurnContextSteeringReceipt::from_input(TurnContextSteeringReceiptInput {
        receipt_id: ProjectRuntimeReceiptId::new("receipt-steer-1"),
        turn_id: ProjectRuntimeTurnId::new("turn-review-8"),
        root_session_id: ProjectRuntimeRootSessionId::new("root-session-1"),
        session_id: ProjectRuntimeSessionId::new("child-session-7"),
        context_pack_id: ProjectRuntimeContextPackId::new("context-pack-1"),
    })
    .with_selected_item(
        TurnContextItemKind::SessionContent,
        "content-node-42-summary",
    )
    .with_selected_item(TurnContextItemKind::ProjectMemory, "memory-runtime-1")
    .with_omitted_item(
        TurnContextItemKind::SessionContent,
        "content-node-raw-sibling",
        TurnContextOmissionReason::VisibilityDenied,
    )
    .with_omitted_item(
        TurnContextItemKind::ProjectMemory,
        "memory-runtime-low-rank",
        TurnContextOmissionReason::LowerRanked,
    )
    .with_memory_citation(
        ProjectRuntimeMemoryCitation::new("citation-memory-1", "memory-runtime-1")
            .with_source_anchor("anchor-memory-1")
            .with_graph_query_receipt("receipt-memory-query-1"),
    )
    .with_source_anchor("anchor-content-42")
    .with_graph_query_receipt("receipt-graph-query-1");

    let value = serde_json::to_value(&steering).expect("steering receipt should serialize");
    assert_eq!(value["receipt_id"], "receipt-steer-1");
    assert_eq!(value["turn_id"], "turn-review-8");
    assert_eq!(value["context_pack_id"], "context-pack-1");
    assert_eq!(value["selected_items"][0]["kind"], "SessionContent");
    assert_eq!(
        value["selected_items"][0]["item_id"],
        "content-node-42-summary"
    );
    assert_eq!(value["omitted_items"][0]["reason"], "VisibilityDenied");
    assert_eq!(
        value["memory_citations"][0]["citation_id"],
        "citation-memory-1"
    );
    assert_eq!(
        value["memory_citations"][0]["source_anchor_id"],
        "anchor-memory-1"
    );
    assert_eq!(value["source_anchor_ids"][0], "anchor-content-42");
    assert_eq!(value["graph_query_receipt_ids"][0], "receipt-graph-query-1");
    assert!(value.get("raw_transcript").is_none());
    assert!(value.get("memory_shard_body").is_none());

    let decoded: TurnContextSteeringReceipt =
        serde_json::from_value(value).expect("steering receipt should deserialize");
    assert_eq!(decoded, steering);
}

#[test]
fn turn_context_item_view_receipt_projects_memory_and_tool_cards() {
    let memory_pack = ProjectMemoryContextPack::new(
        "context-pack-1",
        ProjectMemoryRecallRequest::new(
            GraphQueryContext::new("project-alpha"),
            ProjectMemoryRecallIntent::ContinueWork,
        )
        .with_query_term("runtime"),
    )
    .with_fact(ProjectMemoryContextFact::new(
        GraphQueryMatch::new("project-alpha", "Runtime memory claim", 8_900)
            .with_memory("memory-runtime-1")
            .with_source_anchor("anchor-memory-1")
            .with_receipt("receipt-memory-query-1"),
        "Runtime memory claim",
    ))
    .with_source_receipt("receipt-memory-query-1");
    let memory_citation =
        ProjectRuntimeMemoryCitation::new("citation-memory-1", "memory-runtime-1")
            .with_source_anchor("anchor-memory-1")
            .with_graph_query_receipt("receipt-memory-query-1");
    let tool_card = ProjectRuntimeToolCapabilityCard::new(
        GraphQueryMatch::new("project-alpha", "Rust formatter card", 9_100)
            .with_tool_capability("tool:rustfmt")
            .with_source_anchor("tool-node:rustfmt")
            .with_receipt("receipt-tool-query-1"),
    )
    .with_required_receipt("receipt:format-check")
    .with_required_capability("tool:workspace-status")
    .with_isolation_requirement("isolation:write-worktree")
    .with_backend_requirement("backend:process-sandbox");

    let item_view_receipt = TurnContextItemViewReceipt::from_memory_context_pack(
        "receipt-item-view-1",
        &memory_pack,
        &[memory_citation],
    )
    .with_steering_receipt("receipt-steer-1")
    .with_tool_capability_card(&tool_card);

    let value =
        serde_json::to_value(&item_view_receipt).expect("item view receipt should serialize");
    assert_eq!(value["receipt_id"], "receipt-item-view-1");
    assert_eq!(value["context_pack_id"], "context-pack-1");
    assert_eq!(value["steering_receipt_id"], "receipt-steer-1");
    assert_eq!(value["item_views"][0]["kind"], "ProjectMemory");
    assert_eq!(value["item_views"][0]["item_id"], "memory-runtime-1");
    assert_eq!(
        value["item_views"][0]["memory_citation_id"],
        "citation-memory-1"
    );
    assert_eq!(
        value["item_views"][0]["source_anchor_ids"][0],
        "anchor-memory-1"
    );
    assert_eq!(
        value["item_views"][0]["graph_query_receipt_ids"][0],
        "receipt-memory-query-1"
    );
    assert_eq!(value["item_views"][1]["kind"], "ToolCapability");
    assert_eq!(value["item_views"][1]["item_id"], "tool:rustfmt");
    assert_eq!(
        value["item_views"][1]["required_receipt_ids"][0],
        "receipt:format-check"
    );
    assert_eq!(
        value["item_views"][1]["required_capability_ids"][0],
        "tool:workspace-status"
    );
    assert_eq!(
        value["item_views"][1]["isolation_requirement_ids"][0],
        "isolation:write-worktree"
    );
    assert_eq!(
        value["item_views"][1]["backend_requirement_ids"][0],
        "backend:process-sandbox"
    );
    assert!(value.get("raw_transcript").is_none());
    assert!(value.get("memory_shard_body").is_none());

    let decoded: TurnContextItemViewReceipt =
        serde_json::from_value(value).expect("item view receipt should deserialize");
    assert_eq!(decoded, item_view_receipt);
}
