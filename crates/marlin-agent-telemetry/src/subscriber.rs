// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Local tracing subscriber configuration and receipts.

use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};
use tracing_subscriber::EnvFilter;

/// Local tracing output format.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum TracingSubscriberFormat {
    #[default]
    Compact,
    Json,
}

impl TracingSubscriberFormat {
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
pub enum TracingSubscriberScope {
    Validated,
    ScopedDefault,
}

impl TracingSubscriberScope {
    /// Stable receipt label for this subscriber scope.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Validated => "validated",
            Self::ScopedDefault => "scoped-default",
        }
    }
}

/// Local tracing subscriber configuration.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TracingSubscriberConfig {
    env_filter: String,
    format: TracingSubscriberFormat,
    ansi: bool,
    with_target: bool,
    with_thread_ids: bool,
}

impl Default for TracingSubscriberConfig {
    fn default() -> Self {
        Self {
            env_filter: "marlin_agent=info,marlin=info,warn".to_owned(),
            format: TracingSubscriberFormat::Compact,
            ansi: false,
            with_target: true,
            with_thread_ids: false,
        }
    }
}

impl TracingSubscriberConfig {
    /// Creates a subscriber config using Marlin defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Replaces the tracing EnvFilter directive.
    pub fn with_env_filter(mut self, env_filter: impl Into<String>) -> Self {
        self.env_filter = env_filter.into();
        self
    }

    /// Replaces the output format.
    pub fn with_format(mut self, format: TracingSubscriberFormat) -> Self {
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
    pub fn format(&self) -> TracingSubscriberFormat {
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
    pub fn validate(&self) -> Result<TracingSubscriberReceipt, TracingSubscriberConfigError> {
        self.parse_env_filter()?;
        Ok(TracingSubscriberReceipt::from_config(
            self,
            TracingSubscriberScope::Validated,
        ))
    }

    /// Runs a closure under a scoped default subscriber without installing a process-global subscriber.
    pub fn with_scoped_default<T>(
        &self,
        f: impl FnOnce(&TracingSubscriberReceipt) -> T,
    ) -> Result<T, TracingSubscriberConfigError> {
        let receipt =
            TracingSubscriberReceipt::from_config(self, TracingSubscriberScope::ScopedDefault);

        match self.format {
            TracingSubscriberFormat::Compact => {
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
            TracingSubscriberFormat::Json => {
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

    fn parse_env_filter(&self) -> Result<EnvFilter, TracingSubscriberConfigError> {
        EnvFilter::try_new(self.env_filter.as_str()).map_err(|source| {
            TracingSubscriberConfigError::InvalidEnvFilter {
                env_filter: self.env_filter.clone(),
                message: source.to_string(),
            }
        })
    }
}

/// Receipt proving the tracing subscriber config was validated or scoped.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TracingSubscriberReceipt {
    env_filter: String,
    format: TracingSubscriberFormat,
    ansi: bool,
    with_target: bool,
    with_thread_ids: bool,
    scope: TracingSubscriberScope,
}

impl TracingSubscriberReceipt {
    fn from_config(config: &TracingSubscriberConfig, scope: TracingSubscriberScope) -> Self {
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
    pub fn format(&self) -> TracingSubscriberFormat {
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
    pub fn scope(&self) -> TracingSubscriberScope {
        self.scope
    }
}

/// Invalid tracing subscriber configuration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TracingSubscriberConfigError {
    InvalidEnvFilter { env_filter: String, message: String },
}

impl fmt::Display for TracingSubscriberConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidEnvFilter {
                env_filter,
                message,
            } => write!(
                formatter,
                "invalid tracing EnvFilter `{env_filter}`: {message}"
            ),
        }
    }
}

impl Error for TracingSubscriberConfigError {}
