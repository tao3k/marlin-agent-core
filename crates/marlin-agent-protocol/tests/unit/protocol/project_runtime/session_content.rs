use marlin_agent_protocol::{
    AgentContentCompressionState, AgentContentNode, AgentContentNodeInput, AgentContentRole,
    AgentSessionFact, ContentCompressionReceipt, ContentCompressionStatus, ContentUsageKind,
    ContentUsageReceipt, ContentUsageReceiptInput, ContextPackReceipt, MemoryTriggerReceipt,
    MemoryTriggerStatus, ProjectRuntimeContentBodyRef, ProjectRuntimeContentId,
    ProjectRuntimeProjectId, ProjectRuntimeReceiptId, ProjectRuntimeRootSessionId,
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
