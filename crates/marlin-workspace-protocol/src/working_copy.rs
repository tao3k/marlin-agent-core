//! Working-copy isolation contracts for agent-owned parallel work.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

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
            repository,
            pull_request,
            working_copy: WorkingCopyHandle::new(default_id, PathBuf::new()),
        }
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
