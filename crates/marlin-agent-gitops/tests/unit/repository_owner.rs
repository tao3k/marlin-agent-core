// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::{
    EnrolledRepository, GitOperations, GitRemoteName, OWNER_REF, RepositoryEnrollmentError,
};
use std::{fs, path::Path, process::Command};

fn git(path: &Path, args: &[&str]) {
    let result = Command::new("git")
        .current_dir(path)
        .args(args)
        .output()
        .unwrap();
    assert!(result.status.success(), "{args:?}: {result:?}");
}

fn repository(path: &Path) {
    fs::create_dir(path).unwrap();
    git(path, &["init", "-q"]);
    git(
        path,
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.test",
            "commit",
            "--allow-empty",
            "--no-gpg-sign",
            "-qm",
            "seed",
        ],
    );
}

#[tokio::test]
async fn enrollment_binds_linked_worktrees_and_isolates_identical_origins() {
    let sandbox = tempfile::tempdir().unwrap();
    let home = sandbox.path();
    let main = home.join("main");
    let branch = home.join("branch");
    let clone = home.join("clone");
    let url = "https://example.test/Team/repository.git";
    repository(&main);
    git(&main, &["remote", "add", "origin", url]);
    git(
        &main,
        &["worktree", "add", "--detach", branch.to_str().unwrap()],
    );
    git(home, &["clone", "-q", "--no-local", "main", "clone"]);
    git(&clone, &["remote", "set-url", "origin", url]);
    let primary = GitOperations::context(&main).await.unwrap();
    let linked = GitOperations::context(&branch).await.unwrap();
    let copied = GitOperations::context(&clone).await.unwrap();
    assert_eq!(primary.commit(), copied.commit());
    assert_eq!(linked.common_directory(), primary.common_directory());
    assert!(
        EnrolledRepository::load(home, primary.common_directory())
            .unwrap()
            .is_none()
    );
    assert!(!home.join("marlin-ai").exists());

    let (first, second) = tokio::join!(
        EnrolledRepository::enroll(home, &primary, None),
        EnrolledRepository::enroll(home, &linked, None),
    );
    let owner = first.unwrap();
    assert_eq!(owner, second.unwrap());
    let route = owner.root().parent().unwrap();
    assert!(route.ends_with("marlin-ai/repos/example.test/%54eam/repository"));
    assert_eq!(fs::read_dir(route).unwrap().count(), 1);
    assert_eq!(
        EnrolledRepository::load(home, linked.common_directory()).unwrap(),
        Some(owner.clone())
    );
    assert!(
        EnrolledRepository::load(home, copied.common_directory())
            .unwrap()
            .is_none()
    );
    let independent = EnrolledRepository::enroll(home, &copied, None)
        .await
        .unwrap();
    assert_eq!(independent.root().parent(), Some(route));
    assert_ne!(independent.instance(), owner.instance());

    // Route changes preserve the original producer's custody location.
    git(
        &main,
        &[
            "remote",
            "set-url",
            "origin",
            "https://elsewhere.test/team/repo",
        ],
    );
    let selected = GitRemoteName::new("origin").unwrap();
    assert!(matches!(
        EnrolledRepository::enroll(home, &primary, Some(&selected)).await,
        Err(RepositoryEnrollmentError::RouteChanged)
    ));
    assert_eq!(
        EnrolledRepository::load(home, primary.common_directory()).unwrap(),
        Some(owner)
    );
}

#[tokio::test]
async fn a_moved_owner_and_a_copied_reference_are_refused() {
    let sandbox = tempfile::tempdir().unwrap();
    let home = sandbox.path();
    let original = home.join("original");
    let moved = home.join("moved");
    repository(&original);
    let primary = GitOperations::context(&original).await.unwrap();
    let owner = EnrolledRepository::enroll(home, &primary, None)
        .await
        .unwrap();
    assert!(
        owner
            .root()
            .parent()
            .unwrap()
            .ends_with("marlin-ai/repos/_local")
    );
    fs::rename(&original, &moved).unwrap();
    let observed = GitOperations::context(&moved).await.unwrap();
    assert!(matches!(
        EnrolledRepository::load(home, observed.common_directory()),
        Err(RepositoryEnrollmentError::DirectoryChanged)
    ));
    repository(&original);
    fs::copy(
        moved.join(".git").join(OWNER_REF),
        original.join(".git").join(OWNER_REF),
    )
    .unwrap();
    let replacement = GitOperations::context(&original).await.unwrap();
    assert!(matches!(
        EnrolledRepository::enroll(home, &replacement, None).await,
        Err(RepositoryEnrollmentError::DirectoryChanged)
    ));
}

#[tokio::test]
async fn repository_reference_cannot_escape_the_home_store() {
    let sandbox = tempfile::tempdir().unwrap();
    let home = sandbox.path();
    let path = home.join("main");
    repository(&path);
    let context = GitOperations::context(&path).await.unwrap();
    let reference = context.common_directory().join(OWNER_REF);
    fs::write(&reference, br#"{"path":"../elsewhere"}"#).unwrap();
    assert!(matches!(
        EnrolledRepository::load(home, context.common_directory()),
        Err(RepositoryEnrollmentError::InvalidBinding)
    ));
    assert!(!home.join("marlin-ai").exists());
}
