use marlin_gerbil_scheme::GerbilCompiledArtifact;
use marlin_org_store::{FileSystemOrgSourceStore, OrgSourceWritePolicy};
use marlin_org_workflow::{
    GerbilWorkspacePatchIntentCommit, GerbilWorkspacePatchIntentDryRunner,
    OrgWorkspaceSourceCommitter,
};
use marlin_workspace_patch::PatchId;
use serde_json::json;
use std::fs;
use tempfile::{Builder, TempDir};

#[test]
fn workspace_patch_intent_fixture_dry_runs_through_workflow() {
    let artifact = workspace_patch_intent_artifact();
    let GerbilCompiledArtifact::WorkspacePatchIntent(intent) = artifact else {
        panic!("expected workspace patch intent artifact");
    };
    let receipt = GerbilWorkspacePatchIntentDryRunner::dry_run(&intent);

    assert_eq!(receipt.patch_id, PatchId::new("intent:memory"));
    assert_eq!(receipt.affected_nodes.len(), 1);
    assert_eq!(receipt.affected_nodes[0].as_str(), "memory.org:1:goal");
    assert!(receipt.affected_sources.is_empty());
    assert_eq!(receipt.before_hash, "dry-run:no-workspace-read");
    assert_eq!(receipt.after_hash, "dry-run:no-workspace-write");
    assert!(receipt.validation.accepted);
    assert!(receipt.validation.diagnostics.is_empty());
    assert_eq!(receipt.memory_dispatch.len(), 1);
    assert_eq!(receipt.memory_dispatch[0].target, "long-term");
    assert!(!receipt.memory_dispatch[0].accepted);
}

#[test]
fn workspace_patch_intent_fixture_commits_with_policy() {
    let root = test_root("gerbil-intent-commit");
    fs::create_dir_all(root.path()).expect("create temp root");
    fs::write(
        root.path().join("memory.org"),
        "* TODO Goal\n:PROPERTIES:\n:OWNER: old-owner\n:END:\n",
    )
    .expect("seed document");
    let mut store = FileSystemOrgSourceStore::new(root.path());

    let artifact = workspace_source_commit_intent_artifact();
    let GerbilCompiledArtifact::WorkspacePatchIntent(intent) = artifact else {
        panic!("expected workspace patch intent artifact");
    };
    let request =
        GerbilWorkspacePatchIntentCommit::new("memory.org", intent, OrgSourceWritePolicy::write());

    let receipt = OrgWorkspaceSourceCommitter::commit_gerbil_intent(&mut store, &request);

    assert!(receipt.source.accepted());
    assert_eq!(receipt.plan.edits.len(), 2);
    assert_eq!(receipt.source.applied_edits, 2);
    assert!(receipt.source.wrote_documents);
    assert_eq!(
        fs::read_to_string(root.path().join("memory.org")).expect("read committed document"),
        "* DONE Goal\n:PROPERTIES:\n:OWNER: gerbil\n:END:\n",
    );
}

fn workspace_patch_intent_artifact() -> GerbilCompiledArtifact {
    serde_json::from_value(json!({
        "WorkspacePatchIntent": {
            "intent_id": "intent:memory",
            "patch": {
                "reason": "gerbil intent",
                "source_agent": "gerbil",
                "ops": [
                    {"SetTodo": {"node": "memory.org:1:goal", "state": "Done"}},
                    {"SetProperty": {"node": "memory.org:1:goal", "key": "OWNER", "value": "gerbil"}},
                    {"MarkMemoryCandidate": {"node": "memory.org:1:goal", "dispatch": "long-term"}}
                ]
            },
            "dry_run_first": true
        }
    }))
    .expect("workspace patch intent fixture should decode")
}

fn workspace_source_commit_intent_artifact() -> GerbilCompiledArtifact {
    serde_json::from_value(json!({
        "WorkspacePatchIntent": {
            "intent_id": "intent:source-commit",
            "patch": {
                "reason": "gerbil source commit",
                "source_agent": "gerbil",
                "ops": [
                    {"SetTodo": {"node": "memory.org:1:goal", "state": "Done"}},
                    {"SetProperty": {"node": "memory.org:1:goal", "key": "OWNER", "value": "gerbil"}}
                ]
            },
            "dry_run_first": true
        }
    }))
    .expect("workspace source commit intent fixture should decode")
}

fn test_root(name: &str) -> TempDir {
    Builder::new()
        .prefix(&format!("marlin-gerbil-scheme-{name}-"))
        .tempdir()
        .unwrap_or_else(|error| panic!("creates {name} test root: {error}"))
}
