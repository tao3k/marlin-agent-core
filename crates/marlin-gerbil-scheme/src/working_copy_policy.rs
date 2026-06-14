//! Typed bridge from Scheme workspace policy into working-copy requests.

use marlin_workspace_protocol::{
    WorkingCopyBaseRef, WorkingCopyCreateRequest, WorkingCopyHandle, WorkingCopyIsolationProvider,
    WorkingCopyIsolationRequest, WorkingCopyListOptions, WorkingCopyListRequest,
    WorkingCopyPullRequestCheckoutRequest, WorkingCopyPullRequestNumber, WorkingCopyRemovalMode,
    WorkingCopyRemoveRequest, WorkingCopyRepositoryDiscoveryPath, WorkingCopySwitchMode,
    WorkingCopySwitchRequest, WorkspaceProjectGitHubRepository, WorkspaceProjectId,
};

/// Scheme-selected working-copy operation projected into Rust-owned protocol types.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GerbilWorkingCopyPolicyOperation {
    Create {
        working_copy: WorkingCopyHandle,
        base_ref: Option<WorkingCopyBaseRef>,
    },
    Switch {
        working_copy: WorkingCopyHandle,
        mode: WorkingCopySwitchMode,
    },
    List {
        options: WorkingCopyListOptions,
    },
    Remove {
        working_copy: WorkingCopyHandle,
        mode: WorkingCopyRemovalMode,
    },
    PullRequestCheckout {
        repository: WorkspaceProjectGitHubRepository,
        pull_request: WorkingCopyPullRequestNumber,
        working_copy: WorkingCopyHandle,
    },
}

/// Typed result of a Scheme working-copy policy decision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilWorkingCopyPolicySelection {
    pub project_id: WorkspaceProjectId,
    pub provider: WorkingCopyIsolationProvider,
    pub repository_discovery_path: WorkingCopyRepositoryDiscoveryPath,
    pub operation: GerbilWorkingCopyPolicyOperation,
}

impl GerbilWorkingCopyPolicySelection {
    /// Creates a typed policy selection without any text protocol boundary.
    pub fn new(
        project_id: impl Into<WorkspaceProjectId>,
        provider: WorkingCopyIsolationProvider,
        repository_discovery_path: impl Into<WorkingCopyRepositoryDiscoveryPath>,
        operation: GerbilWorkingCopyPolicyOperation,
    ) -> Self {
        Self {
            project_id: project_id.into(),
            provider,
            repository_discovery_path: repository_discovery_path.into(),
            operation,
        }
    }

    /// Projects the Scheme policy selection into the Rust working-copy protocol.
    pub fn into_working_copy_request(self) -> WorkingCopyIsolationRequest {
        let Self {
            project_id,
            provider,
            repository_discovery_path,
            operation,
        } = self;
        match operation {
            GerbilWorkingCopyPolicyOperation::Create {
                working_copy,
                base_ref,
            } => {
                let mut request = WorkingCopyCreateRequest::new(
                    project_id,
                    provider,
                    repository_discovery_path,
                    working_copy,
                );
                if let Some(base_ref) = base_ref {
                    request = request.with_base_ref(base_ref);
                }
                WorkingCopyIsolationRequest::Create(request)
            }
            GerbilWorkingCopyPolicyOperation::Switch { working_copy, mode } => {
                let mut request = WorkingCopySwitchRequest::new(
                    project_id,
                    provider,
                    repository_discovery_path,
                    working_copy,
                );
                match mode {
                    WorkingCopySwitchMode::ExistingOnly => {}
                    WorkingCopySwitchMode::CreateIfMissing { base_ref: None } => {
                        request = request.create_if_missing();
                    }
                    WorkingCopySwitchMode::CreateIfMissing {
                        base_ref: Some(base_ref),
                    } => {
                        request = request.create_if_missing_from(base_ref);
                    }
                }
                WorkingCopyIsolationRequest::Switch(request)
            }
            GerbilWorkingCopyPolicyOperation::List { options } => {
                let mut request =
                    WorkingCopyListRequest::new(project_id, provider, repository_discovery_path);
                if options.include_branches() {
                    request = request.including_branches();
                }
                if options.include_remotes() {
                    request = request.including_remotes();
                }
                WorkingCopyIsolationRequest::List(request)
            }
            GerbilWorkingCopyPolicyOperation::Remove { working_copy, mode } => {
                let mut request = WorkingCopyRemoveRequest::new(
                    project_id,
                    provider,
                    repository_discovery_path,
                    working_copy,
                );
                if mode.is_force() {
                    request = request.forcing_removal();
                }
                WorkingCopyIsolationRequest::Remove(request)
            }
            GerbilWorkingCopyPolicyOperation::PullRequestCheckout {
                repository,
                pull_request,
                working_copy,
            } => WorkingCopyIsolationRequest::PullRequestCheckout(
                WorkingCopyPullRequestCheckoutRequest::new(
                    project_id,
                    provider,
                    repository,
                    pull_request,
                )
                .with_repository_discovery_path(repository_discovery_path)
                .with_working_copy(working_copy),
            ),
        }
    }
}
