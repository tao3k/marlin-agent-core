//! Configuration loader for runtime model routing.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    fs, io,
    path::{Path, PathBuf},
};

use marlin_agent_protocol::{ModelEndpointContractError, ModelRouteRule, ModelRouteRuleId};
use serde::{Deserialize, Serialize};

use super::resolver::{CompiledModelRouteResolver, ModelRouteCompileError};

/// Runtime model route configuration.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ModelRouteConfig {
    pub rules: Vec<ModelRouteRule>,
}

impl ModelRouteConfig {
    pub fn new(rules: Vec<ModelRouteRule>) -> Self {
        Self { rules }
    }

    pub fn from_toml_str(source: &str) -> Result<Self, ModelRouteConfigError> {
        let config: Self = toml::from_str(source).map_err(ModelRouteConfigError::Toml)?;
        config.validate_endpoint_contracts()?;
        Ok(config)
    }

    pub fn from_toml_path(path: impl AsRef<Path>) -> Result<Self, ModelRouteConfigError> {
        let path = path.as_ref();
        let source = fs::read_to_string(path).map_err(|source| ModelRouteConfigError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        Self::from_toml_str(&source)
    }

    pub fn rules(&self) -> &[ModelRouteRule] {
        self.rules.as_slice()
    }

    pub fn into_rules(self) -> Vec<ModelRouteRule> {
        self.rules
    }

    pub fn validate_endpoint_contracts(&self) -> Result<(), ModelRouteConfigError> {
        for rule in &self.rules {
            rule.validate_endpoint_contract().map_err(|source| {
                ModelRouteConfigError::EndpointContract {
                    rule_id: rule.rule_id.clone(),
                    source,
                }
            })?;
        }
        Ok(())
    }

    pub fn compile_resolver(&self) -> Result<CompiledModelRouteResolver, ModelRouteConfigError> {
        self.validate_endpoint_contracts()?;
        CompiledModelRouteResolver::new(self.rules.clone()).map_err(ModelRouteConfigError::Compile)
    }

    pub fn into_resolver(self) -> Result<CompiledModelRouteResolver, ModelRouteConfigError> {
        self.validate_endpoint_contracts()?;
        CompiledModelRouteResolver::new(self.rules).map_err(ModelRouteConfigError::Compile)
    }
}

/// Error raised while loading or compiling model route configuration.
#[derive(Debug)]
pub enum ModelRouteConfigError {
    Io {
        path: PathBuf,
        source: io::Error,
    },
    Toml(toml::de::Error),
    EndpointContract {
        rule_id: ModelRouteRuleId,
        source: ModelEndpointContractError,
    },
    Compile(ModelRouteCompileError),
}

impl Display for ModelRouteConfigError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(
                    formatter,
                    "failed to read model route config `{}`: {source}",
                    path.display()
                )
            }
            Self::Toml(error) => write!(formatter, "failed to parse model route TOML: {error}"),
            Self::EndpointContract { rule_id, source } => write!(
                formatter,
                "model route rule `{rule_id}` violates endpoint contract: {source}"
            ),
            Self::Compile(error) => {
                write!(formatter, "failed to compile model route config: {error}")
            }
        }
    }
}

impl Error for ModelRouteConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Toml(error) => Some(error),
            Self::EndpointContract { source, .. } => Some(source),
            Self::Compile(error) => Some(error),
        }
    }
}
