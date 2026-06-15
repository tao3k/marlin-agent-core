use marlin_agent_core::{
    ProjectRuntimeQuerySummary,
    protocol::{
        GraphQueryContext, GraphQueryFamily, GraphQueryRelationshipFact, GraphQueryRequest,
    },
    run_marlin_cli_from_args,
};
use std::fs;
use tempfile::tempdir;

#[test]
fn debug_cli_graph_query_executes_store_backed_tool_capability_query() {
    let dir = tempdir().expect("tempdir");
    let request_path = dir.path().join("tool-request.json");
    let tool_root = dir.path().join("tools.org");
    let request = GraphQueryRequest::new(
        GraphQueryContext::new("project-alpha")
            .with_workspace("workspace-a")
            .with_root_session("root-a"),
        GraphQueryFamily::Tool,
        "rustfmt receipt:format-check tools.org",
    )
    .with_tool_capability("tool:rustfmt")
    .with_limit(5);
    fs::write(
        &request_path,
        serde_json::to_string(&request).expect("request JSON"),
    )
    .expect("write request");
    fs::write(
        &tool_root,
        "* Rust formatter capability\n\
         :PROPERTIES:\n\
         :TOOL_CAPABILITY_ID: tool:rustfmt\n\
         :PROJECT_ID: project-alpha\n\
         :WORKSPACE_ID: workspace-a\n\
         :ROOT_SESSION_ID: root-a\n\
         :REQUIRED_RECEIPTS: receipt:format-check\n\
         :CONTRACT_VALIDATED: true\n\
         :END:\n\
         Source path marker: tools.org\n",
    )
    .expect("write tool root");

    let query = run_marlin_cli_from_args([
        "graph",
        "query",
        "--input",
        request_path.to_str().expect("utf8 path"),
        "--org-tool-store-root",
        dir.path().to_str().expect("utf8 path"),
        "--org-tool-root",
        "tools.org",
        "--receipt-id",
        "receipt:cli-store-tool",
    ]);

    assert_eq!(query.status, 0, "{}", query.stderr);
    let summary: ProjectRuntimeQuerySummary =
        serde_json::from_str(&query.stdout).expect("project runtime query summary");
    assert_eq!(summary.receipt_id.as_str(), "receipt:cli-store-tool");
    assert_eq!(summary.family, GraphQueryFamily::Tool);
    assert_eq!(summary.match_count, 1);
    assert_eq!(
        summary
            .tool_capability_ids
            .iter()
            .map(|capability_id| capability_id.as_str())
            .collect::<Vec<_>>(),
        vec!["tool:rustfmt"]
    );
    assert!(
        summary
            .relationship_facts
            .contains(&GraphQueryRelationshipFact::ContractValidated)
    );
}

#[test]
fn debug_cli_graph_query_requires_tool_root_for_tool_store_root() {
    let dir = tempdir().expect("tempdir");
    let query = run_marlin_cli_from_args([
        "graph",
        "query",
        "--org-tool-store-root",
        dir.path().to_str().expect("utf8 path"),
    ]);

    assert_ne!(query.status, 0);
    assert!(
        query
            .stderr
            .contains("--org-tool-store-root requires --org-tool-root")
    );
}

#[test]
fn debug_cli_graph_query_rejects_missing_tool_store_root() {
    let dir = tempdir().expect("tempdir");
    let request_path = dir.path().join("tool-request.json");
    let request = GraphQueryRequest::new(
        GraphQueryContext::new("project-alpha"),
        GraphQueryFamily::Tool,
        "rustfmt",
    );
    fs::write(
        &request_path,
        serde_json::to_string(&request).expect("request JSON"),
    )
    .expect("write request");

    let query = run_marlin_cli_from_args([
        "graph",
        "query",
        "--input",
        request_path.to_str().expect("utf8 path"),
        "--org-tool-store-root",
        dir.path().to_str().expect("utf8 path"),
        "--org-tool-root",
        "missing-tools.org",
    ]);

    assert_ne!(query.status, 0);
    assert!(
        query
            .stderr
            .contains("--org-tool-root points at missing Org root(s): missing-tools.org")
    );
}

#[test]
fn debug_cli_graph_query_rejects_mixed_tool_and_memory_store_roots() {
    let dir = tempdir().expect("tempdir");
    let request_path = dir.path().join("tool-request.json");
    let request = GraphQueryRequest::new(
        GraphQueryContext::new("project-alpha"),
        GraphQueryFamily::Tool,
        "rustfmt",
    );
    fs::write(
        &request_path,
        serde_json::to_string(&request).expect("request JSON"),
    )
    .expect("write request");

    let query = run_marlin_cli_from_args([
        "graph",
        "query",
        "--input",
        request_path.to_str().expect("utf8 path"),
        "--org-memory-root",
        "memory.org",
        "--org-tool-root",
        "tools.org",
    ]);

    assert_ne!(query.status, 0);
    assert!(
        query
            .stderr
            .contains("--org-tool-root cannot be combined with --org-memory-root")
    );
}
