use std::collections::BTreeMap;

use marlin_agent_protocol::{
    AgentExecutionTrace, GraphLoopEvidencePolicy, GraphLoopExecutionBudget,
    GraphLoopExecutionRequest, GraphLoopExecutionResult, GraphLoopExecutionStatus,
    GraphLoopIterationReport, GraphLoopNextAction, GraphLoopRunRequest, GraphLoopStopPolicy,
    LoopGraph, LoopNodeSpec, RuntimePlanSnapshot,
};

#[test]
fn graph_loop_run_request_records_stop_budget_and_replayable_evidence_policy() {
    let initial_request = GraphLoopExecutionRequest::new("loop-run", loop_graph())
        .with_budget(GraphLoopExecutionBudget::max_node_executions(2));

    let request = GraphLoopRunRequest::new(initial_request)
        .with_stop_policy(
            GraphLoopStopPolicy::max_iterations(2)
                .with_max_duration_ms(1_000)
                .stop_on_failed_execution()
                .require_human_gate(),
        )
        .with_iteration_budget(GraphLoopExecutionBudget::max_node_executions(1))
        .with_evidence_policy(GraphLoopEvidencePolicy::replayable_runtime());

    let value = serde_json::to_value(&request).expect("loop run request serializes");

    assert_eq!(value["initial_request"]["run_id"], "loop-run");
    assert_eq!(value["initial_request"]["budget"]["max_node_executions"], 2);
    assert_eq!(value["stop_policy"]["max_iterations"], 2);
    assert_eq!(value["stop_policy"]["max_duration_ms"], 1_000);
    assert_eq!(value["stop_policy"]["stop_on_failed_execution"], true);
    assert_eq!(value["stop_policy"]["human_gate_required"], true);
    assert_eq!(value["iteration_budget"]["max_node_executions"], 1);
    assert_eq!(value["evidence_policy"]["capture_runtime_events"], true);
    assert_eq!(value["evidence_policy"]["capture_node_receipts"], true);
    assert_eq!(value["evidence_policy"]["capture_trace"], true);
    assert_eq!(value["evidence_policy"]["replayable"], true);
}

#[test]
fn graph_loop_iteration_report_records_next_action_and_optional_trace() {
    let snapshot = RuntimePlanSnapshot {
        run_id: "loop-run".to_owned(),
        graph_id: "graph".to_owned(),
        active_node: None,
    };
    let report = GraphLoopIterationReport::new(
        1,
        GraphLoopExecutionResult::completed(snapshot, vec!["rank".to_owned()]),
        GraphLoopNextAction::ContinueWithGraph(loop_graph()),
    )
    .with_trace(AgentExecutionTrace::new(
        "loop-run",
        "graph",
        GraphLoopExecutionStatus::Completed,
    ));

    let value = serde_json::to_value(&report).expect("iteration report serializes");

    assert_eq!(value["iteration"], 1);
    assert_eq!(value["execution_result"]["status"], "Completed");
    assert_eq!(value["execution_result"]["visited_nodes"][0], "rank");
    assert_eq!(
        value["next_action"]["ContinueWithGraph"]["graph_id"],
        "graph"
    );
    assert_eq!(value["trace"]["run_id"], "loop-run");
    assert_eq!(value["trace"]["graph_id"], "graph");
    assert_eq!(value["trace"]["status"], "Completed");
}

fn loop_graph() -> LoopGraph {
    LoopGraph {
        graph_id: "graph".to_owned(),
        nodes: vec![LoopNodeSpec {
            id: "rank".to_owned(),
            executor: "gerbil.rank".to_owned(),
            config: BTreeMap::new(),
        }],
        edges: Vec::new(),
    }
}
