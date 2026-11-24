// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use marlin_agent_gitops::working_copy::WorkingCopyIsolationDriver;
use marlin_agent_gitops::working_copy::{
    WorkingCopyActiveBinding, WorkingCopyCommandProgram, WorkingCopyCreateRequest,
    WorkingCopyHandle, WorkingCopyIsolationProvider, WorkingCopyIsolationRequest,
    WorkingCopyIsolationStatus,
};
use std::{env, fs, path::Path, process::Command};

fn git(directory: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(directory)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

#[tokio::test]
#[ignore = "requires Worktrunk and the PR 34 fixture directory"]
async fn create_pr34_worktrees() {
    let output = fs::canonicalize(env::var_os("MARLIN_GITOPS_FIXTURE").unwrap()).unwrap();
    let driver = WorkingCopyIsolationDriver::new();
    for name in ["a", "b"] {
        let repository = output.join(name);
        fs::create_dir(&repository).unwrap();
        for args in [
            &["init", "-q", "--initial-branch=main"][..],
            &["config", "core.hooksPath", "/dev/null"],
            &["config", "user.name", "Context verification"],
            &[
                "config",
                "user.email",
                "context-verification@example.invalid",
            ],
            &[
                "remote",
                "add",
                "origin",
                &format!("https://example.invalid/context/{name}.git"),
            ],
            &[
                "commit",
                "--quiet",
                "--allow-empty",
                "--no-gpg-sign",
                "-m",
                "Seed Git observation",
            ],
        ] {
            git(&repository, args);
        }

        let requested = WorkingCopyHandle::new("review", output.join(format!("{name}-unused")))
            .with_branch("review");
        let request = WorkingCopyCreateRequest::new(
            name,
            WorkingCopyIsolationProvider::Worktrunk,
            repository.as_path(),
            requested.clone(),
        )
        .with_base_ref("main");
        let receipt = driver
            .isolate(WorkingCopyIsolationRequest::Create(request))
            .await
            .receipt;
        assert_eq!(
            receipt.status,
            WorkingCopyIsolationStatus::Applied,
            "{receipt:?}"
        );
        let active = WorkingCopyActiveBinding::from_receipt(&receipt).unwrap();
        assert_eq!(active.working_copy.branch, requested.branch);
        let [command] = receipt.command_receipts.as_slice() else {
            panic!("expected one Worktrunk command: {receipt:?}");
        };
        assert_eq!(command.program, WorkingCopyCommandProgram::Worktrunk);
        assert_eq!(command.working_copy.as_ref(), Some(&active.working_copy));
        let worktree = &active.working_copy.path;
        assert_eq!(worktree, &output.join(format!("{name}-review")));
        assert_eq!(
            git(worktree, &["symbolic-ref", "HEAD"]).trim(),
            "refs/heads/review"
        );
        let common = git(
            worktree,
            &["rev-parse", "--path-format=absolute", "--git-common-dir"],
        );
        assert_eq!(Path::new(common.trim()), repository.join(".git"));
        assert_ne!(
            git(worktree, &["rev-parse", "--absolute-git-dir"]).trim(),
            common.trim()
        );
        fs::write(
            output.join(format!("{name}-worktrunk.json")),
            serde_json::to_vec_pretty(&receipt).unwrap(),
        )
        .unwrap();
    }
}
