//! Serializable tracing contracts emitted by agent-owned execution.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

/// Stable tracing span name identifier.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct AgentSpanName(String);

impl AgentSpanName {
    /// Creates a tracing span name identifier.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the span name as text.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Converts the span name into its owned string.
    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Display for AgentSpanName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl From<String> for AgentSpanName {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for AgentSpanName {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Captured tracing span metadata for one span creation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentTraceSpanRecord {
    /// Stable span name, such as `agent.provider` or `hook.dispatch`.
    pub name: AgentSpanName,
    /// Field values recorded when the span was created.
    pub fields: BTreeMap<String, String>,
}

impl AgentTraceSpanRecord {
    /// Creates a span record with no captured fields.
    pub fn new(name: impl Into<AgentSpanName>) -> Self {
        Self {
            name: name.into(),
            fields: BTreeMap::new(),
        }
    }

    /// Adds one captured field value to this span record.
    pub fn with_field(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.fields.insert(key.into(), value.into());
        self
    }
}
