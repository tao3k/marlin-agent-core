//! Request envelope for working-copy isolation operations.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::{
    WorkingCopyBaseRef, WorkingCopyBranchName, WorkingCopyId, WorkingCopyIsolationOperationKind,
    WorkingCopyIsolationProvider, WorkingCopyPullRequestNumber, WorkingCopyRepositoryDiscoveryPath,
};
use crate::{WorkspaceProjectGitHubRepository, WorkspaceProjectId};

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
    pub repository_discovery_path: WorkingCopyRepositoryDiscoveryPath,
    pub working_copy: WorkingCopyHandle,
    pub base_ref: Option<WorkingCopyBaseRef>,
}

impl WorkingCopyCreateRequest {
    /// Creates a request for a provider-owned working copy.
    pub fn new(
        project_id: impl Into<WorkspaceProjectId>,
        provider: WorkingCopyIsolationProvider,
        repository_discovery_path: impl Into<WorkingCopyRepositoryDiscoveryPath>,
        working_copy: WorkingCopyHandle,
    ) -> Self {
        Self {
            project_id: project_id.into(),
            provider,
            repository_discovery_path: repository_discovery_path.into(),
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
    pub repository_discovery_path: Option<WorkingCopyRepositoryDiscoveryPath>,
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
            repository_discovery_path: None,
            repository,
            pull_request,
            working_copy: WorkingCopyHandle::new(default_id, PathBuf::new()),
        }
    }

    /// Sets the path used by native Git to discover the real repository root.
    pub fn with_repository_discovery_path(
        mut self,
        repository_discovery_path: impl Into<WorkingCopyRepositoryDiscoveryPath>,
    ) -> Self {
        self.repository_discovery_path = Some(repository_discovery_path.into());
        self
    }

    /// Sets the working-copy handle produced by the provider.
    pub fn with_working_copy(mut self, working_copy: WorkingCopyHandle) -> Self {
        self.working_copy = working_copy;
        self
    }
}

/// Request to switch to an existing or newly-created working copy.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkingCopySwitchRequest {
    pub project_id: WorkspaceProjectId,
    pub provider: WorkingCopyIsolationProvider,
    pub repository_discovery_path: WorkingCopyRepositoryDiscoveryPath,
    pub working_copy: WorkingCopyHandle,
    pub mode: WorkingCopySwitchMode,
}

/// Provider behavior when switching to a branch-addressed working copy.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum WorkingCopySwitchMode {
    #[default]
    ExistingOnly,
    CreateIfMissing {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        base_ref: Option<WorkingCopyBaseRef>,
    },
}

impl WorkingCopySwitchMode {
    /// Returns whether the provider should create the working copy if needed.
    pub fn creates_missing_working_copy(&self) -> bool {
        matches!(self, Self::CreateIfMissing { .. })
    }

    /// Returns the base ref used when creation is requested.
    pub fn base_ref(&self) -> Option<&WorkingCopyBaseRef> {
        match self {
            Self::ExistingOnly => None,
            Self::CreateIfMissing { base_ref } => base_ref.as_ref(),
        }
    }
}

impl WorkingCopySwitchRequest {
    /// Creates a provider request for a branch-addressed working copy switch.
    pub fn new(
        project_id: impl Into<WorkspaceProjectId>,
        provider: WorkingCopyIsolationProvider,
        repository_discovery_path: impl Into<WorkingCopyRepositoryDiscoveryPath>,
        working_copy: WorkingCopyHandle,
    ) -> Self {
        Self {
            project_id: project_id.into(),
            provider,
            repository_discovery_path: repository_discovery_path.into(),
            working_copy,
            mode: WorkingCopySwitchMode::ExistingOnly,
        }
    }

    /// Requests creation of the branch/worktree when the provider supports it.
    pub fn create_if_missing(mut self) -> Self {
        self.mode = WorkingCopySwitchMode::CreateIfMissing { base_ref: None };
        self
    }

    /// Sets the base revision or ref used when creation is requested.
    pub fn create_if_missing_from(mut self, base_ref: impl Into<WorkingCopyBaseRef>) -> Self {
        self.mode = WorkingCopySwitchMode::CreateIfMissing {
            base_ref: Some(base_ref.into()),
        };
        self
    }
}

/// Provider-specific list visibility options.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkingCopyListOptions {
    include_branches: bool,
    include_remotes: bool,
}

impl WorkingCopyListOptions {
    /// Creates default provider list options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Includes branch rows without materialized worktrees when supported.
    pub fn including_branches(mut self) -> Self {
        self.include_branches = true;
        self
    }

    /// Includes remote branch rows when supported.
    pub fn including_remotes(mut self) -> Self {
        self.include_remotes = true;
        self
    }

    /// Returns whether branch rows should be included.
    pub fn include_branches(&self) -> bool {
        self.include_branches
    }

    /// Returns whether remote branch rows should be included.
    pub fn include_remotes(&self) -> bool {
        self.include_remotes
    }
}

/// Request to list working copies known to a provider.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkingCopyListRequest {
    pub project_id: WorkspaceProjectId,
    pub provider: WorkingCopyIsolationProvider,
    pub repository_discovery_path: WorkingCopyRepositoryDiscoveryPath,
    pub options: WorkingCopyListOptions,
}

impl WorkingCopyListRequest {
    /// Creates a provider request for a non-mutating working-copy list.
    pub fn new(
        project_id: impl Into<WorkspaceProjectId>,
        provider: WorkingCopyIsolationProvider,
        repository_discovery_path: impl Into<WorkingCopyRepositoryDiscoveryPath>,
    ) -> Self {
        Self {
            project_id: project_id.into(),
            provider,
            repository_discovery_path: repository_discovery_path.into(),
            options: WorkingCopyListOptions::new(),
        }
    }

    /// Includes branch rows without materialized worktrees when supported.
    pub fn including_branches(mut self) -> Self {
        self.options = self.options.including_branches();
        self
    }

    /// Includes remote branch rows when supported.
    pub fn including_remotes(mut self) -> Self {
        self.options = self.options.including_remotes();
        self
    }
}

/// Request to remove a provider-owned working copy.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkingCopyRemoveRequest {
    pub project_id: WorkspaceProjectId,
    pub provider: WorkingCopyIsolationProvider,
    pub repository_discovery_path: WorkingCopyRepositoryDiscoveryPath,
    pub working_copy: WorkingCopyHandle,
    pub mode: WorkingCopyRemovalMode,
}

/// Provider behavior when removing a working copy.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum WorkingCopyRemovalMode {
    #[default]
    Normal,
    Force,
}

impl WorkingCopyRemovalMode {
    /// Returns whether the provider should force removal.
    pub fn is_force(&self) -> bool {
        matches!(self, Self::Force)
    }
}

impl WorkingCopyRemoveRequest {
    /// Creates a provider request for working-copy removal.
    pub fn new(
        project_id: impl Into<WorkspaceProjectId>,
        provider: WorkingCopyIsolationProvider,
        repository_discovery_path: impl Into<WorkingCopyRepositoryDiscoveryPath>,
        working_copy: WorkingCopyHandle,
    ) -> Self {
        Self {
            project_id: project_id.into(),
            provider,
            repository_discovery_path: repository_discovery_path.into(),
            working_copy,
            mode: WorkingCopyRemovalMode::Normal,
        }
    }

    /// Forces provider removal when dirty working trees are expected.
    pub fn forcing_removal(mut self) -> Self {
        self.mode = WorkingCopyRemovalMode::Force;
        self
    }
}

/// Working-copy isolation request envelope.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum WorkingCopyIsolationRequest {
    Create(WorkingCopyCreateRequest),
    Switch(WorkingCopySwitchRequest),
    List(WorkingCopyListRequest),
    Remove(WorkingCopyRemoveRequest),
    PullRequestCheckout(WorkingCopyPullRequestCheckoutRequest),
}

impl WorkingCopyIsolationRequest {
    /// Returns the requested operation family.
    pub fn operation_kind(&self) -> WorkingCopyIsolationOperationKind {
        match self {
            Self::Create(_) => WorkingCopyIsolationOperationKind::Create,
            Self::Switch(_) => WorkingCopyIsolationOperationKind::Switch,
            Self::List(_) => WorkingCopyIsolationOperationKind::List,
            Self::Remove(_) => WorkingCopyIsolationOperationKind::Remove,
            Self::PullRequestCheckout(_) => WorkingCopyIsolationOperationKind::PullRequestCheckout,
        }
    }

    /// Returns the requested provider.
    pub fn provider(&self) -> &WorkingCopyIsolationProvider {
        match self {
            Self::Create(request) => &request.provider,
            Self::Switch(request) => &request.provider,
            Self::List(request) => &request.provider,
            Self::Remove(request) => &request.provider,
            Self::PullRequestCheckout(request) => &request.provider,
        }
    }

    /// Returns the project id targeted by this request.
    pub fn project_id(&self) -> &WorkspaceProjectId {
        match self {
            Self::Create(request) => &request.project_id,
            Self::Switch(request) => &request.project_id,
            Self::List(request) => &request.project_id,
            Self::Remove(request) => &request.project_id,
            Self::PullRequestCheckout(request) => &request.project_id,
        }
    }

    /// Returns the requested working-copy handle.
    pub fn working_copy(&self) -> Option<&WorkingCopyHandle> {
        match self {
            Self::Create(request) => Some(&request.working_copy),
            Self::Switch(request) => Some(&request.working_copy),
            Self::List(_) => None,
            Self::Remove(request) => Some(&request.working_copy),
            Self::PullRequestCheckout(request) => Some(&request.working_copy),
        }
    }
}
