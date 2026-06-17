//! Command matcher dimensions used by deterministic `model_route` rules.

use serde::{Deserialize, Serialize};

/// Glob-backed command matcher surface. Empty dimensions match everything.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ModelCommandMatcher {
    pub executable_globs: Vec<String>,
    pub argv_globs: Vec<String>,
    pub cwd_globs: Vec<String>,
    pub workspace_globs: Vec<String>,
    pub sub_agent_role_globs: Vec<String>,
    pub agent_scope_globs: Vec<String>,
    pub command_kind_globs: Vec<String>,
}

impl ModelCommandMatcher {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_argv_glob(mut self, glob: impl Into<String>) -> Self {
        self.argv_globs.push(glob.into());
        self
    }

    pub fn with_executable_glob(mut self, glob: impl Into<String>) -> Self {
        self.executable_globs.push(glob.into());
        self
    }

    pub fn with_sub_agent_role_glob(mut self, glob: impl Into<String>) -> Self {
        self.sub_agent_role_globs.push(glob.into());
        self
    }

    pub fn with_agent_scope_glob(mut self, glob: impl Into<String>) -> Self {
        self.agent_scope_globs.push(glob.into());
        self
    }

    pub fn with_cwd_glob(mut self, glob: impl Into<String>) -> Self {
        self.cwd_globs.push(glob.into());
        self
    }

    pub fn with_workspace_glob(mut self, glob: impl Into<String>) -> Self {
        self.workspace_globs.push(glob.into());
        self
    }

    pub fn with_command_kind_glob(mut self, glob: impl Into<String>) -> Self {
        self.command_kind_globs.push(glob.into());
        self
    }
}
