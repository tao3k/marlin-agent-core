//! Resolves `RuntimeEnvironment` snapshots from home, config, sandbox, and sub-agent inputs.

use std::path::PathBuf;

use marlin_agent_protocol::{
    RuntimeConfigLayer, RuntimeConfigLayerSource, RuntimeEnvironment, RuntimeHome,
    RuntimeHomeSource, RuntimeSandboxPolicy,
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
        let mut environment = RuntimeEnvironment::default().with_sandbox(request.sandbox.clone());

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

        if request.session_flags {
            environment = environment.with_config_layer(RuntimeConfigLayer::new(
                RuntimeConfigLayerSource::SessionFlags,
                SESSION_FLAGS_CONFIG_PRECEDENCE,
            ));
        }

        sort_config_layers(&mut environment.config_layers);
        environment
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

/// Input used to resolve a top-level runtime environment.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RuntimeEnvironmentRequest {
    /// Optional default runtime home path.
    pub default_home: Option<PathBuf>,
    /// Optional custom runtime home path. When present, this wins over the default home.
    pub custom_home: Option<PathBuf>,
    /// Optional named profile used by home and user config layers.
    pub profile: Option<String>,
    /// Optional working directory attached to the runtime snapshot.
    pub cwd: Option<PathBuf>,
    /// Sandbox policy visible to runtime-owned work.
    pub sandbox: RuntimeSandboxPolicy,
    /// Optional system config file.
    pub system_config: Option<PathBuf>,
    /// Optional user config file.
    pub user_config: Option<PathBuf>,
    /// Optional project `.marlin` config folder.
    pub project_config: Option<PathBuf>,
    /// Whether explicit session flags should be represented as the top config layer.
    pub session_flags: bool,
}

impl RuntimeEnvironmentRequest {
    /// Sets the default runtime home path.
    pub fn with_default_home(mut self, path: impl Into<PathBuf>) -> Self {
        self.default_home = Some(path.into());
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
