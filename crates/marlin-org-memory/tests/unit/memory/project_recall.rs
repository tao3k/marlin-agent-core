use std::collections::BTreeMap;

use marlin_agent_protocol::{
    GraphQueryContext, ProjectMemoryRecallIntent, ProjectMemoryRecallRequest,
};
use marlin_org_memory::{MemoryOrgWorkspace, ProjectMemoryStoreRecall};
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
    assert!(
        pack.facts[0]
            .graph_match
            .source_anchor_id
            .as_ref()
            .is_some_and(|anchor_id| !anchor_id.as_str().is_empty())
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

#[test]
fn recall_source_spans_follow_matched_org_nodes_not_duplicate_memory_ids() {
    let store = MemoryOrgSourceStore::new(BTreeMap::from([
        (
            ".marlin/memory/a.org".to_string(),
            r#"* Memory: Duplicate A
:PROPERTIES:
:MEMORY_ID: memory:duplicated
:PROJECT_ID: project-alpha
:WORKTREE_ID: worktree-a
:ROOT_SESSION_ID: root-session-1
:RECALL_QUERY: duplicate frontier
:CONTRACT_VALIDATED: true
:END:
First duplicate memory node.
"#
            .to_string(),
        ),
        (
            ".marlin/memory/b.org".to_string(),
            r#"* Memory: Duplicate B
:PROPERTIES:
:MEMORY_ID: memory:duplicated
:PROJECT_ID: project-alpha
:WORKTREE_ID: worktree-b
:ROOT_SESSION_ID: root-session-2
:RECALL_QUERY: duplicate frontier
:CONTRACT_VALIDATED: true
:END:
Second duplicate memory node.
"#
            .to_string(),
        ),
    ]));
    let roots = discover_project_roots(
        &store,
        [
            OrgProjectRootCandidate::project_memory(".marlin/memory/a.org"),
            OrgProjectRootCandidate::project_memory(".marlin/memory/b.org"),
        ],
    );
    let workspace = MemoryOrgWorkspace::new();
    let request = ProjectMemoryRecallRequest::new(
        GraphQueryContext::new("project-alpha"),
        ProjectMemoryRecallIntent::RecoverDecision,
    )
    .with_query_term("duplicate")
    .with_query_term("frontier")
    .with_limit(5);

    let pack = workspace
        .recall_project_memory_from_roots(
            "context-pack-duplicate",
            "receipt-recall-duplicate",
            request,
            &roots,
        )
        .expect("recall succeeds");

    assert_eq!(pack.facts.len(), 2);
    assert!(
        pack.facts
            .iter()
            .all(|fact| fact.evidence_ids[0].as_str() == "memory:duplicated")
    );
    let mut source_spans = pack
        .facts
        .iter()
        .map(|fact| {
            fact.source_span
                .as_ref()
                .expect("source span")
                .as_str()
                .to_owned()
        })
        .collect::<Vec<_>>();
    source_spans.sort();
    assert_eq!(
        source_spans,
        [
            ".marlin/memory/a.org:L1-L10".to_owned(),
            ".marlin/memory/b.org:L1-L10".to_owned()
        ]
    );
}

#[test]
fn recalls_project_memory_from_store_with_contract_indexed_frontier() {
    let store = MemoryOrgSourceStore::new(BTreeMap::from([(
        ".marlin/memory/store-frontier.org".to_string(),
        r#"* Memory: Store-backed contract frontier :graph_frontier:
:PROPERTIES:
:MEMORY_ID: memory:store-backed-frontier
:PROJECT_ID: project-alpha
:WORKTREE_ID: worktree-a
:ROOT_SESSION_ID: root-session-1
:EVIDENCE_FACT: contract-evidence-alpha
:CONTRACT_VALIDATED: true
:END:
[[id:policy-shard-beta][evidence backlink]]
"#
        .to_string(),
    )]));
    let workspace = MemoryOrgWorkspace::new();
    let request = ProjectMemoryRecallRequest::new(
        GraphQueryContext::new("project-alpha")
            .with_worktree("worktree-b")
            .with_root_session("root-session-2"),
        ProjectMemoryRecallIntent::ContinueWork,
    )
    .with_query_term("graph_frontier")
    .with_query_term("contract-evidence-alpha")
    .with_query_term("policy-shard-beta")
    .with_query_term("store-frontier.org")
    .with_limit(5);

    let pack = workspace
        .recall_project_memory_from_store(ProjectMemoryStoreRecall {
            context_pack_id: "context-pack-store-frontier".to_string(),
            receipt_id: "receipt-recall-store-frontier".to_string(),
            request,
            store: &store,
            candidates: vec![
                OrgProjectRootCandidate::project_memory(".marlin/memory/store-frontier.org"),
                OrgProjectRootCandidate::project_memory(".marlin/memory/missing.org"),
            ],
        })
        .expect("recall succeeds");

    assert_eq!(pack.context_pack_id.as_str(), "context-pack-store-frontier");
    assert_eq!(
        pack.source_receipts[0].as_str(),
        "receipt-recall-store-frontier"
    );
    assert_eq!(pack.facts.len(), 1);
    assert_eq!(
        pack.facts[0]
            .graph_match
            .memory_id
            .as_ref()
            .expect("memory id")
            .as_str(),
        "memory:store-backed-frontier"
    );
    assert_eq!(
        pack.facts[0]
            .source_span
            .as_ref()
            .expect("source span")
            .as_str(),
        ".marlin/memory/store-frontier.org:L1-L10"
    );
}
