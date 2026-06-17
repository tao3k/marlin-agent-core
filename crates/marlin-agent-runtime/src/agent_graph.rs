//! Runtime admission for `AgentGraph` coordination plans.

use std::collections::HashSet;

use marlin_agent_graph::{
    AgentCoordinationPlan, AgentEdgeId, AgentGraph, AgentGraphId, AgentGraphPlanningReceipt,
    AgentGraphPlanningStatus, AgentNodeId, GraphLoopEntryRef,
};
use marlin_agent_protocol::AgentGraphProjectionRequest;
use serde::{Deserialize, Serialize};

/// Runtime projection receipt linking `AgentGraph` planning to admission.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeAgentGraphProjectionReceipt {
    /// Planning receipt produced by `marlin-agent-graph`.
    pub planning: AgentGraphPlanningReceipt,
    /// Runtime observation timestamp.
    pub observed_at_ms: u64,
    /// Projection status.
    pub status: RuntimeAgentGraphProjectionStatus,
    /// Runtime admission receipt, when projection reaches admission.
    pub admission: Option<RuntimeAgentCoordinationAdmissionReceipt>,
    /// Typed projection rejection, when projection stops before accepted admission.
    pub rejection: Option<RuntimeAgentGraphProjectionRejection>,
}

impl RuntimeAgentGraphProjectionReceipt {
    fn projected(
        planning: AgentGraphPlanningReceipt,
        observed_at_ms: u64,
        admission: RuntimeAgentCoordinationAdmissionReceipt,
    ) -> Self {
        Self {
            planning,
            observed_at_ms,
            status: RuntimeAgentGraphProjectionStatus::Projected,
            admission: Some(admission),
            rejection: None,
        }
    }

    fn rejected(
        planning: AgentGraphPlanningReceipt,
        observed_at_ms: u64,
        rejection: RuntimeAgentGraphProjectionRejection,
        admission: Option<RuntimeAgentCoordinationAdmissionReceipt>,
    ) -> Self {
        Self {
            planning,
            observed_at_ms,
            status: RuntimeAgentGraphProjectionStatus::Rejected,
            admission,
            rejection: Some(rejection),
        }
    }
}

/// Status for runtime projection of an `AgentGraph` planning receipt.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RuntimeAgentGraphProjectionStatus {
    /// Planning produced a plan and runtime admission accepted it.
    Projected,
    /// Projection stopped before an execution request should be produced.
    Rejected,
}

/// Typed reason for rejecting planning-to-admission projection.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RuntimeAgentGraphProjectionRejection {
    /// The request schema is not supported by this runtime boundary.
    UnsupportedRequestSchema {
        /// Schema identifier carried by the request.
        schema_id: String,
    },
    /// The request targets a different graph than the supplied `AgentGraph`.
    RequestGraphMismatch {
        /// Graph identifier carried by the request.
        request_graph_id: AgentGraphId,
        /// Graph identifier from the supplied graph.
        graph_id: AgentGraphId,
    },
    /// Upstream planning already rejected the target.
    PlanningRejected,
    /// Upstream planning reported success but did not include a plan.
    MissingPlan,
    /// Runtime admission rejected the supplied plan.
    AdmissionRejected(RuntimeAgentCoordinationRejection),
}

/// Runtime receipt for consuming an upstream `AgentCoordinationPlan`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeAgentCoordinationAdmissionReceipt {
    /// Coordination plan received by the runtime layer.
    pub plan: AgentCoordinationPlan,
    /// Runtime observation timestamp.
    pub observed_at_ms: u64,
    /// Admission outcome.
    pub status: RuntimeAgentCoordinationAdmissionStatus,
    /// Graph-loop entrypoint projected from the selected root node.
    pub root_loop_entry: Option<GraphLoopEntryRef>,
    /// Typed rejection reason when admission fails.
    pub rejection: Option<RuntimeAgentCoordinationRejection>,
}

impl RuntimeAgentCoordinationAdmissionReceipt {
    fn accepted(
        plan: AgentCoordinationPlan,
        observed_at_ms: u64,
        root_loop_entry: GraphLoopEntryRef,
    ) -> Self {
        Self {
            plan,
            observed_at_ms,
            status: RuntimeAgentCoordinationAdmissionStatus::Accepted,
            root_loop_entry: Some(root_loop_entry),
            rejection: None,
        }
    }

    fn rejected(
        plan: AgentCoordinationPlan,
        observed_at_ms: u64,
        rejection: RuntimeAgentCoordinationRejection,
    ) -> Self {
        Self {
            plan,
            observed_at_ms,
            status: RuntimeAgentCoordinationAdmissionStatus::Rejected,
            root_loop_entry: None,
            rejection: Some(rejection),
        }
    }
}

/// Admission status for runtime consumption of an `AgentCoordinationPlan`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RuntimeAgentCoordinationAdmissionStatus {
    /// Plan references were valid and runtime can execute the projected graph loop.
    Accepted,
    /// Plan references were invalid and no execution request should be produced.
    Rejected,
}

/// Typed reason for rejecting an `AgentCoordinationPlan` at the runtime boundary.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RuntimeAgentCoordinationRejection {
    /// The supplied `AgentGraph` failed topology validation.
    InvalidGraph {
        /// Validation failure rendered at the boundary.
        message: String,
    },
    /// The plan targets a different graph than the supplied `AgentGraph`.
    GraphMismatch {
        /// Graph identifier from the plan.
        plan_graph_id: AgentGraphId,
        /// Graph identifier from the supplied graph.
        graph_id: AgentGraphId,
    },
    /// The plan root node does not exist in the supplied graph.
    MissingRootNode {
        /// Missing root node identifier.
        node_id: AgentNodeId,
    },
    /// The plan references a candidate edge that does not exist in the supplied graph.
    MissingCandidateEdge {
        /// Missing edge identifier.
        edge_id: AgentEdgeId,
    },
}

/// Dry-run receipt proving an admitted AgentGraph plan is ready for graph-loop execution.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeAgentGraphExecutionReadinessReceipt {
    /// Protocol request that entered the runtime boundary.
    pub request: AgentGraphProjectionRequest,
    /// Runtime projection/admission receipt produced from the request.
    pub projection: RuntimeAgentGraphProjectionReceipt,
    /// Readiness status.
    pub status: RuntimeAgentGraphExecutionReadinessStatus,
    /// Root graph-loop entrypoint when readiness succeeds.
    pub root_loop_entry: Option<GraphLoopEntryRef>,
    /// Typed rejection reason when readiness fails.
    pub rejection: Option<RuntimeAgentGraphExecutionReadinessRejection>,
}

impl RuntimeAgentGraphExecutionReadinessReceipt {
    fn ready(
        request: AgentGraphProjectionRequest,
        projection: RuntimeAgentGraphProjectionReceipt,
        root_loop_entry: GraphLoopEntryRef,
    ) -> Self {
        Self {
            request,
            projection,
            status: RuntimeAgentGraphExecutionReadinessStatus::Ready,
            root_loop_entry: Some(root_loop_entry),
            rejection: None,
        }
    }

    fn not_ready(
        request: AgentGraphProjectionRequest,
        projection: RuntimeAgentGraphProjectionReceipt,
        rejection: RuntimeAgentGraphExecutionReadinessRejection,
    ) -> Self {
        Self {
            request,
            projection,
            status: RuntimeAgentGraphExecutionReadinessStatus::NotReady,
            root_loop_entry: None,
            rejection: Some(rejection),
        }
    }
}

/// Status for AgentGraph execution readiness dry-runs.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RuntimeAgentGraphExecutionReadinessStatus {
    /// Runtime projected a root graph-loop entrypoint without executing it.
    Ready,
    /// Runtime could not project a safe graph-loop entrypoint.
    NotReady,
}

/// Typed reason why an AgentGraph execution readiness dry-run failed.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RuntimeAgentGraphExecutionReadinessRejection {
    /// Projection rejected before admission could provide a root loop entry.
    ProjectionRejected(RuntimeAgentGraphProjectionRejection),
    /// Projection reported success without carrying an admission receipt.
    MissingAdmission,
    /// Admission rejected the request.
    AdmissionRejected(RuntimeAgentCoordinationRejection),
    /// Admission reported success without a root graph-loop entrypoint.
    MissingRootLoopEntry,
}

/// Projects an AgentGraph protocol request into runtime admission evidence.
pub fn project_agent_graph_projection_request(
    graph: &AgentGraph,
    request: AgentGraphProjectionRequest,
) -> RuntimeAgentGraphProjectionReceipt {
    if !request.has_current_schema() {
        let schema_id = request.schema_id.clone();
        return RuntimeAgentGraphProjectionReceipt::rejected(
            request.planning,
            request.observed_at_ms,
            RuntimeAgentGraphProjectionRejection::UnsupportedRequestSchema { schema_id },
            None,
        );
    }

    if request.graph_id != graph.graph_id {
        let request_graph_id = request.graph_id.clone();
        let graph_id = graph.graph_id.clone();
        return RuntimeAgentGraphProjectionReceipt::rejected(
            request.planning,
            request.observed_at_ms,
            RuntimeAgentGraphProjectionRejection::RequestGraphMismatch {
                request_graph_id,
                graph_id,
            },
            None,
        );
    }

    project_agent_graph_planning_receipt(graph, request.planning, request.observed_at_ms)
}

/// Dry-runs AgentGraph projection to prove the root graph-loop entrypoint is available.
pub fn check_agent_graph_execution_readiness(
    graph: &AgentGraph,
    request: AgentGraphProjectionRequest,
) -> RuntimeAgentGraphExecutionReadinessReceipt {
    let projection = project_agent_graph_projection_request(graph, request.clone());
    if projection.status != RuntimeAgentGraphProjectionStatus::Projected {
        let rejection = projection
            .rejection
            .clone()
            .map(RuntimeAgentGraphExecutionReadinessRejection::ProjectionRejected)
            .unwrap_or(RuntimeAgentGraphExecutionReadinessRejection::MissingAdmission);
        return RuntimeAgentGraphExecutionReadinessReceipt::not_ready(
            request, projection, rejection,
        );
    }

    let Some(admission) = projection.admission.clone() else {
        return RuntimeAgentGraphExecutionReadinessReceipt::not_ready(
            request,
            projection,
            RuntimeAgentGraphExecutionReadinessRejection::MissingAdmission,
        );
    };

    if admission.status != RuntimeAgentCoordinationAdmissionStatus::Accepted {
        let rejection = admission
            .rejection
            .clone()
            .map(RuntimeAgentGraphExecutionReadinessRejection::AdmissionRejected)
            .unwrap_or(RuntimeAgentGraphExecutionReadinessRejection::MissingRootLoopEntry);
        return RuntimeAgentGraphExecutionReadinessReceipt::not_ready(
            request, projection, rejection,
        );
    }

    let Some(root_loop_entry) = admission.root_loop_entry.clone() else {
        return RuntimeAgentGraphExecutionReadinessReceipt::not_ready(
            request,
            projection,
            RuntimeAgentGraphExecutionReadinessRejection::MissingRootLoopEntry,
        );
    };

    RuntimeAgentGraphExecutionReadinessReceipt::ready(request, projection, root_loop_entry)
}

/// Projects an `AgentGraph` planning receipt into runtime admission evidence.
pub fn project_agent_graph_planning_receipt(
    graph: &AgentGraph,
    planning: AgentGraphPlanningReceipt,
    observed_at_ms: u64,
) -> RuntimeAgentGraphProjectionReceipt {
    if planning.status != AgentGraphPlanningStatus::Planned {
        return RuntimeAgentGraphProjectionReceipt::rejected(
            planning,
            observed_at_ms,
            RuntimeAgentGraphProjectionRejection::PlanningRejected,
            None,
        );
    }

    let Some(plan) = planning.plan.clone() else {
        return RuntimeAgentGraphProjectionReceipt::rejected(
            planning,
            observed_at_ms,
            RuntimeAgentGraphProjectionRejection::MissingPlan,
            None,
        );
    };

    let admission = admit_agent_coordination_plan(graph, plan, observed_at_ms);
    if let Some(rejection) = admission.rejection.clone() {
        return RuntimeAgentGraphProjectionReceipt::rejected(
            planning,
            observed_at_ms,
            RuntimeAgentGraphProjectionRejection::AdmissionRejected(rejection),
            Some(admission),
        );
    }

    RuntimeAgentGraphProjectionReceipt::projected(planning, observed_at_ms, admission)
}

/// Validates and projects an upstream coordination plan into runtime-owned evidence.
pub fn admit_agent_coordination_plan(
    graph: &AgentGraph,
    plan: AgentCoordinationPlan,
    observed_at_ms: u64,
) -> RuntimeAgentCoordinationAdmissionReceipt {
    if let Err(error) = graph.validate() {
        return RuntimeAgentCoordinationAdmissionReceipt::rejected(
            plan,
            observed_at_ms,
            RuntimeAgentCoordinationRejection::InvalidGraph {
                message: error.to_string(),
            },
        );
    }

    if plan.graph_id != graph.graph_id {
        let plan_graph_id = plan.graph_id.clone();
        let graph_id = graph.graph_id.clone();
        return RuntimeAgentCoordinationAdmissionReceipt::rejected(
            plan,
            observed_at_ms,
            RuntimeAgentCoordinationRejection::GraphMismatch {
                plan_graph_id,
                graph_id,
            },
        );
    }

    let Some(root_node) = graph
        .nodes
        .iter()
        .find(|node| node.node_id == plan.root_node)
    else {
        let node_id = plan.root_node.clone();
        return RuntimeAgentCoordinationAdmissionReceipt::rejected(
            plan,
            observed_at_ms,
            RuntimeAgentCoordinationRejection::MissingRootNode { node_id },
        );
    };

    let graph_edge_ids = graph
        .edges
        .iter()
        .map(|edge| edge.edge_id.clone())
        .collect::<HashSet<_>>();
    if let Some(edge_id) = plan
        .candidate_edges
        .iter()
        .find(|edge_id| !graph_edge_ids.contains(*edge_id))
        .cloned()
    {
        return RuntimeAgentCoordinationAdmissionReceipt::rejected(
            plan,
            observed_at_ms,
            RuntimeAgentCoordinationRejection::MissingCandidateEdge { edge_id },
        );
    }

    RuntimeAgentCoordinationAdmissionReceipt::accepted(
        plan,
        observed_at_ms,
        root_node.loop_entry.clone(),
    )
}
