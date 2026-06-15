//! Plan compiler for provider-specific working-copy operations.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::{
    WorkingCopyBaseRef, WorkingCopyHandle, WorkingCopyIsolationOperationKind,
    WorkingCopyIsolationProvider, WorkingCopyIsolationRequest, WorkingCopyListOptions,
    WorkingCopyPullRequestNumber, WorkingCopyRemovalMode, WorkingCopyRepositoryDiscoveryPath,
    WorkingCopySwitchMode,
};
use crate::{WorkspaceProjectGitHubRepository, WorkspaceProjectId};

/// Provider-specific typed plan step compiled from a working-copy request.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum WorkingCopyIsolationPlanStep {
    /// Ensure the target path is available before provider execution.
    PrepareTargetPath { path: PathBuf },
    /// Native Git worktree create operation.
    GitWorktreeCreate {
        repository_discovery_path: WorkingCopyRepositoryDiscoveryPath,
        working_copy: WorkingCopyHandle,
        base_ref: Option<WorkingCopyBaseRef>,
    },
    /// Native Git switch inside an existing worktree path.
    GitWorktreeSwitch {
        repository_discovery_path: WorkingCopyRepositoryDiscoveryPath,
        working_copy: WorkingCopyHandle,
    },
    /// Native Git worktree list operation.
    GitWorktreeList {
        repository_discovery_path: WorkingCopyRepositoryDiscoveryPath,
    },
    /// Native Git worktree remove operation.
    GitWorktreeRemove {
        repository_discovery_path: WorkingCopyRepositoryDiscoveryPath,
        working_copy: WorkingCopyHandle,
        mode: WorkingCopyRemovalMode,
    },
    /// Git-core branch finalization inside an active working copy.
    GitFinalizeBranch {
        repository_discovery_path: WorkingCopyRepositoryDiscoveryPath,
        working_copy: WorkingCopyHandle,
        branch: super::WorkingCopyBranchName,
    },
    /// Worktrunk create or switch operation for a branch-addressed working copy.
    WorktrunkSwitch {
        repository_discovery_path: WorkingCopyRepositoryDiscoveryPath,
        working_copy: WorkingCopyHandle,
        mode: WorkingCopySwitchMode,
    },
    /// Worktrunk list operation for agent-visible worktree state.
    WorktrunkList {
        repository_discovery_path: WorkingCopyRepositoryDiscoveryPath,
        options: WorkingCopyListOptions,
    },
    /// Worktrunk remove operation.
    WorktrunkRemove {
        repository_discovery_path: WorkingCopyRepositoryDiscoveryPath,
        working_copy: WorkingCopyHandle,
        mode: WorkingCopyRemovalMode,
    },
    /// Worktrunk checkout operation for a GitHub pull request.
    WorktrunkPullRequestCheckout {
        repository_discovery_path: Option<WorkingCopyRepositoryDiscoveryPath>,
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
    /// Returns repository discovery paths required before command projection.
    pub fn repository_discovery_paths(&self) -> Vec<&WorkingCopyRepositoryDiscoveryPath> {
        self.steps
            .iter()
            .filter_map(|step| match step {
                WorkingCopyIsolationPlanStep::PrepareTargetPath { .. } => None,
                WorkingCopyIsolationPlanStep::GitWorktreeCreate {
                    repository_discovery_path,
                    ..
                }
                | WorkingCopyIsolationPlanStep::GitWorktreeSwitch {
                    repository_discovery_path,
                    ..
                }
                | WorkingCopyIsolationPlanStep::GitWorktreeList {
                    repository_discovery_path,
                }
                | WorkingCopyIsolationPlanStep::GitWorktreeRemove {
                    repository_discovery_path,
                    ..
                }
                | WorkingCopyIsolationPlanStep::GitFinalizeBranch {
                    repository_discovery_path,
                    ..
                }
                | WorkingCopyIsolationPlanStep::WorktrunkSwitch {
                    repository_discovery_path,
                    ..
                }
                | WorkingCopyIsolationPlanStep::WorktrunkList {
                    repository_discovery_path,
                    ..
                }
                | WorkingCopyIsolationPlanStep::WorktrunkRemove {
                    repository_discovery_path,
                    ..
                } => Some(repository_discovery_path),
                WorkingCopyIsolationPlanStep::WorktrunkPullRequestCheckout {
                    repository_discovery_path,
                    ..
                } => repository_discovery_path.as_ref(),
            })
            .collect()
    }

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

        Ok(Self {
            project_id: request.project_id().clone(),
            provider,
            operation,
            steps: compile_steps(request),
        })
    }
}

fn compile_steps(request: &WorkingCopyIsolationRequest) -> Vec<WorkingCopyIsolationPlanStep> {
    match request {
        WorkingCopyIsolationRequest::Create(request) => {
            let mut steps = vec![WorkingCopyIsolationPlanStep::PrepareTargetPath {
                path: request.working_copy.path.clone(),
            }];
            steps.push(match request.provider {
                WorkingCopyIsolationProvider::GitWorktree => {
                    WorkingCopyIsolationPlanStep::GitWorktreeCreate {
                        repository_discovery_path: request.repository_discovery_path.clone(),
                        working_copy: request.working_copy.clone(),
                        base_ref: request.base_ref.clone(),
                    }
                }
                WorkingCopyIsolationProvider::Worktrunk => {
                    WorkingCopyIsolationPlanStep::WorktrunkSwitch {
                        repository_discovery_path: request.repository_discovery_path.clone(),
                        working_copy: request.working_copy.clone(),
                        mode: WorkingCopySwitchMode::CreateIfMissing {
                            base_ref: request.base_ref.clone(),
                        },
                    }
                }
            });
            steps
        }
        WorkingCopyIsolationRequest::Switch(request) => match request.provider {
            WorkingCopyIsolationProvider::GitWorktree => {
                vec![WorkingCopyIsolationPlanStep::GitWorktreeSwitch {
                    repository_discovery_path: request.repository_discovery_path.clone(),
                    working_copy: request.working_copy.clone(),
                }]
            }
            WorkingCopyIsolationProvider::Worktrunk => {
                let mut steps = Vec::new();
                if request.mode.creates_missing_working_copy() {
                    steps.push(WorkingCopyIsolationPlanStep::PrepareTargetPath {
                        path: request.working_copy.path.clone(),
                    });
                }
                steps.push(WorkingCopyIsolationPlanStep::WorktrunkSwitch {
                    repository_discovery_path: request.repository_discovery_path.clone(),
                    working_copy: request.working_copy.clone(),
                    mode: request.mode.clone(),
                });
                steps
            }
        },
        WorkingCopyIsolationRequest::List(request) => match request.provider {
            WorkingCopyIsolationProvider::GitWorktree => {
                vec![WorkingCopyIsolationPlanStep::GitWorktreeList {
                    repository_discovery_path: request.repository_discovery_path.clone(),
                }]
            }
            WorkingCopyIsolationProvider::Worktrunk => {
                vec![WorkingCopyIsolationPlanStep::WorktrunkList {
                    repository_discovery_path: request.repository_discovery_path.clone(),
                    options: request.options.clone(),
                }]
            }
        },
        WorkingCopyIsolationRequest::Remove(request) => match request.provider {
            WorkingCopyIsolationProvider::GitWorktree => {
                vec![WorkingCopyIsolationPlanStep::GitWorktreeRemove {
                    repository_discovery_path: request.repository_discovery_path.clone(),
                    working_copy: request.working_copy.clone(),
                    mode: request.mode.clone(),
                }]
            }
            WorkingCopyIsolationProvider::Worktrunk => {
                vec![WorkingCopyIsolationPlanStep::WorktrunkRemove {
                    repository_discovery_path: request.repository_discovery_path.clone(),
                    working_copy: request.working_copy.clone(),
                    mode: request.mode.clone(),
                }]
            }
        },
        WorkingCopyIsolationRequest::FinalizeBranch(request) => {
            vec![WorkingCopyIsolationPlanStep::GitFinalizeBranch {
                repository_discovery_path: request.repository_discovery_path.clone(),
                working_copy: request.working_copy.clone(),
                branch: request.branch.clone(),
            }]
        }
        WorkingCopyIsolationRequest::PullRequestCheckout(request) => {
            vec![
                WorkingCopyIsolationPlanStep::PrepareTargetPath {
                    path: request.working_copy.path.clone(),
                },
                WorkingCopyIsolationPlanStep::WorktrunkPullRequestCheckout {
                    repository_discovery_path: request.repository_discovery_path.clone(),
                    repository: request.repository.clone(),
                    pull_request: request.pull_request.clone(),
                    working_copy: request.working_copy.clone(),
                },
            ]
        }
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
