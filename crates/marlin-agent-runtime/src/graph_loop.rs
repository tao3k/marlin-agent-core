// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Runtime-owned graph-loop run registry and handle receipts.

use std::{collections::BTreeMap, sync::Arc};

use marlin_agent_protocol::{
    AgentFlowReceipt, AgentFlowReceiptId, AgentFlowRuntimeHandoffId, AgentFlowSession,
    AgentFlowSessionTransform, AgentFlowTransformRejection, GraphId, GraphLoopEventId,
    GraphLoopExecutionStatus, GraphLoopIterationId, GraphLoopNextAction, HumanDecision,
    HumanDecisionReceipt, HumanGateId, HumanGateReceipt, LoopGraph, NodeId, RunId,
    build_agent_flow_runtime_handoff, derive_agent_flow_session,
};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tokio::sync::watch;

/// Runtime state for a graph-loop run.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GraphLoopRunStatus {
    Active,
    AwaitingHuman,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_human_gate: Option<HumanGateReceipt>,
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
            pending_human_gate: None,
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
    DuplicateRun {
        run_id: RunId,
    },
    RunNotFound {
        run_id: RunId,
    },
    InvalidRunState {
        run_id: RunId,
        status: GraphLoopRunStatus,
    },
    GateIterationMismatch {
        run_id: RunId,
        iteration_id: GraphLoopIterationId,
    },
    GateMismatch {
        expected: HumanGateId,
        received: HumanGateId,
    },
    InvalidHumanDecision {
        gate_id: HumanGateId,
    },
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
    AwaitingHuman,
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

    /// Suspends an active run at the exact iteration that produced a human gate.
    pub fn defer_run(
        &mut self,
        gate: HumanGateReceipt,
        observed_at_ms: u64,
        last_event_id: impl Into<GraphLoopEventId>,
    ) -> Result<GraphLoopRunObservation, GraphLoopRunRegistryError> {
        let Some(observation) = self.runs.get_mut(&gate.run_id) else {
            return Err(GraphLoopRunRegistryError::RunNotFound {
                run_id: gate.run_id,
            });
        };
        if observation.status == GraphLoopRunStatus::Cancelling {
            observation.updated_at_ms = observed_at_ms;
            observation.status = GraphLoopRunStatus::Cancelled;
            observation.terminal_status = Some(GraphLoopExecutionStatus::Cancelled);
            observation.last_event_id = Some(last_event_id.into());
            observation.pending_node_ids.clear();
            return Ok(observation.clone());
        }
        if observation.status != GraphLoopRunStatus::Active {
            return Err(GraphLoopRunRegistryError::InvalidRunState {
                run_id: gate.run_id,
                status: observation.status.clone(),
            });
        }
        if observation.current_iteration_id.as_ref() != Some(&gate.iteration_id) {
            return Err(GraphLoopRunRegistryError::GateIterationMismatch {
                run_id: gate.run_id,
                iteration_id: gate.iteration_id,
            });
        }
        observation.updated_at_ms = observed_at_ms;
        observation.status = GraphLoopRunStatus::AwaitingHuman;
        observation.last_event_id = Some(last_event_id.into());
        observation.pending_node_ids.clear();
        observation.pending_human_gate = Some(gate);
        Ok(observation.clone())
    }

    /// Admits a decision against the one pending gate and returns its approved graph.
    /// A repeated or stale decision cannot authorize another execution.
    pub fn decide_human_gate(
        &mut self,
        run_id: &RunId,
        decision: &HumanDecisionReceipt,
        observed_at_ms: u64,
        last_event_id: impl Into<GraphLoopEventId>,
    ) -> Result<Option<LoopGraph>, GraphLoopRunRegistryError> {
        let Some(observation) = self.runs.get_mut(run_id) else {
            return Err(GraphLoopRunRegistryError::RunNotFound {
                run_id: run_id.clone(),
            });
        };
        if observation.status != GraphLoopRunStatus::AwaitingHuman {
            return Err(GraphLoopRunRegistryError::InvalidRunState {
                run_id: run_id.clone(),
                status: observation.status.clone(),
            });
        }
        let gate = observation
            .pending_human_gate
            .as_ref()
            .expect("awaiting-human run must retain its gate");
        if gate.gate_id != decision.gate_id {
            return Err(GraphLoopRunRegistryError::GateMismatch {
                expected: gate.gate_id.clone(),
                received: decision.gate_id.clone(),
            });
        }
        if decision.decision != HumanDecision::Approved && decision.approved_next_graph.is_some() {
            return Err(GraphLoopRunRegistryError::InvalidHumanDecision {
                gate_id: decision.gate_id.clone(),
            });
        }
        let next_graph = if decision.decision == HumanDecision::Approved {
            Some(
                decision
                    .approved_next_graph
                    .clone()
                    .or_else(|| match &gate.proposed_next_action {
                        GraphLoopNextAction::ContinueWithGraph(graph) => Some(graph.clone()),
                        _ => None,
                    })
                    .ok_or_else(|| GraphLoopRunRegistryError::InvalidHumanDecision {
                        gate_id: decision.gate_id.clone(),
                    })?,
            )
        } else {
            None
        };
        observation.updated_at_ms = observed_at_ms;
        observation.last_event_id = Some(last_event_id.into());
        match decision.decision {
            HumanDecision::Approved => {
                observation.status = GraphLoopRunStatus::Active;
                observation.pending_human_gate = None;
                Ok(next_graph)
            }
            HumanDecision::Rejected => {
                observation.status = GraphLoopRunStatus::Failed;
                observation.terminal_status = Some(GraphLoopExecutionStatus::Failed);
                observation.pending_human_gate = None;
                Ok(None)
            }
            HumanDecision::NeedsQuestion | HumanDecision::PatchRequested => Ok(None),
        }
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
        } else if observation.status == GraphLoopRunStatus::AwaitingHuman {
            observation.status = GraphLoopRunStatus::Cancelled;
            observation.terminal_status = Some(GraphLoopExecutionStatus::Cancelled);
            observation.pending_human_gate = None;
            GraphLoopRunCancelStatus::Requested
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
        if observation.status == GraphLoopRunStatus::AwaitingHuman {
            return Err(GraphLoopRunRegistryError::InvalidRunState {
                run_id: run_id.clone(),
                status: observation.status.clone(),
            });
        }
        let terminal_status = if observation.status == GraphLoopRunStatus::Cancelling {
            GraphLoopExecutionStatus::Cancelled
        } else {
            terminal_status
        };
        observation.updated_at_ms = observed_at_ms;
        observation.status = GraphLoopRunStatus::from(terminal_status.clone());
        observation.terminal_status = Some(terminal_status);
        observation.last_event_id = Some(last_event_id.into());
        observation.pending_node_ids.clear();
        Ok(observation.clone())
    }

    /// Reads a run status without cloning its graph or pending gate.
    pub fn run_status(&self, run_id: &RunId) -> Option<GraphLoopRunStatus> {
        self.runs.get(run_id).map(|run| run.status.clone())
    }

    /// Borrows the exact pending gate for a known run without cloning its graph.
    pub fn pending_human_gate(&self, run_id: &RunId) -> Option<&HumanGateReceipt> {
        self.runs.get(run_id)?.pending_human_gate.as_ref()
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
        let status = if observation.status == GraphLoopRunStatus::AwaitingHuman {
            GraphLoopRunWaitStatus::AwaitingHuman
        } else if observation.is_active() {
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

/// Monotonic wake revision for reading the authoritative graph-loop registry.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct GraphLoopRegistryRevision(u64);

impl GraphLoopRegistryRevision {
    /// Returns the revision number for change detection.
    pub fn get(self) -> u64 {
        self.0
    }
}

/// Shared graph-loop run registry handle.
#[derive(Clone, Debug)]
pub struct GraphLoopRunRegistryHandle {
    registry: Arc<Mutex<GraphLoopRunRegistry>>,
    changes: watch::Sender<GraphLoopRegistryRevision>,
}

impl Default for GraphLoopRunRegistryHandle {
    fn default() -> Self {
        let (changes, _) = watch::channel(GraphLoopRegistryRevision::default());
        Self {
            registry: Arc::new(Mutex::new(GraphLoopRunRegistry::new())),
            changes,
        }
    }
}

impl GraphLoopRunRegistryHandle {
    /// Creates a shared graph-loop run registry handle.
    pub fn new() -> Self {
        Self::default()
    }

    /// Mutates the registry through a scoped closure.
    pub fn with_registry<T>(&self, f: impl FnOnce(&mut GraphLoopRunRegistry) -> T) -> T {
        let mut registry = self.registry.lock();
        let result = f(&mut registry);
        self.changes.send_modify(|revision| {
            revision.0 = revision
                .0
                .checked_add(1)
                .expect("graph-loop revision exhausted");
        });
        result
    }

    /// Subscribe to coalesced state changes; read the typed registry after wake.
    pub fn subscribe(&self) -> watch::Receiver<GraphLoopRegistryRevision> {
        self.changes.subscribe()
    }

    /// Reads the registry through a scoped closure.
    pub fn read_registry<T>(&self, f: impl FnOnce(&GraphLoopRunRegistry) -> T) -> T {
        let registry = self.registry.lock();
        f(&registry)
    }
}
