use std::collections::BTreeMap;

use marlin_agent_protocol::{
    GraphQueryContext, ProjectMemoryRecallIntent, ProjectMemoryRecallRequest,
};
use marlin_org_memory::MemoryOrgWorkspace;
use marlin_org_store::{MemoryOrgSourceStore, OrgProjectRootCandidate, discover_project_roots};

#[test]
fn recalls_project_memory_context_pack_from_discovered_org_roots() {
    let store = MemoryOrgSourceStore::new(BTreeMap::from([(
        ".marlin/memory/project.org".to_string(),
        r#"* Memory: Org-first recall boundary
:PROPERTIES:
:MEMORY_ID: memory:org-first-recall
:PROJECT_ID: project-alpha
:WORKTREE_ID: worktree-a
:ROOT_SESSION_ID: root-session-1
:RECALL_QUERY: hybrid frontier evidence graph
:CONTRACT_VALIDATED: true
:END:
Org is the project memory substrate.
"#
        .to_string(),
    )]));
    let roots = discover_project_roots(
        &store,
        [OrgProjectRootCandidate::project_memory(
            ".marlin/memory/project.org",
        )],
    );
    let workspace = MemoryOrgWorkspace::new();
    let request = ProjectMemoryRecallRequest::new(
        GraphQueryContext::new("project-alpha")
            .with_worktree("worktree-b")
            .with_root_session("root-session-2"),
        ProjectMemoryRecallIntent::ContinueWork,
    )
    .with_query_term("hybrid")
    .with_query_term("frontier")
    .with_limit(5);

    let pack = workspace
        .recall_project_memory_from_roots("context-pack-1", "receipt-recall-1", request, &roots)
        .expect("recall succeeds");

    assert_eq!(pack.context_pack_id.as_str(), "context-pack-1");
    assert_eq!(pack.source_receipts[0].as_str(), "receipt-recall-1");
    assert_eq!(pack.facts.len(), 1);
    assert_eq!(pack.facts[0].claim, "Memory: Org-first recall boundary");
    assert_eq!(
        pack.facts[0]
            .graph_match
            .memory_id
            .as_ref()
            .expect("memory id")
            .as_str(),
        "memory:org-first-recall"
    );
    assert_eq!(
        pack.facts[0]
            .source_span
            .as_ref()
            .expect("source span")
            .as_str(),
        ".marlin/memory/project.org:L1-L10"
    );
    assert_eq!(pack.facts[0].evidence_ids.len(), 1);
    assert_eq!(
        pack.facts[0].evidence_ids[0].as_str(),
        "memory:org-first-recall"
    );
}
