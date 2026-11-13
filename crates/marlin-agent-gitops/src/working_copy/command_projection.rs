// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Projects typed working-copy plans into provider command `argv`.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::read::DISABLED_HOOKS;

use super::{
    WorkingCopyBranchName, WorkingCopyGitTopLevel, WorkingCopyHandle,
    WorkingCopyIsolationOperationKind, WorkingCopyIsolationPlan, WorkingCopyIsolationPlanStep,
    WorkingCopyIsolationProvider, WorkingCopyRepositoryDiscoveryPath,
};
use super::{WorkspaceProjectGitHubRepository, WorkspaceProjectId};

/// Provider executable used by a working-copy command projection.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum WorkingCopyCommandProgram {
    Git,
    Worktrunk,
}

impl WorkingCopyCommandProgram {
    /// Returns the default executable name for the provider command.
    pub fn executable(&self) -> &'static str {
        match self {
            Self::Git => "git",
            Self::Worktrunk => "wt",
        }
    }
}

/// Structured command invocation produced from a provider-specific plan step.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkingCopyCommandInvocation {
    pub program: WorkingCopyCommandProgram,
    pub git_toplevel: WorkingCopyGitTopLevel,
    pub args: Vec<String>,
    pub expected_working_copy: Option<WorkingCopyHandle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub github_repository: Option<WorkspaceProjectGitHubRepository>,
}

impl WorkingCopyCommandInvocation {
    /// Creates a command invocation without shell string interpretation.
    pub fn new(program: WorkingCopyCommandProgram, git_toplevel: WorkingCopyGitTopLevel) -> Self {
        Self {
            program,
            git_toplevel,
            args: Vec::new(),
            expected_working_copy: None,
            github_repository: None,
        }
    }

    /// Adds argv arguments.
    pub fn with_args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    /// Attaches the expected working-copy handle produced by this command.
    pub fn with_expected_working_copy(mut self, working_copy: WorkingCopyHandle) -> Self {
        self.expected_working_copy = Some(working_copy);
        self
    }
}

/// Command projection consumed by runtime adapters after typed plan compilation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkingCopyCommandProjection {
    pub project_id: WorkspaceProjectId,
    pub provider: WorkingCopyIsolationProvider,
    pub operation: WorkingCopyIsolationOperationKind,
    pub preflight_paths: Vec<PathBuf>,
    pub commands: Vec<WorkingCopyCommandInvocation>,
}

impl WorkingCopyCommandProjection {
    /// Projects provider-specific plan steps into structured command invocations.
    pub fn from_plan<F>(
        plan: &WorkingCopyIsolationPlan,
        mut resolve_git_toplevel: F,
    ) -> Result<Self, WorkingCopyCommandProjectionError>
    where
        F: FnMut(
            &WorkingCopyRepositoryDiscoveryPath,
        ) -> Result<WorkingCopyGitTopLevel, WorkingCopyCommandProjectionError>,
    {
        let mut builder = WorkingCopyCommandProjectionBuilder::from_plan(plan);
        builder.project_steps(plan, &mut resolve_git_toplevel)?;
        Ok(builder.finish())
    }
}

struct WorkingCopyCommandProjectionBuilder<'a> {
    plan: &'a WorkingCopyIsolationPlan,
    preflight_paths: Vec<PathBuf>,
    commands: Vec<WorkingCopyCommandInvocation>,
}

impl<'a> WorkingCopyCommandProjectionBuilder<'a> {
    fn from_plan(plan: &'a WorkingCopyIsolationPlan) -> Self {
        Self {
            plan,
            preflight_paths: Vec::new(),
            commands: Vec::new(),
        }
    }

    fn project_steps<F>(
        &mut self,
        plan: &WorkingCopyIsolationPlan,
        resolve_git_toplevel: &mut F,
    ) -> Result<(), WorkingCopyCommandProjectionError>
    where
        F: FnMut(
            &WorkingCopyRepositoryDiscoveryPath,
        ) -> Result<WorkingCopyGitTopLevel, WorkingCopyCommandProjectionError>,
    {
        for step in &plan.steps {
            self.project_step(step, resolve_git_toplevel)?;
        }
        Ok(())
    }

    fn project_step<F>(
        &mut self,
        step: &WorkingCopyIsolationPlanStep,
        resolve_git_toplevel: &mut F,
    ) -> Result<(), WorkingCopyCommandProjectionError>
    where
        F: FnMut(
            &WorkingCopyRepositoryDiscoveryPath,
        ) -> Result<WorkingCopyGitTopLevel, WorkingCopyCommandProjectionError>,
    {
        match step {
            WorkingCopyIsolationPlanStep::PrepareTargetPath { path } => {
                self.preflight_paths.push(path.clone());
            }
            WorkingCopyIsolationPlanStep::GitWorktreeCreate {
                repository_discovery_path,
                working_copy,
                base_ref,
            } => {
                let git_toplevel = resolve_git_toplevel(repository_discovery_path)?;
                self.commands.push(git_worktree_create(
                    git_toplevel,
                    working_copy,
                    base_ref.as_ref(),
                ));
            }
            WorkingCopyIsolationPlanStep::GitWorktreeSwitch {
                repository_discovery_path,
                working_copy,
            } => {
                let git_toplevel = resolve_git_toplevel(repository_discovery_path)?;
                self.commands
                    .push(git_worktree_switch(git_toplevel, working_copy)?);
            }
            WorkingCopyIsolationPlanStep::GitWorktreeList {
                repository_discovery_path,
            } => {
                let git_toplevel = resolve_git_toplevel(repository_discovery_path)?;
                self.commands.push(git_worktree_list(git_toplevel));
            }
            WorkingCopyIsolationPlanStep::GitWorktreeRemove {
                repository_discovery_path,
                working_copy,
                mode,
            } => {
                let git_toplevel = resolve_git_toplevel(repository_discovery_path)?;
                self.commands.push(git_worktree_remove(
                    git_toplevel,
                    working_copy,
                    mode.is_force(),
                ));
            }
            WorkingCopyIsolationPlanStep::GitFinalizeBranch {
                repository_discovery_path,
                working_copy,
                branch,
            } => {
                let git_toplevel = resolve_git_toplevel(repository_discovery_path)?;
                self.commands
                    .push(git_finalize_branch(git_toplevel, working_copy, branch)?);
            }
            WorkingCopyIsolationPlanStep::WorktrunkSwitch {
                repository_discovery_path,
                working_copy,
                mode,
            } => {
                let git_toplevel = resolve_git_toplevel(repository_discovery_path)?;
                self.commands
                    .push(worktrunk_switch(git_toplevel, working_copy, mode));
            }
            WorkingCopyIsolationPlanStep::WorktrunkList {
                repository_discovery_path,
                options,
            } => {
                let git_toplevel = resolve_git_toplevel(repository_discovery_path)?;
                self.commands.push(worktrunk_list(git_toplevel, options));
            }
            WorkingCopyIsolationPlanStep::WorktrunkRemove {
                repository_discovery_path,
                working_copy,
                mode,
            } => {
                let git_toplevel = resolve_git_toplevel(repository_discovery_path)?;
                self.commands.push(worktrunk_remove(
                    git_toplevel,
                    working_copy,
                    mode.is_force(),
                )?);
            }
            WorkingCopyIsolationPlanStep::WorktrunkPullRequestCheckout {
                repository_discovery_path,
                repository,
                pull_request,
                working_copy,
            } => {
                let repository_discovery_path = repository_discovery_path.as_ref().ok_or(
                    WorkingCopyCommandProjectionError::MissingRepositoryDiscoveryPath {
                        provider: self.plan.provider.clone(),
                        operation: self.plan.operation.clone(),
                    },
                )?;
                let git_toplevel = resolve_git_toplevel(repository_discovery_path)?;
                self.commands.push(worktrunk_pull_request_checkout(
                    git_toplevel,
                    repository,
                    pull_request.get(),
                    working_copy,
                )?);
            }
        }
        Ok(())
    }

    fn finish(self) -> WorkingCopyCommandProjection {
        WorkingCopyCommandProjection {
            project_id: self.plan.project_id.clone(),
            provider: self.plan.provider.clone(),
            operation: self.plan.operation.clone(),
            preflight_paths: self.preflight_paths,
            commands: self.commands,
        }
    }
}

fn git_worktree_create(
    git_toplevel: WorkingCopyGitTopLevel,
    working_copy: &WorkingCopyHandle,
    base_ref: Option<&super::WorkingCopyBaseRef>,
) -> WorkingCopyCommandInvocation {
    let mut args = vec![
        "-c".to_owned(),
        DISABLED_HOOKS.to_owned(),
        "worktree".to_owned(),
        "add".to_owned(),
    ];
    if let Some(branch) = working_copy.branch.as_ref() {
        args.push("-b".to_owned());
        args.push(branch.as_str().to_owned());
    }
    args.push("--".to_owned());
    args.push(working_copy.path.display().to_string());
    if let Some(base_ref) = base_ref {
        args.push(base_ref.as_str().to_owned());
    }
    WorkingCopyCommandInvocation::new(WorkingCopyCommandProgram::Git, git_toplevel)
        .with_args(args)
        .with_expected_working_copy(working_copy.clone())
}

fn git_worktree_switch(
    git_toplevel: WorkingCopyGitTopLevel,
    working_copy: &WorkingCopyHandle,
) -> Result<WorkingCopyCommandInvocation, WorkingCopyCommandProjectionError> {
    Ok(
        WorkingCopyCommandInvocation::new(WorkingCopyCommandProgram::Git, git_toplevel)
            .with_args([
                "-c".to_owned(),
                DISABLED_HOOKS.to_owned(),
                "-C".to_owned(),
                git_checkout_path(working_copy)?,
                "switch".to_owned(),
                "--".to_owned(),
                branch_or_id_target(working_copy),
            ])
            .with_expected_working_copy(working_copy.clone()),
    )
}

fn git_worktree_list(git_toplevel: WorkingCopyGitTopLevel) -> WorkingCopyCommandInvocation {
    WorkingCopyCommandInvocation::new(WorkingCopyCommandProgram::Git, git_toplevel).with_args([
        "worktree".to_owned(),
        "list".to_owned(),
        "--porcelain".to_owned(),
    ])
}

fn git_worktree_remove(
    git_toplevel: WorkingCopyGitTopLevel,
    working_copy: &WorkingCopyHandle,
    force: bool,
) -> WorkingCopyCommandInvocation {
    let mut args = vec!["worktree".to_owned(), "remove".to_owned()];
    if force {
        args.push("--force".to_owned());
    }
    args.push("--".to_owned());
    args.push(working_copy.path.display().to_string());
    WorkingCopyCommandInvocation::new(WorkingCopyCommandProgram::Git, git_toplevel)
        .with_args(args)
        .with_expected_working_copy(working_copy.clone())
}

fn git_finalize_branch(
    git_toplevel: WorkingCopyGitTopLevel,
    working_copy: &WorkingCopyHandle,
    branch: &WorkingCopyBranchName,
) -> Result<WorkingCopyCommandInvocation, WorkingCopyCommandProjectionError> {
    Ok(
        WorkingCopyCommandInvocation::new(WorkingCopyCommandProgram::Git, git_toplevel)
            .with_args([
                "-c".to_owned(),
                DISABLED_HOOKS.to_owned(),
                "-C".to_owned(),
                git_checkout_path(working_copy)?,
                "switch".to_owned(),
                "-c".to_owned(),
                branch.as_str().to_owned(),
            ])
            .with_expected_working_copy(working_copy.clone()),
    )
}

fn git_checkout_path(
    working_copy: &WorkingCopyHandle,
) -> Result<String, WorkingCopyCommandProjectionError> {
    working_copy
        .path
        .to_str()
        .map(str::to_owned)
        .ok_or_else(
            || WorkingCopyCommandProjectionError::UnsupportedGitCheckoutPath {
                path: working_copy.path.clone(),
            },
        )
}

fn worktrunk_switch(
    git_toplevel: WorkingCopyGitTopLevel,
    working_copy: &WorkingCopyHandle,
    mode: &super::WorkingCopySwitchMode,
) -> WorkingCopyCommandInvocation {
    let mut args = vec![
        "switch".to_owned(),
        "--no-cd".to_owned(),
        "--no-hooks".to_owned(),
        "--format".to_owned(),
        "json".to_owned(),
    ];
    if mode.creates_missing_working_copy() {
        args.push("--create".to_owned());
        if let Some(base_ref) = mode.base_ref() {
            args.push("--base".to_owned());
            args.push(base_ref.as_str().to_owned());
        }
    }
    args.push(branch_or_id_target(working_copy));
    WorkingCopyCommandInvocation::new(WorkingCopyCommandProgram::Worktrunk, git_toplevel)
        .with_args(args)
        .with_expected_working_copy(working_copy.clone())
}

fn worktrunk_list(
    git_toplevel: WorkingCopyGitTopLevel,
    options: &super::WorkingCopyListOptions,
) -> WorkingCopyCommandInvocation {
    let mut args = vec!["list".to_owned(), "--format".to_owned(), "json".to_owned()];
    if options.include_branches() {
        args.push("--branches".to_owned());
    }
    if options.include_remotes() {
        args.push("--remotes".to_owned());
    }
    WorkingCopyCommandInvocation::new(WorkingCopyCommandProgram::Worktrunk, git_toplevel)
        .with_args(args)
}

fn worktrunk_remove(
    git_toplevel: WorkingCopyGitTopLevel,
    working_copy: &WorkingCopyHandle,
    force: bool,
) -> Result<WorkingCopyCommandInvocation, WorkingCopyCommandProjectionError> {
    if working_copy.path.as_os_str().is_empty() {
        return Err(WorkingCopyCommandProjectionError::MissingWorktrunkRemovalPath);
    }
    let path = git_toplevel.as_path().join(&working_copy.path);
    let target = path.to_str().ok_or_else(|| {
        WorkingCopyCommandProjectionError::UnsupportedWorktrunkPath { path: path.clone() }
    })?;
    let mut args = vec![
        "remove".to_owned(),
        "--foreground".to_owned(),
        "--no-hooks".to_owned(),
        "--format".to_owned(),
        "json".to_owned(),
    ];
    if force {
        args.push("--force".to_owned());
    }
    args.push("--".to_owned());
    args.push(target.to_owned());
    Ok(
        WorkingCopyCommandInvocation::new(WorkingCopyCommandProgram::Worktrunk, git_toplevel)
            .with_args(args)
            .with_expected_working_copy(working_copy.clone()),
    )
}

fn worktrunk_pull_request_checkout(
    git_toplevel: WorkingCopyGitTopLevel,
    repository: &WorkspaceProjectGitHubRepository,
    pull_request: u64,
    working_copy: &WorkingCopyHandle,
) -> Result<WorkingCopyCommandInvocation, WorkingCopyCommandProjectionError> {
    if pull_request == 0 || u32::try_from(pull_request).is_err() {
        return Err(WorkingCopyCommandProjectionError::InvalidPullRequestNumber(
            pull_request,
        ));
    }
    if !repository.is_valid() {
        return Err(WorkingCopyCommandProjectionError::InvalidGitHubRepository);
    }
    let mut command =
        WorkingCopyCommandInvocation::new(WorkingCopyCommandProgram::Worktrunk, git_toplevel)
            .with_args([
                "switch".to_owned(),
                "--no-cd".to_owned(),
                "--no-hooks".to_owned(),
                "--format".to_owned(),
                "json".to_owned(),
                format!("pr:{pull_request}"),
            ])
            .with_expected_working_copy(working_copy.clone());
    command.github_repository = Some(repository.clone());
    Ok(command)
}

fn branch_or_id_target(working_copy: &WorkingCopyHandle) -> String {
    working_copy
        .branch
        .as_ref()
        .map(WorkingCopyBranchName::as_str)
        .unwrap_or_else(|| working_copy.id.as_str())
        .to_owned()
}

/// Error raised when a typed plan cannot be projected to provider commands.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum WorkingCopyCommandProjectionError {
    #[error(
        "working-copy command projection requires repository discovery path for {provider:?} {operation:?}"
    )]
    MissingRepositoryDiscoveryPath {
        provider: WorkingCopyIsolationProvider,
        operation: WorkingCopyIsolationOperationKind,
    },
    #[error(
        "working-copy command projection could not resolve Git top-level from {path:?}: {message}"
    )]
    GitTopLevelResolution { path: PathBuf, message: String },
    #[error("Git checkout requires a UTF-8 working-copy path: {path:?}")]
    UnsupportedGitCheckoutPath { path: PathBuf },
    #[error("Worktrunk removal requires a working-copy path")]
    MissingWorktrunkRemovalPath,
    #[error("Worktrunk removal requires a UTF-8 working-copy path: {path:?}")]
    UnsupportedWorktrunkPath { path: PathBuf },
    #[error("invalid GitHub repository owner or name")]
    InvalidGitHubRepository,
    #[error("pull request number is outside Worktrunk's supported range: {0}")]
    InvalidPullRequestNumber(u64),
}
