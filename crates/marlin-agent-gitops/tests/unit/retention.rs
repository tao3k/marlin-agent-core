// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::{fs, path::Path, process::Command};

use super::{GitContextCustodian, GitContextHoldId, GitRetentionError};
use crate::{CommittedCheckout, EnrolledRepository, GitOperations, GitRepositoryPath};

fn git(path: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(path)
        .args(args)
        .output()
        .unwrap();
    assert!(output.status.success(), "{args:?}: {output:?}");
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn exists(path: &Path, name: &str) -> bool {
    Command::new("git")
        .current_dir(path)
        .args(["cat-file", "-e", name])
        .output()
        .unwrap()
        .status
        .success()
}

fn commit(path: &Path, content: &str) {
    fs::write(path.join("index.org"), content).unwrap();
    git(path, &["add", "--", "index.org"]);
    git(
        path,
        &[
            "-c",
            "user.name=Retention fixture",
            "-c",
            "user.email=retention@example.test",
            "commit",
            "-qm",
            "Publish Context",
            "--no-gpg-sign",
        ],
    );
}

async fn enroll(home: &Path, path: &Path) -> CommittedCheckout {
    let checkout = GitOperations::committed_checkout(path).await.unwrap();
    let context = GitOperations::context(path).await.unwrap();
    EnrolledRepository::enroll(home, &context, None)
        .await
        .unwrap();
    checkout
}

fn collect_objects(path: &Path) {
    git(
        path,
        &[
            "reflog",
            "expire",
            "--expire=now",
            "--expire-unreachable=now",
            "--all",
        ],
    );
    git(path, &["gc", "--prune=now"]);
}

#[tokio::test]
async fn holds_keep_real_git_objects_and_stale_maintenance_cannot_drop_them() {
    let fixture = tempfile::tempdir().unwrap();
    let home = fixture.path().canonicalize().unwrap();
    let repo = home.join("source");
    fs::create_dir(&repo).unwrap();
    git(&repo, &["init", "-q", "--initial-branch=main"]);
    commit(&repo, "* Starting point\n");
    git(&repo, &["switch", "-qc", "review"]);
    commit(&repo, "* A published Context\n");
    let checkout = enroll(&home, &repo).await;
    let locator = checkout
        .locate_blob(GitRepositoryPath::new("index.org").unwrap())
        .await
        .unwrap();
    let custodian = GitContextCustodian::open(&home, &checkout.owner).unwrap();
    let first_id = GitContextHoldId::new("origin-publication").unwrap();
    let second_id = GitContextHoldId::new("dependent-publication").unwrap();
    git(&home, &["clone", "-q", "--no-local", "source", "other"]);
    let foreign = enroll(&home, &home.join("other")).await;
    assert_eq!(foreign.commit, checkout.commit);
    assert!(matches!(
        custodian.retain(&foreign, &locator, &first_id).await,
        Err(GitRetentionError::OwnerChanged)
    ));
    let first = custodian
        .retain(&checkout, &locator, &first_id)
        .await
        .unwrap();
    assert_eq!(
        custodian
            .retain(&checkout, &locator, &first_id)
            .await
            .unwrap(),
        first
    );
    let second = custodian
        .retain(&checkout, &locator, &second_id)
        .await
        .unwrap();
    assert!(matches!(
        custodian.prepare_disposal(&checkout).await,
        Err(GitRetentionError::ActiveBranch)
    ));

    git(&repo, &["switch", "-q", "main"]);
    git(&repo, &["branch", "-D", "review"]);
    collect_objects(&repo);
    assert!(exists(&repo, locator.blob.as_str()));
    let pin = format!("refs/marlin/context/retained/{}", checkout.commit.as_str());
    let journal = format!("refs/marlin/context/claims/{}", checkout.commit.as_str());
    assert_eq!(git(&repo, &["rev-parse", &pin]), checkout.commit.as_str());
    assert_eq!(git(&repo, &["cat-file", "-t", &journal]), "blob");

    // A new service observes the previous process's claims from Git itself.
    let custodian = GitContextCustodian::open(&home, &checkout.owner).unwrap();
    let recovered = custodian
        .recover_hold(&locator, &first_id, first.generation())
        .await
        .unwrap();
    assert_eq!(recovered, first);
    custodian.release(&recovered).await.unwrap();
    assert!(matches!(
        custodian.prepare_disposal(&checkout).await,
        Err(GitRetentionError::LiveHold)
    ));
    custodian.release(&second).await.unwrap();
    let old_ticket = custodian.prepare_disposal(&checkout).await.unwrap();
    let renewed = custodian
        .retain(&checkout, &locator, &first_id)
        .await
        .unwrap();
    assert_ne!(first, renewed);
    assert!(matches!(
        custodian
            .recover_hold(&locator, &first_id, first.generation())
            .await,
        Err(GitRetentionError::InvalidHold)
    ));
    assert!(matches!(
        custodian.release(&first).await,
        Err(GitRetentionError::InvalidHold)
    ));
    assert!(matches!(
        custodian.dispose(old_ticket).await,
        Err(GitRetentionError::StaleDisposal)
    ));
    custodian.release(&renewed).await.unwrap();

    let ticket = custodian.prepare_disposal(&checkout).await.unwrap();
    git(&repo, &["branch", "review", checkout.commit.as_str()]);
    assert!(matches!(
        custodian.dispose(ticket).await,
        Err(GitRetentionError::ActiveBranch)
    ));
    assert!(exists(&repo, &pin));
    git(&repo, &["branch", "-D", "review"]);
    let ticket = custodian.prepare_disposal(&checkout).await.unwrap();
    custodian.dispose(ticket).await.unwrap();
    assert!(!exists(&repo, &pin));
    collect_objects(&repo);
    assert!(!exists(&repo, checkout.commit.as_str()));
    assert!(!exists(&repo, locator.blob.as_str()));
    assert_eq!(git(&repo, &["cat-file", "-t", &journal]), "blob");
    assert_eq!(git(&repo, &["symbolic-ref", "HEAD"]), "refs/heads/main");
}
