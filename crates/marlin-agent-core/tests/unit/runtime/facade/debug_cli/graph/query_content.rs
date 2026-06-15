use marlin_agent_core::{
    ProjectRuntimeQuerySummary,
    protocol::{
        GraphQueryContext, GraphQueryFamily, GraphQueryMatch, GraphQueryMatchRelationship,
        GraphQueryRelationshipFact, GraphQueryRequest, GraphQueryResponse,
    },
    run_marlin_cli_from_args,
};
use std::fs;
use tempfile::tempdir;

#[test]
fn debug_cli_graph_query_reads_content_query_source_facts() {
    let dir = tempdir().expect("tempdir");
    let input = dir.path().join("content-query.json");
    let request = GraphQueryRequest::new(
        GraphQueryContext::new("project-alpha"),
        GraphQueryFamily::Content,
        "summary",
    )
    .with_content_anchor("content:summary-a");
    let response = GraphQueryResponse::new("receipt:content-query", request).with_match(
        GraphQueryMatch::new("project-alpha", "Packed summary for UI audit", 9_300)
            .with_source_root_session("root-a")
            .with_source_session("session-a")
            .with_source_agent("agent:reviewer")
            .with_content("content:summary-a")
            .with_source_anchor("content-node:summary-a")
            .with_relationship(GraphQueryMatchRelationship::new([
                GraphQueryRelationshipFact::SameProject,
                GraphQueryRelationshipFact::SameContentAncestry,
                GraphQueryRelationshipFact::ContractValidated,
            ])),
    );
    fs::write(
        &input,
        serde_json::to_string(&response).expect("response JSON"),
    )
    .expect("write response");

    let query = run_marlin_cli_from_args([
        "graph",
        "query",
        "--input",
        input.to_str().expect("utf8 path"),
    ]);

    assert_eq!(query.status, 0, "{}", query.stderr);
    let summary: ProjectRuntimeQuerySummary =
        serde_json::from_str(&query.stdout).expect("project runtime query summary");
    assert_eq!(summary.receipt_id.as_str(), "receipt:content-query");
    assert_eq!(summary.family, GraphQueryFamily::Content);
    assert_eq!(summary.match_count, 1);
    assert_eq!(
        summary
            .content_ids
            .iter()
            .map(|content_id| content_id.as_str())
            .collect::<Vec<_>>(),
        vec!["content:summary-a"]
    );
    assert_eq!(
        summary
            .source_agent_ids
            .iter()
            .map(|agent_id| agent_id.as_str())
            .collect::<Vec<_>>(),
        vec!["agent:reviewer"]
    );
    assert_eq!(
        summary
            .source_anchor_ids
            .iter()
            .map(|anchor_id| anchor_id.as_str())
            .collect::<Vec<_>>(),
        vec!["content-node:summary-a"]
    );
    assert!(
        summary
            .relationship_facts
            .contains(&GraphQueryRelationshipFact::SameContentAncestry)
    );
}

#[test]
fn debug_cli_graph_query_executes_content_query_from_org_fixture() {
    let dir = tempdir().expect("tempdir");
    let request_path = dir.path().join("content-request.json");
    let content_path = dir.path().join("content.org");
    let request = GraphQueryRequest::new(
        GraphQueryContext::new("project-alpha")
            .with_workspace("workspace-a")
            .with_worktree("worktree-a")
            .with_root_session("root-a")
            .with_session("session-a")
            .with_content_anchor("content:turn-7"),
        GraphQueryFamily::Content,
        "packed summary body-ref:summary session-a.org",
    )
    .with_content_anchor("content:summary-a")
    .with_limit(5);
    fs::write(
        &request_path,
        serde_json::to_string(&request).expect("request JSON"),
    )
    .expect("write request");
    fs::write(
        &content_path,
        "* Packed summary for UI audit\n\
         :PROPERTIES:\n\
         :CONTENT_ID: content:summary-a\n\
         :PROJECT_ID: project-alpha\n\
         :WORKSPACE_ID: workspace-a\n\
         :WORKTREE_ID: worktree-a\n\
         :ROOT_SESSION_ID: root-a\n\
         :SESSION_ID: session-a\n\
         :AGENT_ID: agent:reviewer\n\
         :PARENT_CONTENT_ID: content:turn-7\n\
         :CONTENT_ROLE: summary\n\
         :BODY_REF: body-ref:summary\n\
         :TOKEN_COUNT: 512\n\
         :COMPRESSION_STATE: packed\n\
         :CONTRACT_VALIDATED: true\n\
         :END:\n\
         Source path marker: session-a.org\n",
    )
    .expect("write content fixture");

    let query = run_marlin_cli_from_args([
        "graph",
        "query",
        "--input",
        request_path.to_str().expect("utf8 path"),
        "--org-memory-fixture",
        content_path.to_str().expect("utf8 path"),
        "--receipt-id",
        "receipt:cli-content",
    ]);

    assert_eq!(query.status, 0, "{}", query.stderr);
    let summary: ProjectRuntimeQuerySummary =
        serde_json::from_str(&query.stdout).expect("project runtime query summary");
    assert_eq!(summary.receipt_id.as_str(), "receipt:cli-content");
    assert_eq!(summary.family, GraphQueryFamily::Content);
    assert_eq!(summary.match_count, 1);
    assert_eq!(
        summary
            .content_ids
            .iter()
            .map(|content_id| content_id.as_str())
            .collect::<Vec<_>>(),
        vec!["content:summary-a"]
    );
    assert_eq!(
        summary
            .source_session_ids
            .iter()
            .map(|session_id| session_id.as_str())
            .collect::<Vec<_>>(),
        vec!["session-a"]
    );
    assert!(
        summary
            .relationship_facts
            .contains(&GraphQueryRelationshipFact::SameContentAncestry)
    );
    assert!(
        summary
            .relationship_facts
            .contains(&GraphQueryRelationshipFact::ContractValidated)
    );
}
