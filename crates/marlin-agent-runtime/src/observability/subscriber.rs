//! Runtime-owned tracing subscriber configuration and receipts.

use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};
use tracing_subscriber::EnvFilter;

/// Runtime tracing output format.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum RuntimeTracingSubscriberFormat {
    #[default]
    Compact,
    Json,
}

impl RuntimeTracingSubscriberFormat {
    /// Stable receipt label for this output format.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Compact => "compact",
            Self::Json => "json",
        }
    }
}

/// Scope for a tracing subscriber receipt.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RuntimeTracingSubscriberScope {
    Validated,
    ScopedDefault,
}

impl RuntimeTracingSubscriberScope {
    /// Stable receipt label for this subscriber scope.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Validated => "validated",
            Self::ScopedDefault => "scoped-default",
        }
    }
}

/// Runtime tracing subscriber configuration.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeTracingSubscriberConfig {
    env_filter: String,
    format: RuntimeTracingSubscriberFormat,
    ansi: bool,
    with_target: bool,
    with_thread_ids: bool,
}

impl Default for RuntimeTracingSubscriberConfig {
    fn default() -> Self {
        Self {
            env_filter: "marlin_agent=info,marlin=info,warn".to_owned(),
            format: RuntimeTracingSubscriberFormat::Compact,
            ansi: false,
            with_target: true,
            with_thread_ids: false,
        }
    }
}

impl RuntimeTracingSubscriberConfig {
    /// Creates a subscriber config using runtime defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Replaces the tracing EnvFilter directive.
    pub fn with_env_filter(mut self, env_filter: impl Into<String>) -> Self {
        self.env_filter = env_filter.into();
        self
    }

    /// Replaces the output format.
    pub fn with_format(mut self, format: RuntimeTracingSubscriberFormat) -> Self {
        self.format = format;
        self
    }

    /// Enables or disables ANSI escape codes.
    pub fn with_ansi(mut self, ansi: bool) -> Self {
        self.ansi = ansi;
        self
    }

    /// Enables or disables tracing target output.
    pub fn with_target(mut self, with_target: bool) -> Self {
        self.with_target = with_target;
        self
    }

    /// Enables or disables thread id output.
    pub fn with_thread_ids(mut self, with_thread_ids: bool) -> Self {
        self.with_thread_ids = with_thread_ids;
        self
    }

    /// Returns the EnvFilter directive string.
    pub fn env_filter(&self) -> &str {
        self.env_filter.as_str()
    }

    /// Returns the configured output format.
    pub fn format(&self) -> RuntimeTracingSubscriberFormat {
        self.format
    }

    /// Whether ANSI output is enabled.
    pub fn ansi(&self) -> bool {
        self.ansi
    }

    /// Whether target output is enabled.
    pub fn target_enabled(&self) -> bool {
        self.with_target
    }

    /// Whether thread id output is enabled.
    pub fn thread_ids_enabled(&self) -> bool {
        self.with_thread_ids
    }

    /// Validates the config and returns a typed receipt.
    pub fn validate(
        &self,
    ) -> Result<RuntimeTracingSubscriberReceipt, RuntimeTracingSubscriberConfigError> {
        self.parse_env_filter()?;
        Ok(RuntimeTracingSubscriberReceipt::from_config(
            self,
            RuntimeTracingSubscriberScope::Validated,
        ))
    }

    /// Runs a closure under a scoped default subscriber without installing a process-global subscriber.
    pub fn with_scoped_default<T>(
        &self,
        f: impl FnOnce(&RuntimeTracingSubscriberReceipt) -> T,
    ) -> Result<T, RuntimeTracingSubscriberConfigError> {
        let receipt = RuntimeTracingSubscriberReceipt::from_config(
            self,
            RuntimeTracingSubscriberScope::ScopedDefault,
        );

        match self.format {
            RuntimeTracingSubscriberFormat::Compact => {
                let subscriber = tracing_subscriber::fmt()
                    .compact()
                    .with_env_filter(self.parse_env_filter()?)
                    .with_ansi(self.ansi)
                    .with_target(self.with_target)
                    .with_thread_ids(self.with_thread_ids)
                    .finish();
                Ok(tracing::subscriber::with_default(subscriber, || {
                    f(&receipt)
                }))
            }
            RuntimeTracingSubscriberFormat::Json => {
                let subscriber = tracing_subscriber::fmt()
                    .json()
                    .with_env_filter(self.parse_env_filter()?)
                    .with_ansi(self.ansi)
                    .with_target(self.with_target)
                    .with_thread_ids(self.with_thread_ids)
                    .finish();
                Ok(tracing::subscriber::with_default(subscriber, || {
                    f(&receipt)
                }))
            }
        }
    }

    fn parse_env_filter(&self) -> Result<EnvFilter, RuntimeTracingSubscriberConfigError> {
        EnvFilter::try_new(self.env_filter.as_str()).map_err(|source| {
            RuntimeTracingSubscriberConfigError::InvalidEnvFilter {
                env_filter: self.env_filter.clone(),
                message: source.to_string(),
            }
        })
    }
}

/// Receipt proving the runtime subscriber config was validated or scoped.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeTracingSubscriberReceipt {
    env_filter: String,
    format: RuntimeTracingSubscriberFormat,
    ansi: bool,
    with_target: bool,
    with_thread_ids: bool,
    scope: RuntimeTracingSubscriberScope,
}

impl RuntimeTracingSubscriberReceipt {
    fn from_config(
        config: &RuntimeTracingSubscriberConfig,
        scope: RuntimeTracingSubscriberScope,
    ) -> Self {
        Self {
            env_filter: config.env_filter.clone(),
            format: config.format,
            ansi: config.ansi,
            with_target: config.with_target,
            with_thread_ids: config.with_thread_ids,
            scope,
        }
    }

    /// EnvFilter directive validated for this subscriber.
    pub fn env_filter(&self) -> &str {
        self.env_filter.as_str()
    }

    /// Output format.
    pub fn format(&self) -> RuntimeTracingSubscriberFormat {
        self.format
    }

    /// Whether ANSI output is enabled.
    pub fn ansi(&self) -> bool {
        self.ansi
    }

    /// Whether tracing target output is enabled.
    pub fn target_enabled(&self) -> bool {
        self.with_target
    }

    /// Whether thread id output is enabled.
    pub fn thread_ids_enabled(&self) -> bool {
        self.with_thread_ids
    }

    /// Scope for this receipt.
    pub fn scope(&self) -> RuntimeTracingSubscriberScope {
        self.scope
    }
}

/// Invalid tracing subscriber configuration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeTracingSubscriberConfigError {
    InvalidEnvFilter { env_filter: String, message: String },
}

impl fmt::Display for RuntimeTracingSubscriberConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidEnvFilter {
                env_filter,
                message,
            } => write!(
                formatter,
                "invalid runtime tracing EnvFilter `{env_filter}`: {message}"
            ),
        }
    }
}

impl Error for RuntimeTracingSubscriberConfigError {}
