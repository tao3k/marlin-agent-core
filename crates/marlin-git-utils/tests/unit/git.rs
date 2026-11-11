// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::process::Command;

use marlin_git_utils::{
    GitDiffRequest, GitDiffScope, GitFileState, GitRepositoryPath, ProcessGitTooling,
};

#[tokio::test]
async fn process_git_tooling_resolves_repository_root_from_nested_path() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let nested = tempdir.path().join("a/b");
    std::fs::create_dir_all(&nested).expect("nested dirs");

    let status = Command::new("git")
        .arg("init")
        .current_dir(tempdir.path())
        .status()
        .expect("git init should run");
    assert!(status.success());

    let root = ProcessGitTooling::resolve_repository_root(&nested)
        .await
        .expect("repository root should resolve");

    let expected = std::fs::canonicalize(tempdir.path()).expect("canonical tempdir");
    assert_eq!(root.as_path(), expected.as_path());
}

#[tokio::test]
async fn status_preserves_staged_untracked_and_renamed_paths() {
    let dir = tempfile::tempdir().unwrap();
    let run = |args: &[&str]| {
        assert!(
            Command::new("git")
                .args(args)
                .current_dir(dir.path())
                .output()
                .unwrap()
                .status
                .success()
        );
    };
    run(&["init", "-q"]);
    run(&["config", "user.email", "fixture@example.test"]);
    run(&["config", "user.name", "Fixture"]);
    run(&["config", "commit.gpgsign", "false"]);
    std::fs::write(dir.path().join("old name"), "old").unwrap();
    run(&["add", "--", "old name"]);
    run(&["commit", "-qm", "fixture"]);
    run(&["mv", "old name", "new name"]);
    std::fs::write(dir.path().join("untracked\nname"), "data").unwrap();
    let status = ProcessGitTooling::status(dir.path()).await.unwrap();
    let renamed = status
        .entries
        .iter()
        .find(|entry| entry.index == GitFileState::Renamed)
        .unwrap();
    assert_eq!(renamed.path.as_bytes(), b"new name");
    assert_eq!(
        renamed.original_path.as_ref().unwrap().as_bytes(),
        b"old name"
    );
    let untracked = status
        .entries
        .iter()
        .find(|entry| entry.index == GitFileState::Untracked)
        .unwrap();
    assert_eq!(untracked.path.as_bytes(), b"untracked\nname");
    assert_eq!(untracked.path.display_escaped(), "untracked\\nname");
}

#[tokio::test]
async fn diff_scopes_match_index_and_worktree_without_external_diff() {
    let dir = tempfile::tempdir().unwrap();
    let run = |args: &[&str]| {
        assert!(
            Command::new("git")
                .args(args)
                .current_dir(dir.path())
                .output()
                .unwrap()
                .status
                .success()
        );
    };
    run(&["init", "-q"]);
    std::fs::write(dir.path().join(".gitattributes"), "tracked diff=fixture\n").unwrap();
    run(&["config", "diff.fixture.textconv", "false"]);
    std::fs::write(dir.path().join("tracked"), "staged-content\n").unwrap();
    run(&["add", "--", "tracked"]);
    std::fs::write(dir.path().join("tracked"), "worktree-content\n").unwrap();
    run(&["config", "diff.external", "false"]);
    let unstaged = ProcessGitTooling::diff(dir.path(), GitDiffRequest::all(GitDiffScope::Worktree))
        .await
        .unwrap();
    let staged = ProcessGitTooling::diff(dir.path(), GitDiffRequest::all(GitDiffScope::Index))
        .await
        .unwrap();
    let unstaged = unstaged.display_escaped();
    let staged = staged.display_escaped();
    assert!(unstaged.contains("-staged-content\n+worktree-content"));
    assert!(staged.contains("+staged-content"));
    assert!(!staged.contains("worktree-content"));
    std::fs::write(dir.path().join("tracked"), "safe\x1b[2J\n").unwrap();
    let escaped = ProcessGitTooling::diff(dir.path(), GitDiffRequest::all(GitDiffScope::Worktree))
        .await
        .unwrap()
        .display_escaped();
    assert!(!escaped.contains('\x1b'));
    assert!(escaped.contains("\\u{1b}"));
    std::fs::write(dir.path().join("tracked"), b"invalid-\xff\n").unwrap();
    let binary_text =
        ProcessGitTooling::diff(dir.path(), GitDiffRequest::all(GitDiffScope::Worktree))
            .await
            .unwrap();
    assert!(binary_text.as_bytes().contains(&0xff));
    assert!(binary_text.display_escaped().contains("\\xff"));
}

#[tokio::test]
async fn diff_selected_path_is_root_relative_and_interpreted_literally() {
    let dir = tempfile::tempdir().unwrap();
    assert!(
        Command::new("git")
            .args(["init", "-q"])
            .current_dir(dir.path())
            .status()
            .unwrap()
            .success()
    );
    std::fs::create_dir(dir.path().join("nested")).unwrap();
    for path in ["file[1].txt", "file1.txt", "-option"] {
        std::fs::write(dir.path().join(path), "old\n").unwrap();
    }
    assert!(
        Command::new("git")
            .args(["add", "--", "."])
            .current_dir(dir.path())
            .status()
            .unwrap()
            .success()
    );
    for path in ["file[1].txt", "file1.txt", "-option"] {
        std::fs::write(dir.path().join(path), format!("changed:{path}\n")).unwrap();
    }
    let request = GitDiffRequest::for_path(
        GitDiffScope::Worktree,
        GitRepositoryPath::new("file[1].txt").unwrap(),
    );
    let diff = ProcessGitTooling::diff(&dir.path().join("nested"), request)
        .await
        .unwrap()
        .display_escaped();
    assert!(diff.contains("changed:file[1].txt"));
    assert!(!diff.contains("changed:file1.txt"));
    let request = GitDiffRequest::for_path(
        GitDiffScope::Worktree,
        GitRepositoryPath::new("-option").unwrap(),
    );
    let diff = ProcessGitTooling::diff(dir.path(), request)
        .await
        .unwrap()
        .display_escaped();
    assert!(diff.contains("changed:-option"));
    for invalid in [
        "",
        ".",
        "../file",
        "folder/../file",
        "/absolute",
        "nul\0file",
    ] {
        assert!(GitRepositoryPath::new(invalid).is_err(), "{invalid:?}");
    }
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn status_preserves_non_utf8_filenames() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};
    let dir = tempfile::tempdir().unwrap();
    assert!(
        Command::new("git")
            .args(["init", "-q"])
            .current_dir(dir.path())
            .status()
            .unwrap()
            .success()
    );
    std::fs::write(
        dir.path().join(OsString::from_vec(b"bad\xffname".to_vec())),
        "x",
    )
    .unwrap();
    let status = ProcessGitTooling::status(dir.path()).await.unwrap();
    assert_eq!(status.entries[0].path.as_bytes(), b"bad\xffname");
    assert_eq!(status.entries[0].path.display_escaped(), "bad\\xffname");
}
