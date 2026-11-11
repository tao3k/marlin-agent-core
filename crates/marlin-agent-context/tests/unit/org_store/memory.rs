use std::collections::BTreeMap;

use marlin_agent_context::org::store::{
    MemoryOrgSourceStore, OrgAtomicSourceStore, OrgAtomicWriteOutcome, OrgSourceDocumentId,
};

#[test]
fn atomic_write_checks_unchanged_source_dependencies() {
    let mut store = MemoryOrgSourceStore::new(BTreeMap::from([
        ("turn.org".into(), "old turn".into()),
        ("scope.org".into(), "current scope".into()),
    ]));
    let expected = BTreeMap::from([
        (OrgSourceDocumentId::from("turn.org"), "old turn".into()),
        (OrgSourceDocumentId::from("scope.org"), "stale scope".into()),
    ]);
    let changes = BTreeMap::from([(OrgSourceDocumentId::from("turn.org"), "new turn".into())]);

    assert_eq!(
        store.compare_and_write(&expected, changes).unwrap(),
        OrgAtomicWriteOutcome::Conflict
    );
    assert_eq!(store.document("turn.org"), Some("old turn"));
    assert_eq!(store.document("scope.org"), Some("current scope"));
}
