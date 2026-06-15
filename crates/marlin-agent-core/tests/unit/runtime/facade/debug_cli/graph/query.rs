use marlin_agent_core::{
    GraphLoopEvent, GraphLoopEventEnvelope, GraphLoopExecutionStatus, GraphLoopMessageRole,
    GraphToolCallReceipt, GraphToolCallStatus, LoopEventQuerySummary, LoopQuerySummary,
    ProjectRuntimeQuerySummary,
    protocol::{
        GraphQueryContext, GraphQueryFamily, GraphQueryMatch, GraphQueryMatchRelationship,
        GraphQueryRelationshipFact, GraphQueryRequest, GraphQueryResponse,
    },
    run_marlin_cli_from_args,
};
use std::fs;
use tempfile::tempdir;

use crate::runtime::facade::debug_cli::fixtures::single_node_graph;

#[test]
fn debug_cli_graph_query_reads_loop_run_receipt_facts() {
    let dir = tempdir().expect("tempdir");
    let input = dir.path().join("graph.json");
    let receipt_path = dir.path().join("loop-run-receipt.json");
    fs::write(
        &input,
        serde_json::to_string(&single_node_graph()).expect("graph JSON"),
    )
    .expect("write graph");

    let run = run_marlin_cli_from_args([
        "loop",
        "run",
        "--input",
        input.to_str().expect("utf8 path"),
        "--max-iterations",
        "1",
        "--no-store",
    ]);
    assert_eq!(run.status, 0, "{}", run.stderr);
    fs::write(&receipt_path, &run.stdout).expect("write loop run receipt");

    let query = run_marlin_cli_from_args([
        "graph",
        "query",
        "--input",
        receipt_path.to_str().expect("utf8 path"),
    ]);

    assert_eq!(query.status, 0, "{}", query.stderr);
    let summary: LoopQuerySummary =
        serde_json::from_str(&query.stdout).expect("loop query summary");
    assert_eq!(summary.iteration_count, 1);
    assert_eq!(
        summary.terminal_status,
        Some(GraphLoopExecutionStatus::Completed)
    );
    assert!(summary.replayable);
    assert_eq!(summary.missing_trace_count, 0);
    assert_eq!(summary.statuses, vec![GraphLoopExecutionStatus::Completed]);
    assert_eq!(summary.visited_nodes_by_iteration, vec![vec!["plan"]]);
    assert_eq!(summary.node_receipt_count, 1);
    assert!(summary.trace_event_count > 0);
}

#[test]
fn debug_cli_graph_query_reads_graph_loop_event_facts() {
    let dir = tempdir().expect("tempdir");
    let input = dir.path().join("events.json");
    let events = vec![
        GraphLoopEventEnvelope::new(
            "run-1",
            "event-1",
            10,
            GraphLoopEvent::MessageStart {
                role: GraphLoopMessageRole::User,
            },
        )
        .with_iteration_id(1)
        .with_trace_id("trace-1"),
        GraphLoopEventEnvelope::new(
            "run-1",
            "event-2",
            11,
            GraphLoopEvent::ToolExecutionEnd {
                receipt: GraphToolCallReceipt::new(
                    "node-exec-1",
                    "tool-call-1",
                    "status",
                    GraphToolCallStatus::Completed,
                ),
            },
        )
        .with_iteration_id(1)
        .with_node_id("tool-batch")
        .with_trace_id("trace-1"),
        GraphLoopEventEnvelope::new(
            "run-1",
            "event-3",
            12,
            GraphLoopEvent::AgentEnd {
                status: GraphLoopExecutionStatus::Completed,
            },
        )
        .with_iteration_id(1),
    ];
    fs::write(&input, serde_json::to_string(&events).expect("event JSON")).expect("write events");

    let query = run_marlin_cli_from_args([
        "graph",
        "query",
        "--input",
        input.to_str().expect("utf8 path"),
    ]);

    assert_eq!(query.status, 0, "{}", query.stderr);
    let summary: LoopEventQuerySummary =
        serde_json::from_str(&query.stdout).expect("loop event query summary");
    assert_eq!(summary.run_ids.len(), 1);
    assert_eq!(summary.run_ids[0].as_str(), "run-1");
    assert_eq!(summary.event_count, 3);
    assert_eq!(summary.iteration_ids, vec![1]);
    assert_eq!(summary.node_ids, vec!["tool-batch"]);
    assert_eq!(summary.trace_ids, vec!["trace-1"]);
    assert_eq!(summary.tool_event_count, 1);
    assert_eq!(
        summary.terminal_status,
        Some(GraphLoopExecutionStatus::Completed)
    );
    assert!(
        summary
            .event_types
            .contains(&"tool_execution_end".to_string())
    );
}

#[test]
fn debug_cli_graph_query_reads_project_runtime_graph_query_facts() {
    let dir = tempdir().expect("tempdir");
    let input = dir.path().join("project-runtime-query.json");
    let request = GraphQueryRequest::new(
        GraphQueryContext::new("project-alpha"),
        GraphQueryFamily::Memory,
        "button audit",
    );
    let response = GraphQueryResponse::new("receipt:project-memory", request).with_match(
        GraphQueryMatch::new(
            "project-alpha",
            "Button audit keeps icon labels stable",
            9_200,
        )
        .with_memory("memory:ui-continuation")
        .with_relationship(GraphQueryMatchRelationship::new([
            GraphQueryRelationshipFact::SameProject,
            GraphQueryRelationshipFact::ContractValidated,
        ])),
    );
    fs::write(
        &input,
        serde_json::to_string(&response).expect("response JSON"),
    )
    .expect("write response");

    let query = run_marlin_cli_from_args([
        "graph",
        "query",
        "--input",
        input.to_str().expect("utf8 path"),
    ]);

    assert_eq!(query.status, 0, "{}", query.stderr);
    let summary: ProjectRuntimeQuerySummary =
        serde_json::from_str(&query.stdout).expect("project runtime query summary");
    assert_eq!(summary.receipt_id.as_str(), "receipt:project-memory");
    assert_eq!(summary.family, GraphQueryFamily::Memory);
    assert_eq!(summary.query, "button audit");
    assert_eq!(summary.match_count, 1);
    assert_eq!(summary.source_project_ids[0].as_str(), "project-alpha");
    assert_eq!(summary.memory_ids[0].as_str(), "memory:ui-continuation");
    assert!(
        summary
            .relationship_facts
            .contains(&GraphQueryRelationshipFact::SameProject)
    );
    assert!(
        summary
            .relationship_facts
            .contains(&GraphQueryRelationshipFact::ContractValidated)
    );
    assert_eq!(summary.score_basis_points, vec![9_200]);
}

#[test]
fn debug_cli_graph_query_executes_project_memory_query_across_session_shards() {
    let dir = tempdir().expect("tempdir");
    let request_path = dir.path().join("project-runtime-request.json");
    let memory_path_a = dir.path().join("memory-a.org");
    let memory_path_b = dir.path().join("memory-b.org");
    let request = GraphQueryRequest::new(
        GraphQueryContext::new("project-alpha")
            .with_worktree("worktree-b")
            .with_root_session("root-b"),
        GraphQueryFamily::Memory,
        "button audit",
    )
    .with_limit(5);
    fs::write(
        &request_path,
        serde_json::to_string(&request).expect("request JSON"),
    )
    .expect("write request");
    fs::write(
        &memory_path_a,
        "* Button audit keeps icon labels stable\n\
         :PROPERTIES:\n\
         :MEMORY_ID: memory:ui-continuation\n\
         :PROJECT_ID: project-alpha\n\
         :WORKTREE_ID: worktree-a\n\
         :ROOT_SESSION_ID: root-a\n\
         :SESSION_ID: session-a\n\
         :CONTRACT_VALIDATED: true\n\
         :END:\n\
         * Button audit raw sibling transcript should stay hidden\n\
         :PROPERTIES:\n\
         :PROJECT_ID: project-alpha\n\
         :WORKTREE_ID: worktree-a\n\
         :ROOT_SESSION_ID: root-a\n\
         :SESSION_ID: session-a\n\
         :END:\n",
    )
    .expect("write memory shard a");
    fs::write(
        &memory_path_b,
        "* Button audit restores fallback tool cards\n\
         :PROPERTIES:\n\
         :MEMORY_ID: memory:ux-fallback\n\
         :PROJECT_ID: project-alpha\n\
         :WORKTREE_ID: worktree-c\n\
         :ROOT_SESSION_ID: root-c\n\
         :SESSION_ID: session-c\n\
         :CONTRACT_VALIDATED: true\n\
         :END:\n\
         * Button audit external memory\n\
         :PROPERTIES:\n\
         :MEMORY_ID: memory:external-ui\n\
         :PROJECT_ID: project-beta\n\
         :WORKTREE_ID: worktree-z\n\
         :ROOT_SESSION_ID: root-z\n\
         :SESSION_ID: session-z\n\
         :END:\n",
    )
    .expect("write memory shard b");

    let query = run_marlin_cli_from_args([
        "graph",
        "query",
        "--input",
        request_path.to_str().expect("utf8 path"),
        "--org-memory-fixture",
        memory_path_a.to_str().expect("utf8 path"),
        "--org-memory-fixture",
        memory_path_b.to_str().expect("utf8 path"),
        "--receipt-id",
        "receipt:cli-project-memory",
    ]);

    assert_eq!(query.status, 0, "{}", query.stderr);
    let summary: ProjectRuntimeQuerySummary =
        serde_json::from_str(&query.stdout).expect("project runtime query summary");
    assert_eq!(summary.receipt_id.as_str(), "receipt:cli-project-memory");
    assert_eq!(summary.family, GraphQueryFamily::Memory);
    assert_eq!(summary.match_count, 2);
    assert_eq!(summary.source_project_ids[0].as_str(), "project-alpha");
    assert_eq!(summary.source_root_session_ids.len(), 2);
    assert_eq!(summary.source_root_session_ids[0].as_str(), "root-a");
    assert_eq!(summary.source_root_session_ids[1].as_str(), "root-c");
    assert_eq!(summary.source_session_ids.len(), 2);
    assert_eq!(summary.source_session_ids[0].as_str(), "session-a");
    assert_eq!(summary.source_session_ids[1].as_str(), "session-c");
    assert_eq!(
        summary
            .memory_ids
            .iter()
            .map(|memory_id| memory_id.as_str())
            .collect::<Vec<_>>(),
        vec!["memory:ui-continuation", "memory:ux-fallback"]
    );
    assert!(
        summary
            .relationship_facts
            .contains(&GraphQueryRelationshipFact::SameProject)
    );
    assert!(
        summary
            .relationship_facts
            .contains(&GraphQueryRelationshipFact::ContractValidated)
    );
}
