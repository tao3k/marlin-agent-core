//! Runtime command facts used as `model_route` resolver input.

use serde::{Deserialize, Serialize};

use super::{ModelCommandKind, ModelRouteAgentScope};

/// Runtime request facts used to resolve a command or sub-agent call to a model.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ModelRouteRequest {
    pub executable: Option<String>,
    pub argv: Vec<String>,
    pub cwd: Option<String>,
    pub workspace: Option<String>,
    pub sub_agent_role: Option<String>,
    #[serde(default)]
    pub agent_scope: Option<ModelRouteAgentScope>,
    pub command_kind: Option<ModelCommandKind>,
}

impl ModelRouteRequest {
    pub fn command(argv: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let argv = argv.into_iter().map(Into::into).collect::<Vec<_>>();
        let executable = argv.first().cloned();
        Self {
            executable,
            argv,
            cwd: None,
            workspace: None,
            sub_agent_role: None,
            agent_scope: None,
            command_kind: None,
        }
    }

    pub fn command_line(&self) -> String {
        self.argv.join(" ")
    }

    pub fn with_cwd(mut self, cwd: impl Into<String>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    pub fn with_workspace(mut self, workspace: impl Into<String>) -> Self {
        self.workspace = Some(workspace.into());
        self
    }

    pub fn with_sub_agent_role(mut self, role: impl Into<String>) -> Self {
        self.sub_agent_role = Some(role.into());
        self
    }

    pub fn with_agent_scope(mut self, agent_scope: impl Into<ModelRouteAgentScope>) -> Self {
        self.agent_scope = Some(agent_scope.into());
        self
    }

    pub fn with_command_kind(mut self, kind: impl Into<ModelCommandKind>) -> Self {
        self.command_kind = Some(kind.into());
        self
    }
}
