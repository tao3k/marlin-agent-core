//! Serializable event contracts emitted by agent-owned execution.

use serde::{Deserialize, Serialize};

/// Stable runtime event topic identifier.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct AgentEventTopic(String);

impl AgentEventTopic {
    /// Creates an event topic identifier.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the event topic as text.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Converts the event topic into its owned string.
    pub fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for AgentEventTopic {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for AgentEventTopic {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Serializable runtime event emitted by agent-owned execution.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentEvent {
    pub topic: String,
    pub message: String,
}

impl AgentEvent {
    pub fn new(topic: impl Into<AgentEventTopic>, message: impl Into<String>) -> Self {
        let topic = topic.into();
        Self {
            topic: topic.into_string(),
            message: message.into(),
        }
    }

    /// Returns this event's topic as a typed protocol identifier.
    pub fn topic_id(&self) -> AgentEventTopic {
        AgentEventTopic::new(self.topic.clone())
    }
}
