use std::{
    collections::BTreeMap,
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use marlin_gerbil_ir::{ReleaseGateSpec, ReleaseTopologySpec, ReleaseVisibilitySpec};
use marlin_org_model::{CheckboxState, OrgNodeId, OrgNodeSourceTokens, OrgSourceSpan};
use marlin_org_patch::{OrgPatchPlan, OrgPatchPlanner, OrgTextEdit};
use marlin_org_store::{
    FileSystemOrgSourceStore, FileSystemReleaseStatusStore, OrgSourceCommit, OrgSourceCommitter,
    OrgSourceDiagnosticKind, OrgSourceDocumentHash, OrgSourceStore, OrgSourceWritePolicy,
};
use marlin_workspace_patch::{AffectedNodeSource, WorkspacePatch, WorkspacePatchOp};
use marlin_workspace_status::{ReleaseGateReceipt, ReleaseGateState};

#[test]
fn filesystem_store_commits_through_source_committer() {
    let root = test_root("commit");
    fs::create_dir_all(&root).expect("create temp root");
    fs::write(root.join("memory.org"), "* Goal\n").expect("seed document");

    let mut store = FileSystemOrgSourceStore::new(&root);
    let mut commit = OrgSourceCommit::new(
        add_text_plan("memory.org", 7, "- [ ] next\n"),
        OrgSourceWritePolicy::write(),
    );
    commit
        .expected_documents
        .push(OrgSourceDocumentHash::from_text("memory.org", "* Goal\n"));

    let receipt = OrgSourceCommitter::commit(&mut store, &commit);

    assert!(receipt.accepted());
    assert!(receipt.wrote_documents);
    assert_eq!(
        fs::read_to_string(root.join("memory.org")).expect("read committed document"),
        "* Goal\n- [ ] next\n"
    );
    let _ = fs::remove_dir_all(root);
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
fn filesystem_commit_missing_document_reports_diagnostic() {
    let root = test_root("missing");
    fs::create_dir_all(&root).expect("create temp root");
    let mut store = FileSystemOrgSourceStore::new(&root);
    let mut commit = OrgSourceCommit::new(
        add_text_plan("missing.org", 0, "* Added\n"),
        OrgSourceWritePolicy::write(),
    );
    commit
        .expected_documents
        .push(OrgSourceDocumentHash::from_text("missing.org", ""));

    let receipt = OrgSourceCommitter::commit(&mut store, &commit);

    assert!(!receipt.accepted());
    assert_eq!(
        receipt.diagnostics[0].kind,
        OrgSourceDiagnosticKind::MissingDocument
    );
    assert!(!root.join("missing.org").exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn filesystem_store_commits_workspace_patch_through_org_planner() {
    let root = test_root("workspace-patch");
    fs::create_dir_all(&root).expect("create temp root");
    let original = "* TODO Goal\nbody\n";
    fs::write(root.join("memory.org"), original).expect("seed document");

    let node = OrgNodeId::new("goal:1");
    let mut patch = WorkspacePatch::new("persist next action");
    patch.ops.push(WorkspacePatchOp::AddCheckbox {
        node: node.clone(),
        text: "verify persisted".to_owned(),
        state: CheckboxState::Open,
    });
    let plan = OrgPatchPlanner::plan(
        &patch,
        &[AffectedNodeSource {
            node,
            source: OrgSourceSpan {
                document: "memory.org".to_owned(),
                start_byte: 0,
                end_byte: original.len(),
                start_line: 1,
                end_line: 2,
            },
            tokens: OrgNodeSourceTokens::default(),
        }],
    );
    assert!(plan.is_applicable());

    let mut commit = OrgSourceCommit::new(plan, OrgSourceWritePolicy::write());
    commit
        .expected_documents
        .push(OrgSourceDocumentHash::from_text("memory.org", original));
    let mut store = FileSystemOrgSourceStore::new(&root);

    let receipt = OrgSourceCommitter::commit(&mut store, &commit);

    assert!(receipt.accepted());
    assert_eq!(receipt.applied_edits, 1);
    assert!(receipt.wrote_documents);
    assert_eq!(
        fs::read_to_string(root.join("memory.org")).expect("read committed document"),
        format!("{original}\n- [ ] verify persisted\n")
    );
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

fn add_text_plan(document: &str, start_byte: usize, replacement: &str) -> OrgPatchPlan {
    OrgPatchPlan {
        edits: vec![OrgTextEdit {
            document: document.to_owned(),
            start_byte,
            end_byte: start_byte,
            replacement: replacement.to_owned(),
            reason: "test-append".to_owned(),
        }],
        diagnostics: Vec::new(),
    }
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
