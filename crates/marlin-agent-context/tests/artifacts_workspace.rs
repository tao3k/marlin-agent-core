// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

#![cfg(feature = "artifacts")]

use std::{env, fs, path::Path, process::Command};

use gerbil_scheme::GerbilRuntime;
use marlin_agent_context::artifacts::{ArtifactKind, WorkspaceArtifacts};

#[test]
#[ignore = "requires the compiled Marlin Scheme package"]
fn linked_worktree_restores_shared_org_after_restart() {
    let fixture = tempfile::tempdir().unwrap();
    let primary = fixture.path().join("primary");
    let branch = fixture.path().join("branch");
    fs::create_dir(&primary).unwrap();
    let git = |directory: &Path, args: &[&str]| {
        let output = Command::new("git")
            .current_dir(directory)
            .args(args)
            .output()
            .unwrap();
        assert!(output.status.success(), "{:?}", output.stderr);
        String::from_utf8(output.stdout).unwrap()
    };
    git(&primary, &["init", "-q"]);
    git(&primary, &["config", "user.name", "Fixture"]);
    git(&primary, &["config", "user.email", "fixture@example.test"]);
    git(&primary, &["config", "commit.gpgsign", "false"]);
    fs::write(primary.join("file"), "base\n").unwrap();
    git(&primary, &["add", "file"]);
    git(&primary, &["commit", "-qm", "base"]);
    let source_commit = git(&primary, &["rev-parse", "HEAD"]);
    git(
        &primary,
        &["worktree", "add", "--detach", branch.to_str().unwrap()],
    );
    fs::write(branch.join("file"), "updated\n").unwrap();
    let run = |directory: &Path, action: &str| {
        let output = Command::new(env::current_exe().unwrap())
            .args(["--exact", "artifact_process", "--ignored", "--nocapture"])
            .env("MARLIN_ARTIFACT_TEST_DIRECTORY", directory)
            .env("MARLIN_ARTIFACT_TEST_ACTION", action)
            .env("MARLIN_AGENT_CORE_DEV", "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    };
    run(&branch, "record");
    git(&branch, &["add", "file"]);
    git(&branch, &["commit", "-qm", "updated"]);
    let org = primary.join(".data/marlin/context.org");
    let before = fs::read_to_string(&org).unwrap();
    assert!(before.contains(source_commit.trim()));
    assert!(!before.contains(git(&branch, &["rev-parse", "HEAD"]).trim()));
    assert!(!branch.join(".data/marlin").exists());
    // Simulate loss of the materialized projection after committing the journal.
    fs::remove_file(&org).unwrap();
    run(&primary, "restore");
    assert_eq!(before, fs::read_to_string(org).unwrap());
}

#[test]
#[ignore = "launched in fresh processes by the workspace integration"]
fn artifact_process() {
    let Some(directory) = env::var_os("MARLIN_ARTIFACT_TEST_DIRECTORY") else {
        return;
    };
    let package = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let package = package.to_str().unwrap();
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let workspace = executor
        .block_on(WorkspaceArtifacts::discover(Path::new(&directory)))
        .unwrap()
        .unwrap();
    let runtime = GerbilRuntime::initialize().unwrap();
    let projection = match env::var("MARLIN_ARTIFACT_TEST_ACTION").unwrap().as_str() {
        "record" => {
            executor
                .block_on(workspace.record_git_patch(&runtime, package))
                .unwrap();
            workspace
                .record(
                    &runtime,
                    package,
                    &ArtifactKind::new("test-receipt"),
                    b"tests passed\n",
                )
                .unwrap()
        }
        "restore" => workspace.restore(&runtime, package).unwrap(),
        other => panic!("unknown test action {other}"),
    };
    assert_eq!(projection.files.len(), 2);
    let patch = fs::read_to_string(&projection.files[0]).unwrap();
    assert!(patch.contains("-base\n+updated\n"));
    assert_eq!(fs::read(&projection.files[1]).unwrap(), b"tests passed\n");
}
