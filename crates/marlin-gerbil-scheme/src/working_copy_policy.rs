// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Typed bridge from Scheme workspace policy into working-copy requests.

use gerbil_scheme::{
    CollectionKind, GerbilRuntime, IdentifierBuffer, IdentifierKind, ObjectKind,
    RootedSchemeObject, SchemeScalar,
};
use marlin_agent_gitops::working_copy::{
    WorkingCopyBaseRef, WorkingCopyCreateRequest, WorkingCopyHandle,
    WorkingCopyIsolationOperationKind, WorkingCopyIsolationProvider, WorkingCopyIsolationRequest,
    WorkingCopyListOptions, WorkingCopyListRequest, WorkingCopyPullRequestCheckoutRequest,
    WorkingCopyPullRequestNumber, WorkingCopyRemovalMode, WorkingCopyRemoveRequest,
    WorkingCopyRepositoryDiscoveryPath, WorkingCopySwitchMode, WorkingCopySwitchRequest,
    WorkspaceProjectGitHubRepository, WorkspaceProjectId,
};

/// Select a provider on the existing Gerbil owner, then keep the Rust request typed.
pub struct GerbilWorkingCopyPolicy<'runtime> {
    runtime: &'runtime GerbilRuntime,
    select: RootedSchemeObject<'runtime>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WorkingCopyProviderAvailability {
    pub git_worktree: bool,
    pub worktrunk: bool,
}

#[derive(Debug)]
pub enum GerbilWorkingCopyPolicyError {
    PackagePath,
    Bridge(gerbil_scheme::NativeError),
    Invocation(gerbil_scheme::RootedCallError),
    InvalidProvider,
}

impl From<gerbil_scheme::NativeError> for GerbilWorkingCopyPolicyError {
    fn from(error: gerbil_scheme::NativeError) -> Self {
        Self::Bridge(error)
    }
}

impl std::fmt::Display for GerbilWorkingCopyPolicyError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PackagePath => {
                formatter.write_str("working-copy policy package path is not UTF-8")
            }
            Self::Bridge(error) => write!(formatter, "working-copy Scheme bridge: {error}"),
            Self::Invocation(error) => write!(formatter, "working-copy Scheme policy: {error}"),
            Self::InvalidProvider => {
                formatter.write_str("working-copy policy returned an invalid provider")
            }
        }
    }
}

impl std::error::Error for GerbilWorkingCopyPolicyError {}

impl<'runtime> GerbilWorkingCopyPolicy<'runtime> {
    /// Resolve the Yuanxu-published selector once on the Scheme VM owner.
    pub fn load(runtime: &'runtime GerbilRuntime) -> Result<Self, GerbilWorkingCopyPolicyError> {
        let directory = crate::gerbil_package_root();
        let directory = directory
            .to_str()
            .ok_or(GerbilWorkingCopyPolicyError::PackagePath)?;
        let module = runtime.intern_identifier(
            IdentifierKind::Symbol,
            ":marlin-agent-core/working-copy/bridge",
        )?;
        let name =
            runtime.intern_identifier(IdentifierKind::Symbol, "select-working-copy-provider")?;
        let select = runtime.resolve_package_export(directory, &module, &name)?;
        if select.as_ref().kind()? != ObjectKind::Procedure {
            return Err(GerbilWorkingCopyPolicyError::InvalidProvider);
        }
        Ok(Self { runtime, select })
    }

    /// Project the Scheme decision into the existing GitOps request type.
    pub fn select_request(
        &self,
        project_id: WorkspaceProjectId,
        repository_discovery_path: WorkingCopyRepositoryDiscoveryPath,
        operation: GerbilWorkingCopyPolicyOperation,
        available: WorkingCopyProviderAvailability,
        preferred: WorkingCopyIsolationProvider,
    ) -> Result<Option<WorkingCopyIsolationRequest>, GerbilWorkingCopyPolicyError> {
        let (tag, kind) = match &operation {
            GerbilWorkingCopyPolicyOperation::Create { .. } => {
                ("create", WorkingCopyIsolationOperationKind::Create)
            }
            GerbilWorkingCopyPolicyOperation::Switch { .. } => {
                ("switch", WorkingCopyIsolationOperationKind::Switch)
            }
            GerbilWorkingCopyPolicyOperation::List { .. } => {
                ("list", WorkingCopyIsolationOperationKind::List)
            }
            GerbilWorkingCopyPolicyOperation::Remove { .. } => {
                ("remove", WorkingCopyIsolationOperationKind::Remove)
            }
            GerbilWorkingCopyPolicyOperation::PullRequestCheckout { .. } => (
                "pull-request-checkout",
                WorkingCopyIsolationOperationKind::PullRequestCheckout,
            ),
        };
        let operation_tag = self
            .runtime
            .intern_identifier(IdentifierKind::Symbol, tag)?;
        let git = self
            .runtime
            .scalar_object(SchemeScalar::Boolean(available.git_worktree))?;
        let worktrunk = self
            .runtime
            .scalar_object(SchemeScalar::Boolean(available.worktrunk))?;
        let preference = self.runtime.scalar_object(SchemeScalar::Boolean(
            preferred == WorkingCopyIsolationProvider::Worktrunk,
        ))?;
        let request = self.runtime.collection_from_objects(
            CollectionKind::Vector,
            &[&operation_tag, &git, &worktrunk, &preference],
        )?;
        let value = self
            .runtime
            .call_rooted(&self.select, &request.into_object())
            .map_err(GerbilWorkingCopyPolicyError::Invocation)?;
        let value = value.as_ref();
        match value.kind()? {
            ObjectKind::Boolean => {
                let denied = self.runtime.scalar_object(SchemeScalar::Boolean(false))?;
                return if value.is_identical(denied.as_ref()) {
                    Ok(None)
                } else {
                    Err(GerbilWorkingCopyPolicyError::InvalidProvider)
                };
            }
            ObjectKind::Symbol => {}
            _ => return Err(GerbilWorkingCopyPolicyError::InvalidProvider),
        }
        let mut names = IdentifierBuffer::default();
        value.copy_identifier_names(&mut names)?;
        let provider = match names.get(0) {
            Some((IdentifierKind::Symbol, "git-worktree")) if available.git_worktree => {
                WorkingCopyIsolationProvider::GitWorktree
            }
            Some((IdentifierKind::Symbol, "worktrunk")) if available.worktrunk => {
                WorkingCopyIsolationProvider::Worktrunk
            }
            _ => return Err(GerbilWorkingCopyPolicyError::InvalidProvider),
        };
        if names.len() != 1 || !provider.supports(&kind) {
            return Err(GerbilWorkingCopyPolicyError::InvalidProvider);
        }
        Ok(Some(
            GerbilWorkingCopyPolicySelection::new(
                project_id,
                provider,
                repository_discovery_path,
                operation,
            )
            .into_working_copy_request(),
        ))
    }
}

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
