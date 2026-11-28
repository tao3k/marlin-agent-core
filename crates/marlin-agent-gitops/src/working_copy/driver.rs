// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Executes working-copy isolation plans through injectable provider commands.

use std::{
    collections::HashMap,
    io::ErrorKind,
    path::{Component, Path, PathBuf},
    process::{ExitStatus, Stdio},
    time::Duration,
};

use super::{
    WorkingCopyBranchName, WorkingCopyCommandInvocation, WorkingCopyCommandProgram,
    WorkingCopyCommandProjection, WorkingCopyCommandProjectionError, WorkingCopyCommandReceipt,
    WorkingCopyCommandStatus, WorkingCopyGitTopLevel, WorkingCopyHandle,
    WorkingCopyIsolationOperationKind, WorkingCopyIsolationPlan, WorkingCopyIsolationPlanError,
    WorkingCopyIsolationPlanStep, WorkingCopyIsolationProvider, WorkingCopyIsolationReceipt,
    WorkingCopyIsolationRequest, WorkingCopyParallelIsolationReceipt,
    WorkingCopyRepositoryDiscoveryPath, WorkspaceProjectGitHubRepository, WorkspaceProjectId,
    command_projection::is_valid_branch_name,
};
use crate::{
    DISABLED_FSMONITOR, DISABLED_HOOKS, GIT_REPOSITORY_OVERRIDES, GitOperations, GitReadError,
    read::{MAX_OUTPUT_BYTES, NULL_DEVICE},
};
use async_trait::async_trait;
use serde::Deserialize;
use thiserror::Error;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::task::JoinSet;

const PROVIDER_COMMAND_TIMEOUT: Duration = Duration::from_secs(30);
const PROBE_COMMAND_TIMEOUT: Duration = Duration::from_secs(10);
const PROBE_MAX_OUTPUT_BYTES: usize = 64 * 1024;

struct BoundedCommandOutput {
    status: ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

enum BoundedCommandError {
    NotFound(std::io::Error),
    Io(std::io::Error),
    OutputLimit(usize),
    Timeout(Duration),
}

async fn read_bounded_stream<R: tokio::io::AsyncRead + Unpin>(
    stream: R,
    limit: usize,
) -> Result<Vec<u8>, std::io::Error> {
    let cap = limit.saturating_add(1);
    let mut bytes = Vec::new();
    stream.take(cap as u64).read_to_end(&mut bytes).await?;
    Ok(bytes)
}

async fn run_bounded_command(
    mut command: Command,
    timeout: Duration,
    max_output_bytes: usize,
) -> Result<BoundedCommandOutput, BoundedCommandError> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let operation = async move {
        let mut child = command.spawn().map_err(|error| {
            if error.kind() == ErrorKind::NotFound {
                BoundedCommandError::NotFound(error)
            } else {
                BoundedCommandError::Io(error)
            }
        })?;
        let stdout_pipe = child.stdout.take().ok_or_else(|| {
            BoundedCommandError::Io(std::io::Error::other("stdout pipe unavailable"))
        })?;
        let stderr_pipe = child.stderr.take().ok_or_else(|| {
            BoundedCommandError::Io(std::io::Error::other("stderr pipe unavailable"))
        })?;
        let (stdout, stderr) = tokio::try_join!(
            read_bounded_stream(stdout_pipe, max_output_bytes),
            read_bounded_stream(stderr_pipe, max_output_bytes),
        )
        .map_err(BoundedCommandError::Io)?;
        if stdout.len() > max_output_bytes || stderr.len() > max_output_bytes {
            let _ = child.kill().await;
            let _ = child.wait().await;
            return Err(BoundedCommandError::OutputLimit(max_output_bytes));
        }
        let status = child.wait().await.map_err(BoundedCommandError::Io)?;
        Ok(BoundedCommandOutput {
            status,
            stdout,
            stderr,
        })
    };
    tokio::time::timeout(timeout, operation)
        .await
        .map_err(|_| BoundedCommandError::Timeout(timeout))?
}

fn map_bounded_error(
    executable: &str,
    error: BoundedCommandError,
) -> WorkingCopyIsolationDriverError {
    match error {
        BoundedCommandError::NotFound(err) | BoundedCommandError::Io(err) => {
            WorkingCopyIsolationDriverError::CommandIo {
                command: executable.to_owned(),
                message: err.to_string(),
            }
        }
        BoundedCommandError::OutputLimit(limit) => {
            WorkingCopyIsolationDriverError::CommandOutputLimit {
                command: executable.to_owned(),
                limit,
            }
        }
        BoundedCommandError::Timeout(timeout) => WorkingCopyIsolationDriverError::CommandTimeout {
            command: executable.to_owned(),
            timeout,
        },
    }
}

/// Result of applying working-copy isolation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkingCopyIsolationResult {
    pub receipt: WorkingCopyIsolationReceipt,
}

/// Result of applying a bounded parallel working-copy fanout.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkingCopyParallelIsolationResult {
    pub receipt: WorkingCopyParallelIsolationReceipt,
}

/// Process output captured by a working-copy command runner.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkingCopyCommandOutput {
    pub success: bool,
    pub status_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

impl WorkingCopyCommandOutput {
    pub fn succeeded(stdout: impl Into<String>) -> Self {
        Self {
            success: true,
            status_code: Some(0),
            stdout: stdout.into(),
            stderr: String::new(),
        }
    }

    pub fn failed(status_code: Option<i32>, stderr: impl Into<String>) -> Self {
        Self {
            success: false,
            status_code,
            stdout: String::new(),
            stderr: stderr.into(),
        }
    }
}

/// Availability state for a provider executable.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WorkingCopyProviderExecutableStatus {
    Available,
    Missing,
    Failed,
}

/// Probe result for a provider executable.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkingCopyProviderExecutableProbe {
    pub program: WorkingCopyCommandProgram,
    pub executable: String,
    pub status: WorkingCopyProviderExecutableStatus,
    pub status_code: Option<i32>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
}

impl WorkingCopyProviderExecutableProbe {
    fn from_output(
        program: WorkingCopyCommandProgram,
        output: BoundedCommandOutput,
    ) -> Result<Self, WorkingCopyIsolationDriverError> {
        let executable = program.executable().to_owned();
        let stdout = String::from_utf8(output.stdout).map_err(|error| {
            WorkingCopyIsolationDriverError::CommandOutput {
                command: executable.clone(),
                message: error.to_string(),
            }
        })?;
        let stderr = String::from_utf8(output.stderr).map_err(|error| {
            WorkingCopyIsolationDriverError::CommandOutput {
                command: executable.clone(),
                message: error.to_string(),
            }
        })?;
        Ok(Self {
            program,
            executable,
            status: if output.status.success() {
                WorkingCopyProviderExecutableStatus::Available
            } else {
                WorkingCopyProviderExecutableStatus::Failed
            },
            status_code: output.status.code(),
            stdout: (!stdout.is_empty()).then_some(stdout),
            stderr: (!stderr.is_empty()).then_some(stderr),
        })
    }

    fn missing(program: WorkingCopyCommandProgram, message: impl Into<String>) -> Self {
        Self {
            executable: program.executable().to_owned(),
            program,
            status: WorkingCopyProviderExecutableStatus::Missing,
            status_code: None,
            stdout: None,
            stderr: Some(message.into()),
        }
    }
}

/// Process command probe boundary for provider executable availability checks.
#[derive(Clone, Debug, Eq, PartialEq)]
struct WorkingCopyProviderExecutableProbeCommand {
    program: WorkingCopyCommandProgram,
    executable: String,
}

impl WorkingCopyProviderExecutableProbeCommand {
    fn new(program: WorkingCopyCommandProgram) -> Self {
        Self {
            executable: program.executable().to_owned(),
            program,
        }
    }

    fn executable(&self) -> &str {
        self.executable.as_str()
    }

    fn command(&self) -> Command {
        let mut command = Command::new(&self.executable);
        confine_git_environment(&mut command);
        command.arg("--version");
        command
    }

    async fn run(&self) -> Result<BoundedCommandOutput, BoundedCommandError> {
        run_bounded_command(self.command(), PROBE_COMMAND_TIMEOUT, PROBE_MAX_OUTPUT_BYTES).await
    }

    fn into_probe(
        self,
        output: BoundedCommandOutput,
    ) -> Result<WorkingCopyProviderExecutableProbe, WorkingCopyIsolationDriverError> {
        WorkingCopyProviderExecutableProbe::from_output(self.program, output)
    }

    fn missing(self, message: impl Into<String>) -> WorkingCopyProviderExecutableProbe {
        WorkingCopyProviderExecutableProbe::missing(self.program, message)
    }
}

/// Runner used by the isolation driver to execute structured provider commands.
#[async_trait]
pub trait WorkingCopyCommandRunner: Send + Sync {
    async fn run(
        &self,
        invocation: &WorkingCopyCommandInvocation,
    ) -> Result<WorkingCopyCommandOutput, WorkingCopyIsolationDriverError>;
}

/// Resolves native Git semantic roots before provider command execution.
#[async_trait]
pub trait WorkingCopyGitRepositoryResolver: Send + Sync {
    async fn resolve_git_toplevel(
        &self,
        discovery_path: &Path,
    ) -> Result<WorkingCopyGitTopLevel, WorkingCopyIsolationDriverError>;

    async fn resolve_repository_worktree(
        &self,
        repository: &WorkingCopyGitTopLevel,
        target: &Path,
    ) -> Result<WorkingCopyGitTopLevel, WorkingCopyIsolationDriverError>;
}

/// Production resolver backed by `git rev-parse --show-toplevel`.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProcessWorkingCopyGitRepositoryResolver;

#[async_trait]
impl WorkingCopyGitRepositoryResolver for ProcessWorkingCopyGitRepositoryResolver {
    async fn resolve_git_toplevel(
        &self,
        discovery_path: &Path,
    ) -> Result<WorkingCopyGitTopLevel, WorkingCopyIsolationDriverError> {
        Ok(GitOperations::resolve_repository_root(discovery_path).await?)
    }

    async fn resolve_repository_worktree(
        &self,
        repository: &WorkingCopyGitTopLevel,
        target: &Path,
    ) -> Result<WorkingCopyGitTopLevel, WorkingCopyIsolationDriverError> {
        Ok(GitOperations::resolve_repository_worktree(repository, target).await?)
    }
}

/// Production runner backed by `tokio::process::Command`.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProcessWorkingCopyCommandRunner;

impl ProcessWorkingCopyCommandRunner {
    async fn verify_pull_request_repository(
        root: &WorkingCopyGitTopLevel,
        expected: &WorkspaceProjectGitHubRepository,
    ) -> Result<(), WorkingCopyIsolationDriverError> {
        let deny = || WorkingCopyIsolationDriverError::PullRequestRepositoryMismatch {
            expected: expected.slug(),
        };
        let urls = GitOperations::fetch_remote_urls(root).await?;
        if urls.is_empty() || !urls.iter().all(|url| expected.matches_fetch_url(url)) {
            return Err(deny());
        }
        let mut wt_config = provider_command("wt", root);
        wt_config.args(["config", "show", "--format", "json"]);
        let config = run_bounded_command(wt_config, PROVIDER_COMMAND_TIMEOUT, MAX_OUTPUT_BYTES)
            .await
            .map_err(|error| map_bounded_error("wt", error))?;
        if !config.status.success() {
            return Err(deny());
        }
        let config: serde_json::Value =
            serde_json::from_slice(&config.stdout).map_err(|_| deny())?;
        if !config
            .get("project")
            .is_some_and(serde_json::Value::is_object)
            || config
                .pointer("/project/config/forge/platform")
                .is_some_and(|value| value.as_str() != Some("github"))
            || config
                .pointer("/project/config/forge/hostname")
                .is_some_and(|value| value.as_str() != Some("github.com"))
        {
            return Err(deny());
        }
        let mut gh_default = provider_command("gh", root);
        gh_default
            .env("GH_HOST", "github.com")
            .env("GH_PROMPT_DISABLED", "1")
            .args(["repo", "set-default", "--view"]);
        let default = run_bounded_command(gh_default, PROVIDER_COMMAND_TIMEOUT, MAX_OUTPUT_BYTES)
            .await
            .map_err(|error| map_bounded_error("gh", error))?;
        let slug = std::str::from_utf8(&default.stdout)
            .map_err(|_| deny())?
            .trim();
        if (default.status.success() && slug.eq_ignore_ascii_case(&expected.slug()))
            || (matches!(default.status.code(), Some(0 | 1)) && slug.is_empty())
        {
            return Ok(());
        }
        Err(deny())
    }

    /// Probes a provider executable without mutating the working tree.
    pub async fn probe_program(
        program: WorkingCopyCommandProgram,
    ) -> Result<WorkingCopyProviderExecutableProbe, WorkingCopyIsolationDriverError> {
        let probe_command = WorkingCopyProviderExecutableProbeCommand::new(program);
        match probe_command.run().await {
            Ok(output) => probe_command.into_probe(output),
            Err(BoundedCommandError::NotFound(error)) => {
                Ok(probe_command.missing(error.to_string()))
            }
            Err(error) => Err(map_bounded_error(probe_command.executable(), error)),
        }
    }
}

#[async_trait]
impl WorkingCopyCommandRunner for ProcessWorkingCopyCommandRunner {
    async fn run(
        &self,
        invocation: &WorkingCopyCommandInvocation,
    ) -> Result<WorkingCopyCommandOutput, WorkingCopyIsolationDriverError> {
        let executable = invocation.program.executable();
        let mut command = provider_command(executable, &invocation.git_toplevel);
        // Enforce dual-layer hook and fsmonitor lockdown for both direct Git
        // commands and child Git processes spawned by Worktrunk.
        command.env(
            "GIT_CONFIG_PARAMETERS",
            format!("'{DISABLED_HOOKS}' '{DISABLED_FSMONITOR}'"),
        );
        if invocation.program == WorkingCopyCommandProgram::Git {
            command.args(["-c", DISABLED_HOOKS, "-c", DISABLED_FSMONITOR]);
        }
        if let Some(repository) = &invocation.github_repository {
            Self::verify_pull_request_repository(&invocation.git_toplevel, repository).await?;
            command
                .env("GH_HOST", "github.com")
                .env("GH_PROMPT_DISABLED", "1");
        }
        command.args(&invocation.args);
        let output = run_bounded_command(command, PROVIDER_COMMAND_TIMEOUT, MAX_OUTPUT_BYTES)
            .await
            .map_err(|error| map_bounded_error(executable, error))?;

        Ok(WorkingCopyCommandOutput {
            success: output.status.success(),
            status_code: output.status.code(),
            stdout: String::from_utf8(output.stdout).map_err(|error| {
                WorkingCopyIsolationDriverError::CommandOutput {
                    command: executable.to_owned(),
                    message: error.to_string(),
                }
            })?,
            stderr: String::from_utf8(output.stderr).map_err(|error| {
                WorkingCopyIsolationDriverError::CommandOutput {
                    command: executable.to_owned(),
                    message: error.to_string(),
                }
            })?,
        })
    }
}

fn provider_command(executable: &str, root: &WorkingCopyGitTopLevel) -> Command {
    let mut command = Command::new(executable);
    command.current_dir(root.as_path());
    confine_git_environment(&mut command);
    command
}

fn confine_git_environment(command: &mut Command) {
    for name in GIT_REPOSITORY_OVERRIDES {
        command.env_remove(name);
    }
    command
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", NULL_DEVICE)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env(
            "GIT_CONFIG_PARAMETERS",
            format!("'{DISABLED_HOOKS}' '{DISABLED_FSMONITOR}'"),
        );
}

/// Driver that compiles isolation requests and executes provider commands.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkingCopyIsolationDriver<
    R = ProcessWorkingCopyCommandRunner,
    G = ProcessWorkingCopyGitRepositoryResolver,
> {
    runner: R,
    git_resolver: G,
}

impl
    WorkingCopyIsolationDriver<
        ProcessWorkingCopyCommandRunner,
        ProcessWorkingCopyGitRepositoryResolver,
    >
{
    pub fn new() -> Self {
        Self {
            runner: ProcessWorkingCopyCommandRunner,
            git_resolver: ProcessWorkingCopyGitRepositoryResolver,
        }
    }
}

impl Default
    for WorkingCopyIsolationDriver<
        ProcessWorkingCopyCommandRunner,
        ProcessWorkingCopyGitRepositoryResolver,
    >
{
    fn default() -> Self {
        Self::new()
    }
}

impl<R> WorkingCopyIsolationDriver<R, ProcessWorkingCopyGitRepositoryResolver>
where
    R: WorkingCopyCommandRunner,
{
    pub fn with_runner(runner: R) -> Self {
        Self {
            runner,
            git_resolver: ProcessWorkingCopyGitRepositoryResolver,
        }
    }
}

impl<R, G> WorkingCopyIsolationDriver<R, G>
where
    R: WorkingCopyCommandRunner,
    G: WorkingCopyGitRepositoryResolver,
{
    pub fn with_runner_and_git_resolver(runner: R, git_resolver: G) -> Self {
        Self {
            runner,
            git_resolver,
        }
    }

    pub async fn isolate(
        &self,
        request: WorkingCopyIsolationRequest,
    ) -> WorkingCopyIsolationResult {
        let plan = match WorkingCopyIsolationPlan::compile(&request) {
            Ok(plan) => plan,
            Err(error) => return rejected(request, error.to_string(), []),
        };
        let projection = match project_commands(plan, &self.git_resolver).await {
            Ok(projection) => projection,
            Err(error) => return rejected(request, error.to_string(), []),
        };
        self.execute_projection(request, projection).await
    }

    async fn execute_projection(
        &self,
        request: WorkingCopyIsolationRequest,
        projection: WorkingCopyCommandProjection,
    ) -> WorkingCopyIsolationResult {
        let command_receipts = match execute_commands_in_order(&self.runner, &projection).await {
            Ok(command_receipts) => command_receipts,
            Err((reason, command_receipts)) => {
                return rejected(request, reason, command_receipts);
            }
        };

        let mut receipt = WorkingCopyIsolationReceipt::applied(&request);
        receipt.working_copy = command_receipts
            .last()
            .and_then(|command| command.working_copy.clone());
        WorkingCopyIsolationResult {
            receipt: receipt.with_command_receipts(command_receipts),
        }
    }
}

impl<R, G> WorkingCopyIsolationDriver<R, G>
where
    R: WorkingCopyCommandRunner + Clone + Send + Sync + 'static,
    G: WorkingCopyGitRepositoryResolver + Clone + Send + Sync + 'static,
{
    /// Applies multiple working-copy isolation requests with bounded fanout.
    pub async fn isolate_parallel(
        &self,
        project_id: impl Into<WorkspaceProjectId>,
        provider: WorkingCopyIsolationProvider,
        max_parallelism: usize,
        requests: impl IntoIterator<Item = WorkingCopyIsolationRequest>,
    ) -> WorkingCopyParallelIsolationResult {
        let effective_parallelism = max_parallelism.max(1);
        let mut pending = requests.into_iter();
        let mut tasks = JoinSet::new();
        let mut in_flight: HashMap<tokio::task::Id, WorkingCopyIsolationRequest> = HashMap::new();
        let mut receipts = Vec::new();

        loop {
            while tasks.len() < effective_parallelism {
                let Some(request) = pending.next() else {
                    break;
                };
                let request_meta = request.clone();
                let driver = self.clone();
                let handle = tasks.spawn(async move { driver.isolate(request).await.receipt });
                in_flight.insert(handle.id(), request_meta);
            }

            let Some(joined) = tasks.join_next_with_id().await else {
                break;
            };
            match joined {
                Ok((task_id, receipt)) => {
                    in_flight.remove(&task_id);
                    receipts.push(receipt);
                }
                Err(join_error) => {
                    let task_id = join_error.id();
                    if let Some(failed_request) = in_flight.remove(&task_id) {
                        let reason = if join_error.is_panic() {
                            "working-copy isolation worker panicked".to_owned()
                        } else {
                            format!("working-copy isolation worker aborted: {join_error}")
                        };
                        receipts.push(WorkingCopyIsolationReceipt::rejected(
                            &failed_request,
                            reason,
                        ));
                    }
                }
            }
        }

        WorkingCopyParallelIsolationResult {
            receipt: WorkingCopyParallelIsolationReceipt::from_receipts(
                project_id,
                provider,
                effective_parallelism,
                receipts,
            ),
        }
    }
}

async fn project_commands<G>(
    mut plan: WorkingCopyIsolationPlan,
    git_resolver: &G,
) -> Result<WorkingCopyCommandProjection, WorkingCopyIsolationDriverError>
where
    G: WorkingCopyGitRepositoryResolver,
{
    let roots = ResolvedWorkingCopyGitTopLevels::resolve(&plan, git_resolver).await?;
    for step in &mut plan.steps {
        if let WorkingCopyIsolationPlanStep::GitWorktreeSwitch {
            repository_discovery_path,
            working_copy,
        }
        | WorkingCopyIsolationPlanStep::GitWorktreeRemove {
            repository_discovery_path,
            working_copy,
            ..
        }
        | WorkingCopyIsolationPlanStep::WorktrunkRemove {
            repository_discovery_path,
            working_copy,
            ..
        }
        | WorkingCopyIsolationPlanStep::GitFinalizeBranch {
            repository_discovery_path,
            working_copy,
            ..
        } = step
        {
            let root = roots.git_toplevel_for(repository_discovery_path)?;
            let target = git_resolver
                .resolve_repository_worktree(&root, &working_copy.path)
                .await?;
            working_copy.path = target.as_path().to_path_buf();
        }
    }
    WorkingCopyCommandProjection::from_plan(&plan, |path| roots.git_toplevel_for(path))
        .map_err(Into::into)
}

struct ResolvedWorkingCopyGitTopLevels {
    roots: Vec<(WorkingCopyRepositoryDiscoveryPath, WorkingCopyGitTopLevel)>,
}

impl ResolvedWorkingCopyGitTopLevels {
    async fn resolve<G>(
        plan: &WorkingCopyIsolationPlan,
        git_resolver: &G,
    ) -> Result<Self, WorkingCopyIsolationDriverError>
    where
        G: WorkingCopyGitRepositoryResolver,
    {
        let mut roots = Vec::new();
        for path in plan.repository_discovery_paths() {
            let git_toplevel = git_resolver.resolve_git_toplevel(path.as_path()).await?;
            roots.push((path.clone(), git_toplevel));
        }
        Ok(Self { roots })
    }

    fn git_toplevel_for(
        &self,
        path: &WorkingCopyRepositoryDiscoveryPath,
    ) -> Result<WorkingCopyGitTopLevel, WorkingCopyCommandProjectionError> {
        self.roots
            .iter()
            .find(|(candidate, _)| candidate == path)
            .map(|(_, git_toplevel)| git_toplevel.clone())
            .ok_or_else(
                || WorkingCopyCommandProjectionError::GitTopLevelResolution {
                    path: path.as_path().to_path_buf(),
                    message: "repository discovery path was not resolved before projection"
                        .to_owned(),
                },
            )
    }
}

async fn execute_commands_in_order<R>(
    runner: &R,
    projection: &WorkingCopyCommandProjection,
) -> Result<Vec<WorkingCopyCommandReceipt>, (String, Vec<WorkingCopyCommandReceipt>)>
where
    R: WorkingCopyCommandRunner,
{
    let mut command_receipts = Vec::with_capacity(projection.commands.len());
    for invocation in &projection.commands {
        let output = match runner.run(invocation).await {
            Ok(output) => output,
            Err(error) => return Err((error.to_string(), command_receipts)),
        };
        let mut receipt = command_receipt_from_output(invocation, output);
        let result = if receipt.status == WorkingCopyCommandStatus::Failed {
            Err("working-copy provider command failed".to_owned())
        } else {
            let expected = receipt.working_copy.take();
            resolve_working_copy(&receipt, &projection.operation, expected).map(|working_copy| {
                receipt.working_copy = working_copy;
            })
        };
        command_receipts.push(receipt);
        if let Err(reason) = result {
            return Err((reason, command_receipts));
        }
    }
    Ok(command_receipts)
}

#[derive(Deserialize)]
struct WorktrunkWorktree {
    path: PathBuf,
    branch: Option<WorkingCopyBranchName>,
}

#[derive(Deserialize)]
struct WorktrunkRemoval {
    kind: String,
    #[serde(flatten)]
    worktree: WorktrunkWorktree,
}

fn resolve_working_copy(
    receipt: &WorkingCopyCommandReceipt,
    operation: &WorkingCopyIsolationOperationKind,
    expected: Option<WorkingCopyHandle>,
) -> Result<Option<WorkingCopyHandle>, String> {
    let Some(mut working_copy) = expected else {
        return Ok(None);
    };
    if receipt.program == WorkingCopyCommandProgram::Git {
        working_copy.path = receipt.git_toplevel.as_path().join(working_copy.path);
        return Ok(Some(working_copy));
    }

    let stdout = receipt.stdout.as_deref().unwrap_or_default();
    let parse_error = |error| format!("invalid Worktrunk working-copy receipt: {error}");
    let worktree = if *operation == WorkingCopyIsolationOperationKind::Remove {
        let [removed]: [WorktrunkRemoval; 1] = serde_json::from_str(stdout).map_err(parse_error)?;
        if removed.kind != "worktree" {
            return Err("Worktrunk did not report a worktree removal".to_owned());
        }
        if removed.worktree.path != working_copy.path {
            return Err("Worktrunk reported removing a different worktree".to_owned());
        }
        removed.worktree
    } else {
        let worktree: WorktrunkWorktree = serde_json::from_str(stdout).map_err(parse_error)?;
        if worktree
            .branch
            .as_ref()
            .is_none_or(|branch| branch.as_str().is_empty())
        {
            return Err("Worktrunk did not report a working-copy branch".to_owned());
        }
        worktree
    };
    if !worktree.path.is_absolute()
        || !worktree
            .path
            .components()
            .any(|component| matches!(component, Component::Normal(_)))
        || worktree
            .path
            .components()
            .any(|component| matches!(component, Component::ParentDir))
    {
        return Err("Worktrunk did not report a confined absolute working-copy path".to_owned());
    }
    if worktree
        .branch
        .as_ref()
        .is_some_and(|branch| !is_valid_branch_name(branch.as_str()))
    {
        return Err("Worktrunk reported an invalid working-copy branch".to_owned());
    }

    working_copy.path = worktree.path;
    working_copy.branch = worktree.branch;
    Ok(Some(working_copy))
}

fn command_receipt_from_output(
    invocation: &WorkingCopyCommandInvocation,
    output: WorkingCopyCommandOutput,
) -> WorkingCopyCommandReceipt {
    if output.success {
        WorkingCopyCommandReceipt::succeeded(
            invocation,
            output.status_code,
            output.stdout,
            output.stderr,
        )
    } else {
        WorkingCopyCommandReceipt::failed(
            invocation,
            output.status_code,
            output.stdout,
            output.stderr,
        )
    }
}

fn rejected(
    request: WorkingCopyIsolationRequest,
    reason: impl Into<String>,
    command_receipts: impl IntoIterator<Item = WorkingCopyCommandReceipt>,
) -> WorkingCopyIsolationResult {
    WorkingCopyIsolationResult {
        receipt: WorkingCopyIsolationReceipt::rejected(&request, reason)
            .with_command_receipts(command_receipts),
    }
}

/// Error raised by the runtime execution boundary.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum WorkingCopyIsolationDriverError {
    #[error("working-copy command {command} could not run: {message}")]
    CommandIo { command: String, message: String },
    #[error("working-copy command {command} output is not utf8: {message}")]
    CommandOutput { command: String, message: String },
    #[error("working-copy command {command} exceeded output limit of {limit} bytes")]
    CommandOutputLimit { command: String, limit: usize },
    #[error("working-copy command {command} timed out after {timeout:?}")]
    CommandTimeout { command: String, timeout: Duration },
    #[error("Worktrunk GitHub checkout could not verify the requested repository {expected}")]
    PullRequestRepositoryMismatch { expected: String },
    #[error(transparent)]
    Git(#[from] GitReadError),
    #[error(transparent)]
    Plan(#[from] WorkingCopyIsolationPlanError),
    #[error(transparent)]
    Projection(#[from] WorkingCopyCommandProjectionError),
}
