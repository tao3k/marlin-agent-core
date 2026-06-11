//! Runtime environment contracts for custom homes, config layers, and sandbox policy.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Root directory used by runtime-owned agent state.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeHome {
    pub path: PathBuf,
    pub source: RuntimeHomeSource,
    pub profile: Option<String>,
}

impl RuntimeHome {
    pub fn custom(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            source: RuntimeHomeSource::Custom,
            profile: None,
        }
    }

    pub fn with_profile(mut self, profile: impl Into<String>) -> Self {
        self.profile = Some(profile.into());
        self
    }
}

/// Source of a runtime home path.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RuntimeHomeSource {
    Default,
    Custom,
    InheritedSubAgent { parent_home: PathBuf },
}

/// Layered configuration source visible to the runtime.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RuntimeConfigLayerSource {
    System {
        file: PathBuf,
    },
    User {
        file: PathBuf,
        profile: Option<String>,
    },
    Project {
        dot_marlin_folder: PathBuf,
    },
    SessionFlags,
    SubAgent {
        agent_reference: String,
    },
    Plugin {
        plugin_id: String,
    },
    Unknown,
}

/// Config layer with explicit precedence. Larger precedence wins.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeConfigLayer {
    pub source: RuntimeConfigLayerSource,
    pub precedence: i16,
}

impl RuntimeConfigLayer {
    pub fn new(source: RuntimeConfigLayerSource, precedence: i16) -> Self {
        Self { source, precedence }
    }
}

/// Sandbox policy attached to runtime-owned execution.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeSandboxPolicy {
    pub writable_roots: Vec<PathBuf>,
    pub network_access: bool,
    pub exclude_tmpdir_env_var: bool,
    pub exclude_slash_tmp: bool,
}

/// Runtime context visible to providers, tools, hooks, and sub-agents.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeEnvironment {
    pub home: Option<RuntimeHome>,
    pub cwd: Option<PathBuf>,
    pub sandbox: RuntimeSandboxPolicy,
    pub config_layers: Vec<RuntimeConfigLayer>,
}

impl RuntimeEnvironment {
    pub fn with_home(mut self, home: RuntimeHome) -> Self {
        self.home = Some(home);
        self
    }

    pub fn with_cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    pub fn with_sandbox(mut self, sandbox: RuntimeSandboxPolicy) -> Self {
        self.sandbox = sandbox;
        self
    }

    pub fn with_config_layer(mut self, layer: RuntimeConfigLayer) -> Self {
        self.config_layers.push(layer);
        self
    }
}
