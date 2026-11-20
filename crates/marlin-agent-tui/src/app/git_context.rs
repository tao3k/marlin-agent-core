// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Worktree I/O runs on the managed host. The terminal thread owns admission.

use std::path::{Path, PathBuf};

use gerbil_scheme::{CollectionKind, GerbilRuntime, ObjectKind, RootedSchemeObject};
use marlin_agent_gitops::{
    CommittedCheckout, ContextDocument, GitOperations,
    working_copy::{
        WorkingCopyActiveBinding, WorkingCopyBranchName, WorkingCopyHandle,
        WorkingCopyIsolationDriver, WorkingCopyIsolationProvider, WorkingCopyIsolationRequest,
        WorkingCopySwitchRequest,
    },
};
use marlin_agent_runtime::RuntimeTask;
use tokio::runtime::Runtime;

use super::{App, MessageEvent, MessageSeverity, MessageSource};
use crate::linked_ui::{WorktreeOperation, worktree_call};

type Completion<T> = RuntimeTask<Result<T, String>>;

enum Pending<'vm> {
    Observe(Completion<(PathBuf, CommittedCheckout)>),
    Read(
        Completion<[String; 2]>,
        RootedSchemeObject<'vm>,
        CommittedCheckout,
    ),
    Switch(Completion<PathBuf>),
}

impl Pending<'_> {
    fn is_finished(&self) -> bool {
        match self {
            Self::Observe(task) => task.is_finished(),
            Self::Read(task, ..) => task.is_finished(),
            Self::Switch(task) => task.is_finished(),
        }
    }

    fn abort(self) {
        match self {
            Self::Observe(task) => task.abort(),
            Self::Read(task, ..) => task.abort(),
            Self::Switch(task) => task.abort(),
        }
    }
}

pub(super) struct GitContext<'vm> {
    scheme: &'vm GerbilRuntime,
    approved: PathBuf,
    directory: PathBuf,
    pending: Option<Pending<'vm>>,
    catalog: Option<RootedSchemeObject<'vm>>,
}

impl<'vm> GitContext<'vm> {
    pub(super) fn new(
        scheme: &'vm GerbilRuntime,
        approved: PathBuf,
        directory: PathBuf,
        app: &App,
        runtime: &Runtime,
    ) -> Self {
        let mut context = Self {
            scheme,
            approved,
            directory,
            pending: None,
            catalog: None,
        };
        context.observe(app, runtime);
        context
    }

    fn observe(&mut self, app: &App, runtime: &Runtime) {
        let approved = self.approved.clone();
        let path = self.directory.clone();
        self.pending = Some(Pending::Observe(app.runtime_owner.spawn_on(
            runtime,
            async move {
                let (owner, checkout) = tokio::try_join!(
                    GitOperations::repository_owner(&approved),
                    GitOperations::committed_checkout(&path),
                )
                .map_err(|error| error.to_string())?;
                Ok((owner, checkout))
            },
        )));
    }

    pub(super) fn switch(&mut self, app: &mut App, runtime: &Runtime, branch: String) {
        if matches!(self.pending, Some(Pending::Switch(_))) {
            publish(
                app,
                MessageSeverity::Warning,
                "Worktree switch already running",
                "",
            );
            return;
        }
        let provider = match self.worktrunk() {
            Ok(provider) => provider,
            Err(error) => {
                publish(
                    app,
                    MessageSeverity::Warning,
                    "Worktree switch refused",
                    &error,
                );
                return;
            }
        };
        // Remove the old delivery ticket and retire its Lab grants before
        // the next checkout can be observed.
        if let Some(old) = self.pending.take() {
            let revoked = match &old {
                Pending::Read(_, permit, _) => {
                    worktree_call(self.scheme, WorktreeOperation::Revoke, permit).map(|_| ())
                }
                _ => Ok(()),
            };
            old.abort();
            if let Err(error) = revoked {
                publish(
                    app,
                    MessageSeverity::Warning,
                    "Worktree switch refused",
                    &error.to_string(),
                );
                return;
            }
        }
        let branch = WorkingCopyBranchName::new(branch);
        let path = self.directory.clone();
        let approved = self.approved.clone();
        self.pending = Some(Pending::Switch(app.runtime_owner.spawn_on(
            runtime,
            async move {
                let checkout = GitOperations::committed_checkout(&path)
                    .await
                    .map_err(|e| e.to_string())?;
                let approved_owner = GitOperations::repository_owner(&approved)
                    .await
                    .map_err(|e| e.to_string())?;
                if approved_owner != checkout.owner
                    || !checkout
                        .has_local_branch(&branch)
                        .await
                        .map_err(|e| e.to_string())?
                {
                    return Err("Select an existing local branch of the approved repository".into());
                }
                let project = checkout
                    .owner
                    .to_str()
                    .ok_or("Repository identity is not UTF-8")?;
                let handle = WorkingCopyHandle::new(branch.as_str(), checkout.root.as_path())
                    .with_branch(branch.clone());
                let request = WorkingCopySwitchRequest::new(
                    project,
                    provider,
                    checkout.root.as_path(),
                    handle,
                );
                let receipt = WorkingCopyIsolationDriver::new()
                    .isolate(WorkingCopyIsolationRequest::Switch(request))
                    .await
                    .receipt;
                let binding =
                    WorkingCopyActiveBinding::from_receipt(&receipt).ok_or_else(|| {
                        receipt
                            .reason
                            .unwrap_or_else(|| "Provider returned no working copy".into())
                    })?;
                let observed = GitOperations::committed_checkout(&binding.working_copy.path)
                    .await
                    .map_err(|e| e.to_string())?;
                if observed.owner != approved_owner
                    || observed.reference.as_str() != format!("refs/heads/{}", branch.as_str())
                {
                    return Err("Provider checkout differs from the approved branch".into());
                }
                Ok(observed.root.as_path().to_owned())
            },
        )));
    }

    pub(super) fn poll(&mut self, app: &mut App, runtime: &Runtime) -> bool {
        if !self.pending.as_ref().is_some_and(Pending::is_finished) {
            return false;
        }
        let pending = self.pending.take().expect("finished Context task");
        let result = match pending {
            Pending::Observe(task) => joined(runtime, task)
                .and_then(|(owner, checkout)| self.read(app, runtime, &owner, checkout)),
            Pending::Read(task, permit, checkout) => joined(runtime, task)
                .and_then(|sources| self.deliver(app, permit, &checkout, &sources)),
            Pending::Switch(task) => match joined(runtime, task) {
                Ok(path) => {
                    self.catalog = None;
                    self.directory = path.clone();
                    app.context_worktree = Some(path);
                    self.observe(app, runtime);
                    Ok(())
                }
                Err(error) => {
                    if self.catalog.is_none() {
                        self.observe(app, runtime);
                    }
                    Err(error)
                }
            },
        };
        if let Err(error) = result {
            publish(
                app,
                MessageSeverity::Warning,
                "Git Context unavailable",
                &error,
            );
        }
        true
    }

    fn read(
        &mut self,
        app: &mut App,
        runtime: &Runtime,
        owner: &Path,
        checkout: CommittedCheckout,
    ) -> Result<(), String> {
        let worktree = format!(
            "{}\n{}",
            utf8(&checkout.git_directory)?,
            checkout.reference.as_str()
        );
        let inputs = [
            utf8(owner)?,
            utf8(&checkout.owner)?,
            &worktree,
            checkout.commit.as_str(),
        ];
        let arguments = roots(self.scheme, &inputs)?;
        let request = self
            .scheme
            .collection_from_objects(
                CollectionKind::Vector,
                &arguments.iter().collect::<Vec<_>>(),
            )
            .map_err(|e| e.to_string())?
            .into_object();
        let permit = worktree_call(self.scheme, WorktreeOperation::Prepare, &request)
            .map_err(|e| e.to_string())?;
        if permit.as_ref().kind().map_err(|e| e.to_string())? != ObjectKind::Procedure {
            return Err(
                "The checkout does not belong to the repository approved in git_context".into(),
            );
        }
        app.context_worktree = Some(checkout.root.as_path().to_owned());
        let bound = checkout.clone();
        let read = app.runtime_owner.spawn_on(runtime, async move {
            let (index, topology) = tokio::try_join!(
                bound.read_context_document(ContextDocument::Index),
                bound.read_context_document(ContextDocument::Topology),
            )
            .map_err(|e| e.to_string())?;
            let observed = GitOperations::committed_checkout(bound.root.as_path())
                .await
                .map_err(|e| e.to_string())?;
            if observed != bound {
                return Err("Checkout changed during Git Context read".into());
            }
            Ok([
                String::from_utf8(index).map_err(|e| e.to_string())?,
                String::from_utf8(topology).map_err(|e| e.to_string())?,
            ])
        });
        self.pending = Some(Pending::Read(read, permit, checkout));
        Ok(())
    }

    fn deliver(
        &mut self,
        app: &mut App,
        permit: RootedSchemeObject<'vm>,
        checkout: &CommittedCheckout,
        sources: &[String; 2],
    ) -> Result<(), String> {
        let inputs = roots(
            self.scheme,
            &[checkout.commit.as_str(), &sources[0], &sources[1]],
        )?;
        let request = self
            .scheme
            .collection_from_objects(
                CollectionKind::Vector,
                &[&permit, &inputs[0], &inputs[1], &inputs[2]],
            )
            .map_err(|e| e.to_string())?
            .into_object();
        let publication = worktree_call(self.scheme, WorktreeOperation::Deliver, &request)
            .map_err(|e| e.to_string())?;
        let bundle = publication
            .into_collection(CollectionKind::Vector)
            .map_err(|e| e.to_string())?;
        let mut detail = format!(
            "{} @ {}",
            checkout.reference.as_str(),
            checkout.commit.as_str()
        );
        for (position, path) in ["index.org", "topology.org"].into_iter().enumerate() {
            let value = bundle.element(position + 2).map_err(|e| e.to_string())?;
            let receipt = value
                .into_collection(CollectionKind::Vector)
                .map_err(|e| e.to_string())?;
            let digest = receipt
                .element(1)
                .map_err(|e| e.to_string())?
                .into_string()
                .map_err(|e| e.to_string())?
                .to_string()
                .into_result()
                .map_err(|e| e.to_string())?;
            let contract = receipt
                .element(2)
                .map_err(|e| e.to_string())?
                .into_string()
                .map_err(|e| e.to_string())?
                .to_string()
                .into_result()
                .map_err(|e| e.to_string())?;
            detail.push_str(&format!("\n{path} {contract} {digest}"));
        }
        self.catalog = Some(bundle.into_object());
        publish(app, MessageSeverity::Info, "Git Context ready", &detail);
        Ok(())
    }

    fn worktrunk(&self) -> Result<WorkingCopyIsolationProvider, String> {
        let request = self
            .scheme
            .collection_from_objects(CollectionKind::Vector, &[])
            .map_err(|e| e.to_string())?
            .into_object();
        let provider = worktree_call(self.scheme, WorktreeOperation::Provider, &request)
            .map_err(|e| e.to_string())?;
        let mut names = gerbil_scheme::IdentifierBuffer::default();
        provider
            .as_ref()
            .copy_identifier_names(&mut names)
            .map_err(|e| e.to_string())?;
        if names.get(0) != Some((gerbil_scheme::IdentifierKind::Symbol, "worktrunk")) {
            return Err("Scheme did not select Worktrunk".into());
        }
        Ok(WorkingCopyIsolationProvider::Worktrunk)
    }
}

impl Drop for GitContext<'_> {
    fn drop(&mut self) {
        if let Some(pending) = self.pending.take() {
            pending.abort();
        }
    }
}

fn joined<T: Send + 'static>(runtime: &Runtime, task: Completion<T>) -> Result<T, String> {
    runtime
        .block_on(task.join())
        .map_err(|e| format!("{e:?}"))?
}

fn utf8(path: &Path) -> Result<&str, String> {
    path.to_str()
        .ok_or_else(|| "Git Context path is not UTF-8".into())
}

fn roots<'vm>(
    runtime: &'vm GerbilRuntime,
    values: &[&str],
) -> Result<Vec<RootedSchemeObject<'vm>>, String> {
    values
        .iter()
        .map(|value| {
            runtime
                .string_from_utf8(value)
                .map(|root| root.into_object())
                .map_err(|e| e.to_string())
        })
        .collect()
}

fn publish(app: &mut App, severity: MessageSeverity, summary: &str, detail: &str) {
    app.publish_message(MessageEvent {
        source: MessageSource::Workspace,
        severity,
        summary,
        detail,
    });
}
