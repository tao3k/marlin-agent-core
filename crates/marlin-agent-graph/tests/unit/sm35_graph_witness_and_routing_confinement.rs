// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use marlin_agent_graph::{
    AdmittedGovernedContext, AdmittedMarlinDispatch, AgentCoordinationEvidenceKind,
    AgentCoordinationEvidenceRef, AgentCoordinationPlan, AgentEdge, AgentEdgeCondition,
    AgentEdgeKind, AgentEvidenceId, AgentGraph, AgentGraphId, AgentGraphPlanningStatus,
    AgentGraphPlanningTarget, AgentNode, AgentNodeId, AgentPolicyDecisionRef,
    AgentPolicyRoutingDecision, AgentPolicyRoutingReceipt, AgentRole, AgentRoutingReason,
    AgentTopologyPolicy, DispatchRequestId, ExecutionReceiptStatus, FencingEpoch,
    GerbilPolicyScopeRef, GraphLoopEntryRef, GraphLoopGraphRef, GraphLoopNodeRef, KvPrefixDigest,
    MarlinExecutionReceipt, TaskActorCursor, TaskActorId, TaskActorScopeMode, TaskActorStatus,
    plan_agent_coordination, plan_agent_coordination_with_policy_receipt,
    project_agent_candidate_routing_receipts, reconcile_marlin_execution_receipt,
};

#[test]
fn sm35_v1_sealed_witness_and_fencing_epoch_serde_forgery_and_detached_settle_refused() {
    // 1. Forged AdmittedGovernedContext JSON bypassing sandbox_admitted / cedar / L3 defeater floor / blocker auto-archive / cut=0
    let forged_capsule_json = r#"{
        "capsule": {
            "task_id": "task-forged",
            "context_cut": 0,
            "kv_prefix_digest": "sha256:forged",
            "disclosure_depth": "L0",
            "has_active_defeater": true,
            "quarantined_memory_count": 0,
            "sandbox_admitted": false,
            "cedar_handoff_valid": false,
            "detour_mode": "BlockerActive",
            "auto_archive_enabled": true
        }
    }"#;
    assert!(
        serde_json::from_str::<AdmittedGovernedContext>(forged_capsule_json).is_err(),
        "SM35 V1: forged AdmittedGovernedContext JSON MUST fail closed on deserialization"
    );

    // 2. Forged AdmittedMarlinDispatch JSON bypassing Detached CommitSettlement + ApplySteering without join_node_id
    let forged_dispatch_json = r#"{
        "envelope": {
            "request_id": "req-forged",
            "task_id": "task-detached",
            "operation": "CommitSettlement",
            "fencing_epoch": 1,
            "scope_mode": "Detached",
            "join_node_id": null,
            "substrate_target": "PureMemory"
        }
    }"#;
    assert!(
        serde_json::from_str::<AdmittedMarlinDispatch>(forged_dispatch_json).is_err(),
        "SM35 V1: forged Detached CommitSettlement AdmittedMarlinDispatch MUST fail closed"
    );

    let forged_steering_json = r#"{
        "envelope": {
            "request_id": "req-steer",
            "task_id": "task-1",
            "operation": "ApplySteering",
            "fencing_epoch": 1,
            "scope_mode": "Project",
            "join_node_id": null,
            "substrate_target": "PureMemory"
        }
    }"#;
    assert!(
        serde_json::from_str::<AdmittedMarlinDispatch>(forged_steering_json).is_err(),
        "SM35 V1: forged ApplySteering without join_node_id MUST fail closed"
    );

    // 3. Forged FencingEpoch(0) JSON bypassing FencingEpoch::new(0)
    assert!(
        serde_json::from_str::<FencingEpoch>("0").is_err(),
        "SM35 V1: FencingEpoch(0) JSON MUST fail closed"
    );

    // 4. Reconciling Settled receipt on a Detached actor MUST fail closed
    let detached_cursor = TaskActorCursor {
        task_id: TaskActorId::new("task-detached").unwrap(),
        fencing_epoch: FencingEpoch::new(2).unwrap(),
        status: TaskActorStatus::Running,
        scope_mode: TaskActorScopeMode::Detached,
        pending_steering_branches: 0,
    };
    let detached_settle = reconcile_marlin_execution_receipt(
        &detached_cursor,
        &MarlinExecutionReceipt {
            request_id: DispatchRequestId::new("req-detached-settle").unwrap(),
            task_id: TaskActorId::new("task-detached").unwrap(),
            fencing_epoch: FencingEpoch::new(2).unwrap(),
            status: ExecutionReceiptStatus::Settled,
        },
    );
    assert!(
        !detached_settle.accepted,
        "SM35 V1: Detached actor cursor MUST refuse ExecutionReceiptStatus::Settled"
    );

    // 5. Reconciling at u64::MAX epoch MUST fail closed instead of silently saturating and allowing replay
    let max_epoch_cursor = TaskActorCursor {
        task_id: TaskActorId::new("task-max").unwrap(),
        fencing_epoch: FencingEpoch::new(u64::MAX).unwrap(),
        status: TaskActorStatus::Running,
        scope_mode: TaskActorScopeMode::Project,
        pending_steering_branches: 0,
    };
    let overflow_decision = reconcile_marlin_execution_receipt(
        &max_epoch_cursor,
        &MarlinExecutionReceipt {
            request_id: DispatchRequestId::new("req-overflow").unwrap(),
            task_id: TaskActorId::new("task-max").unwrap(),
            fencing_epoch: FencingEpoch::new(u64::MAX).unwrap(),
            status: ExecutionReceiptStatus::Ok,
        },
    );
    assert!(
        !overflow_decision.accepted,
        "SM35 V1: reconciling at u64::MAX fencing epoch MUST fail closed rather than saturating"
    );

    // 6. Control characters in graph semantic identifiers MUST fail closed
    assert!(TaskActorId::new("task\0evil").is_err());
    assert!(KvPrefixDigest::new("sha256:abc\r\n").is_err());
    assert!(AgentGraphId::new("graph\x1b[31m").is_err());
}

fn test_node(id: &str) -> AgentNode {
    AgentNode {
        node_id: AgentNodeId::new(id).unwrap(),
        role: AgentRole::new(id).unwrap(),
        capabilities: Vec::new(),
        loop_entry: GraphLoopEntryRef {
            graph: GraphLoopGraphRef::new(format!("loop.{id}")).unwrap(),
            entry_node: GraphLoopNodeRef::new("entry").unwrap(),
        },
        memory_scope: None,
        policy_scope: None,
    }
}

#[test]
fn sm35_v2_policy_decision_routing_bypass_and_duplicate_candidate_edges_refused() {
    let graph_id = AgentGraphId::new("graph.sm35").unwrap();
    let e_priv = marlin_agent_graph::AgentEdgeId::new("edge.escalate").unwrap();

    // 1. Deterministic graph MUST NOT admit an edge guarded by PolicyDecision(_) without a policy receipt!
    let det_graph = AgentGraph {
        graph_id: graph_id.clone(),
        nodes: vec![test_node("planner"), test_node("privileged")],
        edges: vec![AgentEdge {
            edge_id: e_priv.clone(),
            from: AgentNodeId::new("planner").unwrap(),
            to: AgentNodeId::new("privileged").unwrap(),
            kind: AgentEdgeKind::Escalate,
            condition: AgentEdgeCondition::PolicyDecision(
                AgentPolicyDecisionRef::new("policy.require-human-escalation").unwrap(),
            ),
        }],
        topology_policy: AgentTopologyPolicy::Deterministic,
    };
    let det_receipt = plan_agent_coordination(
        &det_graph,
        AgentGraphPlanningTarget::new(graph_id.clone(), AgentNodeId::new("planner").unwrap()),
    );
    let selected = det_receipt
        .plan
        .as_ref()
        .map(|p| p.candidate_edges.as_slice())
        .unwrap_or(&[]);
    assert!(
        !selected.contains(&e_priv),
        "SM35 V2: Deterministic graph MUST NOT select a PolicyDecision edge without a policy receipt, got {selected:?}"
    );

    // 2. PolicyScoped graph MUST reject duplicate candidate_edges in policy_receipt
    let scope = GerbilPolicyScopeRef::new("gerbil.scope.sm35").unwrap();
    let policy_graph = AgentGraph {
        topology_policy: AgentTopologyPolicy::PolicyScoped(scope.clone()),
        ..det_graph
    };
    let dup_receipt = AgentPolicyRoutingReceipt {
        graph_id: graph_id.clone(),
        policy_scope: scope.clone(),
        root_node: AgentNodeId::new("planner").unwrap(),
        decision: AgentPolicyRoutingDecision::SelectEdges,
        candidate_edges: vec![e_priv.clone(), e_priv.clone()],
        evidence: vec![AgentCoordinationEvidenceRef {
            kind: AgentCoordinationEvidenceKind::GerbilPolicyReceipt,
            evidence_id: AgentEvidenceId::new("policy.receipt.1").unwrap(),
        }],
    };
    let dup_plan = plan_agent_coordination_with_policy_receipt(
        &policy_graph,
        AgentGraphPlanningTarget::new(graph_id.clone(), AgentNodeId::new("planner").unwrap()),
        dup_receipt,
    );
    assert_eq!(
        dup_plan.status,
        AgentGraphPlanningStatus::Rejected,
        "SM35 V2: duplicate candidate_edges in policy_receipt MUST be rejected"
    );

    // 3. PolicyScoped graph selecting PolicyDecision edge WITHOUT GerbilPolicyReceipt evidence MUST be rejected
    let no_ev_receipt = AgentPolicyRoutingReceipt {
        graph_id: graph_id.clone(),
        policy_scope: scope,
        root_node: AgentNodeId::new("planner").unwrap(),
        decision: AgentPolicyRoutingDecision::SelectEdges,
        candidate_edges: vec![e_priv.clone()],
        evidence: Vec::new(),
    };
    let no_ev_plan = plan_agent_coordination_with_policy_receipt(
        &policy_graph,
        AgentGraphPlanningTarget::new(graph_id.clone(), AgentNodeId::new("planner").unwrap()),
        no_ev_receipt,
    );
    assert_eq!(
        no_ev_plan.status,
        AgentGraphPlanningStatus::Rejected,
        "SM35 V2: PolicyDecision edge selected without GerbilPolicyReceipt evidence MUST be rejected"
    );

    // 4. project_agent_candidate_routing_receipts MUST reject duplicate candidate_edges in plan
    let dup_coord_plan = AgentCoordinationPlan {
        graph_id,
        root_node: AgentNodeId::new("planner").unwrap(),
        candidate_edges: vec![e_priv.clone(), e_priv],
    };
    assert!(
        project_agent_candidate_routing_receipts(
            &policy_graph,
            &dup_coord_plan,
            AgentRoutingReason::new("sm35.dup").unwrap(),
            Vec::new(),
        )
        .is_err(),
        "SM35 V2: routing projection MUST reject duplicate candidate_edges"
    );
}
