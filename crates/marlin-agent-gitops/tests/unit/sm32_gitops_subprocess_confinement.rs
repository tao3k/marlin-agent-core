// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! SM32 (ADV-L4) hostile runtime exploit suite for `marlin-agent-gitops`.
//!
//! Exercises 5 real vulnerabilities on live Git repositories and provider runners:
//! 1. `GIT_CONFIG_GLOBAL` URL-rewrite hijack & `GIT_EXEC_PATH` command hijack.
//! 2. Hostile `.git/config` `[core] fsmonitor` shell command execution during worktree ops.
//! 3. Dash-prefixed CLI option injection in command projection & `plan_sweep` path traversal.
//! 4. Unbounded `.output().await` stdout flood (>1 MiB) in `ProcessWorkingCopyCommandRunner`.
//! 5. Coordinator panic via `.expect()` in `isolate_parallel` & `/` / `..` Worktrunk receipt paths.

use std::{
    future::Future,
    path::{Component, Path, PathBuf},
    pin::Pin,
    process::Command,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use marlin_agent_gitops::{
    GIT_REPOSITORY_OVERRIDES, GitOperations,
    working_copy::{
        ProcessWorkingCopyCommandRunner, WorkingCopyBaseRef, WorkingCopyBranchName,
        WorkingCopyCommandInvocation, WorkingCopyCommandOutput, WorkingCopyCommandProgram,
        WorkingCopyCommandProjection, WorkingCopyCommandProjectionError, WorkingCopyCommandRunner,
        WorkingCopyCreateRequest, WorkingCopyFinalizeBranchRequest, WorkingCopyGitRepositoryResolver,
        WorkingCopyGitTopLevel, WorkingCopyHandle, WorkingCopyId, WorkingCopyIsolationDriver,
        WorkingCopyIsolationDriverError, WorkingCopyIsolationPlan, WorkingCopyIsolationProvider,
        WorkingCopyIsolationRequest, WorkingCopyIsolationStatus, WorkingCopyRemoveRequest,
        WorkingCopyRepositoryDiscoveryPath, WorkingCopyRetentionPolicy, WorkingCopySwitchRequest,
    },
};
use marlin_agent_protocol::{
    BoundedProcessExecutor, BoundedProcessOutput, BoundedProcessRequest, ProcessExecutionError,
};

fn git(path: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(path)
        .args(args)
        .output()
        .expect("git command should run");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("utf8 stdout")
}

fn init_commit_repo(repo: &Path) {
    std::fs::create_dir_all(repo).expect("create repo dir");
    git(repo, &["init", "-q", "-b", "main"]);
    git(
        repo,
        &[
            "-c",
            "user.name=SM32 Fixture",
            "-c",
            "user.email=sm32@example.test",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-qm",
            "seed",
            "--allow-empty",
        ],
    );
}

#[derive(Clone, Default)]
struct CapturingExecutor {
    removed: Arc<Mutex<Vec<String>>>,
}

impl BoundedProcessExecutor for CapturingExecutor {
    fn execute(
        &self,
        request: BoundedProcessRequest,
    ) -> Pin<
        Box<dyn Future<Output = Result<BoundedProcessOutput, ProcessExecutionError>> + Send + '_>,
    > {
        let removed = self.removed.clone();
        Box::pin(async move {
            let mut guard = removed.lock().expect("lock");
            guard.extend(
                request
                    .remove_environment
                    .into_iter()
                    .map(|v| v.to_string_lossy().into_owned()),
            );
            Ok(BoundedProcessOutput {
                stdout: Vec::new(),
                exit_code: Some(0),
                success: true,
            })
        })
    }
}

#[tokio::test]
async fn sm32_v1_git_env_config_global_and_exec_path_cannot_hijack_gitops() {
    if let Some(repo_os) = std::env::var_os("SM32_CHILD_PROBE_REPO") {
        let repo = PathBuf::from(repo_os);
        let resolved = GitOperations::resolve_repository_root(&repo)
            .await
            .expect("resolve_repository_root should succeed under poisoned GIT_EXEC_PATH");
        assert_eq!(
            resolved.as_path(),
            repo.as_path(),
            "SM32-V1: GIT_EXEC_PATH must not hijack resolve_repository_root"
        );

        let urls = GitOperations::fetch_remote_urls(&resolved)
            .await
            .expect("fetch_remote_urls should succeed");
        assert!(
            !urls.iter().any(|u| u.contains("attacker/pwn")),
            "SM32-V1: inherited GIT_CONFIG_GLOBAL rewrote remote URLs: {urls:?}"
        );

        let runner = ProcessWorkingCopyCommandRunner;
        let inv = WorkingCopyCommandInvocation::new(WorkingCopyCommandProgram::Git, resolved)
            .with_args(["rev-parse", "--show-toplevel"]);
        let out = runner
            .run(&inv)
            .await
            .expect("provider runner rev-parse should succeed");
        assert!(out.success, "provider runner failed: {}", out.stderr);
        assert_eq!(
            Path::new(out.stdout.trim()),
            repo.as_path(),
            "SM32-V1: GIT_EXEC_PATH hijacked ProcessWorkingCopyCommandRunner"
        );
        return;
    }

    let required_overrides = [
        "GIT_CONFIG_GLOBAL",
        "GIT_CONFIG_SYSTEM",
        "GIT_CONFIG_NOSYSTEM",
        "GIT_EXEC_PATH",
        "GIT_TEMPLATE_DIR",
        "GIT_SSH",
        "GIT_SSH_COMMAND",
        "GIT_EXTERNAL_DIFF",
        "GIT_ASKPASS",
        "SSH_ASKPASS",
        "GIT_PAGER",
        "GIT_EDITOR",
        "GIT_SEQUENCE_EDITOR",
    ];
    for key in required_overrides {
        assert!(
            GIT_REPOSITORY_OVERRIDES.contains(&key),
            "SM32-V1: GIT_REPOSITORY_OVERRIDES must strip {key}"
        );
    }

    let capture = CapturingExecutor::default();
    let _ = GitOperations::status_with_executor(&capture, Path::new(".")).await;
    let removed = capture.removed.lock().expect("lock").clone();
    for key in ["GIT_CONFIG_GLOBAL", "GIT_EXEC_PATH", "GIT_SSH_COMMAND"] {
        assert!(
            removed.iter().any(|item| item == key),
            "SM32-V1: BoundedProcessRequest.remove_environment must include {key}"
        );
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path().canonicalize().expect("canonical tempdir");
        let repo = root.join("repo");
        init_commit_repo(&repo);
        git(
            &repo,
            &[
                "remote",
                "add",
                "origin",
                "https://github.com/expected/project.git",
            ],
        );

        let evil_gitconfig = root.join("evil.gitconfig");
        std::fs::write(
            &evil_gitconfig,
            "[url \"https://github.com/attacker/pwn.git\"]\n\tinsteadOf = https://github.com/expected/project.git\n",
        )
        .expect("write evil gitconfig");

        let evil_exec_dir = root.join("evil-exec");
        std::fs::create_dir(&evil_exec_dir).expect("create evil-exec");
        let hijack_marker = root.join("exec_path_hijacked");
        let trojan = evil_exec_dir.join("git-rev-parse");
        std::fs::write(
            &trojan,
            format!(
                "#!/bin/sh\necho HIJACKED > \"{}\"\necho /tmp/hijacked\n",
                hijack_marker.display()
            ),
        )
        .expect("write trojan git-rev-parse");
        std::fs::set_permissions(&trojan, std::fs::Permissions::from_mode(0o755))
            .expect("chmod trojan");

        let exe = std::env::current_exe().expect("current test binary");
        let child = Command::new(exe)
            .env("GIT_CONFIG_GLOBAL", &evil_gitconfig)
            .env("GIT_EXEC_PATH", &evil_exec_dir)
            .env("SM32_CHILD_PROBE_REPO", &repo)
            .args([
                "--exact",
                "sm32_gitops_subprocess_confinement::sm32_v1_git_env_config_global_and_exec_path_cannot_hijack_gitops",
                "--nocapture",
            ])
            .output()
            .expect("spawn child probe");

        assert!(
            !hijack_marker.exists(),
            "SM32-V1: inherited GIT_EXEC_PATH hijacked git-rev-parse and wrote marker file"
        );
        assert!(
            child.status.success(),
            "SM32-V1 child probe failed (env hijack succeeded):\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&child.stdout),
            String::from_utf8_lossy(&child.stderr),
        );
    }
}

#[cfg(unix)]
#[tokio::test]
async fn sm32_v2_hostile_repo_fsmonitor_cannot_execute_during_worktree_ops() {
    use std::os::unix::fs::PermissionsExt;

    let temp = tempfile::tempdir().expect("tempdir");
    let parent = temp.path().canonicalize().expect("canonical tempdir");
    let repo = parent.join("project");
    init_commit_repo(&repo);

    let fsmonitor_marker = parent.join("fsmonitor_pwned");
    let hook_marker = parent.join("hook_pwned");

    // Hostile repository `.git/config` configuring `core.fsmonitor`.
    git(
        &repo,
        &[
            "config",
            "core.fsmonitor",
            &format!("sh -c 'echo PWNED > \"{}\"; exit 0'", fsmonitor_marker.display()),
        ],
    );
    // Also place a hostile `post-checkout` hook in `.git/hooks`.
    let post_checkout = repo.join(".git/hooks/post-checkout");
    std::fs::write(
        &post_checkout,
        format!("#!/bin/sh\necho HOOK > \"{}\"\n", hook_marker.display()),
    )
    .expect("write hook");
    std::fs::set_permissions(&post_checkout, std::fs::Permissions::from_mode(0o755))
        .expect("chmod hook");

    let driver = WorkingCopyIsolationDriver::new();
    let linked = parent.join("linked-sm32");
    let create_req = WorkingCopyIsolationRequest::Create(
        WorkingCopyCreateRequest::new(
            "project",
            WorkingCopyIsolationProvider::GitWorktree,
            repo.as_path(),
            WorkingCopyHandle::new("agent-sm32", &linked).with_branch("agent/sm32"),
        )
        .with_base_ref("HEAD"),
    );
    let create_receipt = driver.isolate(create_req).await.receipt;
    assert_eq!(
        create_receipt.status,
        WorkingCopyIsolationStatus::Applied,
        "create failed: {create_receipt:?}"
    );
    assert!(
        !fsmonitor_marker.exists(),
        "SM32-V2: hostile .git/config core.fsmonitor executed during git worktree add!"
    );
    assert!(
        !hook_marker.exists(),
        "SM32-V2: hostile post-checkout hook executed during git worktree add!"
    );

    let remove_req = WorkingCopyIsolationRequest::Remove(
        WorkingCopyRemoveRequest::new(
            "project",
            WorkingCopyIsolationProvider::GitWorktree,
            repo.as_path(),
            WorkingCopyHandle::new("agent-sm32", &linked),
        )
        .forcing_removal(),
    );
    let remove_receipt = driver.isolate(remove_req).await.receipt;
    assert_eq!(
        remove_receipt.status,
        WorkingCopyIsolationStatus::Applied,
        "remove failed: {remove_receipt:?}"
    );
    assert!(
        !fsmonitor_marker.exists(),
        "SM32-V2: hostile .git/config core.fsmonitor executed during git worktree remove!"
    );
}

fn stub_toplevel(
    path: &WorkingCopyRepositoryDiscoveryPath,
) -> Result<WorkingCopyGitTopLevel, WorkingCopyCommandProjectionError> {
    Ok(WorkingCopyGitTopLevel::from_resolved_path(
        path.as_path().to_path_buf(),
    ))
}

#[test]
fn sm32_v3_option_injection_in_projection_and_snapshot_path_traversal_refused() {
    let hostile_requests = vec![
        WorkingCopyIsolationRequest::Switch(WorkingCopySwitchRequest::new(
            "proj",
            WorkingCopyIsolationProvider::Worktrunk,
            "/repo",
            WorkingCopyHandle::new("copy-a", "/repo.copy-a")
                .with_branch(WorkingCopyBranchName::new("--execute=sh -c id")),
        )),
        WorkingCopyIsolationRequest::Switch(WorkingCopySwitchRequest::new(
            "proj",
            WorkingCopyIsolationProvider::Worktrunk,
            "/repo",
            WorkingCopyHandle::new(WorkingCopyId::new("--force"), "/repo.copy-a"),
        )),
        WorkingCopyIsolationRequest::Switch(
            WorkingCopySwitchRequest::new(
                "proj",
                WorkingCopyIsolationProvider::Worktrunk,
                "/repo",
                WorkingCopyHandle::new("copy-a", "/repo.copy-a").with_branch("feature/ok"),
            )
            .create_if_missing_from(WorkingCopyBaseRef::new("--no-verify")),
        ),
        WorkingCopyIsolationRequest::Create(WorkingCopyCreateRequest::new(
            "proj",
            WorkingCopyIsolationProvider::GitWorktree,
            "/repo",
            WorkingCopyHandle::new("copy-a", "/repo.copy-a")
                .with_branch(WorkingCopyBranchName::new("--orphan")),
        )),
        WorkingCopyIsolationRequest::FinalizeBranch(WorkingCopyFinalizeBranchRequest::new(
            "proj",
            WorkingCopyIsolationProvider::GitWorktree,
            WorkingCopyHandle::new("copy-a", "/repo.copy-a"),
            WorkingCopyBranchName::new("-f"),
        )),
        WorkingCopyIsolationRequest::Create(
            WorkingCopyCreateRequest::new(
                "proj",
                WorkingCopyIsolationProvider::GitWorktree,
                "/repo",
                WorkingCopyHandle::new("copy-a", "/repo.copy-a").with_branch("feature/ok"),
            )
            .with_base_ref(WorkingCopyBaseRef::new("--detach")),
        ),
    ];

    for (idx, req) in hostile_requests.into_iter().enumerate() {
        let plan = WorkingCopyIsolationPlan::compile(&req).expect("plan compile");
        let projected = WorkingCopyCommandProjection::from_plan(&plan, stub_toplevel);
        assert!(
            projected.is_err(),
            "SM32-V3 case #{idx}: hostile dash-prefixed option must be rejected during projection, got {projected:?}"
        );
    }

    // Retention sweep snapshot path confinement.
    let snapshot_root = PathBuf::from("/repo/.marlin/snapshots");
    let sweep = WorkingCopyRetentionPolicy::new(0).plan_sweep(
        "proj",
        WorkingCopyIsolationProvider::Worktrunk,
        [
            WorkingCopyHandle::new("../../etc/pwn", "/repo.a"),
            WorkingCopyHandle::new("/tmp/escape", "/repo.b"),
            WorkingCopyHandle::new("safe-agent", "/repo.c"),
        ],
        &snapshot_root,
    );
    for action in &sweep.actions {
        if let Some(path) = &action.snapshot_path {
            assert_eq!(
                path.parent(),
                Some(snapshot_root.as_path()),
                "SM32-V3: snapshot_path {path:?} escaped snapshot_root {snapshot_root:?}"
            );
            assert!(
                !path
                    .components()
                    .any(|c| matches!(c, Component::ParentDir)),
                "SM32-V3: snapshot_path {path:?} contains parent traversal"
            );
        }
    }
    assert_eq!(
        sweep.actions[2].snapshot_path.as_deref(),
        Some(Path::new("/repo/.marlin/snapshots/safe-agent.patch"))
    );
}

#[tokio::test]
async fn sm32_v4_provider_runner_enforces_bounded_output_limit() {
    let temp = tempfile::tempdir().expect("tempdir");
    let repo = temp.path().canonicalize().expect("canonical tempdir");
    init_commit_repo(&repo);

    // Create a 1.5 MiB blob in the repo object database (> 1 MiB cap).
    let flood_path = repo.join("flood.bin");
    std::fs::write(&flood_path, vec![b'A'; 1_600_000]).expect("write flood blob");
    let oid = git(&repo, &["hash-object", "-w", "flood.bin"])
        .trim()
        .to_owned();

    let runner = ProcessWorkingCopyCommandRunner;
    let invocation = WorkingCopyCommandInvocation::new(
        WorkingCopyCommandProgram::Git,
        WorkingCopyGitTopLevel::from_resolved_path(repo),
    )
    .with_args(["cat-file", "blob", &oid]);

    let result = runner.run(&invocation).await;
    assert!(
        result.is_err(),
        "SM32-V4: ProcessWorkingCopyCommandRunner must refuse >1 MiB stdout flood instead of unbounded .output().await"
    );
}

#[derive(Clone)]
struct PanickingWorkerRunner;

#[async_trait]
impl WorkingCopyCommandRunner for PanickingWorkerRunner {
    async fn run(
        &self,
        invocation: &WorkingCopyCommandInvocation,
    ) -> Result<WorkingCopyCommandOutput, WorkingCopyIsolationDriverError> {
        if invocation
            .expected_working_copy
            .as_ref()
            .is_some_and(|w| w.id.as_str() == "agent-b")
        {
            panic!("simulated hostile worker panic in agent-b");
        }
        Ok(WorkingCopyCommandOutput::succeeded(""))
    }
}

#[derive(Clone)]
struct StaticRootResolver(WorkingCopyGitTopLevel);

#[async_trait]
impl WorkingCopyGitRepositoryResolver for StaticRootResolver {
    async fn resolve_git_toplevel(
        &self,
        _discovery_path: &Path,
    ) -> Result<WorkingCopyGitTopLevel, WorkingCopyIsolationDriverError> {
        Ok(self.0.clone())
    }

    async fn resolve_repository_worktree(
        &self,
        repository: &WorkingCopyGitTopLevel,
        target: &Path,
    ) -> Result<WorkingCopyGitTopLevel, WorkingCopyIsolationDriverError> {
        Ok(WorkingCopyGitTopLevel::from_resolved_path(
            repository.as_path().join(target),
        ))
    }
}

#[derive(Clone)]
struct SingleOutputRunner(WorkingCopyCommandOutput);

#[async_trait]
impl WorkingCopyCommandRunner for SingleOutputRunner {
    async fn run(
        &self,
        _invocation: &WorkingCopyCommandInvocation,
    ) -> Result<WorkingCopyCommandOutput, WorkingCopyIsolationDriverError> {
        Ok(self.0.clone())
    }
}

#[tokio::test]
async fn sm32_v5_isolate_parallel_isolates_worker_panic_and_rejects_hostile_worktrunk_paths() {
    let resolver = StaticRootResolver(WorkingCopyGitTopLevel::from_resolved_path(PathBuf::from(
        "/repo",
    )));
    let driver =
        WorkingCopyIsolationDriver::with_runner_and_git_resolver(PanickingWorkerRunner, resolver.clone());
    let requests = ["agent-a", "agent-b", "agent-c"].into_iter().map(|id| {
        WorkingCopyIsolationRequest::Create(WorkingCopyCreateRequest::new(
            "proj",
            WorkingCopyIsolationProvider::GitWorktree,
            "/repo",
            WorkingCopyHandle::new(id, format!("/repo.{id}")).with_branch(format!("branch/{id}")),
        ))
    });

    let parallel = driver
        .isolate_parallel("proj", WorkingCopyIsolationProvider::GitWorktree, 2, requests)
        .await;
    assert_eq!(parallel.receipt.requested, 3);
    assert_eq!(parallel.receipt.applied, 2);
    assert_eq!(parallel.receipt.rejected, 1);
    assert!(parallel.receipt.has_consistent_scope());
    assert!(
        parallel
            .receipt
            .receipts
            .iter()
            .any(|r| r.status == WorkingCopyIsolationStatus::Rejected
                && r.working_copy.as_ref().is_some_and(|w| w.id.as_str() == "agent-b")),
        "SM32-V5: panicked worker agent-b must yield a rejected receipt"
    );

    // Hostile Worktrunk JSON receipts attempting root "/" bind, ".." traversal, or flag branch name.
    for hostile_json in [
        r#"{"action":"created","branch":"feature/a","path":"/"}"#,
        r#"{"action":"created","branch":"feature/a","path":"/copies/../etc"}"#,
        r#"{"action":"created","branch":"--evil","path":"/copies/a"}"#,
    ] {
        let wt_driver = WorkingCopyIsolationDriver::with_runner_and_git_resolver(
            SingleOutputRunner(WorkingCopyCommandOutput::succeeded(hostile_json)),
            resolver.clone(),
        );
        let req = WorkingCopyIsolationRequest::Switch(WorkingCopySwitchRequest::new(
            "proj",
            WorkingCopyIsolationProvider::Worktrunk,
            "/repo",
            WorkingCopyHandle::new("copy-a", "/copies/a").with_branch("feature/a"),
        ));
        let receipt = wt_driver.isolate(req).await.receipt;
        assert_eq!(
            receipt.status,
            WorkingCopyIsolationStatus::Rejected,
            "SM32-V5: hostile Worktrunk receipt {hostile_json} must be rejected, got {receipt:?}"
        );
    }
}
