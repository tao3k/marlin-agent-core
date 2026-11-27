// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! `TOML` configuration envelope for hook dispatch policy and registration defaults.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    fs::File,
    io::{self, Read},
    path::{Path, PathBuf},
};

use marlin_agent_protocol::{
    HookAgentScope, HookConfigurationReloadReceipt, HookConfigurationVersion, HookPolicyExtension,
    HookPolicyExtensionKind, HookPolicyMode, HookScope, HookSource, HookTrustStatus,
};
use serde::{Deserialize, Serialize};

use crate::{HookDispatchPolicy, HookDispatcher, HookRegistration, HookRegistry};

/// Maximum `TOML` hook configuration file size accepted from disk (256 KiB).
pub const MAX_HOOK_CONFIG_BYTES: usize = 256 * 1024;

const MAX_EXTENSION_IDENTIFIER_BYTES: usize = 256;

/// Hook configuration envelope loaded from `TOML`.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HookConfigurationEnvelope {
    pub version: Option<HookConfigurationVersion>,
    pub policy: HookPolicyConfiguration,
    pub registration_defaults: HookRegistrationDefaults,
}

impl HookConfigurationEnvelope {
    /// Creates a configuration envelope from typed parts.
    pub fn new(
        version: Option<HookConfigurationVersion>,
        policy: HookPolicyConfiguration,
        registration_defaults: HookRegistrationDefaults,
    ) -> Self {
        Self {
            version,
            policy,
            registration_defaults,
        }
    }

    /// Loads a hook configuration envelope from a `TOML` string.
    pub fn from_toml_str(source: &str) -> Result<Self, HookConfigurationError> {
        let envelope: Self = toml::from_str(source).map_err(HookConfigurationError::Toml)?;
        validate_policy_extension(&envelope.policy.extension)?;
        Ok(envelope)
    }

    /// Loads a hook configuration envelope from a `TOML` file.
    pub fn from_toml_path(path: impl AsRef<Path>) -> Result<Self, HookConfigurationError> {
        let path = path.as_ref();
        let file = File::open(path).map_err(|source| HookConfigurationError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let mut source = String::new();
        file.take((MAX_HOOK_CONFIG_BYTES + 1) as u64)
            .read_to_string(&mut source)
            .map_err(|source| HookConfigurationError::Io {
                path: path.to_path_buf(),
                source,
            })?;
        if source.len() > MAX_HOOK_CONFIG_BYTES {
            return Err(HookConfigurationError::ConfigTooLarge {
                path: path.to_path_buf(),
                limit: MAX_HOOK_CONFIG_BYTES,
            });
        }
        Self::from_toml_str(&source)
    }

    /// Builds the dispatcher policy declared by this envelope.
    pub fn dispatch_policy(&self) -> HookDispatchPolicy {
        self.policy.to_dispatch_policy()
    }

    /// Applies configured registration defaults to one hook registration.
    pub fn apply_registration_defaults(&self, registration: HookRegistration) -> HookRegistration {
        self.registration_defaults
            .apply_to_registration(registration)
    }

    /// Creates a dispatcher from a registry and the configured policy.
    pub fn into_dispatcher(self, registry: HookRegistry) -> HookDispatcher {
        HookDispatcher::new(registry).with_policy(self.policy.to_dispatch_policy())
    }

    /// Builds a typed receipt for reloading this `TOML` envelope.
    pub fn reload_receipt(&self, registry: &HookRegistry) -> HookConfigurationReloadReceipt {
        HookConfigurationReloadReceipt {
            configuration_version: self.version.clone(),
            policy_mode: self.policy.mode.clone(),
            policy_extension: self.policy.extension.clone(),
            registration_default_scope: self.registration_defaults.scope.clone(),
            registration_default_agent_scope: self.registration_defaults.agent_scope.clone(),
            registration_count: registry.registrations().len(),
        }
    }
}

/// Hook dispatch policy configuration.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HookPolicyConfiguration {
    pub mode: HookPolicyMode,
    pub extension: HookPolicyExtension,
}

impl HookPolicyConfiguration {
    /// Creates hook policy configuration with a specific mode.
    pub fn new(mode: HookPolicyMode) -> Self {
        Self {
            mode,
            extension: HookPolicyExtension::none(),
        }
    }

    /// Adds a complex policy extension boundary.
    pub fn with_extension(mut self, extension: HookPolicyExtension) -> Self {
        self.extension = extension;
        self
    }

    /// Converts this configuration into a runtime dispatch policy.
    pub fn to_dispatch_policy(&self) -> HookDispatchPolicy {
        HookDispatchPolicy::from_mode(self.mode.clone()).with_extension(self.extension.clone())
    }
}

impl Default for HookPolicyConfiguration {
    fn default() -> Self {
        Self {
            mode: HookPolicyMode::ObserveOnly,
            extension: HookPolicyExtension::none(),
        }
    }
}

/// Defaults assigned to registrations created from configuration.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HookRegistrationDefaults {
    pub scope: HookScope,
    pub agent_scope: HookAgentScope,
    pub source: HookSource,
    pub trust: HookTrustStatus,
}

impl HookRegistrationDefaults {
    /// Creates registration defaults from typed hook metadata.
    pub fn new(
        scope: HookScope,
        agent_scope: HookAgentScope,
        source: HookSource,
        trust: HookTrustStatus,
    ) -> Self {
        Self {
            scope,
            agent_scope,
            source,
            trust,
        }
    }

    /// Applies registration defaults to a runtime hook registration.
    pub fn apply_to_registration(&self, registration: HookRegistration) -> HookRegistration {
        registration
            .with_scope(self.scope.clone())
            .with_agent_scope(self.agent_scope.clone())
            .with_source(self.source.clone())
            .with_trust(self.trust.clone())
    }
}

impl Default for HookRegistrationDefaults {
    fn default() -> Self {
        Self {
            scope: HookScope::Turn,
            agent_scope: HookAgentScope::Any,
            source: HookSource::Unknown,
            trust: HookTrustStatus::Untrusted,
        }
    }
}

/// Error raised while loading hook configuration.
#[derive(Debug)]
pub enum HookConfigurationError {
    Io { path: PathBuf, source: io::Error },
    ConfigTooLarge { path: PathBuf, limit: usize },
    Toml(toml::de::Error),
    InvalidExtension { reason: String },
}

impl Display for HookConfigurationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(
                    formatter,
                    "failed to read hook config `{}`: {source}",
                    path.display()
                )
            }
            Self::ConfigTooLarge { path, limit } => {
                write!(
                    formatter,
                    "hook config `{}` exceeds maximum allowed size of {limit} bytes",
                    path.display()
                )
            }
            Self::Toml(error) => write!(formatter, "failed to parse hook `TOML`: {error}"),
            Self::InvalidExtension { reason } => {
                write!(formatter, "invalid hook policy extension: {reason}")
            }
        }
    }
}

impl Error for HookConfigurationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Toml(error) => Some(error),
            Self::ConfigTooLarge { .. } | Self::InvalidExtension { .. } => None,
        }
    }
}

fn validate_policy_extension(
    extension: &HookPolicyExtension,
) -> Result<(), HookConfigurationError> {
    match extension.kind {
        HookPolicyExtensionKind::None => {
            if extension.module.is_some() || extension.procedure.is_some() {
                return Err(HookConfigurationError::InvalidExtension {
                    reason:
                        "policy extension kind `None` must not declare `module` or `procedure`"
                            .to_owned(),
                });
            }
        }
        HookPolicyExtensionKind::GerbilScheme => {
            let Some(module) = extension.module.as_ref() else {
                return Err(HookConfigurationError::InvalidExtension {
                    reason: "GerbilScheme policy extension requires a `module`".to_owned(),
                });
            };
            let Some(procedure) = extension.procedure.as_ref() else {
                return Err(HookConfigurationError::InvalidExtension {
                    reason: "GerbilScheme policy extension requires a `procedure`".to_owned(),
                });
            };
            validate_extension_module(module.as_str())?;
            validate_extension_procedure(procedure.as_str())?;
        }
    }
    Ok(())
}

fn validate_extension_module(module: &str) -> Result<(), HookConfigurationError> {
    if module.is_empty()
        || module != module.trim()
        || module.len() > MAX_EXTENSION_IDENTIFIER_BYTES
        || module.starts_with('/')
        || module.contains("..")
        || module
            .chars()
            .any(|ch| ch.is_control() || ch.is_whitespace() || ch == '\\')
    {
        return Err(HookConfigurationError::InvalidExtension {
            reason: format!("invalid GerbilScheme module path `{module}`"),
        });
    }
    Ok(())
}

fn validate_extension_procedure(procedure: &str) -> Result<(), HookConfigurationError> {
    if procedure.is_empty()
        || procedure != procedure.trim()
        || procedure.len() > MAX_EXTENSION_IDENTIFIER_BYTES
        || procedure.contains("..")
        || procedure
            .chars()
            .any(|ch| ch.is_control() || ch.is_whitespace() || matches!(ch, '/' | '\\'))
    {
        return Err(HookConfigurationError::InvalidExtension {
            reason: format!("invalid GerbilScheme procedure name `{procedure}`"),
        });
    }
    Ok(())
}
