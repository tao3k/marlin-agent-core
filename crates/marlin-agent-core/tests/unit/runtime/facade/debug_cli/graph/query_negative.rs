use marlin_agent_core::{
    protocol::{GraphQueryContext, GraphQueryFamily, GraphQueryRequest},
    run_marlin_cli_from_args,
};
use std::fs;
use tempfile::tempdir;

#[test]
fn debug_cli_graph_query_rejects_missing_memory_store_root() {
    let dir = tempdir().expect("tempdir");
    let request_path = dir.path().join("memory-request.json");
    let request = GraphQueryRequest::new(
        GraphQueryContext::new("project-alpha"),
        GraphQueryFamily::Memory,
        "memory",
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
        "--org-memory-store-root",
        dir.path().to_str().expect("utf8 path"),
        "--org-memory-root",
        "missing-memory.org",
    ]);

    assert_ne!(query.status, 0);
    assert!(
        query
            .stderr
            .contains("--org-memory-root points at missing Org root(s): missing-memory.org")
    );
}

#[test]
fn debug_cli_graph_query_rejects_unsupported_org_family_fixture_execution() {
    let dir = tempdir().expect("tempdir");
    let request_path = dir.path().join("org-request.json");
    let fixture_path = dir.path().join("workspace.org");
    let request = GraphQueryRequest::new(
        GraphQueryContext::new("project-alpha"),
        GraphQueryFamily::Org,
        "workspace",
    );
    fs::write(
        &request_path,
        serde_json::to_string(&request).expect("request JSON"),
    )
    .expect("write request");
    fs::write(&fixture_path, "* Workspace fact\n").expect("write fixture");

    let query = run_marlin_cli_from_args([
        "graph",
        "query",
        "--input",
        request_path.to_str().expect("utf8 path"),
        "--org-memory-fixture",
        fixture_path.to_str().expect("utf8 path"),
    ]);

    assert_ne!(query.status, 0);
    assert!(
        query
            .stderr
            .contains("--org-memory-fixture does not execute GraphQueryFamily::Org")
    );
}
