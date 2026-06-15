use async_trait::async_trait;
use marlin_agent_environment::{
    WorkingCopyCommandOutput, WorkingCopyCommandRunner, WorkingCopyGitRepositoryResolver,
    WorkingCopyIsolationDriverError,
};
use marlin_workspace_protocol::{WorkingCopyCommandInvocation, WorkingCopyGitTopLevel};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

mod driver;
mod parallel;
mod probe;

#[derive(Clone, Debug)]
struct ScriptedWorkingCopyRunner {
    calls: Arc<Mutex<Vec<WorkingCopyCommandInvocation>>>,
    outputs: Arc<Mutex<Vec<WorkingCopyCommandOutput>>>,
}

impl ScriptedWorkingCopyRunner {
    fn new(outputs: impl IntoIterator<Item = WorkingCopyCommandOutput>) -> Self {
        Self {
            calls: Arc::new(Mutex::new(Vec::new())),
            outputs: Arc::new(Mutex::new(outputs.into_iter().collect())),
        }
    }

    fn calls(&self) -> Vec<WorkingCopyCommandInvocation> {
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
struct ScriptedGitRepositoryResolver {
    calls: Arc<Mutex<Vec<PathBuf>>>,
    root: WorkingCopyGitTopLevel,
}

impl ScriptedGitRepositoryResolver {
    fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            calls: Arc::new(Mutex::new(Vec::new())),
            root: WorkingCopyGitTopLevel::from_resolved_path(root.into()),
        }
    }

    fn calls(&self) -> Vec<PathBuf> {
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
}
