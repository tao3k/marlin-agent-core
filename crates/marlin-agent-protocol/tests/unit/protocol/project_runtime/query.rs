use marlin_agent_protocol::{
    GraphQueryContext, GraphQueryExternalProjectPolicy, GraphQueryFallbackPolicy,
    GraphQueryFallbackScope, GraphQueryFamily, GraphQueryMatch, GraphQueryMatchRelationship,
    GraphQueryRelationshipFact, GraphQueryRequest, GraphQueryResponse, GraphQuerySecretVisibility,
    GraphQueryVisibleSurface,
};

#[test]
fn memory_query_defaults_to_same_project_without_external_projects() {
    let request: GraphQueryRequest = serde_json::from_value(serde_json::json!({
        "context": {
            "project_id": "project-alpha"
        },
        "family": "Memory",
        "query": "routing policy"
    }))
    .expect("minimal memory query should deserialize");

    assert_eq!(request.context.project_id.as_str(), "project-alpha");
    assert_eq!(request.family, GraphQueryFamily::Memory);
    assert!(
        request
            .context
            .visibility
            .allows_surface(GraphQueryVisibleSurface::Memory)
    );
    assert!(
        request
            .context
            .visibility
            .allows_surface(GraphQueryVisibleSurface::Tools)
    );
    assert_eq!(
        request.context.visibility.secrets,
        GraphQuerySecretVisibility::Denied
    );
    assert!(
        request
            .context
            .fallback_policy
            .includes_scope(GraphQueryFallbackScope::SessionLocal)
    );
    assert!(
        request
            .context
            .fallback_policy
            .includes_scope(GraphQueryFallbackScope::Project)
    );
    assert!(
        request
            .context
            .fallback_policy
            .includes_scope(GraphQueryFallbackScope::Workspace)
    );
    assert!(
        !request
            .context
            .fallback_policy
            .includes_scope(GraphQueryFallbackScope::Global)
    );
    assert_eq!(
        request.context.fallback_policy.external_projects,
        GraphQueryExternalProjectPolicy::Disabled
    );
}

#[test]
fn child_session_content_anchor_query_keeps_root_session_boundary() {
    let request = GraphQueryRequest::new(
        GraphQueryContext::new("project-alpha")
            .with_workspace("workspace-main")
            .with_worktree("worktree-ui")
            .with_branch("feature/runtime-graph")
            .with_root_session("root-session-1")
            .with_session("child-session-7")
            .with_agent("agent-ui")
            .with_content_anchor("content-node-42"),
        GraphQueryFamily::Content,
        "selected component ancestry",
    )
    .with_content_anchor("content-node-42")
    .with_limit(12);

    let value = serde_json::to_value(&request).expect("request should serialize");
    assert_eq!(value["context"]["project_id"], "project-alpha");
    assert_eq!(value["context"]["root_session_id"], "root-session-1");
    assert_eq!(value["context"]["session_id"], "child-session-7");
    assert_eq!(value["context"]["content_anchor"], "content-node-42");
    assert_eq!(value["content_id"], "content-node-42");
    assert_eq!(value["limit"], 12);

    let decoded: GraphQueryRequest =
        serde_json::from_value(value).expect("request should deserialize");
    assert_eq!(
        decoded
            .context
            .root_session_id
            .as_ref()
            .expect("root session")
            .as_str(),
        "root-session-1"
    );
    assert_eq!(
        decoded
            .context
            .session_id
            .as_ref()
            .expect("child session")
            .as_str(),
        "child-session-7"
    );
}

#[test]
fn tool_capability_query_uses_project_fallback_without_backend_fields() {
    let request = GraphQueryRequest::new(
        GraphQueryContext::new("project-alpha")
            .with_workspace("workspace-main")
            .with_fallback_policy(GraphQueryFallbackPolicy::same_project()),
        GraphQueryFamily::Tool,
        "format org RFC index",
    )
    .with_tool_capability("tool.org.query");

    let value = serde_json::to_value(&request).expect("tool query should serialize");

    assert_eq!(value["family"], "Tool");
    assert_eq!(value["capability_id"], "tool.org.query");
    assert!(
        value["context"]["fallback_policy"]["scopes"]
            .as_array()
            .expect("fallback scopes")
            .contains(&serde_json::json!("Project"))
    );
    assert_eq!(
        value["context"]["fallback_policy"]["external_projects"],
        "Disabled"
    );
    assert!(value["context"].get("sandbox").is_none());
    assert!(value["context"].get("backend").is_none());
}

#[test]
fn topology_query_defaults_to_visible_overview_surface() {
    let request = GraphQueryRequest::new(
        GraphQueryContext::new("project-alpha"),
        GraphQueryFamily::Topology,
        "imports-project project-overview",
    );

    let value = serde_json::to_value(&request).expect("topology query should serialize");
    assert_eq!(value["family"], "Topology");
    assert!(
        request
            .context
            .visibility
            .allows_surface(GraphQueryVisibleSurface::Topology)
    );

    let decoded: GraphQueryRequest =
        serde_json::from_value(value).expect("topology query should deserialize");
    assert_eq!(decoded.family, GraphQueryFamily::Topology);
}

#[test]
fn evidence_and_failure_queries_preserve_typed_receipt_anchors() {
    let evidence_request = GraphQueryRequest::new(
        GraphQueryContext::new("project-alpha"),
        GraphQueryFamily::Evidence,
        "no-live replay receipts",
    );
    let evidence_response = GraphQueryResponse::new("receipt:evidence-query", evidence_request)
        .with_match(
            GraphQueryMatch::new("project-alpha", "replay evidence captured", 9_300)
                .with_evidence("evidence:replay:run-1")
                .with_receipt("receipt:loop:run-1")
                .with_relationship(GraphQueryMatchRelationship::new([
                    GraphQueryRelationshipFact::SameProject,
                    GraphQueryRelationshipFact::ContractValidated,
                ])),
        );

    let value = serde_json::to_value(&evidence_response).expect("evidence response serializes");
    assert_eq!(value["request"]["family"], "Evidence");
    assert_eq!(value["matches"][0]["evidence_id"], "evidence:replay:run-1");
    assert_eq!(value["matches"][0]["receipt_id"], "receipt:loop:run-1");

    let decoded: GraphQueryResponse =
        serde_json::from_value(value).expect("evidence response deserializes");
    assert_eq!(decoded, evidence_response);

    let failure_request = GraphQueryRequest::new(
        GraphQueryContext::new("project-alpha"),
        GraphQueryFamily::Failure,
        "policy failure root cause",
    );
    let failure_response = GraphQueryResponse::new("receipt:failure-query", failure_request)
        .with_match(
            GraphQueryMatch::new("project-alpha", "sandbox denied policy failure", 8_800)
                .with_evidence("failure:run-1:0")
                .with_receipt("receipt:failure:run-1:0")
                .with_source_anchor("source:diagnostic:sandbox-denied"),
        );

    let value = serde_json::to_value(&failure_response).expect("failure response serializes");
    assert_eq!(value["request"]["family"], "Failure");
    assert_eq!(value["matches"][0]["evidence_id"], "failure:run-1:0");
    assert_eq!(value["matches"][0]["receipt_id"], "receipt:failure:run-1:0");
}

#[test]
fn response_receipt_preserves_typed_sources_and_relationships() {
    let request = GraphQueryRequest::new(
        GraphQueryContext::new("project-alpha").with_root_session("root-session-1"),
        GraphQueryFamily::Memory,
        "runtime graph boundary",
    );
    let response = GraphQueryResponse::new("receipt-query-1", request).with_match(
        GraphQueryMatch::new("project-alpha", "same project memory hit", 9_200)
            .with_source_workspace("workspace-main")
            .with_source_worktree("worktree-ui")
            .with_source_root_session("root-session-1")
            .with_source_session("child-session-7")
            .with_source_agent("agent-ui")
            .with_memory("memory-runtime-1")
            .with_content("content-node-42")
            .with_relationship(GraphQueryMatchRelationship::new([
                GraphQueryRelationshipFact::SameProject,
                GraphQueryRelationshipFact::SameWorkspace,
                GraphQueryRelationshipFact::SameRootSession,
                GraphQueryRelationshipFact::SameSessionLineage,
                GraphQueryRelationshipFact::SameContentAncestry,
                GraphQueryRelationshipFact::SameWorktreeProvenance,
                GraphQueryRelationshipFact::ExplicitBacklink,
                GraphQueryRelationshipFact::ContractValidated,
            ])),
    );

    let value = serde_json::to_value(&response).expect("response should serialize");
    assert_eq!(value["receipt_id"], "receipt-query-1");
    assert_eq!(value["matches"][0]["source_project_id"], "project-alpha");
    assert_eq!(value["matches"][0]["memory_id"], "memory-runtime-1");
    assert!(
        value["matches"][0]["relationship"]["facts"]
            .as_array()
            .expect("relationship facts")
            .contains(&serde_json::json!("SameWorktreeProvenance"))
    );
    assert!(
        !value["matches"][0]["relationship"]["facts"]
            .as_array()
            .expect("relationship facts")
            .contains(&serde_json::json!("ExternalProject"))
    );

    let decoded: GraphQueryResponse =
        serde_json::from_value(value).expect("response should deserialize");
    assert_eq!(decoded, response);
}
