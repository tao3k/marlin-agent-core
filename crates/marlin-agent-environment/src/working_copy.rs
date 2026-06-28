//! Executes working-copy isolation plans through injectable provider commands.

use std::{io::ErrorKind, path::Path};

use async_trait::async_trait;
use marlin_git_utils::{GitToolingError, ProcessGitTooling};
use marlin_workspace_protocol::{
    WorkingCopyCommandInvocation, WorkingCopyCommandProgram, WorkingCopyCommandProjection,
    WorkingCopyCommandProjectionError, WorkingCopyCommandReceipt, WorkingCopyCommandStatus,
    WorkingCopyGitTopLevel, WorkingCopyIsolationPlan, WorkingCopyIsolationPlanError,
    WorkingCopyIsolationProvider, WorkingCopyIsolationReceipt, WorkingCopyIsolationRequest,
    WorkingCopyParallelIsolationReceipt, WorkingCopyRepositoryDiscoveryPath, WorkspaceProjectId,
};
use thiserror::Error;
use tokio::process::Command;
use tokio::task::JoinSet;

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
        output: std::process::Output,
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
        command.arg("--version");
        command
    }

    async fn run(&self) -> Result<std::process::Output, std::io::Error> {
        let mut command = self.command();
        command.output().await
    }

    fn into_probe(
        self,
        output: std::process::Output,
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
        let root = ProcessGitTooling::resolve_repository_root(discovery_path).await?;
        Ok(WorkingCopyGitTopLevel::from_resolved_path(
            root.into_path_buf(),
        ))
    }
}

/// Production runner backed by `tokio::process::Command`.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProcessWorkingCopyCommandRunner;

impl ProcessWorkingCopyCommandRunner {
    /// Probes a provider executable without mutating the working tree.
    pub async fn probe_program(
        program: WorkingCopyCommandProgram,
    ) -> Result<WorkingCopyProviderExecutableProbe, WorkingCopyIsolationDriverError> {
        let probe_command = WorkingCopyProviderExecutableProbeCommand::new(program);
        match probe_command.run().await {
            Ok(output) => probe_command.into_probe(output),
            Err(error) if error.kind() == ErrorKind::NotFound => {
                Ok(probe_command.missing(error.to_string()))
            }
            Err(error) => Err(WorkingCopyIsolationDriverError::CommandIo {
                command: probe_command.executable().to_owned(),
                message: error.to_string(),
            }),
        }
    }
}

#[async_trait]
impl WorkingCopyCommandRunner for ProcessWorkingCopyCommandRunner {
    async fn run(
        &self,
        invocation: &WorkingCopyCommandInvocation,
    ) -> Result<WorkingCopyCommandOutput, WorkingCopyIsolationDriverError> {
        let output = Command::new(invocation.program.executable())
            .current_dir(invocation.git_toplevel.as_path())
            .args(&invocation.args)
            .output()
            .await
            .map_err(|error| WorkingCopyIsolationDriverError::CommandIo {
                command: invocation.program.executable().to_owned(),
                message: error.to_string(),
            })?;

        Ok(WorkingCopyCommandOutput {
            success: output.status.success(),
            status_code: output.status.code(),
            stdout: String::from_utf8(output.stdout).map_err(|error| {
                WorkingCopyIsolationDriverError::CommandOutput {
                    command: invocation.program.executable().to_owned(),
                    message: error.to_string(),
                }
            })?,
            stderr: String::from_utf8(output.stderr).map_err(|error| {
                WorkingCopyIsolationDriverError::CommandOutput {
                    command: invocation.program.executable().to_owned(),
                    message: error.to_string(),
                }
            })?,
        })
    }
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
        let projection = match project_commands(&plan, &self.git_resolver).await {
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
        let command_receipts =
            match execute_commands_in_order(&self.runner, &projection.commands).await {
                Ok(command_receipts) => command_receipts,
                Err((reason, command_receipts)) => {
                    return rejected(request, reason, command_receipts);
                }
            };

        WorkingCopyIsolationResult {
            receipt: WorkingCopyIsolationReceipt::applied(&request)
                .with_command_receipts(command_receipts),
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
        let mut receipts = Vec::new();

        loop {
            while tasks.len() < effective_parallelism {
                let Some(request) = pending.next() else {
                    break;
                };
                let driver = self.clone();
                tasks.spawn(async move { driver.isolate(request).await.receipt });
            }

            let Some(joined) = tasks.join_next().await else {
                break;
            };
            receipts.push(joined.expect("working-copy isolation task should not panic"));
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
    plan: &WorkingCopyIsolationPlan,
    git_resolver: &G,
) -> Result<WorkingCopyCommandProjection, WorkingCopyIsolationDriverError>
where
    G: WorkingCopyGitRepositoryResolver,
{
    let roots = ResolvedWorkingCopyGitTopLevels::resolve(plan, git_resolver).await?;
    WorkingCopyCommandProjection::from_plan(plan, |path| roots.git_toplevel_for(path))
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
    commands: &[WorkingCopyCommandInvocation],
) -> Result<Vec<WorkingCopyCommandReceipt>, (String, Vec<WorkingCopyCommandReceipt>)>
where
    R: WorkingCopyCommandRunner,
{
    let mut command_receipts = Vec::with_capacity(commands.len());
    execute_remaining_commands(runner, commands, &mut command_receipts).await?;
    Ok(command_receipts)
}

async fn execute_remaining_commands<R>(
    runner: &R,
    commands: &[WorkingCopyCommandInvocation],
    command_receipts: &mut Vec<WorkingCopyCommandReceipt>,
) -> Result<(), (String, Vec<WorkingCopyCommandReceipt>)>
where
    R: WorkingCopyCommandRunner,
{
    let Some((invocation, remaining)) = commands.split_first() else {
        return Ok(());
    };
    let output = runner
        .run(invocation)
        .await
        .map_err(|error| (error.to_string(), command_receipts.clone()))?;
    let receipt = command_receipt_from_output(invocation, output);
    let failed = receipt.status == WorkingCopyCommandStatus::Failed;
    command_receipts.push(receipt);
    if failed {
        return Err((
            "working-copy provider command failed".to_owned(),
            command_receipts.clone(),
        ));
    }

    Box::pin(execute_remaining_commands(
        runner,
        remaining,
        command_receipts,
    ))
    .await
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
    #[error(transparent)]
    Git(#[from] GitToolingError),
    #[error(transparent)]
    Plan(#[from] WorkingCopyIsolationPlanError),
    #[error(transparent)]
    Projection(#[from] WorkingCopyCommandProjectionError),
}
