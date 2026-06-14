//! Executes runtime workspace project import side effects.

use std::collections::BTreeMap;

use marlin_agent_protocol::{
    RuntimeEnvironment, RuntimeEnvironmentActivation, RuntimeWorkspaceProject,
    RuntimeWorkspaceProjectImportAction, RuntimeWorkspaceProjectImportActionReceipt,
    RuntimeWorkspaceProjectImportReceipt, RuntimeWorkspaceProjectTrust,
};

use crate::activation::{command_environment, direnv_cwd};
use crate::{DirenvCommandRunner, ProcessDirenvCommandRunner};

/// Input used to import one runtime workspace project.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeWorkspaceProjectImportRequest {
    pub project: RuntimeWorkspaceProject,
    pub base_environment: BTreeMap<String, String>,
}

impl RuntimeWorkspaceProjectImportRequest {
    pub fn new(
        project: RuntimeWorkspaceProject,
        base_environment: BTreeMap<String, String>,
    ) -> Self {
        Self {
            project,
            base_environment,
        }
    }
}

/// Receipt emitted after importing one runtime workspace project.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeWorkspaceProjectImportResult {
    pub receipt: RuntimeWorkspaceProjectImportReceipt,
}

/// Applies trusted workspace project import side effects with an injectable runner.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeWorkspaceProjectImporter<R = ProcessDirenvCommandRunner> {
    runner: R,
}

impl RuntimeWorkspaceProjectImporter<ProcessDirenvCommandRunner> {
    pub fn new() -> Self {
        Self {
            runner: ProcessDirenvCommandRunner,
        }
    }
}

impl Default for RuntimeWorkspaceProjectImporter<ProcessDirenvCommandRunner> {
    fn default() -> Self {
        Self::new()
    }
}

impl<R> RuntimeWorkspaceProjectImporter<R>
where
    R: DirenvCommandRunner,
{
    pub fn with_runner(runner: R) -> Self {
        Self { runner }
    }

    pub async fn import_project(
        &self,
        request: RuntimeWorkspaceProjectImportRequest,
    ) -> RuntimeWorkspaceProjectImportResult {
        let project = request.project;
        if !project.is_trusted() {
            let reason = rejected_trust_reason(&project.trust);
            return RuntimeWorkspaceProjectImportResult {
                receipt: RuntimeWorkspaceProjectImportReceipt::rejected(project.id, reason),
            };
        }

        let mut actions = Vec::new();
        if let RuntimeEnvironmentActivation::Direnv { envrc, .. } = &project.activation.activation {
            let runtime_environment = RuntimeEnvironment::default()
                .with_cwd(project.root.clone())
                .with_activation(project.activation.clone());
            let cwd = match direnv_cwd(&runtime_environment, envrc) {
                Ok(cwd) => cwd,
                Err(error) => {
                    actions.push(RuntimeWorkspaceProjectImportActionReceipt::rejected(
                        RuntimeWorkspaceProjectImportAction::DirenvAllow,
                        error.to_string(),
                    ));
                    return RuntimeWorkspaceProjectImportResult {
                        receipt: RuntimeWorkspaceProjectImportReceipt::rejected_with_actions(
                            project.id,
                            error.to_string(),
                            actions,
                        ),
                    };
                }
            };
            let command_environment =
                command_environment(&request.base_environment, &project.activation.shell);
            if let Err(error) = self.runner.allow(&cwd, &command_environment).await {
                actions.push(RuntimeWorkspaceProjectImportActionReceipt::rejected(
                    RuntimeWorkspaceProjectImportAction::DirenvAllow,
                    error.to_string(),
                ));
                return RuntimeWorkspaceProjectImportResult {
                    receipt: RuntimeWorkspaceProjectImportReceipt::rejected_with_actions(
                        project.id,
                        error.to_string(),
                        actions,
                    ),
                };
            }
            actions.push(RuntimeWorkspaceProjectImportActionReceipt::applied(
                RuntimeWorkspaceProjectImportAction::DirenvAllow,
            ));
        }

        RuntimeWorkspaceProjectImportResult {
            receipt: RuntimeWorkspaceProjectImportReceipt::imported_with_actions(&project, actions),
        }
    }
}

fn rejected_trust_reason(trust: &RuntimeWorkspaceProjectTrust) -> &'static str {
    match trust {
        RuntimeWorkspaceProjectTrust::Trusted => "workspace project is trusted",
        RuntimeWorkspaceProjectTrust::ReviewRequired => {
            "workspace project requires trust review before import"
        }
        RuntimeWorkspaceProjectTrust::Denied => "workspace project trust was denied",
    }
}
