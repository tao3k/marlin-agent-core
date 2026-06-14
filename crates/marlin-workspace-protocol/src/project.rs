//! Typed project profile contracts for Git-backed agent workspaces.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Stable identity for a repository-backed workspace project.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct WorkspaceProjectId(String);

impl WorkspaceProjectId {
    /// Creates a project id newtype from a caller-owned identifier.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Returns the string form used in receipts and project metadata.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<&str> for WorkspaceProjectId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for WorkspaceProjectId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// Trust gate for a project repository or additional root.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum WorkspaceProjectTrust {
    /// The path is approved for normal project-scoped workspace operations.
    Trusted,
    /// The path exists in the profile but must not be used without review.
    #[default]
    ReviewRequired,
    /// The path is known but explicitly denied for agent-owned mutations.
    Denied,
}

/// Rust-owned version-control backend that defines the project repository boundary.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum WorkspaceProjectVcsBackend {
    Git,
}

/// Extension policy plane for ecosystem VCS choices above the Rust core.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum WorkspaceProjectPolicyPlane {
    RustCore,
    SchemeExtension,
}

/// Ecosystem VCS extension layered above the Git core boundary.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum WorkspaceProjectVcsExtensionKind {
    Jj,
}

/// VCS extension metadata and policy routing for project-specific behavior.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceProjectVcsExtension {
    pub kind: WorkspaceProjectVcsExtensionKind,
    pub policy_plane: WorkspaceProjectPolicyPlane,
    pub metadata_dir: Option<PathBuf>,
}

impl WorkspaceProjectVcsExtension {
    /// Creates a jj extension whose complex policy is resolved by Scheme.
    pub fn jj_scheme_policy() -> Self {
        Self {
            kind: WorkspaceProjectVcsExtensionKind::Jj,
            policy_plane: WorkspaceProjectPolicyPlane::SchemeExtension,
            metadata_dir: None,
        }
    }

    /// Sets an explicit VCS extension metadata directory.
    pub fn with_metadata_dir(mut self, metadata_dir: impl Into<PathBuf>) -> Self {
        self.metadata_dir = Some(metadata_dir.into());
        self
    }

    /// Returns the metadata path for the extension.
    pub fn metadata_path(&self, worktree_root: &Path) -> PathBuf {
        self.metadata_dir
            .clone()
            .unwrap_or_else(|| worktree_root.join(self.default_metadata_name()))
    }

    /// Returns the default metadata directory name for this extension.
    pub fn default_metadata_name(&self) -> &'static str {
        match self.kind {
            WorkspaceProjectVcsExtensionKind::Jj => ".jj",
        }
    }
}

/// VCS repository or worktree that defines the project boundary.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceProjectRepository {
    pub worktree_root: PathBuf,
    pub vcs_backend: WorkspaceProjectVcsBackend,
    pub vcs_metadata_dir: Option<PathBuf>,
    pub vcs_extensions: Vec<WorkspaceProjectVcsExtension>,
    pub trust: WorkspaceProjectTrust,
}

impl WorkspaceProjectRepository {
    /// Creates a repository boundary with an explicit trust gate.
    pub fn new(
        worktree_root: impl Into<PathBuf>,
        vcs_backend: WorkspaceProjectVcsBackend,
        trust: WorkspaceProjectTrust,
    ) -> Self {
        Self {
            worktree_root: worktree_root.into(),
            vcs_backend,
            vcs_metadata_dir: None,
            vcs_extensions: Vec::new(),
            trust,
        }
    }

    /// Creates a Git repository boundary with an explicit trust gate.
    pub fn git(worktree_root: impl Into<PathBuf>, trust: WorkspaceProjectTrust) -> Self {
        Self::new(worktree_root, WorkspaceProjectVcsBackend::Git, trust)
    }

    /// Creates a trusted Git repository boundary.
    pub fn trusted_git(worktree_root: impl Into<PathBuf>) -> Self {
        Self::git(worktree_root, WorkspaceProjectTrust::Trusted)
    }

    /// Creates a Git repository boundary that requires review before use.
    pub fn review_required_git(worktree_root: impl Into<PathBuf>) -> Self {
        Self::git(worktree_root, WorkspaceProjectTrust::ReviewRequired)
    }

    /// Sets an explicit VCS metadata path for worktrees or non-standard layouts.
    pub fn with_vcs_metadata_dir(mut self, vcs_metadata_dir: impl Into<PathBuf>) -> Self {
        self.vcs_metadata_dir = Some(vcs_metadata_dir.into());
        self
    }

    /// Adds an ecosystem VCS extension above the Git core boundary.
    pub fn with_vcs_extension(mut self, extension: WorkspaceProjectVcsExtension) -> Self {
        self.vcs_extensions.push(extension);
        self
    }

    /// Returns the metadata path used to identify the backing VCS repository.
    pub fn vcs_metadata_path(&self) -> PathBuf {
        self.vcs_metadata_dir
            .clone()
            .unwrap_or_else(|| self.worktree_root.join(self.default_vcs_metadata_name()))
    }

    /// Returns the default metadata directory name for the repository backend.
    pub fn default_vcs_metadata_name(&self) -> &'static str {
        match self.vcs_backend {
            WorkspaceProjectVcsBackend::Git => ".git",
        }
    }

    /// Returns true when the repository boundary is trusted for runtime use.
    pub fn is_trusted(&self) -> bool {
        self.trust == WorkspaceProjectTrust::Trusted
    }
}

/// GitHub repository slug used by project-level remote operations.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceProjectGitHubRepository {
    pub owner: String,
    pub name: String,
}

impl WorkspaceProjectGitHubRepository {
    /// Creates a GitHub repository identifier from owner and repository name.
    pub fn new(owner: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            owner: owner.into(),
            name: name.into(),
        }
    }

    /// Returns the canonical `owner/repo` slug.
    pub fn slug(&self) -> String {
        format!("{}/{}", self.owner, self.name)
    }
}

/// GitHub operation families a project profile may authorize.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum WorkspaceProjectGitHubOperation {
    PullRequests,
    Issues,
    Checks,
    Actions,
    Releases,
}

/// GitHub operations attached to a repository-backed project.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceProjectGitHubOps {
    pub repository: WorkspaceProjectGitHubRepository,
    pub remote_name: Option<String>,
    pub operations: Vec<WorkspaceProjectGitHubOperation>,
}

impl WorkspaceProjectGitHubOps {
    /// Creates GitHub operations for one repository.
    pub fn new(repository: WorkspaceProjectGitHubRepository) -> Self {
        Self {
            repository,
            remote_name: None,
            operations: Vec::new(),
        }
    }

    /// Sets the local VCS remote name backing these GitHub operations.
    pub fn with_remote_name(mut self, remote_name: impl Into<String>) -> Self {
        self.remote_name = Some(remote_name.into());
        self
    }

    /// Adds one authorized GitHub operation family.
    pub fn with_operation(mut self, operation: WorkspaceProjectGitHubOperation) -> Self {
        self.operations.push(operation);
        self
    }

    /// Returns true when this project authorizes the operation family.
    pub fn supports(&self, operation: &WorkspaceProjectGitHubOperation) -> bool {
        self.operations
            .iter()
            .any(|candidate| candidate == operation)
    }
}

/// Additional root directory attached to a repository-backed workspace project.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceProjectRoot {
    pub path: PathBuf,
    pub trust: WorkspaceProjectTrust,
}

impl WorkspaceProjectRoot {
    /// Creates a root with an explicit trust gate.
    pub fn new(path: impl Into<PathBuf>, trust: WorkspaceProjectTrust) -> Self {
        Self {
            path: path.into(),
            trust,
        }
    }

    /// Creates a trusted project root.
    pub fn trusted(path: impl Into<PathBuf>) -> Self {
        Self::new(path, WorkspaceProjectTrust::Trusted)
    }

    /// Creates a root that requires explicit review before use.
    pub fn review_required(path: impl Into<PathBuf>) -> Self {
        Self::new(path, WorkspaceProjectTrust::ReviewRequired)
    }

    /// Creates a denied project root.
    pub fn denied(path: impl Into<PathBuf>) -> Self {
        Self::new(path, WorkspaceProjectTrust::Denied)
    }

    /// Returns true when the root is trusted for runtime use.
    pub fn is_trusted(&self) -> bool {
        self.trust == WorkspaceProjectTrust::Trusted
    }
}

/// Persistence policy for project-owned workspace state.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum WorkspaceProjectPersistence {
    /// No durable project state is written.
    #[default]
    Ephemeral,
    /// Durable project state is written under a repository-local state directory.
    RepositoryStateDir { state_dir: PathBuf },
}

impl WorkspaceProjectPersistence {
    /// Creates a repository-local state directory persistence policy.
    pub fn repository_state_dir(state_dir: impl Into<PathBuf>) -> Self {
        Self::RepositoryStateDir {
            state_dir: state_dir.into(),
        }
    }

    /// Returns the directory used for durable project state, when present.
    pub fn state_dir(&self) -> Option<&Path> {
        match self {
            Self::Ephemeral => None,
            Self::RepositoryStateDir { state_dir } => Some(state_dir.as_path()),
        }
    }

    /// Returns whether this policy writes durable project state.
    pub fn is_durable(&self) -> bool {
        !matches!(self, Self::Ephemeral)
    }
}

/// Durable project profile attached to an agent workspace.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceProjectProfile {
    pub id: WorkspaceProjectId,
    pub display_name: Option<String>,
    pub repository: WorkspaceProjectRepository,
    pub additional_roots: Vec<WorkspaceProjectRoot>,
    pub github_ops: Option<WorkspaceProjectGitHubOps>,
    pub org_metadata_file: Option<PathBuf>,
    pub persistence: WorkspaceProjectPersistence,
}

impl WorkspaceProjectProfile {
    /// Creates a project profile from the backing Git repository boundary.
    pub fn new(id: impl Into<WorkspaceProjectId>, repository: WorkspaceProjectRepository) -> Self {
        Self {
            id: id.into(),
            display_name: None,
            repository,
            additional_roots: Vec::new(),
            github_ops: None,
            org_metadata_file: None,
            persistence: WorkspaceProjectPersistence::Ephemeral,
        }
    }

    /// Sets the human-facing display name.
    pub fn with_display_name(mut self, display_name: impl Into<String>) -> Self {
        self.display_name = Some(display_name.into());
        self
    }

    /// Sets the Org metadata file used for project notes and workspace memory.
    pub fn with_org_metadata_file(mut self, file: impl Into<PathBuf>) -> Self {
        self.org_metadata_file = Some(file.into());
        self
    }

    /// Adds an additional project root.
    pub fn with_additional_root(mut self, root: WorkspaceProjectRoot) -> Self {
        self.additional_roots.push(root);
        self
    }

    /// Attaches GitHub operations to the project profile.
    pub fn with_github_ops(mut self, github_ops: WorkspaceProjectGitHubOps) -> Self {
        self.github_ops = Some(github_ops);
        self
    }

    /// Sets the durable persistence policy.
    pub fn with_persistence(mut self, persistence: WorkspaceProjectPersistence) -> Self {
        self.persistence = persistence;
        self
    }

    /// Returns the repository worktree root that defines the project boundary.
    pub fn repository_root(&self) -> &Path {
        self.repository.worktree_root.as_path()
    }

    /// Iterates additional roots trusted for runtime use.
    pub fn trusted_additional_roots(&self) -> impl Iterator<Item = &WorkspaceProjectRoot> {
        self.additional_roots
            .iter()
            .filter(|root| root.is_trusted())
    }

    /// Returns true when the profile has a durable persistence target.
    pub fn is_durable(&self) -> bool {
        self.persistence.is_durable()
    }
}
