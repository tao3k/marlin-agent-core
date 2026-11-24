// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::{path::Path, process::Command};

use super::{FetchRoute, fetch_route};
use crate::{GitOperations, GitRemoteName, RepositoryRoute, RepositoryRouteError};

fn network_path(url: &str) -> String {
    let FetchRoute::Network(route) = fetch_route(url).unwrap() else {
        panic!("expected network route");
    };
    route.as_path().to_string_lossy().replace('\\', "/")
}

#[test]
fn aliases_share_a_route_and_case_sensitive_paths_remain_distinct() {
    for url in [
        "https://u:p@GitHub.com/Team/Example.git",
        "ssh://git@github.com:22/Team/Example/",
        "git@github.com:Team/Example",
        "https://github.com/%54eam/%45xample.git",
    ] {
        assert_eq!(network_path(url), "github.com/%54eam/%45xample");
    }
    assert_ne!(
        network_path("https://github.com/team/example"),
        network_path("https://github.com/Team/Example")
    );
    assert_eq!(
        network_path("ssh://git@BÜCHER.example:443/design/notes+schema.git"),
        "xn--bcher-kva.example%3A443/design/notes%2Bschema"
    );
    assert_eq!(
        network_path("git@[2001:db8::1]:tools/alpha]beta.git"),
        "%5B2001%3Adb8%3A%3A1%5D/tools/alpha%5Dbeta"
    );
    assert_eq!(
        network_path("ssh://_local/tools/repo"),
        "%5Flocal/tools/repo"
    );
    assert_eq!(
        network_path("https://github.com/con/lpt8.git"),
        "github.com/%63on/%6Cpt8"
    );
}

#[test]
fn invalid_paths_and_transport_helpers_cannot_supply_a_route() {
    for url in [
        "https://github.com/Team/../Example",
        "https://github.com/Team/%2E%2e/Example",
        "https://github.com/Team//Example",
        "https://github.com/Team/%2fExample",
        "https://github.com/Team/%5CExample",
        "https://github.com/Team/Example%00",
        "https://github.com/Team/Example%ZZ",
        "https://github.com/Team/.git",
        "https://github.com/Team/Example?source=1",
        "https://github.com/Team/Example#fragment",
        "https:///Team/Example",
        "https://github.com",
        "ext::helper",
        "file://other-host/repo",
    ] {
        assert_eq!(
            fetch_route(url),
            Err(RepositoryRouteError::InvalidFetchUrl),
            "{url}"
        );
    }
    assert_eq!(
        fetch_route("file:///repo"),
        Ok(FetchRoute::Local("file:///repo"))
    );
    assert_eq!(
        fetch_route("../local:mirror"),
        Ok(FetchRoute::Local("../local:mirror"))
    );
}

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[tokio::test]
async fn real_git_remotes_use_explicit_origin_and_sole_network_selection() {
    let sandbox = tempfile::tempdir().unwrap();
    let repo = sandbox.path();
    git(repo, &["init", "-q", "-b", "main"]);
    git(
        repo,
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.test",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--allow-empty",
            "-qm",
            "fixture",
        ],
    );
    let context = GitOperations::context(repo).await.unwrap();
    assert_eq!(
        GitOperations::repository_route(&context, None)
            .await
            .unwrap(),
        RepositoryRoute::Local
    );

    git(repo, &["remote", "add", "local", "../mirror"]);
    assert_eq!(
        GitOperations::repository_route(&context, None).await,
        Err(RepositoryRouteError::NoEligibleRemote)
    );
    git(
        repo,
        &[
            "remote",
            "add",
            "review",
            "https://review.example/Team/repo.git",
        ],
    );
    let review = GitOperations::repository_route(&context, None)
        .await
        .unwrap();
    git(
        repo,
        &["config", "url.ssh://git@work.example/.insteadOf", "alias:"],
    );
    git(repo, &["remote", "add", "origin", "alias:team/repo.git"]);
    let origin = GitOperations::repository_route(&context, None)
        .await
        .unwrap();
    let FetchRoute::Network(expected) = fetch_route("ssh://work.example/team/repo").unwrap() else {
        panic!("expected network route");
    };
    assert_eq!(origin, RepositoryRoute::Network(expected));
    let preferred = GitRemoteName::new("review").unwrap();
    assert_eq!(
        GitOperations::repository_route(&context, Some(&preferred))
            .await
            .unwrap(),
        review
    );

    git(
        repo,
        &[
            "remote",
            "set-url",
            "--add",
            "origin",
            "https://work.example/team/repo",
        ],
    );
    assert_eq!(
        GitOperations::repository_route(&context, None)
            .await
            .unwrap(),
        origin
    );
    git(
        repo,
        &[
            "remote",
            "set-url",
            "--add",
            "origin",
            "https://work.example/team/other",
        ],
    );
    assert_eq!(
        GitOperations::repository_route(&context, None).await,
        Err(RepositoryRouteError::ConflictingFetchRoutes)
    );
    git(repo, &["remote", "rename", "origin", "other"]);
    git(
        repo,
        &[
            "remote",
            "set-url",
            "--delete",
            "other",
            "https://work.example/team/other",
        ],
    );
    assert_eq!(
        GitOperations::repository_route(&context, None).await,
        Err(RepositoryRouteError::AmbiguousRemotes)
    );
    assert_eq!(
        GitOperations::repository_route(&context, Some(&GitRemoteName::new("origin").unwrap()))
            .await,
        Err(RepositoryRouteError::MissingRemote)
    );
}
