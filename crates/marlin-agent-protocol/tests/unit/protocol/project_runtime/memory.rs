use marlin_agent_protocol::{
    GraphQueryContext, GraphQueryFamily, GraphQueryMatch, GraphQueryMatchRelationship,
    GraphQueryRelationshipFact, ProjectMemoryContextFact, ProjectMemoryContextPack,
    ProjectMemoryRecallIntent, ProjectMemoryRecallRequest,
};

#[test]
fn project_memory_recall_request_compiles_to_memory_graph_query() {
    let recall = ProjectMemoryRecallRequest::new(
        GraphQueryContext::new("project-alpha")
            .with_workspace("workspace-main")
            .with_root_session("root-session-1")
            .with_session("child-session-7")
            .with_content_anchor("content-node-42"),
        ProjectMemoryRecallIntent::ContinueWork,
    )
    .with_query_term("hybrid")
    .with_query_term("frontier")
    .with_content_anchor("content-node-42")
    .with_limit(3);

    let graph_query = recall.as_graph_query_request();

    assert_eq!(graph_query.family, GraphQueryFamily::Memory);
    assert_eq!(graph_query.query, "hybrid frontier");
    assert_eq!(
        graph_query
            .content_id
            .as_ref()
            .expect("content anchor")
            .as_str(),
        "content-node-42"
    );
    assert_eq!(graph_query.limit.expect("result budget").as_u16(), 3);

    let value = serde_json::to_value(&recall).expect("recall should serialize");
    assert_eq!(value["intent"], "ContinueWork");
    assert_eq!(value["query_terms"][0], "hybrid");
    assert_eq!(value["query_terms"][1], "frontier");
    assert!(value.get("org_memory").is_none());
    assert!(value.get("memory_shards").is_none());

    let decoded: ProjectMemoryRecallRequest =
        serde_json::from_value(value).expect("recall should deserialize");
    assert_eq!(decoded, recall);
}

#[test]
fn project_memory_context_pack_preserves_compact_facts_not_shards() {
    let recall = ProjectMemoryRecallRequest::new(
        GraphQueryContext::new("project-alpha").with_root_session("root-session-1"),
        ProjectMemoryRecallIntent::RecoverDecision,
    )
    .with_query_term("org-first")
    .with_query_term("runtime");

    let fact = ProjectMemoryContextFact::new(
        GraphQueryMatch::new("project-alpha", "Org-first runtime design", 9_400)
            .with_memory("memory-org-first-runtime")
            .with_content("content-node-42")
            .with_source_anchor("org-node-memory-42")
            .with_relationship(GraphQueryMatchRelationship::new([
                GraphQueryRelationshipFact::SameProject,
                GraphQueryRelationshipFact::ContractValidated,
                GraphQueryRelationshipFact::ExplicitBacklink,
            ])),
        "Org is the core reference substrate for project memory recall.",
    )
    .with_source_span(
        "docs/superpowers/specs/2026-06-15-org-first-project-memory-runtime-design.md:8-23",
    )
    .with_evidence("evidence:org-first-runtime-design");

    let pack = ProjectMemoryContextPack::new("context-pack-org-first-runtime", recall)
        .with_fact(fact)
        .with_source_receipt("receipt-query-1");

    let value = serde_json::to_value(&pack).expect("context pack should serialize");
    assert_eq!(value["context_pack_id"], "context-pack-org-first-runtime");
    assert_eq!(
        value["facts"][0]["claim"],
        "Org is the core reference substrate for project memory recall."
    );
    assert_eq!(
        value["facts"][0]["source_span"],
        "docs/superpowers/specs/2026-06-15-org-first-project-memory-runtime-design.md:8-23"
    );
    assert_eq!(
        value["facts"][0]["evidence_ids"][0],
        "evidence:org-first-runtime-design"
    );
    assert_eq!(
        value["facts"][0]["graph_match"]["source_anchor_id"],
        "org-node-memory-42"
    );
    assert_eq!(value["source_receipts"][0], "receipt-query-1");
    assert!(value.get("raw_transcript").is_none());
    assert!(value.get("org_memory").is_none());
    assert!(value.get("shard_body").is_none());

    let decoded: ProjectMemoryContextPack =
        serde_json::from_value(value).expect("context pack should deserialize");
    assert_eq!(decoded, pack);
}
