//! Runtime environment contracts for custom homes, config layers, and sandbox policy.

use std::{collections::BTreeMap, path::PathBuf};

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

/// Explicit environment activation policy for project shell state.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeEnvironmentActivationPolicy {
    #[serde(default)]
    pub activation: RuntimeEnvironmentActivation,
    #[serde(default)]
    pub shell: RuntimeShellIsolationPolicy,
}

impl RuntimeEnvironmentActivationPolicy {
    pub fn disabled() -> Self {
        Self::default()
    }

    pub fn direnv_project() -> Self {
        Self {
            activation: RuntimeEnvironmentActivation::Direnv {
                envrc: RuntimeEnvrcPolicy::Project,
                capture_delta: true,
            },
            shell: RuntimeShellIsolationPolicy::default(),
        }
    }

    pub fn direnv_file(file: impl Into<PathBuf>) -> Self {
        Self {
            activation: RuntimeEnvironmentActivation::Direnv {
                envrc: RuntimeEnvrcPolicy::Explicit { file: file.into() },
                capture_delta: true,
            },
            shell: RuntimeShellIsolationPolicy::default(),
        }
    }

    pub fn with_shell(mut self, shell: RuntimeShellIsolationPolicy) -> Self {
        self.shell = shell;
        self
    }
}

/// Environment activation mechanism used before runtime-owned command execution.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum RuntimeEnvironmentActivation {
    #[default]
    Disabled,
    Direnv {
        envrc: RuntimeEnvrcPolicy,
        capture_delta: bool,
    },
}

/// `.envrc` source accepted by a direnv activation policy.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RuntimeEnvrcPolicy {
    Project,
    Explicit { file: PathBuf },
}

/// Shell environment isolation applied around activation and process execution.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeShellIsolationPolicy {
    #[serde(default)]
    pub isolate_host_environment: bool,
    #[serde(default)]
    pub allowlist: Vec<String>,
    #[serde(default)]
    pub denylist: Vec<String>,
}

impl RuntimeShellIsolationPolicy {
    pub fn isolated() -> Self {
        Self {
            isolate_host_environment: true,
            allowlist: Vec::new(),
            denylist: Vec::new(),
        }
    }

    pub fn with_allowed(mut self, name: impl Into<String>) -> Self {
        self.allowlist.push(name.into());
        self
    }

    pub fn with_denied(mut self, name: impl Into<String>) -> Self {
        self.denylist.push(name.into());
        self
    }
}

/// Status recorded for an environment activation attempt or plan.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum RuntimeEnvironmentActivationStatus {
    #[default]
    Disabled,
    Planned,
    Applied,
    Rejected,
}

/// Name-only environment delta. Values are intentionally omitted to avoid
/// leaking secrets into receipts.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeEnvironmentDelta {
    #[serde(default)]
    pub added: Vec<String>,
    #[serde(default)]
    pub changed: Vec<String>,
    #[serde(default)]
    pub removed: Vec<String>,
}

impl RuntimeEnvironmentDelta {
    pub fn from_snapshots(
        before: &BTreeMap<String, String>,
        after: &BTreeMap<String, String>,
    ) -> Self {
        let added = after
            .keys()
            .filter(|name| !before.contains_key(*name))
            .cloned()
            .collect();
        let changed = after
            .iter()
            .filter(|(name, value)| before.get(*name).is_some_and(|before| before != *value))
            .map(|(name, _)| name.clone())
            .collect();
        let removed = before
            .keys()
            .filter(|name| !after.contains_key(*name))
            .cloned()
            .collect();

        Self {
            added,
            changed,
            removed,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.changed.is_empty() && self.removed.is_empty()
    }
}

/// Audit receipt for shell/environment activation.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeEnvironmentActivationReceipt {
    #[serde(default)]
    pub activation: RuntimeEnvironmentActivation,
    #[serde(default)]
    pub shell: RuntimeShellIsolationPolicy,
    #[serde(default)]
    pub status: RuntimeEnvironmentActivationStatus,
    #[serde(default)]
    pub delta: RuntimeEnvironmentDelta,
    pub reason: Option<String>,
}

impl RuntimeEnvironmentActivationReceipt {
    pub fn disabled(policy: &RuntimeEnvironmentActivationPolicy) -> Self {
        Self {
            activation: policy.activation.clone(),
            shell: policy.shell.clone(),
            status: RuntimeEnvironmentActivationStatus::Disabled,
            delta: RuntimeEnvironmentDelta::default(),
            reason: None,
        }
    }

    pub fn planned(policy: &RuntimeEnvironmentActivationPolicy) -> Self {
        Self {
            activation: policy.activation.clone(),
            shell: policy.shell.clone(),
            status: RuntimeEnvironmentActivationStatus::Planned,
            delta: RuntimeEnvironmentDelta::default(),
            reason: None,
        }
    }

    pub fn applied(
        policy: &RuntimeEnvironmentActivationPolicy,
        delta: RuntimeEnvironmentDelta,
    ) -> Self {
        Self {
            activation: policy.activation.clone(),
            shell: policy.shell.clone(),
            status: RuntimeEnvironmentActivationStatus::Applied,
            delta,
            reason: None,
        }
    }

    pub fn rejected(
        policy: &RuntimeEnvironmentActivationPolicy,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            activation: policy.activation.clone(),
            shell: policy.shell.clone(),
            status: RuntimeEnvironmentActivationStatus::Rejected,
            delta: RuntimeEnvironmentDelta::default(),
            reason: Some(reason.into()),
        }
    }
}

/// Runtime context visible to providers, tools, hooks, and sub-agents.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeEnvironment {
    pub home: Option<RuntimeHome>,
    pub cwd: Option<PathBuf>,
    pub sandbox: RuntimeSandboxPolicy,
    pub config_layers: Vec<RuntimeConfigLayer>,
    #[serde(default)]
    pub activation: RuntimeEnvironmentActivationPolicy,
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

    pub fn with_activation(mut self, activation: RuntimeEnvironmentActivationPolicy) -> Self {
        self.activation = activation;
        self
    }
}

/// Resolved runtime environment and the receipt for its activation policy.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeEnvironmentResolution {
    pub environment: RuntimeEnvironment,
    pub activation_receipt: RuntimeEnvironmentActivationReceipt,
}
