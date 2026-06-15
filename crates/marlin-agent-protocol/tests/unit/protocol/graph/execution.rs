use std::collections::BTreeMap;

use marlin_agent_protocol::{
    GraphLoopExecutionBudget, GraphLoopExecutionRequest, GraphLoopExecutionResult,
    GraphNodeExecutionReceipt, LoopGraph, LoopNodeSpec, RuntimePlanSnapshot,
};

#[test]
fn graph_loop_execution_request_records_optional_budget() {
    let graph = LoopGraph {
        graph_id: "graph".to_string(),
        nodes: vec![LoopNodeSpec {
            id: "plan".to_string(),
            executor: "provider.stream".to_string(),
            config: BTreeMap::new(),
        }],
        edges: Vec::new(),
    };

    let request_without_budget = GraphLoopExecutionRequest::new("run", graph.clone());
    let value = serde_json::to_value(&request_without_budget).expect("request serializes");
    assert!(value.get("budget").is_none());

    let request = GraphLoopExecutionRequest::new("run", graph)
        .with_budget(GraphLoopExecutionBudget::max_node_executions(1));
    let value = serde_json::to_value(&request).expect("budgeted request serializes");

    assert_eq!(
        request
            .budget
            .as_ref()
            .and_then(|budget| budget.max_node_executions),
        Some(1)
    );
    assert_eq!(value["budget"]["max_node_executions"], 1);
}

#[test]
fn graph_loop_execution_result_records_optional_node_receipts() {
    let snapshot = RuntimePlanSnapshot {
        run_id: "run".to_owned(),
        graph_id: "graph".to_owned(),
        active_node: None,
    };
    let result_without_receipts =
        GraphLoopExecutionResult::completed(snapshot.clone(), vec!["plan".to_owned()]);
    let value = serde_json::to_value(&result_without_receipts)
        .expect("result without node receipts serializes");

    assert!(value.get("node_receipts").is_none());

    let result = GraphLoopExecutionResult::completed(snapshot, vec!["plan".to_owned()])
        .with_node_receipts(vec![GraphNodeExecutionReceipt::completed(
            "plan", "provider",
        )]);
    let value = serde_json::to_value(&result).expect("result with node receipts serializes");

    assert_eq!(result.node_receipts.len(), 1);
    assert_eq!(value["node_receipts"][0]["node_id"], "plan");
    assert_eq!(value["node_receipts"][0]["executor"], "provider");
    assert_eq!(value["node_receipts"][0]["status"], "Completed");
}
