use futures_executor::block_on;
use marlin_agent_context::org::MemoryOrgWorkspace;
use marlin_agent_context::org::workspace::OrgDocument;
use marlin_workspace_protocol::{AgentWorkspace, WorkspaceCtx};
use marlin_workspace_query::{QueryFilter, SourceRange, WorkspaceQuery, WorkspaceScope};
use marlin_workspace_view::WorkspaceViewSpec;

#[test]
fn memory_workspace_loads_document_for_query_and_view() {
    let workspace = MemoryOrgWorkspace::new();
    let document = OrgDocument::new(
        "doc:goal",
        "* TODO Build native workspace\n\
         :PROPERTIES:\n\
         :OWNER: marlin-org-memory\n\
         :END:\n\
         - [ ] Add file-backed adapter\n",
    );

    let ids = workspace
        .load_document(document)
        .expect("document inserted");
    let ctx = WorkspaceCtx::new("unit-test");
    let mut query = WorkspaceQuery::new(WorkspaceScope::WholeWorkspace);
    query
        .filters
        .push(QueryFilter::TodoState("TODO".to_string()));
    let query_result = block_on(workspace.query(query, ctx.clone())).expect("query succeeds");
    let view = block_on(workspace.render_view(
        WorkspaceViewSpec::compact(vec![ids[0].clone()]),
        ctx.clone(),
    ))
    .expect("view renders");

    assert_eq!(query_result.matches.len(), 1);
    assert!(
        query_result.matches[0]
            .reason
            .as_deref()
            .expect("query reason")
            .contains("doc:goal:1-")
    );
    assert!(view.text.contains("Build native workspace"));
    assert!(view.text.contains("Add file-backed adapter"));
    assert!(view.text.contains("source: doc:goal:1-"));

    let source_query = WorkspaceQuery::new(WorkspaceScope::SourceRange(SourceRange {
        document: "doc:goal".to_string(),
        start_line: 1,
        end_line: 1,
    }));
    let source_result =
        block_on(workspace.query(source_query, ctx.clone())).expect("source query succeeds");
    assert_eq!(source_result.matches.len(), 1);

    let mut document_query = WorkspaceQuery::new(WorkspaceScope::WholeWorkspace);
    document_query
        .filters
        .push(QueryFilter::SourceDocument("doc:goal".to_string()));
    let document_result =
        block_on(workspace.query(document_query, ctx.clone())).expect("document filter succeeds");
    assert_eq!(document_result.matches.len(), 1);
}
