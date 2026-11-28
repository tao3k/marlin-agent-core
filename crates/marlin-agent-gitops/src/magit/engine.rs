// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Bounded, SM32-hardened Magit engine supporting colocated Jujutsu (`jj`)
//! and native Git repositories with Lambda Aitia causal provenance receipts.

use std::{
    collections::BTreeMap,
    ffi::OsString,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use thiserror::Error;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::Command,
};

use crate::{
    DISABLED_FSMONITOR, DISABLED_HOOKS, GIT_REPOSITORY_OVERRIDES, GitDiffRequest, GitDiffScope,
    GitFileState, GitOperations, GitReadError, GitRepositoryPath, GitStatusError,
};

use super::{
    aitia_client::{AitiaLiveReport, AitiaMarlinGrpcClient},
    diff_parser::{classify_structural_concern, fuse_hunk_evidence, parse_unified_diff},
    types::{
        MagitAction, MagitActionReceipt, MagitAitiaProvenance, MagitAuditConcern, MagitAuditRisk,
        MagitBackendKind, MagitChangeId, MagitCommitSha, MagitDiffHunk, MagitFileEntry,
        MagitFileSection, MagitFormalStatus, MagitJointAuditRecord, MagitLogEntry, MagitOpId,
        MagitOpLogEntry, MagitSnapshot, MagitStashEntry,
    },
};

const MAX_MAGIT_OUTPUT_BYTES: usize = 1024 * 1024;
const MAGIT_CMD_TIMEOUT: Duration = Duration::from_secs(10);
const LOG_LIMIT: usize = 20;
const OPLOG_LIMIT: usize = 12;

/// Errors returned by the Magit engine.
#[derive(Debug, Error)]
pub enum MagitError {
    #[error("repository path is not a valid git or jj workspace: {0}")]
    InvalidRepository(String),
    #[error("invalid repository-relative path: {0}")]
    InvalidPath(String),
    #[error("invalid branch or bookmark name: {0}")]
    InvalidRefName(String),
    #[error("empty commit or describe message is not allowed")]
    EmptyMessage,
    #[error("action requires a colocated jj repository")]
    RequiresJujutsu,
    #[error(transparent)]
    GitRead(#[from] GitReadError),
    #[error(transparent)]
    GitStatus(#[from] GitStatusError),
    #[error("vcs command `{program}` failed: {detail}")]
    CommandFailed { program: String, detail: String },
}

/// Authoritative Rust Magit engine bound to a workspace root.
#[derive(Clone, Debug)]
pub struct MagitEngine {
    repo_root: PathBuf,
    backend: MagitBackendKind,
    jj_program: Option<PathBuf>,
}

impl MagitEngine {
    /// Discover the repository root and detect whether Jujutsu (`jj`) is colocated.
    pub async fn discover(start_dir: &Path) -> Result<Self, MagitError> {
        let root_raw = run_git_checked(start_dir, &["rev-parse", "--show-toplevel"]).await?;
        let root_str = String::from_utf8_lossy(&root_raw).trim().to_owned();
        if root_str.is_empty() {
            return Err(MagitError::InvalidRepository(
                start_dir.display().to_string(),
            ));
        }
        let repo_root = PathBuf::from(root_str);
        let has_jj_dir = repo_root.join(".jj").is_dir();
        let jj_program = if has_jj_dir { find_jj_binary() } else { None };
        let backend = if has_jj_dir && jj_program.is_some() {
            MagitBackendKind::JjColocated
        } else {
            MagitBackendKind::GitNative
        };
        Ok(Self {
            repo_root,
            backend,
            jj_program,
        })
    }

    pub fn repo_root(&self) -> &Path {
        &self.repo_root
    }

    pub fn backend(&self) -> MagitBackendKind {
        self.backend
    }

    /// Capture a complete, deterministic Magit status snapshot using the default
    /// Lambda Aitia gRPC client (`MARLIN_AITIA_GRPC_ADDR` or managed local sidecar).
    pub async fn snapshot(&self, revision: u64) -> Result<MagitSnapshot, MagitError> {
        let aitia_client = AitiaMarlinGrpcClient::from_env();
        let report = aitia_client.audit_diff_defense(&self.repo_root, "HEAD").await;
        self.snapshot_with_evidence(revision, &report, &[]).await
    }

    /// Capture a complete Magit status snapshot fusing a caller-provided live
    /// Lambda Aitia gRPC report (`AitiaLiveReport`) and Human/LLM joint audit records.
    pub async fn snapshot_with_evidence(
        &self,
        revision: u64,
        aitia_report: &AitiaLiveReport,
        joint_records: &[MagitJointAuditRecord],
    ) -> Result<MagitSnapshot, MagitError> {
        let repo_name = self
            .repo_root
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("repository")
            .to_owned();

        let status = GitOperations::status(&self.repo_root).await?;
        let unstaged_raw = GitOperations::diff(
            &self.repo_root,
            GitDiffRequest::all(GitDiffScope::Worktree),
        )
        .await
        .map(|d| d.display_escaped())
        .unwrap_or_default();
        let staged_raw =
            GitOperations::diff(&self.repo_root, GitDiffRequest::all(GitDiffScope::Index))
                .await
                .map(|d| d.display_escaped())
                .unwrap_or_default();

        let mut unstaged_hunks = parse_unified_diff(&unstaged_raw, MagitFileSection::Unstaged);
        let mut staged_hunks = parse_unified_diff(&staged_raw, MagitFileSection::Staged);

        let formal_models = discover_formal_models(&self.repo_root);
        let fuse_all = |hunks: &mut [MagitDiffHunk], is_conflict: bool| {
            for h in hunks.iter_mut() {
                let (f_status, f_hint) = match_formal_model(&h.file_path, &formal_models);
                fuse_hunk_evidence(
                    h,
                    aitia_report,
                    f_status,
                    f_hint.as_deref(),
                    joint_records,
                    is_conflict,
                );
            }
        };

        let mut conflicts = Vec::new();
        let mut unstaged = Vec::new();
        let mut staged = Vec::new();
        let mut untracked = Vec::new();

        for entry in status.entries {
            let path = entry.path.display_escaped();
            let orig = entry.original_path.map(|p| p.display_escaped());

            if entry.index == GitFileState::Unmerged || entry.worktree == GitFileState::Unmerged {
                let mut hunks = unstaged_hunks.remove(&path).unwrap_or_default();
                fuse_all(&mut hunks, true);
                conflicts.push(build_file_entry(
                    path,
                    orig,
                    MagitFileSection::Conflicts,
                    "U".to_owned(),
                    hunks,
                    Some((
                        MagitAuditRisk::High,
                        MagitAuditConcern::SecurityBoundary,
                        "Merge/rebase conflict requires resolution".to_owned(),
                    )),
                ));
                continue;
            }
            if entry.index == GitFileState::Untracked && entry.worktree == GitFileState::Untracked {
                let concern = classify_structural_concern(&path);
                let (risk, reason) = if aitia_report.online {
                    (
                        MagitAuditRisk::Low,
                        format!("Untracked file ({}) · Aitia {}", concern.label(), aitia_report.decision),
                    )
                } else {
                    (
                        MagitAuditRisk::Unassessed,
                        format!("Untracked file ({}) · Aitia offline", concern.label()),
                    )
                };
                untracked.push(MagitFileEntry {
                    path,
                    original_path: orig,
                    section: MagitFileSection::Untracked,
                    status_code: "?".to_owned(),
                    added: 0,
                    removed: 0,
                    risk,
                    concern,
                    risk_reason: reason,
                    hunks: Vec::new(),
                });
                continue;
            }
            if entry.index != GitFileState::Unchanged && entry.index != GitFileState::Untracked {
                let mut hunks = staged_hunks.remove(&path).unwrap_or_default();
                fuse_all(&mut hunks, false);
                staged.push(build_file_entry(
                    path.clone(),
                    orig.clone(),
                    MagitFileSection::Staged,
                    entry.index.code().to_string(),
                    hunks,
                    None,
                ));
            }
            if entry.worktree != GitFileState::Unchanged
                && entry.worktree != GitFileState::Untracked
            {
                let mut hunks = unstaged_hunks.remove(&path).unwrap_or_default();
                fuse_all(&mut hunks, false);
                unstaged.push(build_file_entry(
                    path,
                    orig,
                    MagitFileSection::Unstaged,
                    entry.worktree.code().to_string(),
                    hunks,
                    None,
                ));
            }
        }

        // Priority-order files by evidence-backed risk (High -> Medium -> Unassessed -> Low -> Info).
        sort_by_risk(&mut conflicts);
        sort_by_risk(&mut unstaged);
        sort_by_risk(&mut staged);
        sort_by_risk(&mut untracked);

        let (branch, upstream, latest_tag, head_sha, head_description, ahead, behind) =
            self.read_git_head_meta().await;
        let stashes = self.read_git_stashes().await;
        let (unpushed, unpulled) = if upstream.is_some() {
            (
                self.read_git_log_range("@{upstream}..HEAD").await,
                self.read_git_log_range("HEAD..@{upstream}").await,
            )
        } else {
            (Vec::new(), Vec::new())
        };
        let (change_id, jj_desc, recent_log, op_log) = match self.backend {
            MagitBackendKind::JjColocated => self.read_jj_metadata().await,
            MagitBackendKind::GitNative => (None, None, self.read_git_log().await, Vec::new()),
        };
        let final_desc = jj_desc
            .filter(|s| !s.trim().is_empty())
            .unwrap_or(head_description);

        let (high_risk_hunks, medium_risk_hunks, total_hunks) =
            count_hunk_risks(&conflicts, &unstaged, &staged);
        let current_op = op_log.first().map(|o| o.op_id.clone());
        let evidence_id = if let Some(ref digest) = aitia_report.report_digest {
            let short = digest.strip_prefix("sha256:").unwrap_or(digest);
            let short12: String = short.chars().take(12).collect();
            format!("aitia:grpc:{repo_name}:{short12}:r{revision}")
        } else {
            format!(
                "aitia:magit:{}:{}:r{}",
                repo_name,
                change_id
                    .as_ref()
                    .map(MagitChangeId::as_str)
                    .or_else(|| head_sha.as_ref().map(MagitCommitSha::as_str))
                    .unwrap_or("unborn"),
                revision
            )
        };

        let aitia = MagitAitiaProvenance {
            evidence_id,
            repo_root: self.repo_root.display().to_string(),
            backend: self.backend,
            head_commit: head_sha.clone(),
            change_id: change_id.clone(),
            op_id: current_op,
            aitia_online: aitia_report.online,
            aitia_decision: aitia_report.decision.clone(),
            aitia_engine_owner: aitia_report.engine_owner.clone(),
            aitia_report_digest: aitia_report.report_digest.clone(),
            causal_acyclic: aitia_report.causal_acyclic,
            aitia_total_findings: aitia_report.total_findings,
            aitia_note: aitia_report.note.clone(),
            high_risk_hunks,
            medium_risk_hunks,
            total_hunks,
            conflict_count: conflicts.len() as u32,
        };

        Ok(MagitSnapshot {
            revision,
            repo_name,
            repo_root: self.repo_root.display().to_string(),
            backend: self.backend,
            branch,
            upstream,
            latest_tag,
            head_sha,
            change_id,
            head_description: final_desc,
            ahead,
            behind,
            conflicts,
            unstaged,
            staged,
            untracked,
            stashes,
            unpushed,
            unpulled,
            recent_log,
            op_log,
            aitia,
        })
    }

    /// Execute a typed local Magit mutation under SM32 environment confinement
    /// and return a causal receipt bound to Lambda Aitia provenance.
    pub async fn execute(&self, action: MagitAction) -> Result<MagitActionReceipt, MagitError> {
        let op_before = self.current_op_id().await;
        let action_label = action_name(&action);

        let summary = match action {
            MagitAction::Refresh => "Refreshed Magit status".to_owned(),
            MagitAction::StageFile { path } => {
                let rel = validate_rel_path(&path)?;
                let literal = format!(":(top,literal){}", rel.as_str());
                run_git_checked(&self.repo_root, &["add", "--", &literal]).await?;
                format!("Staged {}", rel.as_str())
            }
            MagitAction::UnstageFile { path } => {
                let rel = validate_rel_path(&path)?;
                let literal = format!(":(top,literal){}", rel.as_str());
                let res =
                    run_git_checked(&self.repo_root, &["restore", "--staged", "--", &literal])
                        .await;
                if res.is_err() {
                    // Fallback for unborn HEAD where `git restore --staged` cannot resolve HEAD.
                    run_git_checked(&self.repo_root, &["rm", "--cached", "-q", "--", &literal])
                        .await?;
                }
                format!("Unstaged {}", rel.as_str())
            }
            MagitAction::StageHunk { path, patch } => {
                let rel = validate_rel_path(&path)?;
                run_git_with_stdin(
                    &self.repo_root,
                    &["apply", "--cached", "--unidiff-zero", "-"],
                    patch.as_bytes(),
                )
                .await?;
                format!("Staged hunk in {}", rel.as_str())
            }
            MagitAction::UnstageHunk { path, patch } => {
                let rel = validate_rel_path(&path)?;
                run_git_with_stdin(
                    &self.repo_root,
                    &["apply", "--cached", "--reverse", "--unidiff-zero", "-"],
                    patch.as_bytes(),
                )
                .await?;
                format!("Unstaged hunk in {}", rel.as_str())
            }
            MagitAction::DiscardHunk { path, patch } => {
                let rel = validate_rel_path(&path)?;
                run_git_with_stdin(
                    &self.repo_root,
                    &["apply", "--reverse", "--unidiff-zero", "-"],
                    patch.as_bytes(),
                )
                .await?;
                format!("Discarded hunk in {}", rel.as_str())
            }
            MagitAction::StageAll => {
                run_git_checked(&self.repo_root, &["add", "-A"]).await?;
                "Staged all changes".to_owned()
            }
            MagitAction::UnstageAll => {
                let res = run_git_checked(&self.repo_root, &["reset", "HEAD", "--"]).await;
                if res.is_err() {
                    run_git_checked(&self.repo_root, &["rm", "--cached", "-r", "-q", "--", "."])
                        .await?;
                }
                "Unstaged all changes".to_owned()
            }
            MagitAction::DiscardFile { path } => {
                let rel = validate_rel_path(&path)?;
                let literal = format!(":(top,literal){}", rel.as_str());
                run_git_checked(&self.repo_root, &["checkout", "--", &literal]).await?;
                format!("Discarded unstaged changes in {}", rel.as_str())
            }
            MagitAction::Commit { message, amend } => {
                let msg = message.trim();
                if msg.is_empty() {
                    return Err(MagitError::EmptyMessage);
                }
                if self.backend == MagitBackendKind::JjColocated && !amend {
                    self.run_jj_checked(&["commit", "-m", msg]).await?;
                    format!("jj commit: {msg}")
                } else if amend {
                    run_git_checked(&self.repo_root, &["commit", "--amend", "-m", msg]).await?;
                    format!("Amended commit: {msg}")
                } else {
                    run_git_checked(&self.repo_root, &["commit", "-m", msg]).await?;
                    format!("Committed: {msg}")
                }
            }
            MagitAction::ExtendHead => {
                run_git_checked(&self.repo_root, &["commit", "--amend", "--no-edit"]).await?;
                "Extended HEAD commit with staged changes".to_owned()
            }
            MagitAction::JjDescribe { message } => {
                let msg = message.trim();
                if msg.is_empty() {
                    return Err(MagitError::EmptyMessage);
                }
                self.run_jj_checked(&["describe", "-m", msg]).await?;
                format!("jj describe: {msg}")
            }
            MagitAction::JjNew { message } => {
                if let Some(msg) = message.as_deref().map(str::trim).filter(|m| !m.is_empty()) {
                    self.run_jj_checked(&["new", "-m", msg]).await?;
                    format!("jj new: {msg}")
                } else {
                    self.run_jj_checked(&["new"]).await?;
                    "Created fresh jj change (@)".to_owned()
                }
            }
            MagitAction::JjSquash => {
                self.run_jj_checked(&["squash"]).await?;
                "Squashed working copy (@) into parent (@-)".to_owned()
            }
            MagitAction::JjUndo => {
                self.run_jj_checked(&["undo"]).await?;
                "Reverted last jj operation (jj undo)".to_owned()
            }
            MagitAction::CheckoutBranch { name, create } => {
                let clean = validate_ref_name(&name)?;
                if create {
                    run_git_checked(&self.repo_root, &["checkout", "-b", &clean]).await?;
                    format!("Created and switched to branch {clean}")
                } else {
                    run_git_checked(&self.repo_root, &["checkout", &clean]).await?;
                    format!("Switched to branch {clean}")
                }
            }
            MagitAction::DeleteBranch { name } => {
                let clean = validate_ref_name(&name)?;
                run_git_checked(&self.repo_root, &["branch", "-d", &clean]).await?;
                format!("Deleted branch {clean}")
            }
            MagitAction::StashPush { message } => {
                if let Some(msg) = message.as_deref().map(str::trim).filter(|m| !m.is_empty()) {
                    run_git_checked(&self.repo_root, &["stash", "push", "-u", "-m", msg]).await?;
                    format!("Stashed working tree: {msg}")
                } else {
                    run_git_checked(&self.repo_root, &["stash", "push", "-u"]).await?;
                    "Stashed working tree & untracked files".to_owned()
                }
            }
            MagitAction::StashPop => {
                run_git_checked(&self.repo_root, &["stash", "pop"]).await?;
                "Popped latest stash".to_owned()
            }
            MagitAction::StashApply => {
                run_git_checked(&self.repo_root, &["stash", "apply"]).await?;
                "Applied latest stash".to_owned()
            }
            MagitAction::StashDrop => {
                run_git_checked(&self.repo_root, &["stash", "drop"]).await?;
                "Dropped latest stash".to_owned()
            }
            MagitAction::Fetch { all } => {
                if all {
                    run_git_checked(&self.repo_root, &["fetch", "--all", "--prune"]).await?;
                    "Fetched all remotes".to_owned()
                } else {
                    run_git_checked(&self.repo_root, &["fetch", "--prune"]).await?;
                    "Fetched upstream".to_owned()
                }
            }
            MagitAction::Pull { rebase } => {
                if rebase {
                    run_git_checked(&self.repo_root, &["pull", "--rebase"]).await?;
                    "Pulled with --rebase".to_owned()
                } else {
                    run_git_checked(&self.repo_root, &["pull", "--ff-only"]).await?;
                    "Pulled (--ff-only)".to_owned()
                }
            }
            MagitAction::Push => {
                if self.backend == MagitBackendKind::JjColocated {
                    self.run_jj_checked(&["git", "push"]).await?;
                    "Pushed bookmarks via jj git push".to_owned()
                } else {
                    run_git_checked(&self.repo_root, &["push"]).await?;
                    "Pushed current branch".to_owned()
                }
            }
            MagitAction::ResetHead { mode, target } => {
                let flag = match mode.trim() {
                    "soft" => "--soft",
                    "mixed" => "--mixed",
                    other => {
                        return Err(MagitError::InvalidRefName(format!(
                            "unsupported reset mode {other:?} (only soft|mixed admitted without confirmation gate)"
                        )));
                    }
                };
                let clean = validate_ref_name(&target)?;
                run_git_checked(&self.repo_root, &["reset", flag, &clean]).await?;
                format!("Reset {flag} {clean}")
            }
            MagitAction::CherryPick { rev } => {
                let clean = validate_ref_name(&rev)?;
                run_git_checked(&self.repo_root, &["cherry-pick", &clean]).await?;
                format!("Cherry-picked {clean}")
            }
            MagitAction::RevertCommit { rev } => {
                let clean = validate_ref_name(&rev)?;
                run_git_checked(&self.repo_root, &["revert", "--no-edit", &clean]).await?;
                format!("Reverted commit {clean}")
            }
            MagitAction::RecordJointAudit {
                hunk_id,
                actor,
                verdict,
                rationale,
            } => {
                format!("Recorded {actor} audit verdict ({verdict}) on {hunk_id}: {rationale}")
            }
        };

        let op_after = self.current_op_id().await;
        let commit_sha = self.current_head_sha().await;
        let aitia_evidence_id = format!(
            "aitia:magit-act:{}:{}",
            action_label,
            op_after
                .as_ref()
                .map(MagitOpId::as_str)
                .or_else(|| commit_sha.as_ref().map(MagitCommitSha::as_str))
                .unwrap_or("local")
        );

        Ok(MagitActionReceipt {
            action: action_label.to_owned(),
            backend: self.backend,
            ok: true,
            summary,
            op_id_before: op_before,
            op_id_after: op_after,
            commit_sha,
            aitia_evidence_id,
            requires_gate: false,
        })
    }

    async fn read_git_head_meta(
        &self,
    ) -> (
        String,
        Option<String>,
        Option<String>,
        Option<MagitCommitSha>,
        String,
        u32,
        u32,
    ) {
        let branch = run_git_checked(&self.repo_root, &["symbolic-ref", "--short", "HEAD"])
            .await
            .ok()
            .map(|b| String::from_utf8_lossy(&b).trim().to_owned())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "HEAD (detached)".to_owned());

        let upstream = run_git_checked(
            &self.repo_root,
            &["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{upstream}"],
        )
        .await
        .ok()
        .map(|b| String::from_utf8_lossy(&b).trim().to_owned())
        .filter(|s| !s.is_empty());

        let latest_tag = run_git_checked(&self.repo_root, &["describe", "--tags", "--abbrev=0"])
            .await
            .ok()
            .map(|b| String::from_utf8_lossy(&b).trim().to_owned())
            .filter(|s| !s.is_empty());

        let head_sha = self.current_head_sha().await;
        let desc = run_git_checked(&self.repo_root, &["log", "-1", "--format=%s"])
            .await
            .ok()
            .map(|b| String::from_utf8_lossy(&b).trim().to_owned())
            .unwrap_or_default();

        let (ahead, behind) = run_git_checked(
            &self.repo_root,
            &["rev-list", "--left-right", "--count", "HEAD...@{upstream}"],
        )
        .await
        .ok()
        .and_then(|b| {
            let s = String::from_utf8_lossy(&b);
            let mut parts = s.split_whitespace();
            let a = parts.next()?.parse().ok()?;
            let beh = parts.next()?.parse().ok()?;
            Some((a, beh))
        })
        .unwrap_or((0, 0));

        (branch, upstream, latest_tag, head_sha, desc, ahead, behind)
    }

    async fn read_git_stashes(&self) -> Vec<MagitStashEntry> {
        let Ok(raw) = run_git_checked(
            &self.repo_root,
            &["stash", "list", "-n", "12", "--format=%gd%x00%gs%x00%cr"],
        )
        .await
        else {
            return Vec::new();
        };
        String::from_utf8_lossy(&raw)
            .lines()
            .filter_map(|line| {
                let mut parts = line.split('\0');
                let reference = parts.next()?.trim().to_owned();
                if reference.is_empty() {
                    return None;
                }
                let summary = parts.next().unwrap_or("").trim().to_owned();
                let relative_time = parts.next().unwrap_or("").trim().to_owned();
                Some(MagitStashEntry {
                    reference,
                    summary,
                    relative_time,
                })
            })
            .collect()
    }

    async fn read_git_log_range(&self, range: &str) -> Vec<MagitLogEntry> {
        let Ok(raw) = run_git_checked(
            &self.repo_root,
            &[
                "log",
                range,
                "-n",
                "12",
                "--format=%h%x00%s%x00%an%x00%cr%x00%D",
            ],
        )
        .await
        else {
            return Vec::new();
        };
        String::from_utf8_lossy(&raw)
            .lines()
            .filter_map(|line| {
                let mut parts = line.split('\0');
                let sha = parts.next()?.trim();
                if sha.is_empty() {
                    return None;
                }
                let summary = parts.next().unwrap_or("").trim().to_owned();
                let author = parts.next().unwrap_or("").trim().to_owned();
                let relative_time = parts.next().unwrap_or("").trim().to_owned();
                let refs = parts.next().unwrap_or("");
                let bookmarks = refs
                    .split(',')
                    .map(|r| r.trim().trim_start_matches("HEAD -> ").trim().to_owned())
                    .filter(|r| !r.is_empty())
                    .collect();
                Some(MagitLogEntry {
                    change_id: None,
                    commit_sha: MagitCommitSha::new(sha),
                    bookmarks,
                    summary,
                    author,
                    relative_time,
                    has_conflict: false,
                    is_working_copy: false,
                })
            })
            .collect()
    }

    async fn current_head_sha(&self) -> Option<MagitCommitSha> {
        run_git_checked(&self.repo_root, &["rev-parse", "--short=10", "HEAD"])
            .await
            .ok()
            .map(|b| String::from_utf8_lossy(&b).trim().to_owned())
            .filter(|s| !s.is_empty())
            .map(MagitCommitSha::new)
    }

    async fn current_op_id(&self) -> Option<MagitOpId> {
        if self.backend != MagitBackendKind::JjColocated {
            return None;
        }
        let out = self
            .run_jj_checked(&[
                "op",
                "log",
                "--no-graph",
                "-n",
                "1",
                "-T",
                "self.id().short(12) ++ \"\\n\"",
            ])
            .await
            .ok()?;
        let id = String::from_utf8_lossy(&out).trim().to_owned();
        if id.is_empty() {
            None
        } else {
            Some(MagitOpId::new(id))
        }
    }

    async fn read_git_log(&self) -> Vec<MagitLogEntry> {
        let limit = format!("-n{LOG_LIMIT}");
        let Ok(out) = run_git_checked(
            &self.repo_root,
            &["log", &limit, "--format=%h%x1f%D%x1f%s%x1f%an%x1f%cr"],
        )
        .await
        else {
            return Vec::new();
        };
        let text = String::from_utf8_lossy(&out);
        text.lines()
            .enumerate()
            .filter_map(|(idx, line)| {
                let mut cols = line.split('\x1f');
                let sha = cols.next()?.trim();
                if sha.is_empty() {
                    return None;
                }
                let refs = cols.next().unwrap_or("");
                let summary = cols.next().unwrap_or("").trim().to_owned();
                let author = cols.next().unwrap_or("").trim().to_owned();
                let rel = cols.next().unwrap_or("").trim().to_owned();
                let bookmarks = refs
                    .split(',')
                    .map(|r| r.trim().trim_start_matches("HEAD -> ").trim())
                    .filter(|r| !r.is_empty() && *r != "HEAD")
                    .map(ToOwned::to_owned)
                    .collect();
                Some(MagitLogEntry {
                    change_id: None,
                    commit_sha: MagitCommitSha::new(sha),
                    bookmarks,
                    summary,
                    author,
                    relative_time: rel,
                    has_conflict: false,
                    is_working_copy: idx == 0,
                })
            })
            .collect()
    }

    async fn read_jj_metadata(
        &self,
    ) -> (
        Option<MagitChangeId>,
        Option<String>,
        Vec<MagitLogEntry>,
        Vec<MagitOpLogEntry>,
    ) {
        let tpl = "change_id.short(8) ++ \"\\x1f\" ++ commit_id.short(10) ++ \"\\x1f\" ++ bookmarks ++ \"\\x1f\" ++ if(description, description.first_line(), \"(no description set)\") ++ \"\\x1f\" ++ author.name() ++ \"\\x1f\" ++ if(conflict, \"1\", \"0\") ++ \"\\x1f\" ++ if(current_working_copy, \"1\", \"0\") ++ \"\\n\"";
        let limit = LOG_LIMIT.to_string();
        let mut recent_log = Vec::new();
        let mut head_change = None;
        let mut head_desc = None;

        if let Ok(out) = self
            .run_jj_checked(&["log", "--no-graph", "-n", &limit, "-T", tpl])
            .await
        {
            for line in String::from_utf8_lossy(&out).lines() {
                let mut cols = line.split('\x1f');
                let Some(cid) = cols.next().map(str::trim).filter(|s| !s.is_empty()) else {
                    continue;
                };
                let sha = cols.next().unwrap_or("").trim();
                let bms_raw = cols.next().unwrap_or("");
                let summary = cols.next().unwrap_or("").trim().to_owned();
                let author = cols.next().unwrap_or("").trim().to_owned();
                let conflict = cols.next().unwrap_or("0").trim() == "1";
                let is_wc = cols.next().unwrap_or("0").trim() == "1";
                let bookmarks: Vec<String> = bms_raw
                    .split_whitespace()
                    .map(|s| s.trim_end_matches('*').to_owned())
                    .filter(|s| !s.is_empty())
                    .collect();
                if is_wc {
                    head_change = Some(MagitChangeId::new(cid));
                    head_desc = Some(summary.clone());
                }
                recent_log.push(MagitLogEntry {
                    change_id: Some(MagitChangeId::new(cid)),
                    commit_sha: MagitCommitSha::new(sha),
                    bookmarks,
                    summary,
                    author,
                    relative_time: if is_wc { "@".to_owned() } else { String::new() },
                    has_conflict: conflict,
                    is_working_copy: is_wc,
                });
            }
        }
        if recent_log.is_empty() {
            recent_log = self.read_git_log().await;
        }

        let op_tpl = "self.id().short(12) ++ \"\\x1f\" ++ self.description() ++ \"\\x1f\" ++ self.time().start().ago() ++ \"\\x1f\" ++ if(current_operation, \"1\", \"0\") ++ \"\\n\"";
        let op_limit = OPLOG_LIMIT.to_string();
        let mut op_log = Vec::new();
        if let Ok(out) = self
            .run_jj_checked(&["op", "log", "--no-graph", "-n", &op_limit, "-T", op_tpl])
            .await
        {
            for line in String::from_utf8_lossy(&out).lines() {
                let mut cols = line.split('\x1f');
                let Some(oid) = cols.next().map(str::trim).filter(|s| !s.is_empty()) else {
                    continue;
                };
                let desc = cols.next().unwrap_or("").trim().to_owned();
                let ago = cols.next().unwrap_or("").trim().to_owned();
                let current = cols.next().unwrap_or("0").trim() == "1";
                op_log.push(MagitOpLogEntry {
                    op_id: MagitOpId::new(oid),
                    description: desc,
                    timestamp: ago,
                    is_current: current,
                });
            }
        }

        (head_change, head_desc, recent_log, op_log)
    }

    async fn run_jj_checked(&self, args: &[&str]) -> Result<Vec<u8>, MagitError> {
        let Some(jj_bin) = &self.jj_program else {
            return Err(MagitError::RequiresJujutsu);
        };
        let mut full_args: Vec<OsString> = vec!["--no-pager".into(), "--color=never".into()];
        full_args.extend(args.iter().map(OsString::from));
        run_bounded_command(jj_bin.as_os_str(), &self.repo_root, &full_args, false).await
    }
}

fn build_file_entry(
    path: String,
    original_path: Option<String>,
    section: MagitFileSection,
    status_code: String,
    hunks: Vec<MagitDiffHunk>,
    forced_triage: Option<(MagitAuditRisk, MagitAuditConcern, String)>,
) -> MagitFileEntry {
    let added: u32 = hunks.iter().map(|h| h.added).sum();
    let removed: u32 = hunks.iter().map(|h| h.removed).sum();
    let (risk, concern, risk_reason) = if let Some(forced) = forced_triage {
        forced
    } else if let Some(top) = hunks.iter().max_by_key(|h| h.risk) {
        (top.risk, top.concern, top.risk_reason.clone())
    } else {
        let concern = classify_structural_concern(&path);
        (
            MagitAuditRisk::Unassessed,
            concern,
            format!("Structural change ({})", concern.label()),
        )
    };
    MagitFileEntry {
        path,
        original_path,
        section,
        status_code,
        added,
        removed,
        risk,
        concern,
        risk_reason,
        hunks,
    }
}

fn sort_by_risk(files: &mut [MagitFileEntry]) {
    files.sort_by(|a, b| b.risk.cmp(&a.risk).then_with(|| a.path.cmp(&b.path)));
}

fn count_hunk_risks(
    conflicts: &[MagitFileEntry],
    unstaged: &[MagitFileEntry],
    staged: &[MagitFileEntry],
) -> (u32, u32, u32) {
    let mut high = 0u32;
    let mut med = 0u32;
    let mut total = 0u32;
    for list in [conflicts, unstaged, staged] {
        for f in list {
            for h in &f.hunks {
                total += 1;
                match h.risk {
                    MagitAuditRisk::High => high += 1,
                    MagitAuditRisk::Medium => med += 1,
                    _ => {}
                }
            }
        }
    }
    (high, med, total)
}

fn validate_rel_path(path: &str) -> Result<GitRepositoryPath, MagitError> {
    GitRepositoryPath::new(path).map_err(|_| MagitError::InvalidPath(path.to_owned()))
}

fn validate_ref_name(name: &str) -> Result<String, MagitError> {
    let clean = name.trim();
    if clean.is_empty()
        || clean.starts_with('-')
        || clean.contains("..")
        || clean.contains('\0')
        || clean.chars().any(char::is_whitespace)
    {
        return Err(MagitError::InvalidRefName(name.to_owned()));
    }
    Ok(clean.to_owned())
}

fn discover_formal_models(repo_root: &Path) -> Vec<String> {
    let mut models = Vec::new();
    for rel in ["dev/quint", "dev/sysml"] {
        let dir = repo_root.join(rel);
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file()
                && let Some(name) = p.file_name().and_then(|s| s.to_str())
                && (name.ends_with(".qnt") || name.ends_with(".sysml"))
            {
                models.push(format!("{rel}/{name}"));
            }
        }
    }
    models.sort();
    models
}

fn match_formal_model(file_path: &str, models: &[String]) -> (MagitFormalStatus, Option<String>) {
    if models.is_empty() {
        return (MagitFormalStatus::None, None);
    }
    let lower = file_path.to_ascii_lowercase();
    for m in models {
        let stem = Path::new(m)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let norm_stem = stem.replace('-', "_");
        if (!stem.is_empty() && lower.contains(&stem))
            || (!norm_stem.is_empty() && lower.contains(&norm_stem))
        {
            return (MagitFormalStatus::ModelPresentUnrun, Some(m.clone()));
        }
    }
    (MagitFormalStatus::None, None)
}

fn action_name(action: &MagitAction) -> &'static str {
    match action {
        MagitAction::Refresh => "refresh",
        MagitAction::StageFile { .. } => "stage-file",
        MagitAction::UnstageFile { .. } => "unstage-file",
        MagitAction::StageHunk { .. } => "stage-hunk",
        MagitAction::UnstageHunk { .. } => "unstage-hunk",
        MagitAction::DiscardHunk { .. } => "discard-hunk",
        MagitAction::StageAll => "stage-all",
        MagitAction::UnstageAll => "unstage-all",
        MagitAction::DiscardFile { .. } => "discard-file",
        MagitAction::Commit { amend: true, .. } => "commit-amend",
        MagitAction::Commit { amend: false, .. } => "commit",
        MagitAction::ExtendHead => "commit-extend",
        MagitAction::JjDescribe { .. } => "jj-describe",
        MagitAction::JjNew { .. } => "jj-new",
        MagitAction::JjSquash => "jj-squash",
        MagitAction::JjUndo => "jj-undo",
        MagitAction::CheckoutBranch { create: true, .. } => "branch-create",
        MagitAction::CheckoutBranch { create: false, .. } => "branch-checkout",
        MagitAction::DeleteBranch { .. } => "branch-delete",
        MagitAction::StashPush { .. } => "stash-push",
        MagitAction::StashPop => "stash-pop",
        MagitAction::StashApply => "stash-apply",
        MagitAction::StashDrop => "stash-drop",
        MagitAction::Fetch { .. } => "fetch",
        MagitAction::Pull { .. } => "pull",
        MagitAction::Push => "push",
        MagitAction::ResetHead { .. } => "reset",
        MagitAction::CherryPick { .. } => "cherry-pick",
        MagitAction::RevertCommit { .. } => "revert",
        MagitAction::RecordJointAudit { .. } => "audit-record",
    }
}

fn find_jj_binary() -> Option<PathBuf> {
    if let Some(paths) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&paths) {
            let cand = dir.join(if cfg!(windows) { "jj.exe" } else { "jj" });
            if cand.is_file() {
                return Some(cand);
            }
        }
    }
    for fallback in ["/opt/homebrew/bin/jj", "/usr/local/bin/jj", "/usr/bin/jj"] {
        let cand = PathBuf::from(fallback);
        if cand.is_file() {
            return Some(cand);
        }
    }
    None
}

async fn run_git_checked(dir: &Path, args: &[&str]) -> Result<Vec<u8>, MagitError> {
    let mut full: Vec<OsString> = vec![
        "--no-pager".into(),
        "-c".into(),
        DISABLED_HOOKS.into(),
        "-c".into(),
        DISABLED_FSMONITOR.into(),
    ];
    full.extend(args.iter().map(OsString::from));
    run_bounded_command_with_stdin(std::ffi::OsStr::new("git"), dir, &full, true, None).await
}

async fn run_git_with_stdin(
    dir: &Path,
    args: &[&str],
    stdin_bytes: &[u8],
) -> Result<Vec<u8>, MagitError> {
    let mut full: Vec<OsString> = vec![
        "--no-pager".into(),
        "-c".into(),
        DISABLED_HOOKS.into(),
        "-c".into(),
        DISABLED_FSMONITOR.into(),
    ];
    full.extend(args.iter().map(OsString::from));
    run_bounded_command_with_stdin(
        std::ffi::OsStr::new("git"),
        dir,
        &full,
        true,
        Some(stdin_bytes),
    )
    .await
}

async fn run_bounded_command(
    program: &std::ffi::OsStr,
    dir: &Path,
    args: &[OsString],
    neutralize_git_env: bool,
) -> Result<Vec<u8>, MagitError> {
    run_bounded_command_with_stdin(program, dir, args, neutralize_git_env, None).await
}

async fn run_bounded_command_with_stdin(
    program: &std::ffi::OsStr,
    dir: &Path,
    args: &[OsString],
    neutralize_git_env: bool,
    stdin_bytes: Option<&[u8]>,
) -> Result<Vec<u8>, MagitError> {
    let mut cmd = Command::new(program);
    cmd.current_dir(dir)
        .args(args)
        .stdin(if stdin_bytes.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    if neutralize_git_env {
        for key in GIT_REPOSITORY_OVERRIDES {
            cmd.env_remove(key);
        }
        cmd.env("GIT_TERMINAL_PROMPT", "0");
    }

    let mut child = cmd.spawn().map_err(|err| MagitError::CommandFailed {
        program: program.to_string_lossy().into_owned(),
        detail: err.to_string(),
    })?;
    if let Some(payload) = stdin_bytes
        && let Some(mut child_stdin) = child.stdin.take()
    {
        child_stdin
            .write_all(payload)
            .await
            .map_err(|e| MagitError::CommandFailed {
                program: program.to_string_lossy().into_owned(),
                detail: format!("failed to write stdin patch: {e}"),
            })?;
        drop(child_stdin);
    }
    let stdout = child.stdout.take().ok_or_else(|| MagitError::CommandFailed {
        program: program.to_string_lossy().into_owned(),
        detail: "missing stdout pipe".to_owned(),
    })?;
    let stderr = child.stderr.take().ok_or_else(|| MagitError::CommandFailed {
        program: program.to_string_lossy().into_owned(),
        detail: "missing stderr pipe".to_owned(),
    })?;

    let io_future = async move {
        let mut out_buf = Vec::new();
        let mut err_buf = Vec::new();
        let mut out_reader = stdout.take((MAX_MAGIT_OUTPUT_BYTES + 1) as u64);
        let mut err_reader = stderr.take(64 * 1024);
        let (r_out, r_err) = tokio::join!(
            out_reader.read_to_end(&mut out_buf),
            err_reader.read_to_end(&mut err_buf),
        );
        r_out.map_err(|e| e.to_string())?;
        r_err.map_err(|e| e.to_string())?;
        let status = child.wait().await.map_err(|e| e.to_string())?;
        Ok::<_, String>((status, out_buf, err_buf))
    };

    let (status, out_buf, err_buf) = tokio::time::timeout(MAGIT_CMD_TIMEOUT, io_future)
        .await
        .map_err(|_| MagitError::CommandFailed {
            program: program.to_string_lossy().into_owned(),
            detail: "command timed out".to_owned(),
        })?
        .map_err(|detail| MagitError::CommandFailed {
            program: program.to_string_lossy().into_owned(),
            detail,
        })?;

    if out_buf.len() > MAX_MAGIT_OUTPUT_BYTES {
        return Err(MagitError::CommandFailed {
            program: program.to_string_lossy().into_owned(),
            detail: "output exceeded 1 MiB bound".to_owned(),
        });
    }
    if !status.success() {
        let err_msg = String::from_utf8_lossy(&err_buf).trim().to_owned();
        return Err(MagitError::CommandFailed {
            program: program.to_string_lossy().into_owned(),
            detail: if err_msg.is_empty() {
                format!("exit status {:?}", status.code())
            } else {
                err_msg
            },
        });
    }
    Ok(out_buf)
}

/// Convert a `MagitSnapshot` into flat `BTreeMap` facets suitable for projection
/// over `marlin.projection.v1::ProjectionQueryService`.
pub fn format_hunk_preview(hunk: &MagitDiffHunk, max_lines: usize) -> String {
    let mut out = String::new();
    out.push_str(&hunk.header);
    out.push('\n');
    for line in hunk.lines.iter().take(max_lines) {
        let prefix = match line.kind {
            super::types::MagitDiffLineKind::Added => '+',
            super::types::MagitDiffLineKind::Removed => '-',
            super::types::MagitDiffLineKind::Context => ' ',
        };
        out.push(prefix);
        out.push_str(&line.text);
        out.push('\n');
    }
    out
}

/// Summarize risk distribution by concern for AI-assisted audit headers.
pub fn summarize_concerns(snap: &MagitSnapshot) -> BTreeMap<&'static str, u32> {
    let mut counts: BTreeMap<&'static str, u32> = BTreeMap::new();
    for list in [&snap.conflicts, &snap.unstaged, &snap.staged] {
        for file in list {
            *counts.entry(file.concern.label()).or_insert(0) += 1;
        }
    }
    counts
}
