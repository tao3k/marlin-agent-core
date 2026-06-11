//! Sub-agent source and activity protocol contracts.

use serde::{Deserialize, Serialize};

use crate::RunId;

/// Source that caused a sub-agent to run.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SubAgentSource {
    Review,
    Compact,
    ThreadSpawn {
        parent_run_id: Option<RunId>,
        depth: u32,
        agent_path: Option<String>,
        agent_nickname: Option<String>,
        agent_role: Option<String>,
    },
    MemoryConsolidation,
    Other(String),
}

/// Activity state emitted for a sub-agent.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum SubAgentActivityKind {
    Started,
    Interacted,
    Interrupted,
    Stopped,
}

/// Typed activity notification for a sub-agent runtime.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SubAgentActivity {
    pub agent_reference: String,
    pub source: SubAgentSource,
    pub kind: SubAgentActivityKind,
    pub status_message: Option<String>,
}

impl SubAgentActivity {
    pub fn new(
        agent_reference: impl Into<String>,
        source: SubAgentSource,
        kind: SubAgentActivityKind,
    ) -> Self {
        Self {
            agent_reference: agent_reference.into(),
            source,
            kind,
            status_message: None,
        }
    }

    pub fn with_status_message(mut self, status_message: impl Into<String>) -> Self {
        self.status_message = Some(status_message.into());
        self
    }
}
