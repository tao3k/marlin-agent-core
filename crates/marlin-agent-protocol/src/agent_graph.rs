//! AgentGraph protocol envelopes for runtime projection boundaries.

use marlin_agent_graph::{AgentGraphId, AgentGraphPlanningReceipt};
use serde::{Deserialize, Serialize};

/// Stable schema id for AgentGraph projection requests.
pub const AGENT_GRAPH_PROJECTION_REQUEST_SCHEMA_ID: &str =
    "marlin.agent_graph.projection_request.v1";

/// Protocol request for projecting an AgentGraph planning receipt into runtime evidence.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentGraphProjectionRequest {
    /// Schema identifier carried by serialized requests.
    pub schema_id: String,
    /// AgentGraph the caller expects runtime projection to use.
    pub graph_id: AgentGraphId,
    /// Topology planning receipt produced before runtime admission.
    pub planning: AgentGraphPlanningReceipt,
    /// Runtime observation timestamp supplied by the caller.
    pub observed_at_ms: u64,
}

impl AgentGraphProjectionRequest {
    /// Creates a projection request using the current protocol schema.
    pub fn new(
        graph_id: AgentGraphId,
        planning: AgentGraphPlanningReceipt,
        observed_at_ms: u64,
    ) -> Self {
        Self {
            schema_id: AGENT_GRAPH_PROJECTION_REQUEST_SCHEMA_ID.to_owned(),
            graph_id,
            planning,
            observed_at_ms,
        }
    }

    /// Returns true when the request uses the current schema identifier.
    pub fn has_current_schema(&self) -> bool {
        self.schema_id == AGENT_GRAPH_PROJECTION_REQUEST_SCHEMA_ID
    }
}
