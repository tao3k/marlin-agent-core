//! POO Flow Agent-Flow session transform protocol.

use serde::{Deserialize, Serialize};

/// Stable session identifier in the Agent-Flow model.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AgentFlowSessionId(String);

impl AgentFlowSessionId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for AgentFlowSessionId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for AgentFlowSessionId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// Stable transform identifier in the Agent-Flow model.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AgentFlowTransformId(String);

impl AgentFlowTransformId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for AgentFlowTransformId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for AgentFlowTransformId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// Stable intent identifier in the Agent-Flow model.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AgentFlowIntentId(String);

impl AgentFlowIntentId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for AgentFlowIntentId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for AgentFlowIntentId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// Stable runtime handoff identifier in the Agent-Flow model.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AgentFlowRuntimeHandoffId(String);

impl AgentFlowRuntimeHandoffId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for AgentFlowRuntimeHandoffId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for AgentFlowRuntimeHandoffId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// Stable receipt identifier in the Agent-Flow model.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AgentFlowReceiptId(String);

impl AgentFlowReceiptId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for AgentFlowReceiptId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for AgentFlowReceiptId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// Stable tool name identifier for tool intents.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AgentFlowToolName(String);

impl AgentFlowToolName {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for AgentFlowToolName {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for AgentFlowToolName {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// Stable memory target identifier for memory intents.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AgentFlowMemoryTarget(String);

impl AgentFlowMemoryTarget {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for AgentFlowMemoryTarget {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for AgentFlowMemoryTarget {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// Stable placement target identifier for placement intents.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AgentFlowPlacementTarget(String);

impl AgentFlowPlacementTarget {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for AgentFlowPlacementTarget {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for AgentFlowPlacementTarget {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// Continuing Agent-Flow session state. It is not a workflow run.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentFlowSession {
    pub session_id: AgentFlowSessionId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_session_id: Option<AgentFlowSessionId>,
    pub generation: u64,
    pub status: AgentFlowSessionStatus,
}

impl AgentFlowSession {
    pub fn root(session_id: impl Into<AgentFlowSessionId>) -> Self {
        Self {
            session_id: session_id.into(),
            parent_session_id: None,
            generation: 0,
            status: AgentFlowSessionStatus::Active,
        }
    }
}

/// Session lifecycle status for Agent-Flow state.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentFlowSessionStatus {
    Active,
    Blocked,
    Completed,
}

/// Tool intent produced by a session transform.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentFlowToolIntent {
    pub intent_id: AgentFlowIntentId,
    pub tool_name: AgentFlowToolName,
}

/// Memory intent produced by a session transform.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentFlowMemoryIntent {
    pub intent_id: AgentFlowIntentId,
    pub target: AgentFlowMemoryTarget,
    pub operation: AgentFlowMemoryOperation,
}

/// Memory operation requested by an Agent-Flow memory intent.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentFlowMemoryOperation {
    Recall,
    Store,
    Compact,
}

/// Placement intent produced by a session transform.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentFlowPlacementIntent {
    pub intent_id: AgentFlowIntentId,
    pub target: AgentFlowPlacementTarget,
    pub operation: AgentFlowPlacementOperation,
}

/// Placement operation requested by an Agent-Flow placement intent.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentFlowPlacementOperation {
    ForkSession,
    BindWorkspace,
    Delegate,
}

/// Typed intent family produced by a session transform.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "intent_family", rename_all = "snake_case")]
pub enum AgentFlowIntent {
    Tool(AgentFlowToolIntent),
    Memory(AgentFlowMemoryIntent),
    Placement(AgentFlowPlacementIntent),
}

impl AgentFlowIntent {
    pub fn intent_id(&self) -> &AgentFlowIntentId {
        match self {
            Self::Tool(intent) => &intent.intent_id,
            Self::Memory(intent) => &intent.intent_id,
            Self::Placement(intent) => &intent.intent_id,
        }
    }
}

/// Pure session transform from one session state to runtime handoff intent.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentFlowSessionTransform {
    pub transform_id: AgentFlowTransformId,
    pub source_session_id: AgentFlowSessionId,
    pub intents: Vec<AgentFlowIntent>,
}

impl AgentFlowSessionTransform {
    pub fn new(
        transform_id: impl Into<AgentFlowTransformId>,
        source_session_id: impl Into<AgentFlowSessionId>,
        intents: impl IntoIterator<Item = AgentFlowIntent>,
    ) -> Self {
        Self {
            transform_id: transform_id.into(),
            source_session_id: source_session_id.into(),
            intents: intents.into_iter().collect(),
        }
    }
}

/// Runtime handoff produced from a valid session transform.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentFlowRuntimeHandoff {
    pub handoff_id: AgentFlowRuntimeHandoffId,
    pub transform_id: AgentFlowTransformId,
    pub source_session_id: AgentFlowSessionId,
    pub intents: Vec<AgentFlowIntent>,
    pub admitted_at_ms: u64,
}

/// Receipt for one Agent-Flow transform handoff.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentFlowReceipt {
    pub receipt_id: AgentFlowReceiptId,
    pub handoff: AgentFlowRuntimeHandoff,
    pub status: AgentFlowReceiptStatus,
    pub derived_session: AgentFlowDerivedSession,
}

/// Receipt status for Agent-Flow transform handoff.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentFlowReceiptStatus {
    Derived,
}

/// Derived session state produced after a transform receipt.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentFlowDerivedSession {
    pub session: AgentFlowSession,
    pub source_session_id: AgentFlowSessionId,
    pub transform_id: AgentFlowTransformId,
    pub receipt_id: AgentFlowReceiptId,
}

/// Typed reason a session transform cannot be handed to runtime.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum AgentFlowTransformRejection {
    SourceSessionMismatch {
        session_id: AgentFlowSessionId,
        transform_source_session_id: AgentFlowSessionId,
    },
    EmptyIntentSet {
        transform_id: AgentFlowTransformId,
    },
}

/// Builds a runtime handoff from an Agent-Flow session transform after validating session ownership.
pub fn build_agent_flow_runtime_handoff(
    session: &AgentFlowSession,
    transform: AgentFlowSessionTransform,
    handoff_id: impl Into<AgentFlowRuntimeHandoffId>,
    admitted_at_ms: u64,
) -> Result<AgentFlowRuntimeHandoff, AgentFlowTransformRejection> {
    if transform.source_session_id != session.session_id {
        return Err(AgentFlowTransformRejection::SourceSessionMismatch {
            session_id: session.session_id.clone(),
            transform_source_session_id: transform.source_session_id,
        });
    }

    if transform.intents.is_empty() {
        return Err(AgentFlowTransformRejection::EmptyIntentSet {
            transform_id: transform.transform_id,
        });
    }

    Ok(AgentFlowRuntimeHandoff {
        handoff_id: handoff_id.into(),
        transform_id: transform.transform_id,
        source_session_id: transform.source_session_id,
        intents: transform.intents,
        admitted_at_ms,
    })
}

/// Derives the next Agent-Flow session receipt from an admitted runtime handoff.
pub fn derive_agent_flow_session(
    source_session: &AgentFlowSession,
    handoff: AgentFlowRuntimeHandoff,
    receipt_id: impl Into<AgentFlowReceiptId>,
) -> AgentFlowReceipt {
    let receipt_id = receipt_id.into();
    let derived_session = AgentFlowSession {
        session_id: source_session.session_id.clone(),
        parent_session_id: source_session.parent_session_id.clone(),
        generation: source_session.generation + 1,
        status: AgentFlowSessionStatus::Active,
    };

    AgentFlowReceipt {
        receipt_id: receipt_id.clone(),
        derived_session: AgentFlowDerivedSession {
            session: derived_session,
            source_session_id: source_session.session_id.clone(),
            transform_id: handoff.transform_id.clone(),
            receipt_id,
        },
        handoff,
        status: AgentFlowReceiptStatus::Derived,
    }
}
