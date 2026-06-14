//! Working-copy isolation contracts for agent-owned parallel work.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{WorkspaceProjectGitHubRepository, WorkspaceProjectId};

/// Stable local name for an isolated working copy.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct WorkingCopyId(String);

impl WorkingCopyId {
    /// Creates a working-copy id from a caller-owned name.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Returns the string form used in receipts.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<&str> for WorkingCopyId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for WorkingCopyId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// Git branch name associated with a Git-backed working copy.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct WorkingCopyBranchName(String);

impl WorkingCopyBranchName {
    /// Creates a branch name newtype.
    pub fn new(branch: impl Into<String>) -> Self {
        Self(branch.into())
    }

    /// Returns the branch string.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<&str> for WorkingCopyBranchName {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for WorkingCopyBranchName {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// Base revision or ref used to create an isolated working copy.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct WorkingCopyBaseRef(String);

impl WorkingCopyBaseRef {
    /// Creates a base ref newtype.
    pub fn new(base_ref: impl Into<String>) -> Self {
        Self(base_ref.into())
    }

    /// Returns the base ref string.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<&str> for WorkingCopyBaseRef {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for WorkingCopyBaseRef {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// GitHub pull request number used by provider-level PR checkout.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct WorkingCopyPullRequestNumber(u64);

impl WorkingCopyPullRequestNumber {
    /// Creates a pull request number.
    pub fn new(number: u64) -> Self {
        Self(number)
    }

    /// Returns the raw pull request number.
    pub fn get(&self) -> u64 {
        self.0
    }
}

/// Provider used to isolate agent-owned working copies.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum WorkingCopyIsolationProvider {
    /// Rust-owned baseline over native `git worktree` semantics.
    GitWorktree,
    /// Worktrunk provider for agent-oriented Git worktree orchestration.
    Worktrunk,
}

impl WorkingCopyIsolationProvider {
    /// Returns true when the provider supports the operation family.
    pub fn supports(&self, operation: &WorkingCopyIsolationOperationKind) -> bool {
        match self {
            Self::GitWorktree => matches!(
                operation,
                WorkingCopyIsolationOperationKind::Create
                    | WorkingCopyIsolationOperationKind::Switch
                    | WorkingCopyIsolationOperationKind::List
                    | WorkingCopyIsolationOperationKind::Remove
            ),
            Self::Worktrunk => true,
        }
    }
}

/// Operation family requested from a working-copy isolation provider.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum WorkingCopyIsolationOperationKind {
    Create,
    Switch,
    List,
    Remove,
    Merge,
    PullRequestCheckout,
}

/// Common working-copy handle emitted by requests and receipts.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkingCopyHandle {
    pub id: WorkingCopyId,
    pub path: PathBuf,
    pub branch: Option<WorkingCopyBranchName>,
}

impl WorkingCopyHandle {
    /// Creates a handle for an isolated working copy path.
    pub fn new(id: impl Into<WorkingCopyId>, path: impl Into<PathBuf>) -> Self {
        Self {
            id: id.into(),
            path: path.into(),
            branch: None,
        }
    }

    /// Attaches the Git branch associated with the working copy.
    pub fn with_branch(mut self, branch: impl Into<WorkingCopyBranchName>) -> Self {
        self.branch = Some(branch.into());
        self
    }
}

/// Request to create an isolated working copy from a Git-backed project.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkingCopyCreateRequest {
    pub project_id: WorkspaceProjectId,
    pub provider: WorkingCopyIsolationProvider,
    pub repository_root: PathBuf,
    pub working_copy: WorkingCopyHandle,
    pub base_ref: Option<WorkingCopyBaseRef>,
}

impl WorkingCopyCreateRequest {
    /// Creates a request for a provider-owned working copy.
    pub fn new(
        project_id: impl Into<WorkspaceProjectId>,
        provider: WorkingCopyIsolationProvider,
        repository_root: impl Into<PathBuf>,
        working_copy: WorkingCopyHandle,
    ) -> Self {
        Self {
            project_id: project_id.into(),
            provider,
            repository_root: repository_root.into(),
            working_copy,
            base_ref: None,
        }
    }

    /// Sets the base revision or ref used to create the working copy.
    pub fn with_base_ref(mut self, base_ref: impl Into<WorkingCopyBaseRef>) -> Self {
        self.base_ref = Some(base_ref.into());
        self
    }
}

/// Request to checkout a GitHub pull request into an isolated working copy.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkingCopyPullRequestCheckoutRequest {
    pub project_id: WorkspaceProjectId,
    pub provider: WorkingCopyIsolationProvider,
    pub repository_root: Option<PathBuf>,
    pub repository: WorkspaceProjectGitHubRepository,
    pub pull_request: WorkingCopyPullRequestNumber,
    pub working_copy: WorkingCopyHandle,
}

impl WorkingCopyPullRequestCheckoutRequest {
    /// Creates a provider request for a GitHub pull request checkout.
    pub fn new(
        project_id: impl Into<WorkspaceProjectId>,
        provider: WorkingCopyIsolationProvider,
        repository: WorkspaceProjectGitHubRepository,
        pull_request: WorkingCopyPullRequestNumber,
    ) -> Self {
        let default_id = format!("pr-{}", pull_request.get());
        Self {
            project_id: project_id.into(),
            provider,
            repository_root: None,
            repository,
            pull_request,
            working_copy: WorkingCopyHandle::new(default_id, PathBuf::new()),
        }
    }

    /// Sets the local repository root used by provider command projection.
    pub fn with_repository_root(mut self, repository_root: impl Into<PathBuf>) -> Self {
        self.repository_root = Some(repository_root.into());
        self
    }

    /// Sets the working-copy handle produced by the provider.
    pub fn with_working_copy(mut self, working_copy: WorkingCopyHandle) -> Self {
        self.working_copy = working_copy;
        self
    }
}

/// Working-copy isolation request envelope.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum WorkingCopyIsolationRequest {
    Create(WorkingCopyCreateRequest),
    PullRequestCheckout(WorkingCopyPullRequestCheckoutRequest),
}

impl WorkingCopyIsolationRequest {
    /// Returns the requested operation family.
    pub fn operation_kind(&self) -> WorkingCopyIsolationOperationKind {
        match self {
            Self::Create(_) => WorkingCopyIsolationOperationKind::Create,
            Self::PullRequestCheckout(_) => WorkingCopyIsolationOperationKind::PullRequestCheckout,
        }
    }

    /// Returns the requested provider.
    pub fn provider(&self) -> &WorkingCopyIsolationProvider {
        match self {
            Self::Create(request) => &request.provider,
            Self::PullRequestCheckout(request) => &request.provider,
        }
    }

    /// Returns the project id targeted by this request.
    pub fn project_id(&self) -> &WorkspaceProjectId {
        match self {
            Self::Create(request) => &request.project_id,
            Self::PullRequestCheckout(request) => &request.project_id,
        }
    }

    /// Returns the requested working-copy handle.
    pub fn working_copy(&self) -> &WorkingCopyHandle {
        match self {
            Self::Create(request) => &request.working_copy,
            Self::PullRequestCheckout(request) => &request.working_copy,
        }
    }
}

/// Provider-specific typed plan step compiled from a working-copy request.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum WorkingCopyIsolationPlanStep {
    /// Ensure the target path is available before provider execution.
    PrepareTargetPath { path: PathBuf },
    /// Native Git worktree create operation.
    GitWorktreeCreate {
        repository_root: PathBuf,
        working_copy: WorkingCopyHandle,
        base_ref: Option<WorkingCopyBaseRef>,
    },
    /// Worktrunk create or switch operation for a branch-addressed working copy.
    WorktrunkSwitch {
        repository_root: PathBuf,
        working_copy: WorkingCopyHandle,
        create: bool,
        base_ref: Option<WorkingCopyBaseRef>,
    },
    /// Worktrunk checkout operation for a GitHub pull request.
    WorktrunkPullRequestCheckout {
        repository_root: Option<PathBuf>,
        repository: WorkspaceProjectGitHubRepository,
        pull_request: WorkingCopyPullRequestNumber,
        working_copy: WorkingCopyHandle,
    },
}

/// Typed plan consumed by runtime adapters before command execution.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkingCopyIsolationPlan {
    pub project_id: WorkspaceProjectId,
    pub provider: WorkingCopyIsolationProvider,
    pub operation: WorkingCopyIsolationOperationKind,
    pub steps: Vec<WorkingCopyIsolationPlanStep>,
}

impl WorkingCopyIsolationPlan {
    /// Compiles a request into provider-specific typed plan steps.
    pub fn compile(
        request: &WorkingCopyIsolationRequest,
    ) -> Result<Self, WorkingCopyIsolationPlanError> {
        let operation = request.operation_kind();
        let provider = request.provider().clone();
        if !provider.supports(&operation) {
            return Err(WorkingCopyIsolationPlanError::UnsupportedOperation {
                provider,
                operation,
            });
        }

        let steps = match request {
            WorkingCopyIsolationRequest::Create(request) => {
                let mut steps = vec![WorkingCopyIsolationPlanStep::PrepareTargetPath {
                    path: request.working_copy.path.clone(),
                }];
                steps.push(match request.provider {
                    WorkingCopyIsolationProvider::GitWorktree => {
                        WorkingCopyIsolationPlanStep::GitWorktreeCreate {
                            repository_root: request.repository_root.clone(),
                            working_copy: request.working_copy.clone(),
                            base_ref: request.base_ref.clone(),
                        }
                    }
                    WorkingCopyIsolationProvider::Worktrunk => {
                        WorkingCopyIsolationPlanStep::WorktrunkSwitch {
                            repository_root: request.repository_root.clone(),
                            working_copy: request.working_copy.clone(),
                            create: true,
                            base_ref: request.base_ref.clone(),
                        }
                    }
                });
                steps
            }
            WorkingCopyIsolationRequest::PullRequestCheckout(request) => {
                vec![
                    WorkingCopyIsolationPlanStep::PrepareTargetPath {
                        path: request.working_copy.path.clone(),
                    },
                    WorkingCopyIsolationPlanStep::WorktrunkPullRequestCheckout {
                        repository_root: request.repository_root.clone(),
                        repository: request.repository.clone(),
                        pull_request: request.pull_request.clone(),
                        working_copy: request.working_copy.clone(),
                    },
                ]
            }
        };

        Ok(Self {
            project_id: request.project_id().clone(),
            provider,
            operation,
            steps,
        })
    }
}

/// Error raised when a working-copy request cannot be compiled into a plan.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum WorkingCopyIsolationPlanError {
    #[error("working-copy provider {provider:?} does not support operation {operation:?}")]
    UnsupportedOperation {
        provider: WorkingCopyIsolationProvider,
        operation: WorkingCopyIsolationOperationKind,
    },
}

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
    pub cwd: PathBuf,
    pub args: Vec<String>,
    pub expected_working_copy: Option<WorkingCopyHandle>,
}

impl WorkingCopyCommandInvocation {
    /// Creates a command invocation without shell string interpretation.
    pub fn new(program: WorkingCopyCommandProgram, cwd: impl Into<PathBuf>) -> Self {
        Self {
            program,
            cwd: cwd.into(),
            args: Vec::new(),
            expected_working_copy: None,
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
    pub fn from_plan(
        plan: &WorkingCopyIsolationPlan,
    ) -> Result<Self, WorkingCopyCommandProjectionError> {
        let mut preflight_paths = Vec::new();
        let mut commands = Vec::new();

        for step in &plan.steps {
            match step {
                WorkingCopyIsolationPlanStep::PrepareTargetPath { path } => {
                    preflight_paths.push(path.clone());
                }
                WorkingCopyIsolationPlanStep::GitWorktreeCreate {
                    repository_root,
                    working_copy,
                    base_ref,
                } => {
                    let mut args = vec!["worktree".to_owned(), "add".to_owned()];
                    if let Some(branch) = working_copy.branch.as_ref() {
                        args.push("-b".to_owned());
                        args.push(branch.as_str().to_owned());
                    }
                    args.push(working_copy.path.display().to_string());
                    if let Some(base_ref) = base_ref {
                        args.push(base_ref.as_str().to_owned());
                    }
                    commands.push(
                        WorkingCopyCommandInvocation::new(
                            WorkingCopyCommandProgram::Git,
                            repository_root.clone(),
                        )
                        .with_args(args)
                        .with_expected_working_copy(working_copy.clone()),
                    );
                }
                WorkingCopyIsolationPlanStep::WorktrunkSwitch {
                    repository_root,
                    working_copy,
                    create,
                    base_ref: _,
                } => {
                    let target = working_copy
                        .branch
                        .as_ref()
                        .map(WorkingCopyBranchName::as_str)
                        .unwrap_or_else(|| working_copy.id.as_str())
                        .to_owned();
                    let mut args = vec!["switch".to_owned()];
                    if *create {
                        args.push("-c".to_owned());
                    }
                    args.push(target);
                    commands.push(
                        WorkingCopyCommandInvocation::new(
                            WorkingCopyCommandProgram::Worktrunk,
                            repository_root.clone(),
                        )
                        .with_args(args)
                        .with_expected_working_copy(working_copy.clone()),
                    );
                }
                WorkingCopyIsolationPlanStep::WorktrunkPullRequestCheckout {
                    repository_root,
                    repository: _,
                    pull_request,
                    working_copy,
                } => {
                    let repository_root = repository_root.clone().ok_or(
                        WorkingCopyCommandProjectionError::MissingRepositoryRoot {
                            provider: plan.provider.clone(),
                            operation: plan.operation.clone(),
                        },
                    )?;
                    commands.push(
                        WorkingCopyCommandInvocation::new(
                            WorkingCopyCommandProgram::Worktrunk,
                            repository_root,
                        )
                        .with_args(vec![
                            "switch".to_owned(),
                            format!("pr:{}", pull_request.get()),
                        ])
                        .with_expected_working_copy(working_copy.clone()),
                    );
                }
            }
        }

        Ok(Self {
            project_id: plan.project_id.clone(),
            provider: plan.provider.clone(),
            operation: plan.operation.clone(),
            preflight_paths,
            commands,
        })
    }
}

/// Error raised when a typed plan cannot be projected to provider commands.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum WorkingCopyCommandProjectionError {
    #[error(
        "working-copy command projection requires repository root for {provider:?} {operation:?}"
    )]
    MissingRepositoryRoot {
        provider: WorkingCopyIsolationProvider,
        operation: WorkingCopyIsolationOperationKind,
    },
}

/// Status of a working-copy isolation operation.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum WorkingCopyIsolationStatus {
    #[default]
    Planned,
    Applied,
    Rejected,
}

/// Receipt for a provider-owned working-copy isolation operation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkingCopyIsolationReceipt {
    pub project_id: WorkspaceProjectId,
    pub provider: WorkingCopyIsolationProvider,
    pub operation: WorkingCopyIsolationOperationKind,
    pub status: WorkingCopyIsolationStatus,
    pub working_copy: Option<WorkingCopyHandle>,
    pub reason: Option<String>,
}

impl WorkingCopyIsolationReceipt {
    /// Creates a planned receipt from a request.
    pub fn planned(request: &WorkingCopyIsolationRequest) -> Self {
        Self {
            project_id: request.project_id().clone(),
            provider: request.provider().clone(),
            operation: request.operation_kind(),
            status: WorkingCopyIsolationStatus::Planned,
            working_copy: Some(request.working_copy().clone()),
            reason: None,
        }
    }

    /// Creates an applied receipt from a request.
    pub fn applied(request: &WorkingCopyIsolationRequest) -> Self {
        Self {
            status: WorkingCopyIsolationStatus::Applied,
            ..Self::planned(request)
        }
    }

    /// Creates a rejected receipt from a request.
    pub fn rejected(request: &WorkingCopyIsolationRequest, reason: impl Into<String>) -> Self {
        Self {
            project_id: request.project_id().clone(),
            provider: request.provider().clone(),
            operation: request.operation_kind(),
            status: WorkingCopyIsolationStatus::Rejected,
            working_copy: Some(request.working_copy().clone()),
            reason: Some(reason.into()),
        }
    }
}
