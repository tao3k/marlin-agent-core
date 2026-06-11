use super::{
    WORKSPACE_PATCH_INTENT_SOURCE, WORKSPACE_SOURCE_COMMIT_INTENT_SOURCE, real_gxi_module_compiler,
};
use marlin_gerbil_scheme::{
    GerbilArtifactKind, GerbilCompiledArtifact, GerbilCompiler, GerbilSource,
};
use marlin_org_store::{FileSystemOrgSourceStore, OrgSourceWritePolicy};
use marlin_org_workflow::{
    GerbilWorkspacePatchIntentCommit, GerbilWorkspacePatchIntentDryRunner,
    OrgWorkspaceSourceCommitter,
};
use marlin_workspace_patch::PatchId;
use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
#[ignore = "requires a local Gerbil gxi executable"]
fn command_compiler_real_gxi_workspace_patch_intent_dry_runs_through_workflow() {
    let Some(compiler) = real_gxi_module_compiler() else {
        return;
    };

    let artifact = compiler
        .compile(
            GerbilSource::new(
                "audit/workspace-patch-intent",
                WORKSPACE_PATCH_INTENT_SOURCE,
            ),
            GerbilArtifactKind::WorkspacePatchIntent,
        )
        .expect("real gxi module entry should compile a workspace patch intent before dry-run");

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
#[ignore = "requires a local Gerbil gxi executable"]
fn command_compiler_real_gxi_workspace_patch_intent_commits_with_policy() {
    let Some(compiler) = real_gxi_module_compiler() else {
        return;
    };
    let root = test_root("real-gxi-gerbil-intent-commit");
    fs::create_dir_all(&root).expect("create temp root");
    fs::write(
        root.join("memory.org"),
        "* TODO Goal\n:PROPERTIES:\n:OWNER: old-owner\n:END:\n",
    )
    .expect("seed document");
    let mut store = FileSystemOrgSourceStore::new(&root);

    let artifact = compiler
        .compile(
            GerbilSource::new(
                "audit/workspace-source-commit",
                WORKSPACE_SOURCE_COMMIT_INTENT_SOURCE,
            ),
            GerbilArtifactKind::WorkspacePatchIntent,
        )
        .expect("real gxi should compile a policy-gated workspace source commit intent");
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
        fs::read_to_string(root.join("memory.org")).expect("read committed document"),
        "* DONE Goal\n:PROPERTIES:\n:OWNER: gerbil\n:END:\n",
    );
    let _ = fs::remove_dir_all(root);
}

fn test_root(name: &str) -> std::path::PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    std::env::temp_dir().join(format!(
        "marlin-gerbil-scheme-{name}-{}-{suffix}",
        std::process::id()
    ))
}
