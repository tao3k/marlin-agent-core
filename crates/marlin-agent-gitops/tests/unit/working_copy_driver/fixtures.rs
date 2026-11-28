// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use async_trait::async_trait;
use marlin_agent_gitops::working_copy::{
    WorkingCopyCommandInvocation, WorkingCopyCommandOutput, WorkingCopyCommandRunner,
    WorkingCopyGitRepositoryResolver, WorkingCopyGitTopLevel, WorkingCopyIsolationDriverError,
};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

#[derive(Clone, Debug)]
pub(super) struct ScriptedWorkingCopyRunner {
    calls: Arc<Mutex<Vec<WorkingCopyCommandInvocation>>>,
    outputs: Arc<Mutex<Vec<WorkingCopyCommandOutput>>>,
}

impl ScriptedWorkingCopyRunner {
    pub(super) fn new(outputs: impl IntoIterator<Item = WorkingCopyCommandOutput>) -> Self {
        Self {
            calls: Arc::new(Mutex::new(Vec::new())),
            outputs: Arc::new(Mutex::new(outputs.into_iter().collect())),
        }
    }

    pub(super) fn calls(&self) -> Vec<WorkingCopyCommandInvocation> {
        self.calls.lock().expect("calls lock").clone()
    }
}

#[async_trait]
impl WorkingCopyCommandRunner for ScriptedWorkingCopyRunner {
    async fn run(
        &self,
        invocation: &WorkingCopyCommandInvocation,
    ) -> Result<WorkingCopyCommandOutput, WorkingCopyIsolationDriverError> {
        self.calls
            .lock()
            .expect("calls lock")
            .push(invocation.clone());
        Ok(self.outputs.lock().expect("outputs lock").remove(0))
    }
}

#[derive(Clone, Debug)]
pub(super) struct ScriptedGitRepositoryResolver {
    calls: Arc<Mutex<Vec<PathBuf>>>,
    root: WorkingCopyGitTopLevel,
}

impl ScriptedGitRepositoryResolver {
    pub(super) fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            calls: Arc::new(Mutex::new(Vec::new())),
            root: WorkingCopyGitTopLevel::from_resolved_path(root.into()),
        }
    }

    pub(super) fn calls(&self) -> Vec<PathBuf> {
        self.calls.lock().expect("calls lock").clone()
    }
}

#[async_trait]
impl WorkingCopyGitRepositoryResolver for ScriptedGitRepositoryResolver {
    async fn resolve_git_toplevel(
        &self,
        discovery_path: &Path,
    ) -> Result<WorkingCopyGitTopLevel, WorkingCopyIsolationDriverError> {
        self.calls
            .lock()
            .expect("calls lock")
            .push(discovery_path.to_path_buf());
        Ok(self.root.clone())
    }

    async fn resolve_repository_worktree(
        &self,
        repository: &WorkingCopyGitTopLevel,
        target: &Path,
    ) -> Result<WorkingCopyGitTopLevel, WorkingCopyIsolationDriverError> {
        assert_eq!(repository, &self.root);
        Ok(WorkingCopyGitTopLevel::from_resolved_path(
            repository.as_path().join(target),
        ))
    }
}
