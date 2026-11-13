// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use marlin_agent_context::org::MemoryOrgWorkspace;
use marlin_agent_context::org::workspace::OrgDocument;
use marlin_workspace_query::{QueryFilter, SourceRange, WorkspaceQuery, WorkspaceScope};
use marlin_workspace_view::WorkspaceViewSpec;

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
    let mut query = WorkspaceQuery::new(WorkspaceScope::WholeWorkspace);
    query
        .filters
        .push(QueryFilter::TodoState("TODO".to_string()));
    let query_result = workspace.query(query).expect("query succeeds");
    let view = workspace
        .render_view(WorkspaceViewSpec::compact(vec![ids[0].clone()]))
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
    let source_result = workspace
        .query(source_query)
        .expect("source query succeeds");
    assert_eq!(source_result.matches.len(), 1);

    let mut document_query = WorkspaceQuery::new(WorkspaceScope::WholeWorkspace);
    document_query
        .filters
        .push(QueryFilter::SourceDocument("doc:goal".to_string()));
    let document_result = workspace
        .query(document_query)
        .expect("document filter succeeds");
    assert_eq!(document_result.matches.len(), 1);
}

// Invoked after exclusive parser startup by unit_test.
pub(super) const CASES: &[(&str, fn())] = &[(
    "memory_workspace_loads_document_for_query_and_view",
    memory_workspace_loads_document_for_query_and_view,
)];
