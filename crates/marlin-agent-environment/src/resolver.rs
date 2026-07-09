//! Resolves `RuntimeEnvironment` snapshots from home, config, sandbox, and sub-agent inputs.

use std::path::PathBuf;

use marlin_agent_protocol::{
    MARLIN_HOME_ENV_VAR, MARLIN_SESSION_ID_ENV_VAR, RuntimeConfigLayer, RuntimeConfigLayerSource,
    RuntimeEnvironment, RuntimeEnvironmentActivation, RuntimeEnvironmentActivationPolicy,
    RuntimeEnvironmentActivationReceipt, RuntimeEnvironmentResolution, RuntimeHome,
    RuntimeHomeSource, RuntimeSandboxPolicy, RuntimeSession, RuntimeSessionIdSource,
    RuntimeStateStorageReceipt, RuntimeWorkspaceProject, RuntimeWorkspaceProjectId,
    RuntimeWorkspaceProjectImportReceipt, RuntimeWorkspaceProjectTrust,
};
use thiserror::Error;

/// Precedence used for machine or installation-level configuration.
pub const SYSTEM_CONFIG_PRECEDENCE: i16 = 10;

/// Precedence used for user-level configuration.
pub const USER_CONFIG_PRECEDENCE: i16 = 20;

/// Precedence used for project-level configuration.
pub const PROJECT_CONFIG_PRECEDENCE: i16 = 40;

/// Precedence used for sub-agent-specific configuration overlays.
pub const SUB_AGENT_CONFIG_PRECEDENCE: i16 = 90;

/// Precedence used for explicit session flags.
pub const SESSION_FLAGS_CONFIG_PRECEDENCE: i16 = 100;

/// Host environment variable used to derive the default Marlin state home.
pub const HOST_HOME_ENV_VAR: &str = "HOME";

/// Resolves runtime environment inputs into immutable protocol snapshots.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RuntimeEnvironmentResolver;

impl RuntimeEnvironmentResolver {
    /// Creates a stateless resolver.
    pub fn new() -> Self {
        Self
    }

    /// Resolves a top-level runtime environment snapshot.
    pub fn resolve(&self, request: RuntimeEnvironmentRequest) -> RuntimeEnvironment {
        self.resolve_with_receipt(request).environment
    }

    /// Resolves a top-level runtime environment snapshot with activation receipt.
    pub fn resolve_with_receipt(
        &self,
        request: RuntimeEnvironmentRequest,
    ) -> RuntimeEnvironmentResolution {
        let mut environment = RuntimeEnvironment::default()
            .with_sandbox(request.sandbox.clone())
            .with_activation(request.activation.clone());
        let mut project_import_receipts = Vec::new();

        if let Some(session) = request.session.clone() {
            environment = environment.with_session(session);
        }

        if let Some(home) = request.resolve_home() {
            environment = environment.with_home(home);
        }

        if let Some(cwd) = request.cwd {
            environment = environment.with_cwd(cwd);
        }

        if let Some(file) = request.system_config {
            environment = environment.with_config_layer(RuntimeConfigLayer::new(
                RuntimeConfigLayerSource::System { file },
                SYSTEM_CONFIG_PRECEDENCE,
            ));
        }

        if let Some(file) = request.user_config {
            environment = environment.with_config_layer(RuntimeConfigLayer::new(
                RuntimeConfigLayerSource::User {
                    file,
                    profile: request.profile.clone(),
                },
                USER_CONFIG_PRECEDENCE,
            ));
        }

        if let Some(dot_marlin_folder) = request.project_config {
            environment = environment.with_config_layer(RuntimeConfigLayer::new(
                RuntimeConfigLayerSource::Project { dot_marlin_folder },
                PROJECT_CONFIG_PRECEDENCE,
            ));
        }

        for project in request.workspace_projects.clone() {
            if project.is_trusted() {
                project_import_receipts
                    .push(RuntimeWorkspaceProjectImportReceipt::imported(&project));
                environment.workspace_projects.push(project);
            } else {
                let reason = project_trust_rejection_reason(&project.trust);
                project_import_receipts.push(RuntimeWorkspaceProjectImportReceipt::rejected(
                    project.id, reason,
                ));
            }
        }
        if let Some(project_id) = request.active_workspace_project.clone() {
            let active_project = environment
                .workspace_projects
                .iter()
                .find(|project| project.id.as_str() == project_id.as_str())
                .cloned();
            match active_project {
                Some(project) => {
                    environment.active_workspace_project = Some(project_id.clone());
                    environment.cwd = Some(project.root.clone());
                    environment.sandbox = project.sandbox.clone();
                    environment.activation = project.activation.clone();
                    if let Some(dot_marlin_folder) = project.project_config.clone() {
                        environment = environment.with_config_layer(RuntimeConfigLayer::new(
                            RuntimeConfigLayerSource::Project { dot_marlin_folder },
                            PROJECT_CONFIG_PRECEDENCE,
                        ));
                    }
                }
                None => {
                    project_import_receipts.push(RuntimeWorkspaceProjectImportReceipt::rejected(
                        project_id,
                        "active workspace project was not imported",
                    ));
                }
            }
        }

        if request.session_flags {
            environment = environment.with_config_layer(RuntimeConfigLayer::new(
                RuntimeConfigLayerSource::SessionFlags,
                SESSION_FLAGS_CONFIG_PRECEDENCE,
            ));
        }

        sort_config_layers(&mut environment.config_layers);
        let activation_receipt = activation_receipt_for_policy(&environment.activation);
        let state_storage_receipt = environment
            .state_layout
            .as_ref()
            .map(RuntimeStateStorageReceipt::planned);

        RuntimeEnvironmentResolution {
            environment,
            activation_receipt,
            state_storage_receipt,
            project_import_receipts,
        }
    }

    /// Resolves a child environment for a sub-agent spawned from an existing parent snapshot.
    pub fn resolve_sub_agent(
        &self,
        parent: &RuntimeEnvironment,
        request: SubAgentEnvironmentRequest,
    ) -> Result<RuntimeEnvironment, RuntimeEnvironmentError> {
        let parent_home = parent
            .home
            .as_ref()
            .map(|home| home.path.clone())
            .ok_or(RuntimeEnvironmentError::MissingParentHome)?;
        let child_home = request.home_path.unwrap_or_else(|| {
            parent_home
                .join("sub-agents")
                .join(agent_home_slug(&request.agent_reference))
        });
        let child_profile = request.profile.or_else(|| {
            parent
                .home
                .as_ref()
                .and_then(|home| home.profile.as_ref().cloned())
        });

        let mut environment = parent.clone();
        environment.home = Some(RuntimeHome {
            path: child_home,
            source: RuntimeHomeSource::InheritedSubAgent {
                parent_home: parent_home.clone(),
            },
            profile: child_profile,
        });

        if let Some(cwd) = request.cwd {
            environment.cwd = Some(cwd);
        }

        if let Some(sandbox) = request.sandbox {
            environment.sandbox = sandbox;
        }

        environment.config_layers.push(RuntimeConfigLayer::new(
            RuntimeConfigLayerSource::SubAgent {
                agent_reference: request.agent_reference,
            },
            SUB_AGENT_CONFIG_PRECEDENCE,
        ));
        sort_config_layers(&mut environment.config_layers);

        Ok(environment)
    }
}

fn project_trust_rejection_reason(trust: &RuntimeWorkspaceProjectTrust) -> &'static str {
    match trust {
        RuntimeWorkspaceProjectTrust::Trusted => "workspace project is trusted",
        RuntimeWorkspaceProjectTrust::ReviewRequired => {
            "workspace project requires trust review before import"
        }
        RuntimeWorkspaceProjectTrust::Denied => "workspace project trust was denied",
    }
}

fn activation_receipt_for_policy(
    policy: &RuntimeEnvironmentActivationPolicy,
) -> RuntimeEnvironmentActivationReceipt {
    match policy.activation {
        RuntimeEnvironmentActivation::Disabled => {
            RuntimeEnvironmentActivationReceipt::disabled(policy)
        }
        RuntimeEnvironmentActivation::Direnv { .. } => {
            RuntimeEnvironmentActivationReceipt::planned(policy)
        }
    }
}

fn resolve_session_from_host_env(marlin_session_id: Option<String>) -> Option<RuntimeSession> {
    marlin_session_id
        .and_then(|id| RuntimeSession::try_new(id, RuntimeSessionIdSource::MarlinSessionEnv))
}

/// Input used to resolve a top-level runtime environment.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RuntimeEnvironmentRequest {
    /// Optional default runtime home path.
    pub default_home: Option<PathBuf>,
    /// Optional custom runtime home path. When present, this wins over the default home.
    pub custom_home: Option<PathBuf>,
    /// Optional runtime session identity.
    pub session: Option<RuntimeSession>,
    /// Optional named profile used by home and user config layers.
    pub profile: Option<String>,
    /// Optional working directory attached to the runtime snapshot.
    pub cwd: Option<PathBuf>,
    /// Sandbox policy visible to runtime-owned work.
    pub sandbox: RuntimeSandboxPolicy,
    /// Explicit shell/environment activation policy.
    pub activation: RuntimeEnvironmentActivationPolicy,
    /// Optional system config file.
    pub system_config: Option<PathBuf>,
    /// Optional user config file.
    pub user_config: Option<PathBuf>,
    /// Optional project `.marlin` config folder.
    pub project_config: Option<PathBuf>,
    /// Projects imported into this runtime workspace.
    pub workspace_projects: Vec<RuntimeWorkspaceProject>,
    /// Project selected as the active runtime project.
    pub active_workspace_project: Option<RuntimeWorkspaceProjectId>,
    /// Whether explicit session flags should be represented as the top config layer.
    pub session_flags: bool,
}

impl RuntimeEnvironmentRequest {
    /// Sets the default runtime home path.
    pub fn with_default_home(mut self, path: impl Into<PathBuf>) -> Self {
        self.default_home = Some(path.into());
        self
    }

    /// Sets the default runtime home to `<user_home>/.marlin`.
    pub fn with_default_marlin_home(mut self, user_home: impl Into<PathBuf>) -> Self {
        self.default_home = Some(RuntimeHome::default_for_user_home(user_home).path);
        self
    }

    /// Resolves runtime home and session identity from host environment pairs.
    ///
    /// `MARLIN_HOME` wins as a custom home. If it is absent, `HOME` resolves
    /// the default `<HOME>/.marlin` runtime state home. `MARLIN_SESSION_ID`
    /// resolves the runtime session id when present.
    pub fn with_home_from_host_env<I, K, V>(mut self, env: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let mut marlin_home = None;
        let mut user_home = None;
        let mut marlin_session_id = None;
        for (key, value) in env {
            let value = value.as_ref();
            if value.trim().is_empty() {
                continue;
            }
            match key.as_ref() {
                MARLIN_HOME_ENV_VAR => marlin_home = Some(PathBuf::from(value)),
                HOST_HOME_ENV_VAR => user_home = Some(PathBuf::from(value)),
                MARLIN_SESSION_ID_ENV_VAR => marlin_session_id = Some(value.to_owned()),
                _ => {}
            }
        }
        if let Some(path) = marlin_home {
            self.custom_home = Some(path);
        } else if let Some(path) = user_home {
            self.default_home = Some(RuntimeHome::default_for_user_home(path).path);
        }
        if let Some(session) = resolve_session_from_host_env(marlin_session_id) {
            self.session = Some(session);
        }
        self
    }

    /// Sets an explicit runtime session id.
    pub fn with_session_id(mut self, id: impl Into<String>) -> Self {
        self.session = Some(RuntimeSession::explicit(id.into()));
        self
    }

    /// Sets an already typed runtime session.
    pub fn with_session(mut self, session: RuntimeSession) -> Self {
        self.session = Some(session);
        self
    }

    /// Sets a custom runtime home path.
    pub fn with_custom_home(mut self, path: impl Into<PathBuf>) -> Self {
        self.custom_home = Some(path.into());
        self
    }

    /// Sets the named runtime profile.
    pub fn with_profile(mut self, profile: impl Into<String>) -> Self {
        self.profile = Some(profile.into());
        self
    }

    /// Sets the runtime working directory.
    pub fn with_cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    /// Sets the runtime sandbox policy.
    pub fn with_sandbox(mut self, sandbox: RuntimeSandboxPolicy) -> Self {
        self.sandbox = sandbox;
        self
    }

    /// Sets the explicit shell/environment activation policy.
    pub fn with_activation(mut self, activation: RuntimeEnvironmentActivationPolicy) -> Self {
        self.activation = activation;
        self
    }

    /// Adds a system config file.
    pub fn with_system_config(mut self, file: impl Into<PathBuf>) -> Self {
        self.system_config = Some(file.into());
        self
    }

    /// Adds a user config file.
    pub fn with_user_config(mut self, file: impl Into<PathBuf>) -> Self {
        self.user_config = Some(file.into());
        self
    }

    /// Adds a project `.marlin` config folder.
    pub fn with_project_config(mut self, dot_marlin_folder: impl Into<PathBuf>) -> Self {
        self.project_config = Some(dot_marlin_folder.into());
        self
    }

    /// Imports a project into the runtime workspace.
    pub fn with_workspace_project(mut self, project: RuntimeWorkspaceProject) -> Self {
        self.workspace_projects.push(project);
        self
    }

    /// Selects one imported workspace project as the active runtime project.
    pub fn with_active_workspace_project(mut self, project_id: impl Into<String>) -> Self {
        self.active_workspace_project = Some(RuntimeWorkspaceProjectId::new(project_id));
        self
    }

    /// Marks explicit session flags as present.
    pub fn with_session_flags(mut self) -> Self {
        self.session_flags = true;
        self
    }

    fn resolve_home(&self) -> Option<RuntimeHome> {
        if let Some(path) = self.custom_home.clone() {
            return Some(RuntimeHome {
                path,
                source: RuntimeHomeSource::Custom,
                profile: self.profile.clone(),
            });
        }

        self.default_home.clone().map(|path| RuntimeHome {
            path,
            source: RuntimeHomeSource::Default,
            profile: self.profile.clone(),
        })
    }
}

/// Input used to resolve a sub-agent child runtime environment.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubAgentEnvironmentRequest {
    /// Stable sub-agent reference used in config layer receipts.
    pub agent_reference: String,
    /// Optional explicit child home path.
    pub home_path: Option<PathBuf>,
    /// Optional child working directory override.
    pub cwd: Option<PathBuf>,
    /// Optional child profile override.
    pub profile: Option<String>,
    /// Optional child sandbox override.
    pub sandbox: Option<RuntimeSandboxPolicy>,
}

impl SubAgentEnvironmentRequest {
    /// Creates a sub-agent environment request.
    pub fn new(agent_reference: impl Into<String>) -> Self {
        Self {
            agent_reference: agent_reference.into(),
            home_path: None,
            cwd: None,
            profile: None,
            sandbox: None,
        }
    }

    /// Sets an explicit child home path.
    pub fn with_home_path(mut self, home_path: impl Into<PathBuf>) -> Self {
        self.home_path = Some(home_path.into());
        self
    }

    /// Sets a child working directory override.
    pub fn with_cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    /// Sets a child profile override.
    pub fn with_profile(mut self, profile: impl Into<String>) -> Self {
        self.profile = Some(profile.into());
        self
    }

    /// Sets a child sandbox override.
    pub fn with_sandbox(mut self, sandbox: RuntimeSandboxPolicy) -> Self {
        self.sandbox = Some(sandbox);
        self
    }
}

/// Error raised when environment resolution cannot produce a valid snapshot.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum RuntimeEnvironmentError {
    /// A sub-agent environment needs a parent home to derive its inherited home.
    #[error("sub-agent environment requires a parent runtime home")]
    MissingParentHome,
}

fn sort_config_layers(layers: &mut [RuntimeConfigLayer]) {
    layers.sort_by_key(|layer| layer.precedence);
}

fn agent_home_slug(agent_reference: &str) -> String {
    let slug: String = agent_reference
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = slug.trim_matches(['.', '-']);

    if trimmed.is_empty() {
        "agent".to_owned()
    } else {
        trimmed.to_owned()
    }
}
