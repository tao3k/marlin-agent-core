use std::collections::BTreeMap;

use marlin_org_store::{
    MemoryOrgSourceStore, OrgProjectRootCandidate, OrgProjectRootKind, discover_project_roots,
};

#[test]
fn discovers_existing_typed_project_roots_without_returning_missing_candidates() {
    let store = MemoryOrgSourceStore::new(BTreeMap::from([
        (
            ".marlin/memory/project.org".to_string(),
            "* Project memory\n".to_string(),
        ),
        (
            ".marlin/contracts/agent.memory.v1.org".to_string(),
            "* Contract registry\n".to_string(),
        ),
        (
            ".marlin/evidence/receipts.org".to_string(),
            "* Evidence receipts\n".to_string(),
        ),
    ]));

    let roots = discover_project_roots(
        &store,
        [
            OrgProjectRootCandidate::project_memory(".marlin/memory/project.org"),
            OrgProjectRootCandidate::session_summary(".marlin/sessions/root-a.org"),
            OrgProjectRootCandidate::contract_registry(".marlin/contracts/agent.memory.v1.org"),
            OrgProjectRootCandidate::evidence_receipt(".marlin/evidence/receipts.org"),
        ],
    );

    assert_eq!(roots.len(), 3);
    assert_eq!(roots[0].document, ".marlin/memory/project.org");
    assert_eq!(roots[0].kind, OrgProjectRootKind::ProjectMemory);
    assert_eq!(roots[0].body, "* Project memory\n");
    assert_eq!(roots[1].kind, OrgProjectRootKind::ContractRegistry);
    assert_eq!(roots[2].kind, OrgProjectRootKind::EvidenceReceipt);
    assert!(
        roots
            .iter()
            .all(|root| root.document != ".marlin/sessions/root-a.org")
    );
}
