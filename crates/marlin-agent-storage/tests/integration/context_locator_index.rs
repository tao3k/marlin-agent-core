// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

#![cfg(unix)]

use std::{fs, path::Path, process::Command};

use marlin_agent_gitops::{
    CommittedCheckout, EnrolledRepository, GitCommitId, GitOperations, GitRepositoryPath,
    retention::{GitContextCustodian, GitContextHoldId},
};
use marlin_agent_storage::context_index::{
    ContextCut, ContextCutChange, ContextIndexRead, ContextIndexUpdate, ContextLocator,
    ContextOwnerRevision, ContextRetirement, TursoContextIndex,
};

fn git(directory: &Path, args: &[&str]) {
    let output = Command::new("git")
        .current_dir(directory)
        .args(args)
        .output()
        .unwrap();
    assert!(output.status.success(), "git {args:?}: {output:?}");
}

fn commit(directory: &Path, file: &str, value: &str) {
    fs::write(directory.join(file), value).unwrap();
    git(directory, &["add", "--", file]);
    git(
        directory,
        &[
            "-c",
            "user.name=Context fixture",
            "-c",
            "user.email=context@example.test",
            "commit",
            "-qm",
            "Publish Context",
            "--no-gpg-sign",
        ],
    );
}

async fn enroll(home: &Path, directory: &Path, file: &str) -> EnrolledRepository {
    fs::create_dir(directory).unwrap();
    git(directory, &["init", "-q", "--initial-branch=main"]);
    git(
        directory,
        &[
            "remote",
            "add",
            "origin",
            "https://example.test/shared/context.git",
        ],
    );
    commit(directory, file, "* Generation 1\n");
    git(directory, &["switch", "-qc", "review"]);
    let context = GitOperations::context(directory).await.unwrap();
    EnrolledRepository::enroll(home, &context, None)
        .await
        .unwrap()
}

async fn capture(
    directory: &Path,
    path: &GitRepositoryPath,
    generation: u16,
) -> (CommittedCheckout, ContextCut) {
    let checkout = GitOperations::committed_checkout(directory).await.unwrap();
    let entry = ContextLocator {
        generation,
        git: checkout.locate_blob(path.clone()).await.unwrap(),
    };
    let cut = ContextCut::for_checkout(&checkout, vec![entry]);
    (checkout, cut)
}

fn revision(sequence: i64) -> ContextOwnerRevision {
    ContextOwnerRevision::new(sequence, format!("owner-receipt:{sequence}")).unwrap()
}

async fn published_cut(
    fixture: &Path,
    label: &str,
    path: &GitRepositoryPath,
    checkout: &CommittedCheckout,
) -> ContextCut {
    let receipt = fs::read_to_string(fixture.join(format!("{label}.git"))).unwrap();
    let lines: Vec<_> = receipt.lines().collect();
    assert_eq!(lines.len(), 10);
    assert_eq!(Path::new(lines[1]), checkout.owner);
    assert_eq!(Path::new(lines[2]), checkout.git_directory);
    assert_eq!(Path::new(lines[3]), checkout.root.as_path());
    assert_eq!(lines[4], checkout.reference.as_str());
    let mut publication = checkout.clone();
    publication.commit = GitCommitId::parse(lines[7]).unwrap();
    let locator = publication.locate_blob(path.clone()).await.unwrap();
    assert_eq!(locator.blob.as_str(), lines[8]);
    ContextCut::for_checkout(
        &publication,
        vec![ContextLocator {
            generation: lines[6].parse().unwrap(),
            git: locator,
        }],
    )
}

#[tokio::test]
#[ignore = "run with the admitted PR #34 two-Repo fixture"]
async fn index_replays_the_real_worktrunk_publications() {
    let fixture = std::env::var_os("MARLIN_GITOPS_FIXTURE").unwrap();
    let fixture = Path::new(&fixture);
    let home = fixture.join("locator-home");
    fs::create_dir(&home).unwrap();
    let db = home.join("marlin-ai/index.db");
    let paths = [
        GitRepositoryPath::new("index.org").unwrap(),
        GitRepositoryPath::new("branches/review/topology.org").unwrap(),
    ];
    for (repo, path) in ["a", "b"].into_iter().zip(paths) {
        let directory = fixture.join(format!("{repo}-review"));
        let context = GitOperations::context(&directory).await.unwrap();
        let owner = EnrolledRepository::enroll(&home, &context, None)
            .await
            .unwrap();
        let checkout = GitOperations::committed_checkout(&directory).await.unwrap();
        let custodian = GitContextCustodian::open(&home, &checkout.owner).unwrap();
        let hold_id = GitContextHoldId::new(format!("pr34-historical-{repo}")).unwrap();
        let suffix = if repo == "a" { "index" } else { "branch" };
        let index = TursoContextIndex::open(&db).await.unwrap();
        let mut historical = None;
        for generation in 1..=2 {
            let label = format!("{repo}{generation}-{suffix}");
            let cut = published_cut(fixture, &label, &path, &checkout).await;
            assert_eq!(
                index
                    .apply(
                        owner.instance(),
                        &revision(generation),
                        ContextCutChange::Publish(&cut)
                    )
                    .await
                    .unwrap(),
                ContextIndexUpdate::Applied,
            );
            assert_eq!(
                index
                    .lookup(owner.instance(), &cut.key, &path)
                    .await
                    .unwrap(),
                ContextIndexRead::Current(cut.entries[0].clone()),
            );
            if generation == 1 {
                let mut source = checkout.clone();
                source.commit = cut.tip.clone();
                let locator = cut.entries[0].clone();
                index
                    .retain_current_locator(&custodian, &source, &locator, &hold_id)
                    .await
                    .unwrap();
                historical = Some((source, locator));
            } else {
                let receipt = checkout
                    .read_located_blob(&cut.entries[0].git)
                    .await
                    .unwrap();
                assert_eq!(
                    receipt,
                    fs::read(fixture.join(format!("{label}.org"))).unwrap()
                );
            }
        }
        let (source, locator) = historical.unwrap();
        assert!(
            index
                .retain_current_locator(&custodian, &source, &locator, &hold_id)
                .await
                .is_err()
        );
        let pin = format!("refs/marlin/context/retained/{}", source.commit.as_str());
        git(
            &directory,
            &["cat-file", "-e", &format!("{pin}^{{commit}}")],
        );
    }
}

#[tokio::test]
async fn index_recovers_owner_gaps_and_retires_only_the_observed_cut() {
    let temporary = tempfile::tempdir().unwrap();
    let home = temporary.path();
    let a = home.join("a");
    let b = home.join("b");
    let a_owner = enroll(home, &a, "index.org").await;
    let b_owner = enroll(home, &b, "topology.org").await;
    assert_ne!(a_owner.instance(), b_owner.instance());
    let a_file = GitRepositoryPath::new("index.org").unwrap();
    let b_file = GitRepositoryPath::new("topology.org").unwrap();
    let (a_checkout, a1) = capture(&a, &a_file, 1).await;
    let (b_checkout, b1) = capture(&b, &b_file, 1).await;

    let db = home.join("marlin-ai/index.db");
    let index = TursoContextIndex::open(&db).await.unwrap();
    assert_eq!(
        index
            .lookup(a_owner.instance(), &a1.key, &a_file)
            .await
            .unwrap(),
        ContextIndexRead::Incomplete,
    );
    for (owner, cut) in [(&a_owner, &a1), (&b_owner, &b1)] {
        assert_eq!(
            index
                .apply(
                    owner.instance(),
                    &revision(1),
                    ContextCutChange::Publish(cut)
                )
                .await
                .unwrap(),
            ContextIndexUpdate::Applied,
        );
    }
    drop(index);

    let index = TursoContextIndex::open(&db).await.unwrap();
    assert_eq!(
        index
            .lookup(a_owner.instance(), &a1.key, &a_file)
            .await
            .unwrap(),
        ContextIndexRead::Current(a1.entries[0].clone()),
    );
    assert_eq!(
        index
            .lookup(a_owner.instance(), &b1.key, &b_file)
            .await
            .unwrap(),
        ContextIndexRead::Absent,
    );
    assert!(
        b_checkout
            .read_located_blob(&a1.entries[0].git)
            .await
            .is_err()
    );

    commit(&a, "index.org", "* Generation 2\n");
    let (_, a2) = capture(&a, &a_file, 2).await;
    commit(&a, "index.org", "* Generation 3\n");
    let (_, a3) = capture(&a, &a_file, 3).await;

    assert_eq!(
        index
            .apply(
                a_owner.instance(),
                &revision(3),
                ContextCutChange::Publish(&a3)
            )
            .await
            .unwrap(),
        ContextIndexUpdate::Gap {
            cursor: 1,
            observed: 3
        },
    );
    drop(index);
    let index = TursoContextIndex::open(&db).await.unwrap();
    assert_eq!(
        index
            .lookup(a_owner.instance(), &a1.key, &a_file)
            .await
            .unwrap(),
        ContextIndexRead::Incomplete,
    );
    assert_eq!(
        index
            .lookup(b_owner.instance(), &b1.key, &b_file)
            .await
            .unwrap(),
        ContextIndexRead::Current(b1.entries[0].clone()),
    );
    assert_eq!(
        index
            .rebuild(a_owner.instance(), &revision(2), std::slice::from_ref(&a2))
            .await
            .unwrap(),
        ContextIndexUpdate::Superseded,
    );
    assert_eq!(
        index
            .apply(
                a_owner.instance(),
                &revision(2),
                ContextCutChange::Publish(&a2)
            )
            .await
            .unwrap(),
        ContextIndexUpdate::Applied,
    );
    assert_eq!(
        index
            .lookup(a_owner.instance(), &a2.key, &a_file)
            .await
            .unwrap(),
        ContextIndexRead::Incomplete,
    );
    assert_eq!(
        index
            .rebuild(a_owner.instance(), &revision(3), std::slice::from_ref(&a3))
            .await
            .unwrap(),
        ContextIndexUpdate::Applied,
    );
    assert_eq!(
        index
            .apply(
                a_owner.instance(),
                &revision(3),
                ContextCutChange::Publish(&a3)
            )
            .await
            .unwrap(),
        ContextIndexUpdate::Replayed,
    );

    let retire = |tip| ContextCutChange::Retire {
        key: &a3.key,
        expected_tip: tip,
        reason: ContextRetirement::Deleted,
    };
    assert_eq!(
        index
            .apply(a_owner.instance(), &revision(4), retire(&a1.tip))
            .await
            .unwrap(),
        ContextIndexUpdate::Conflict,
    );
    assert_eq!(
        index
            .lookup(a_owner.instance(), &a3.key, &a_file)
            .await
            .unwrap(),
        ContextIndexRead::Current(a3.entries[0].clone()),
    );
    git(&a, &["switch", "-q", "main"]);
    git(&a, &["branch", "-D", "review"]);
    assert_eq!(
        index
            .apply(a_owner.instance(), &revision(4), retire(&a3.tip))
            .await
            .unwrap(),
        ContextIndexUpdate::Applied,
    );
    assert_eq!(
        index
            .lookup(a_owner.instance(), &a3.key, &a_file)
            .await
            .unwrap(),
        ContextIndexRead::Absent,
    );
    assert_eq!(
        a_checkout
            .read_located_blob(&a1.entries[0].git)
            .await
            .unwrap(),
        b"* Generation 1\n",
    );
    let rebuilt = TursoContextIndex::open(&home.join("marlin-ai/rebuilt.db"))
        .await
        .unwrap();
    rebuilt
        .rebuild(a_owner.instance(), &revision(4), &[])
        .await
        .unwrap();
    rebuilt
        .rebuild(b_owner.instance(), &revision(1), std::slice::from_ref(&b1))
        .await
        .unwrap();
    for (owner, key, path) in [(&a_owner, &a3.key, &a_file), (&b_owner, &b1.key, &b_file)] {
        assert_eq!(
            index.lookup(owner.instance(), key, path).await.unwrap(),
            rebuilt.lookup(owner.instance(), key, path).await.unwrap(),
        );
    }
}

#[tokio::test]
async fn sm31_adv_l4_rebuild_refuses_divergent_overwrite_of_finalized_revision() {
    let temporary = tempfile::tempdir().unwrap();
    let home = temporary.path();
    let a = home.join("a");
    let a_owner = enroll(home, &a, "index.org").await;
    let a_file = GitRepositoryPath::new("index.org").unwrap();
    let (_, a1) = capture(&a, &a_file, 1).await;

    let db = home.join("marlin-ai/sm31-index.db");
    let index = TursoContextIndex::open(&db).await.unwrap();
    assert_eq!(
        index
            .apply(
                a_owner.instance(),
                &revision(1),
                ContextCutChange::Publish(&a1)
            )
            .await
            .unwrap(),
        ContextIndexUpdate::Applied,
    );

    // Hostile same-(sequence, receipt) rebuild with divergent empty cuts MUST be refused as Conflict
    // and MUST NOT wipe already-published context_locators.
    assert_eq!(
        index
            .rebuild(a_owner.instance(), &revision(1), &[])
            .await
            .unwrap(),
        ContextIndexUpdate::Conflict,
    );
    assert_eq!(
        index
            .lookup(a_owner.instance(), &a1.key, &a_file)
            .await
            .unwrap(),
        ContextIndexRead::Current(a1.entries[0].clone()),
    );

    // Exact idempotent replay with identical cuts MUST return Replayed cleanly.
    assert_eq!(
        index
            .rebuild(a_owner.instance(), &revision(1), std::slice::from_ref(&a1))
            .await
            .unwrap(),
        ContextIndexUpdate::Replayed,
    );
}
