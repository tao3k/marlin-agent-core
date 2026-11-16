// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::{
    collections::BTreeMap,
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use marlin_agent_context::org::store::{
    FileSystemOrgSourceStore, FileSystemReleaseStatusStore, OrgProjectRootCandidate,
    OrgSourceCleanState, OrgSourceStore, discover_project_roots,
};
use marlin_gerbil_ir::{ReleaseGateSpec, ReleaseTopologySpec, ReleaseVisibilitySpec};
use marlin_workspace_status::{ReleaseGateReceipt, ReleaseGateState};

#[test]
fn filesystem_read_failures_do_not_look_like_missing_documents() {
    let root = test_root("read-failure");
    fs::create_dir_all(&root).expect("create temp root");
    fs::write(root.join("broken.org"), [0xff]).expect("seed invalid UTF-8");
    let store = FileSystemOrgSourceStore::new(&root);
    assert_eq!(
        store
            .read_document("absent.org")
            .expect("missing is not an error"),
        None
    );
    assert!(store.read_document("../escape.org").is_err());
    assert!(store.read_document("broken.org").is_err());
    assert!(
        discover_project_roots(
            &store,
            [OrgProjectRootCandidate::project_memory("broken.org")]
        )
        .is_err()
    );
    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn filesystem_batch_rejects_lexical_path_aliases_before_writing() {
    let root = test_root("path-alias");
    fs::create_dir_all(root.join("notes")).expect("create temp root");
    fs::write(root.join("notes/a.org"), "original").expect("seed document");
    let mut store = FileSystemOrgSourceStore::new(&root);

    let result = store.write_documents(BTreeMap::from([
        ("notes/./a.org".to_owned(), "first".to_owned()),
        ("notes/a.org".to_owned(), "second".to_owned()),
    ]));

    assert!(result.is_err());
    assert_eq!(
        fs::read_to_string(root.join("notes/a.org")).unwrap(),
        "original"
    );
    assert_eq!(store.clean_state(), OrgSourceCleanState::Unknown);
    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn partial_multi_document_failure_does_not_establish_clean_state() {
    let root = test_root("partial-write");
    fs::create_dir_all(&root).expect("create temp root");
    fs::write(root.join("z"), "not a directory").expect("block second parent");
    let mut store = FileSystemOrgSourceStore::new(&root);
    let result = store.write_documents(BTreeMap::from([
        ("a.org".to_owned(), "first".to_owned()),
        ("z/second.org".to_owned(), "second".to_owned()),
    ]));
    assert!(result.is_err());
    assert_eq!(fs::read_to_string(root.join("a.org")).unwrap(), "first");
    assert_eq!(store.clean_state(), OrgSourceCleanState::Unknown);
    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn filesystem_store_rejects_root_escape_on_write() {
    let root = test_root("escape");
    fs::create_dir_all(&root).expect("create temp root");
    let mut store = FileSystemOrgSourceStore::new(&root);

    let result = store.write_documents(BTreeMap::from([(
        "../outside.org".to_owned(),
        "* Escape\n".to_owned(),
    )]));

    assert!(result.is_err());
    assert!(!root.join("../outside.org").exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn filesystem_release_status_store_persists_gate_receipts() {
    let root = test_root("release-status");
    fs::create_dir_all(&root).expect("create temp root");
    let store = FileSystemReleaseStatusStore::new(&root);
    let topology = ReleaseTopologySpec {
        topology_id: "release:gerbil".to_owned(),
        crate_name: "marlin-gerbil-scheme".to_owned(),
        publish_enabled: false,
        asset_audit_command: "cargo package -p marlin-gerbil-scheme --list --allow-dirty"
            .to_owned(),
        package_assets: vec!["fixtures/gerbil/build.ss".to_owned()],
        runtime_dependency_chain: vec!["marlin-gerbil-ir".to_owned()],
        workflow_dependency_chain: vec!["marlin-org-workflow".to_owned()],
        gates: vec![ReleaseGateSpec {
            gate_id: "package-assets".to_owned(),
            command: "cargo package -p marlin-gerbil-scheme --list --allow-dirty".to_owned(),
            requires_local_gerbil: false,
            required_artifacts: vec!["fixtures/gerbil/build.ss".to_owned()],
            visibility: vec![ReleaseVisibilitySpec {
                report_key: "package_asset_audit".to_owned(),
                evidence_keys: vec!["required_artifacts".to_owned()],
                artifact_paths: vec!["fixtures/gerbil/build.ss".to_owned()],
            }],
        }],
    };

    let pending = store
        .record_release_topology(&topology)
        .expect("release topology persisted");
    assert_eq!(pending.gates[0].state, ReleaseGateState::Pending);
    assert!(store.path().exists());
    assert!(
        store
            .record_release_gate_receipt(ReleaseGateReceipt::passed(
                "package-assets",
                vec!["required_artifacts".to_owned()],
                vec!["fixtures/gerbil/build.ss".to_owned()],
            ))
            .expect("gate receipt persisted")
    );

    let reopened = FileSystemReleaseStatusStore::new(&root);
    let status = reopened
        .read_status()
        .expect("release status readable")
        .expect("release status present");
    assert_eq!(status.topology_id, "release:gerbil");
    assert_eq!(status.gates[0].state, ReleaseGateState::Passed);
    assert!(
        status.visibility_reports[0].observed,
        "passing gate receipt should mark matching visibility as observed"
    );
    assert_eq!(
        status.gates[0]
            .last_receipt
            .as_ref()
            .expect("gate receipt")
            .artifact_paths,
        ["fixtures/gerbil/build.ss"]
    );
    let report = reopened
        .read_landing_report()
        .expect("landing report readable")
        .expect("landing report present");
    assert!(report.landing_complete);
    assert_eq!(report.crate_name, "marlin-gerbil-scheme");
    assert_eq!(report.passed_gates, 1);
    assert_eq!(report.observed_visibility_reports, 1);
    assert!(report.blocking_gates.is_empty());
    let _ = fs::remove_dir_all(root);
}

fn test_root(name: &str) -> std::path::PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    std::env::temp_dir().join(format!(
        "marlin-org-store-{name}-{}-{suffix}",
        std::process::id()
    ))
}
