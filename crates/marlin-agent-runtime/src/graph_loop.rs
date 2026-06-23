//! Runtime-owned graph-loop run registry and handle receipts.

use std::{collections::BTreeMap, sync::Arc};

use marlin_agent_protocol::{
    AgentFlowReceipt, AgentFlowReceiptId, AgentFlowRuntimeHandoffId, AgentFlowSession,
    AgentFlowSessionTransform, AgentFlowTransformRejection, GraphId, GraphLoopEventId,
    GraphLoopExecutionStatus, GraphLoopIterationId, NodeId, RunId,
    build_agent_flow_runtime_handoff, derive_agent_flow_session,
};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

/// Runtime state for a graph-loop run.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GraphLoopRunStatus {
    Active,
    Cancelling,
    Completed,
    Failed,
    Cancelled,
}

impl GraphLoopRunStatus {
    /// Returns true when no further runtime work is expected for this run.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

impl From<GraphLoopExecutionStatus> for GraphLoopRunStatus {
    fn from(status: GraphLoopExecutionStatus) -> Self {
        match status {
            GraphLoopExecutionStatus::Completed => Self::Completed,
            GraphLoopExecutionStatus::Failed => Self::Failed,
            GraphLoopExecutionStatus::Cancelled => Self::Cancelled,
        }
    }
}

/// Projects one Agent-Flow session transform through the runtime handoff gate.
///
/// This is the loop substrate step for the POO Flow model:
/// Session -> SessionTransform -> RuntimeHandoff -> Receipt -> DerivedSession.
pub fn project_agent_flow_loop_step(
    request: AgentFlowLoopStepRequest,
) -> Result<AgentFlowReceipt, AgentFlowTransformRejection> {
    let handoff = build_agent_flow_runtime_handoff(
        &request.session,
        request.transform,
        request.handoff_id,
        request.admitted_at_ms,
    )?;
    Ok(derive_agent_flow_session(
        &request.session,
        handoff,
        request.receipt_id,
    ))
}

/// Named request for projecting one Agent-Flow loop substrate step.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentFlowLoopStepRequest {
    /// Source Agent-Flow session being transformed.
    pub session: AgentFlowSession,
    /// Typed session transform proposed by the Agent-Flow policy plane.
    pub transform: AgentFlowSessionTransform,
    /// Stable runtime handoff identifier to assign to the admitted transform.
    pub handoff_id: AgentFlowRuntimeHandoffId,
    /// Stable receipt identifier for the derived session receipt.
    pub receipt_id: AgentFlowReceiptId,
    /// Runtime admission timestamp in milliseconds.
    pub admitted_at_ms: u64,
}

/// Point-in-time runtime observation for one graph-loop run.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GraphLoopRunObservation {
    pub run_id: RunId,
    pub graph_id: GraphId,
    pub status: GraphLoopRunStatus,
    pub started_at_ms: u64,
    pub updated_at_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_iteration_id: Option<GraphLoopIterationId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pending_node_ids: Vec<NodeId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_event_id: Option<GraphLoopEventId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal_status: Option<GraphLoopExecutionStatus>,
}

impl GraphLoopRunObservation {
    /// Creates an active graph-loop run observation.
    pub fn active(run_id: impl Into<RunId>, graph_id: impl Into<GraphId>, now_ms: u64) -> Self {
        Self {
            run_id: run_id.into(),
            graph_id: graph_id.into(),
            status: GraphLoopRunStatus::Active,
            started_at_ms: now_ms,
            updated_at_ms: now_ms,
            current_iteration_id: None,
            pending_node_ids: Vec::new(),
            last_event_id: None,
            terminal_status: None,
        }
    }

    /// Returns true when this run is still active from the registry perspective.
    pub fn is_active(&self) -> bool {
        !self.status.is_terminal()
    }

    /// Updates controller progress fields.
    pub fn with_progress(
        mut self,
        observed_at_ms: u64,
        current_iteration_id: impl Into<GraphLoopIterationId>,
        pending_node_ids: Vec<NodeId>,
        last_event_id: impl Into<GraphLoopEventId>,
    ) -> Self {
        self.updated_at_ms = observed_at_ms;
        self.current_iteration_id = Some(current_iteration_id.into());
        self.pending_node_ids = pending_node_ids;
        self.last_event_id = Some(last_event_id.into());
        self
    }
}

/// Error returned by graph-loop run registry mutation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GraphLoopRunRegistryError {
    DuplicateRun { run_id: RunId },
    RunNotFound { run_id: RunId },
}

/// Receipt returned after registering a graph-loop run.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GraphLoopRunStartReceipt {
    pub observation: GraphLoopRunObservation,
}

/// Named request for recording graph-loop run progress.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GraphLoopRunProgressUpdate {
    pub run_id: RunId,
    pub observed_at_ms: u64,
    pub current_iteration_id: GraphLoopIterationId,
    pub pending_node_ids: Vec<NodeId>,
    pub last_event_id: GraphLoopEventId,
}

impl GraphLoopRunProgressUpdate {
    /// Creates a graph-loop run progress update.
    pub fn new(
        run_id: impl Into<RunId>,
        current_iteration_id: impl Into<GraphLoopIterationId>,
        observed_at_ms: u64,
        last_event_id: impl Into<GraphLoopEventId>,
    ) -> Self {
        Self {
            run_id: run_id.into(),
            observed_at_ms,
            current_iteration_id: current_iteration_id.into(),
            pending_node_ids: Vec::new(),
            last_event_id: last_event_id.into(),
        }
    }

    /// Replaces pending node ids for this progress update.
    pub fn with_pending_node_ids(mut self, pending_node_ids: Vec<NodeId>) -> Self {
        self.pending_node_ids = pending_node_ids;
        self
    }
}

/// Receipt returned after inspecting one graph-loop run.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GraphLoopRunInspectReceipt {
    pub run_id: RunId,
    pub observed_at_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observation: Option<GraphLoopRunObservation>,
}

/// Status returned by a cancellation request.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GraphLoopRunCancelStatus {
    Requested,
    AlreadyTerminal,
    NotFound,
}

/// Receipt returned after requesting cancellation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GraphLoopRunCancelReceipt {
    pub run_id: RunId,
    pub observed_at_ms: u64,
    pub status: GraphLoopRunCancelStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observation: Option<GraphLoopRunObservation>,
}

/// Status returned by a wait/idle observation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GraphLoopRunWaitStatus {
    Idle,
    Active,
    NotFound,
}

/// Receipt returned after observing whether a run is idle.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GraphLoopRunWaitReceipt {
    pub run_id: RunId,
    pub observed_at_ms: u64,
    pub status: GraphLoopRunWaitStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observation: Option<GraphLoopRunObservation>,
}

/// Snapshot of all known graph-loop runs.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct GraphLoopRunRegistrySnapshot {
    pub observed_at_ms: u64,
    pub active_count: usize,
    pub run_count: usize,
    pub runs: Vec<GraphLoopRunObservation>,
}

/// In-memory graph-loop run registry.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GraphLoopRunRegistry {
    runs: BTreeMap<RunId, GraphLoopRunObservation>,
}

impl GraphLoopRunRegistry {
    /// Creates an empty graph-loop run registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers one active run.
    pub fn start_run(
        &mut self,
        run_id: impl Into<RunId>,
        graph_id: impl Into<GraphId>,
        started_at_ms: u64,
    ) -> Result<GraphLoopRunStartReceipt, GraphLoopRunRegistryError> {
        let run_id = run_id.into();
        if self.runs.contains_key(&run_id) {
            return Err(GraphLoopRunRegistryError::DuplicateRun { run_id });
        }
        let observation = GraphLoopRunObservation::active(run_id.clone(), graph_id, started_at_ms);
        self.runs.insert(run_id, observation.clone());
        Ok(GraphLoopRunStartReceipt { observation })
    }

    /// Updates progress fields for an active run.
    pub fn record_progress(
        &mut self,
        update: GraphLoopRunProgressUpdate,
    ) -> Result<GraphLoopRunObservation, GraphLoopRunRegistryError> {
        let Some(observation) = self.runs.get_mut(&update.run_id) else {
            return Err(GraphLoopRunRegistryError::RunNotFound {
                run_id: update.run_id,
            });
        };
        observation.updated_at_ms = update.observed_at_ms;
        observation.current_iteration_id = Some(update.current_iteration_id);
        observation.pending_node_ids = update.pending_node_ids;
        observation.last_event_id = Some(update.last_event_id);
        Ok(observation.clone())
    }

    /// Marks an active run as cancellation-requested.
    pub fn cancel_run(&mut self, run_id: &RunId, observed_at_ms: u64) -> GraphLoopRunCancelReceipt {
        let Some(observation) = self.runs.get_mut(run_id) else {
            return GraphLoopRunCancelReceipt {
                run_id: run_id.clone(),
                observed_at_ms,
                status: GraphLoopRunCancelStatus::NotFound,
                observation: None,
            };
        };

        observation.updated_at_ms = observed_at_ms;
        let status = if observation.status.is_terminal() {
            GraphLoopRunCancelStatus::AlreadyTerminal
        } else {
            observation.status = GraphLoopRunStatus::Cancelling;
            GraphLoopRunCancelStatus::Requested
        };
        GraphLoopRunCancelReceipt {
            run_id: run_id.clone(),
            observed_at_ms,
            status,
            observation: Some(observation.clone()),
        }
    }

    /// Marks a run as terminal.
    pub fn complete_run(
        &mut self,
        run_id: &RunId,
        terminal_status: GraphLoopExecutionStatus,
        observed_at_ms: u64,
        last_event_id: impl Into<GraphLoopEventId>,
    ) -> Result<GraphLoopRunObservation, GraphLoopRunRegistryError> {
        let Some(observation) = self.runs.get_mut(run_id) else {
            return Err(GraphLoopRunRegistryError::RunNotFound {
                run_id: run_id.clone(),
            });
        };
        observation.updated_at_ms = observed_at_ms;
        observation.status = GraphLoopRunStatus::from(terminal_status.clone());
        observation.terminal_status = Some(terminal_status);
        observation.last_event_id = Some(last_event_id.into());
        observation.pending_node_ids.clear();
        Ok(observation.clone())
    }

    /// Inspects one known run.
    pub fn inspect_run(&self, run_id: &RunId, observed_at_ms: u64) -> GraphLoopRunInspectReceipt {
        GraphLoopRunInspectReceipt {
            run_id: run_id.clone(),
            observed_at_ms,
            observation: self.runs.get(run_id).cloned(),
        }
    }

    /// Observes whether a run is idle.
    pub fn wait_run(&self, run_id: &RunId, observed_at_ms: u64) -> GraphLoopRunWaitReceipt {
        let Some(observation) = self.runs.get(run_id).cloned() else {
            return GraphLoopRunWaitReceipt {
                run_id: run_id.clone(),
                observed_at_ms,
                status: GraphLoopRunWaitStatus::NotFound,
                observation: None,
            };
        };
        let status = if observation.is_active() {
            GraphLoopRunWaitStatus::Active
        } else {
            GraphLoopRunWaitStatus::Idle
        };
        GraphLoopRunWaitReceipt {
            run_id: run_id.clone(),
            observed_at_ms,
            status,
            observation: Some(observation),
        }
    }

    /// Returns a point-in-time registry snapshot.
    pub fn snapshot(&self, observed_at_ms: u64) -> GraphLoopRunRegistrySnapshot {
        let runs = self.runs.values().cloned().collect::<Vec<_>>();
        GraphLoopRunRegistrySnapshot {
            observed_at_ms,
            active_count: runs.iter().filter(|run| run.is_active()).count(),
            run_count: runs.len(),
            runs,
        }
    }
}

/// Shared graph-loop run registry handle.
#[derive(Clone, Debug, Default)]
pub struct GraphLoopRunRegistryHandle {
    registry: Arc<Mutex<GraphLoopRunRegistry>>,
}

impl GraphLoopRunRegistryHandle {
    /// Creates a shared graph-loop run registry handle.
    pub fn new() -> Self {
        Self::default()
    }

    /// Mutates the registry through a scoped closure.
    pub fn with_registry<T>(&self, f: impl FnOnce(&mut GraphLoopRunRegistry) -> T) -> T {
        let mut registry = self.registry.lock();
        f(&mut registry)
    }

    /// Reads the registry through a scoped closure.
    pub fn read_registry<T>(&self, f: impl FnOnce(&GraphLoopRunRegistry) -> T) -> T {
        let registry = self.registry.lock();
        f(&registry)
    }
}
