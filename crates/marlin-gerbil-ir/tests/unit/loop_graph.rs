use marlin_gerbil_ir::{
    CompiledLoopGraph, LoopEdgeSpec, LoopGraphCompileError, LoopGraphCompileLimits,
    LoopGraphExecutionFrontier, LoopGraphExecutionPlan, LoopGraphValidationError, LoopNodeSpec,
};
use std::collections::BTreeMap;

#[test]
fn compiled_loop_graph_validates_typed_node_and_edge_shape() {
    let graph = CompiledLoopGraph {
        graph_id: "agent-loop".to_string(),
        nodes: vec![
            LoopNodeSpec {
                id: "plan".to_string(),
                executor: "scheme-policy".to_string(),
                config: BTreeMap::from([("profile".to_string(), "interactive".to_string())]),
            },
            LoopNodeSpec {
                id: "apply".to_string(),
                executor: "rust-runtime".to_string(),
                config: BTreeMap::new(),
            },
        ],
        edges: vec![LoopEdgeSpec {
            from: "plan".to_string(),
            to: "apply".to_string(),
            condition: Some("policy-approved".to_string()),
        }],
    };

    assert_eq!(graph.validate(), Ok(()));
}

#[test]
fn compiled_loop_graph_rejects_duplicate_nodes_before_runtime_scheduling() {
    let graph = CompiledLoopGraph {
        graph_id: "agent-loop".to_string(),
        nodes: vec![
            LoopNodeSpec {
                id: "plan".to_string(),
                executor: "scheme-policy".to_string(),
                config: BTreeMap::new(),
            },
            LoopNodeSpec {
                id: "plan".to_string(),
                executor: "rust-runtime".to_string(),
                config: BTreeMap::new(),
            },
        ],
        edges: vec![],
    };

    assert_eq!(
        graph.validate(),
        Err(LoopGraphValidationError::DuplicateNodeId {
            node_id: "plan".to_string()
        })
    );
}

#[test]
fn compiled_loop_graph_rejects_edges_that_reference_unknown_nodes() {
    let graph = CompiledLoopGraph {
        graph_id: "agent-loop".to_string(),
        nodes: vec![LoopNodeSpec {
            id: "plan".to_string(),
            executor: "scheme-policy".to_string(),
            config: BTreeMap::new(),
        }],
        edges: vec![LoopEdgeSpec {
            from: "plan".to_string(),
            to: "missing".to_string(),
            condition: None,
        }],
    };

    assert_eq!(
        graph.validate(),
        Err(LoopGraphValidationError::UnknownEdgeTarget {
            edge_index: 0,
            node_id: "missing".to_string()
        })
    );
}

#[test]
fn compiled_loop_graph_compiles_bounded_acyclic_frontiers_for_rust_runtime() {
    let graph = CompiledLoopGraph {
        graph_id: "agent-loop".to_string(),
        nodes: vec![
            node("plan", "scheme-policy"),
            node("observe", "rust-observability"),
            node("apply", "rust-runtime"),
        ],
        edges: vec![edge("plan", "apply"), edge("observe", "apply")],
    };

    assert_eq!(
        graph.compile_execution_plan(LoopGraphCompileLimits {
            max_node_executions: Some(3),
        }),
        Ok(LoopGraphExecutionPlan {
            graph_id: "agent-loop".to_string(),
            frontiers: vec![
                LoopGraphExecutionFrontier {
                    node_ids: vec!["observe".to_string(), "plan".to_string()],
                },
                LoopGraphExecutionFrontier {
                    node_ids: vec!["apply".to_string()],
                },
            ],
            required_node_executions: 3,
        })
    );
}

#[test]
fn compiled_loop_graph_rejects_cycles_before_runtime_scheduling() {
    let graph = CompiledLoopGraph {
        graph_id: "agent-loop".to_string(),
        nodes: vec![node("plan", "scheme-policy"), node("apply", "rust-runtime")],
        edges: vec![edge("plan", "apply"), edge("apply", "plan")],
    };

    assert_eq!(
        graph.compile_execution_plan(LoopGraphCompileLimits::default()),
        Err(LoopGraphCompileError::CycleDetected {
            remaining_node_ids: vec!["plan".to_string(), "apply".to_string()],
        })
    );
}

#[test]
fn compiled_loop_graph_rejects_node_execution_budget_overflow() {
    let graph = CompiledLoopGraph {
        graph_id: "agent-loop".to_string(),
        nodes: vec![node("plan", "scheme-policy"), node("apply", "rust-runtime")],
        edges: vec![edge("plan", "apply")],
    };

    assert_eq!(
        graph.compile_execution_plan(LoopGraphCompileLimits {
            max_node_executions: Some(1),
        }),
        Err(LoopGraphCompileError::NodeExecutionBudgetExceeded {
            max_node_executions: 1,
            required_node_executions: 2,
        })
    );
}

#[test]
fn compiled_loop_graph_rejects_empty_graph_execution_plan() {
    let graph = CompiledLoopGraph {
        graph_id: "agent-loop".to_string(),
        nodes: vec![],
        edges: vec![],
    };

    assert_eq!(
        graph.compile_execution_plan(LoopGraphCompileLimits::default()),
        Err(LoopGraphCompileError::EmptyGraph)
    );
}

fn node(id: &str, executor: &str) -> LoopNodeSpec {
    LoopNodeSpec {
        id: id.to_string(),
        executor: executor.to_string(),
        config: BTreeMap::new(),
    }
}

fn edge(from: &str, to: &str) -> LoopEdgeSpec {
    LoopEdgeSpec {
        from: from.to_string(),
        to: to.to_string(),
        condition: None,
    }
}
