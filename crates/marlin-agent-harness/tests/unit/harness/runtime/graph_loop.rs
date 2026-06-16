use marlin_agent_harness::{
    AgentHarnessGerbilLoopContinuationError, AgentHarnessGerbilLoopContinuationPlanner,
    AgentHarnessGerbilLoopContinuationProjector, AgentHarnessGraphBuilder, AgentHarnessRuntime,
    AgentHarnessScenario,
};
use marlin_agent_kernel::{
    GraphLoopContinuationAction, GraphLoopContinuationDecision, GraphLoopContinuationInput,
    GraphLoopContinuationPlanner, GraphLoopContinuationReceipt, GraphLoopEvidencePolicy,
    GraphLoopExecutionRequest, GraphLoopExecutionStatus, GraphLoopNextAction, GraphLoopRunRequest,
    GraphLoopStopPolicy, LoopGraph, LoopNodeSpec, TokioGraphLoopController, TokioGraphLoopKernel,
};
use marlin_agent_protocol::{AgentEventTopic, GERBIL_LOOP_GRAPH_CONTINUATION_SCHEMA_ID};
use marlin_agent_runtime::RuntimeFuture;
use marlin_gerbil_scheme::{
    GERBIL_LOOP_GRAPH_CONTINUATION_TYPE_ID, GerbilSchemeSchemaId, GerbilSchemeTypeId,
    GerbilSchemeTypedValue, GerbilSchemeValue,
};

use super::support::EventfulExecutor;

#[tokio::test]
async fn harness_graph_loop_report_captures_ordered_iteration_reports_and_events() {
    let scenario = AgentHarnessScenario::new("graph-loop-report");
    let graph = AgentHarnessGraphBuilder::new("graph")
        .node("node-1", "eventful")
        .build();
    let request = GraphLoopRunRequest::new(GraphLoopExecutionRequest::new("run", graph))
        .with_evidence_policy(GraphLoopEvidencePolicy::replayable_runtime());
    let kernel =
        TokioGraphLoopKernel::new("run", "graph").with_executor("eventful", EventfulExecutor);
    let controller = TokioGraphLoopController::new(kernel);
    let mut harness = AgentHarnessRuntime::new(16);

    let report = harness
        .execute_graph_loop(&scenario, &controller, request)
        .await;

    assert_eq!(report.summary.iteration_count, 1);
    assert_eq!(
        report.summary.final_status,
        Some(GraphLoopExecutionStatus::Completed)
    );
    assert_eq!(
        report
            .final_result()
            .expect("loop should produce a final result")
            .visited_nodes,
        vec!["node-1"]
    );
    assert!(report.assertion.is_none());
    assert!(report.has_event_topic(&AgentEventTopic::new("test.harness")));

    let trace = report
        .iteration_traces()
        .next()
        .expect("replayable loop request should include an iteration trace");
    assert!(
        trace
            .events
            .iter()
            .any(|event| event.topic == "test.harness" && event.message == "node node-1 observed")
    );
    assert_eq!(report.summary.event_count, report.events.len());
    assert!(report.evidence_graph.summary().nodes > 0);
}

#[tokio::test]
async fn harness_graph_loop_report_keeps_ordered_continuation_iterations() {
    let scenario = AgentHarnessScenario::new("graph-loop-continuation-report");
    let graph = AgentHarnessGraphBuilder::new("graph")
        .node("node-1", "eventful")
        .build();
    let request = GraphLoopRunRequest::new(GraphLoopExecutionRequest::new("run", graph))
        .with_stop_policy(GraphLoopStopPolicy::max_iterations(2));
    let kernel =
        TokioGraphLoopKernel::new("run", "graph").with_executor("eventful", EventfulExecutor);
    let controller = TokioGraphLoopController::new(kernel)
        .with_continuation_planner(ContinueWithSecondGraphPlanner);
    let mut harness = AgentHarnessRuntime::new(16);

    let report = harness
        .execute_graph_loop(&scenario, &controller, request)
        .await;

    assert_eq!(report.summary.iteration_count, 2);
    assert_eq!(
        report.summary.final_status,
        Some(GraphLoopExecutionStatus::Completed)
    );
    assert_eq!(
        report
            .iteration_reports
            .iter()
            .map(|report| report.iteration)
            .collect::<Vec<_>>(),
        vec![0, 1]
    );
    assert!(matches!(
        report.iteration_reports[0].next_action,
        GraphLoopNextAction::ContinueWithGraph(_)
    ));
    assert_eq!(
        report
            .final_result()
            .expect("loop should produce a final result")
            .visited_nodes,
        vec!["node-2"]
    );
    assert_eq!(
        report
            .events
            .iter()
            .filter(|event| event.topic == "test.harness")
            .count(),
        2
    );
}

#[tokio::test]
async fn harness_graph_loop_can_continue_from_gerbil_native_projection() {
    let scenario = AgentHarnessScenario::new("gerbil-continuation-projection");
    let graph = AgentHarnessGraphBuilder::new("graph")
        .node("node-1", "eventful")
        .build();
    let request = GraphLoopRunRequest::new(GraphLoopExecutionRequest::new("run", graph))
        .with_stop_policy(GraphLoopStopPolicy::max_iterations(2));
    let kernel =
        TokioGraphLoopKernel::new("run", "graph").with_executor("eventful", EventfulExecutor);
    let planner = AgentHarnessGerbilLoopContinuationPlanner::from_native_projection_manifest(
        StaticGerbilContinuationProjector,
    )
    .expect("Gerbil continuation projection planner");
    let controller = TokioGraphLoopController::new(kernel).with_continuation_planner(planner);
    let mut harness = AgentHarnessRuntime::new(16);

    let report = harness
        .execute_graph_loop(&scenario, &controller, request)
        .await;

    assert_eq!(report.summary.iteration_count, 2);
    assert!(matches!(
        report.iteration_reports[0].next_action,
        GraphLoopNextAction::ContinueWithGraph(_)
    ));
    assert_eq!(
        report
            .final_result()
            .expect("loop should produce a final result")
            .visited_nodes,
        vec!["node-2"]
    );
}

#[derive(Clone, Debug)]
struct ContinueWithSecondGraphPlanner;

impl GraphLoopContinuationPlanner for ContinueWithSecondGraphPlanner {
    fn decide(
        &self,
        input: GraphLoopContinuationInput,
    ) -> RuntimeFuture<GraphLoopContinuationDecision> {
        Box::pin(async move {
            if input.iteration_id.get() == 0 {
                GraphLoopContinuationDecision::new(GraphLoopContinuationReceipt::new(
                    input.run_id,
                    input.iteration_id,
                    GraphLoopContinuationAction::Rewrite {
                        graph: LoopGraph {
                            graph_id: "graph-second".to_owned(),
                            nodes: vec![LoopNodeSpec {
                                id: "node-2".to_owned(),
                                executor: "eventful".to_owned(),
                                config: Default::default(),
                            }],
                            edges: Vec::new(),
                        },
                        reason: "test.continue_with_second_graph".to_owned(),
                    },
                ))
            } else {
                GraphLoopContinuationDecision::new(GraphLoopContinuationReceipt::new(
                    input.run_id,
                    input.iteration_id,
                    GraphLoopContinuationAction::Accept,
                ))
            }
        })
    }
}

#[derive(Clone, Debug)]
struct StaticGerbilContinuationProjector;

impl AgentHarnessGerbilLoopContinuationProjector for StaticGerbilContinuationProjector {
    fn project_continuation(
        &self,
        input: GraphLoopContinuationInput,
    ) -> RuntimeFuture<Result<GerbilSchemeTypedValue, AgentHarnessGerbilLoopContinuationError>>
    {
        Box::pin(async move {
            if input.iteration_id.get() == 0 {
                Ok(continuation_envelope(continue_with_second_graph_action()))
            } else {
                Ok(continuation_envelope(GerbilSchemeValue::record([(
                    "kind",
                    "stop_completed".into(),
                )])))
            }
        })
    }
}

fn continuation_envelope(action: GerbilSchemeValue) -> GerbilSchemeTypedValue {
    GerbilSchemeTypedValue::new(
        GerbilSchemeTypeId::new(GERBIL_LOOP_GRAPH_CONTINUATION_TYPE_ID),
        GerbilSchemeValue::record([
            ("schema_id", GERBIL_LOOP_GRAPH_CONTINUATION_SCHEMA_ID.into()),
            ("action", action),
        ]),
    )
    .with_schema_id(GerbilSchemeSchemaId::new(
        GERBIL_LOOP_GRAPH_CONTINUATION_SCHEMA_ID,
    ))
}

fn continue_with_second_graph_action() -> GerbilSchemeValue {
    GerbilSchemeValue::record([
        ("kind", "continue_with_graph".into()),
        (
            "compiled_graph",
            GerbilSchemeValue::record([
                ("graph_id", "gerbil-projected-graph".into()),
                (
                    "nodes",
                    GerbilSchemeValue::vector([GerbilSchemeValue::record([
                        ("id", "node-2".into()),
                        ("executor", "eventful".into()),
                        ("config", GerbilSchemeValue::empty_record()),
                    ])]),
                ),
                ("edges", GerbilSchemeValue::vector([])),
            ]),
        ),
    ])
}
