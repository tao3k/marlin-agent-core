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

/// Stable identifier for one imported workspace project.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct RuntimeWorkspaceProjectId(String);

impl RuntimeWorkspaceProjectId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// Trust decision attached to a runtime workspace project import.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum RuntimeWorkspaceProjectTrust {
    Trusted,
    #[default]
    ReviewRequired,
    Denied,
}

/// Sandbox policy attached to runtime-owned execution.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeSandboxPolicy {
    pub writable_roots: Vec<PathBuf>,
    pub network_access: bool,
    pub exclude_tmpdir_env_var: bool,
    pub exclude_slash_tmp: bool,
}

/// One project imported into the runtime workspace.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeWorkspaceProject {
    pub id: RuntimeWorkspaceProjectId,
    pub root: PathBuf,
    #[serde(default)]
    pub trust: RuntimeWorkspaceProjectTrust,
    pub project_config: Option<PathBuf>,
    pub activation: RuntimeEnvironmentActivationPolicy,
    pub sandbox: RuntimeSandboxPolicy,
}

impl RuntimeWorkspaceProject {
    pub fn new(id: impl Into<String>, root: impl Into<PathBuf>) -> Self {
        Self {
            id: RuntimeWorkspaceProjectId::new(id),
            root: root.into(),
            trust: RuntimeWorkspaceProjectTrust::ReviewRequired,
            project_config: None,
            activation: RuntimeEnvironmentActivationPolicy::disabled(),
            sandbox: RuntimeSandboxPolicy::default(),
        }
    }

    pub fn trusted(id: impl Into<String>, root: impl Into<PathBuf>) -> Self {
        Self::new(id, root).with_trust(RuntimeWorkspaceProjectTrust::Trusted)
    }

    pub fn with_trust(mut self, trust: RuntimeWorkspaceProjectTrust) -> Self {
        self.trust = trust;
        self
    }

    pub fn with_project_config(mut self, dot_marlin_folder: impl Into<PathBuf>) -> Self {
        self.project_config = Some(dot_marlin_folder.into());
        self
    }

    pub fn with_activation(mut self, activation: RuntimeEnvironmentActivationPolicy) -> Self {
        self.activation = activation;
        self
    }

    pub fn with_sandbox(mut self, sandbox: RuntimeSandboxPolicy) -> Self {
        self.sandbox = sandbox;
        self
    }

    pub fn is_trusted(&self) -> bool {
        self.trust == RuntimeWorkspaceProjectTrust::Trusted
    }

    pub fn uses_direnv(&self) -> bool {
        matches!(
            self.activation.activation,
            RuntimeEnvironmentActivation::Direnv { .. }
        )
    }
}

/// Status for importing a workspace project into a resolved runtime environment.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum RuntimeWorkspaceProjectImportStatus {
    #[default]
    Imported,
    Rejected,
}

/// Native action executed while importing a workspace project.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RuntimeWorkspaceProjectImportAction {
    DirenvAllow,
}

/// Status for one native workspace project import action.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum RuntimeWorkspaceProjectImportActionStatus {
    #[default]
    Applied,
    Skipped,
    Rejected,
}

/// Receipt for one native workspace project import action.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeWorkspaceProjectImportActionReceipt {
    pub action: RuntimeWorkspaceProjectImportAction,
    pub status: RuntimeWorkspaceProjectImportActionStatus,
    pub reason: Option<String>,
}

impl RuntimeWorkspaceProjectImportActionReceipt {
    pub fn applied(action: RuntimeWorkspaceProjectImportAction) -> Self {
        Self {
            action,
            status: RuntimeWorkspaceProjectImportActionStatus::Applied,
            reason: None,
        }
    }

    pub fn skipped(action: RuntimeWorkspaceProjectImportAction, reason: impl Into<String>) -> Self {
        Self {
            action,
            status: RuntimeWorkspaceProjectImportActionStatus::Skipped,
            reason: Some(reason.into()),
        }
    }

    pub fn rejected(
        action: RuntimeWorkspaceProjectImportAction,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            action,
            status: RuntimeWorkspaceProjectImportActionStatus::Rejected,
            reason: Some(reason.into()),
        }
    }
}

/// Receipt recording whether a requested workspace project import was applied.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeWorkspaceProjectImportReceipt {
    pub project_id: RuntimeWorkspaceProjectId,
    pub root: Option<PathBuf>,
    pub status: RuntimeWorkspaceProjectImportStatus,
    pub reason: Option<String>,
    #[serde(default)]
    pub actions: Vec<RuntimeWorkspaceProjectImportActionReceipt>,
}

impl RuntimeWorkspaceProjectImportReceipt {
    pub fn imported(project: &RuntimeWorkspaceProject) -> Self {
        Self::imported_with_actions(project, Vec::new())
    }

    pub fn imported_with_actions(
        project: &RuntimeWorkspaceProject,
        actions: Vec<RuntimeWorkspaceProjectImportActionReceipt>,
    ) -> Self {
        Self {
            project_id: project.id.clone(),
            root: Some(project.root.clone()),
            status: RuntimeWorkspaceProjectImportStatus::Imported,
            reason: None,
            actions,
        }
    }

    pub fn rejected(project_id: RuntimeWorkspaceProjectId, reason: impl Into<String>) -> Self {
        Self::rejected_with_actions(project_id, reason, Vec::new())
    }

    pub fn rejected_with_actions(
        project_id: RuntimeWorkspaceProjectId,
        reason: impl Into<String>,
        actions: Vec<RuntimeWorkspaceProjectImportActionReceipt>,
    ) -> Self {
        Self {
            project_id,
            root: None,
            status: RuntimeWorkspaceProjectImportStatus::Rejected,
            reason: Some(reason.into()),
            actions,
        }
    }
}

/// Explicit environment activation policy for project shell state.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeEnvironmentActivationPolicy {
    #[serde(default)]
    pub activation: RuntimeEnvironmentActivation,
    #[serde(default)]
    pub shell: RuntimeShellIsolationPolicy,
    #[serde(default)]
    pub preflight_actions: Vec<RuntimeEnvironmentActivationAction>,
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
            preflight_actions: Vec::new(),
        }
    }

    pub fn direnv_file(file: impl Into<PathBuf>) -> Self {
        Self {
            activation: RuntimeEnvironmentActivation::Direnv {
                envrc: RuntimeEnvrcPolicy::Explicit { file: file.into() },
                capture_delta: true,
            },
            shell: RuntimeShellIsolationPolicy::default(),
            preflight_actions: Vec::new(),
        }
    }

    pub fn with_shell(mut self, shell: RuntimeShellIsolationPolicy) -> Self {
        self.shell = shell;
        self
    }

    pub fn with_direnv_reload(mut self) -> Self {
        self.preflight_actions
            .push(RuntimeEnvironmentActivationAction::DirenvReload);
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

/// Runtime environment activation action executed by the native activator.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RuntimeEnvironmentActivationAction {
    DirenvReload,
    DirenvExportJson,
}

/// Status for one native activation action.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum RuntimeEnvironmentActivationActionStatus {
    #[default]
    Applied,
    Rejected,
}

/// Receipt for one native activation action.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeEnvironmentActivationActionReceipt {
    pub action: RuntimeEnvironmentActivationAction,
    pub status: RuntimeEnvironmentActivationActionStatus,
    pub reason: Option<String>,
}

impl RuntimeEnvironmentActivationActionReceipt {
    pub fn applied(action: RuntimeEnvironmentActivationAction) -> Self {
        Self {
            action,
            status: RuntimeEnvironmentActivationActionStatus::Applied,
            reason: None,
        }
    }

    pub fn rejected(action: RuntimeEnvironmentActivationAction, reason: impl Into<String>) -> Self {
        Self {
            action,
            status: RuntimeEnvironmentActivationActionStatus::Rejected,
            reason: Some(reason.into()),
        }
    }
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

/// Execution placement for refreshing project environment state.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum RuntimeEnvironmentRefreshExecution {
    #[default]
    Foreground,
    Background,
}

/// Cache ownership for refresh. Marlin records this but does not own direnv,
/// nix-direnv, or devenv cache files.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum RuntimeEnvironmentRefreshCachePolicy {
    #[default]
    ExternalToolOwned,
    Disabled,
}

/// Status recorded for an environment refresh attempt.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum RuntimeEnvironmentRefreshStatus {
    #[default]
    Skipped,
    Applied,
    Rejected,
    TimedOut,
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
    #[serde(default)]
    pub actions: Vec<RuntimeEnvironmentActivationActionReceipt>,
    pub reason: Option<String>,
}

impl RuntimeEnvironmentActivationReceipt {
    pub fn disabled(policy: &RuntimeEnvironmentActivationPolicy) -> Self {
        Self {
            activation: policy.activation.clone(),
            shell: policy.shell.clone(),
            status: RuntimeEnvironmentActivationStatus::Disabled,
            delta: RuntimeEnvironmentDelta::default(),
            actions: Vec::new(),
            reason: None,
        }
    }

    pub fn planned(policy: &RuntimeEnvironmentActivationPolicy) -> Self {
        Self {
            activation: policy.activation.clone(),
            shell: policy.shell.clone(),
            status: RuntimeEnvironmentActivationStatus::Planned,
            delta: RuntimeEnvironmentDelta::default(),
            actions: Vec::new(),
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
            actions: Vec::new(),
            reason: None,
        }
    }

    pub fn applied_with_actions(
        policy: &RuntimeEnvironmentActivationPolicy,
        delta: RuntimeEnvironmentDelta,
        actions: Vec<RuntimeEnvironmentActivationActionReceipt>,
    ) -> Self {
        Self {
            activation: policy.activation.clone(),
            shell: policy.shell.clone(),
            status: RuntimeEnvironmentActivationStatus::Applied,
            delta,
            actions,
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
            actions: Vec::new(),
            reason: Some(reason.into()),
        }
    }

    pub fn rejected_with_actions(
        policy: &RuntimeEnvironmentActivationPolicy,
        reason: impl Into<String>,
        actions: Vec<RuntimeEnvironmentActivationActionReceipt>,
    ) -> Self {
        Self {
            activation: policy.activation.clone(),
            shell: policy.shell.clone(),
            status: RuntimeEnvironmentActivationStatus::Rejected,
            delta: RuntimeEnvironmentDelta::default(),
            actions,
            reason: Some(reason.into()),
        }
    }
}

/// Receipt for a runtime environment refresh. The activation receipt remains
/// the source of truth for the actual shell/env changes.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeEnvironmentRefreshReceipt {
    #[serde(default)]
    pub execution: RuntimeEnvironmentRefreshExecution,
    #[serde(default)]
    pub cache_policy: RuntimeEnvironmentRefreshCachePolicy,
    #[serde(default)]
    pub status: RuntimeEnvironmentRefreshStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub elapsed_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    pub activation_receipt: RuntimeEnvironmentActivationReceipt,
    pub reason: Option<String>,
}

/// Named request for constructing a timed-out runtime refresh receipt.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeEnvironmentRefreshTimeout {
    pub execution: RuntimeEnvironmentRefreshExecution,
    pub cache_policy: RuntimeEnvironmentRefreshCachePolicy,
    pub activation_receipt: RuntimeEnvironmentActivationReceipt,
    pub elapsed_ms: u64,
    pub timeout_ms: u64,
}

impl RuntimeEnvironmentRefreshTimeout {
    pub fn new(
        activation_receipt: RuntimeEnvironmentActivationReceipt,
        elapsed_ms: u64,
        timeout_ms: u64,
    ) -> Self {
        Self {
            execution: RuntimeEnvironmentRefreshExecution::Foreground,
            cache_policy: RuntimeEnvironmentRefreshCachePolicy::ExternalToolOwned,
            activation_receipt,
            elapsed_ms,
            timeout_ms,
        }
    }

    pub fn with_execution(mut self, execution: RuntimeEnvironmentRefreshExecution) -> Self {
        self.execution = execution;
        self
    }

    pub fn with_cache_policy(mut self, cache_policy: RuntimeEnvironmentRefreshCachePolicy) -> Self {
        self.cache_policy = cache_policy;
        self
    }
}

impl RuntimeEnvironmentRefreshReceipt {
    pub fn from_activation(
        execution: RuntimeEnvironmentRefreshExecution,
        cache_policy: RuntimeEnvironmentRefreshCachePolicy,
        activation_receipt: RuntimeEnvironmentActivationReceipt,
    ) -> Self {
        let status = match &activation_receipt.status {
            RuntimeEnvironmentActivationStatus::Applied => RuntimeEnvironmentRefreshStatus::Applied,
            RuntimeEnvironmentActivationStatus::Rejected => {
                RuntimeEnvironmentRefreshStatus::Rejected
            }
            RuntimeEnvironmentActivationStatus::Disabled
            | RuntimeEnvironmentActivationStatus::Planned => {
                RuntimeEnvironmentRefreshStatus::Skipped
            }
        };
        let reason = activation_receipt.reason.clone();

        Self {
            execution,
            cache_policy,
            status,
            elapsed_ms: None,
            timeout_ms: None,
            activation_receipt,
            reason,
        }
    }

    pub fn timed_out(timeout: RuntimeEnvironmentRefreshTimeout) -> Self {
        let reason = timeout.activation_receipt.reason.clone();

        Self {
            execution: timeout.execution,
            cache_policy: timeout.cache_policy,
            status: RuntimeEnvironmentRefreshStatus::TimedOut,
            elapsed_ms: Some(timeout.elapsed_ms),
            timeout_ms: Some(timeout.timeout_ms),
            activation_receipt: timeout.activation_receipt,
            reason,
        }
    }

    pub fn with_timing(mut self, elapsed_ms: u64, timeout_ms: Option<u64>) -> Self {
        self.elapsed_ms = Some(elapsed_ms);
        self.timeout_ms = timeout_ms;
        self
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
    #[serde(default)]
    pub workspace_projects: Vec<RuntimeWorkspaceProject>,
    #[serde(default)]
    pub active_workspace_project: Option<RuntimeWorkspaceProjectId>,
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

    pub fn with_workspace_project(mut self, project: RuntimeWorkspaceProject) -> Self {
        self.workspace_projects.push(project);
        self
    }

    pub fn with_active_workspace_project(mut self, project_id: impl Into<String>) -> Self {
        self.active_workspace_project = Some(RuntimeWorkspaceProjectId::new(project_id));
        self
    }
}

/// Resolved runtime environment and the receipt for its activation policy.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeEnvironmentResolution {
    pub environment: RuntimeEnvironment,
    pub activation_receipt: RuntimeEnvironmentActivationReceipt,
    #[serde(default)]
    pub project_import_receipts: Vec<RuntimeWorkspaceProjectImportReceipt>,
}
