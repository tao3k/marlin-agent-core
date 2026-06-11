//! Serializable event contracts emitted by agent-owned execution.

use serde::{Deserialize, Serialize};

/// Serializable runtime event emitted by agent-owned execution.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentEvent {
    pub topic: String,
    pub message: String,
}

impl AgentEvent {
    pub fn new(topic: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            topic: topic.into(),
            message: message.into(),
        }
    }
}
