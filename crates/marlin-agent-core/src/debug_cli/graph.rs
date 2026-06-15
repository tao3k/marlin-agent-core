//! `marlin graph ...` command implementations.

use std::{collections::BTreeSet, path::Path};

use crate::{
    GerbilLoopGraphPolicyCompilationRequest, GraphId, GraphLoopExecutionRequest,
    GraphLoopExecutionResult, GraphLoopIterationReport, GraphLoopKernel, GraphLoopStrategy,
    GraphLoopStrategyRuntime, GraphPolicyProposal, LoopGraph, RunId, TokioAgentRuntime,
    compile_gerbil_loop_graph_policy,
};

use super::{
    MarlinCliResult,
    args::{ArgCursor, CommonOptions, GraphProposeOptions, GraphRunOptions},
    catalog::DebugExecutorCatalog,
    executor::{debug_kernel, read_debug_executor_catalog},
    graph_usage,
    io::{block_on, read_input, read_json_input},
    receipts::{GraphQueryOutput, GraphQuerySummary, LoopQuerySummary, LoopRunReceipt},
};

pub(super) fn dispatch_graph(cursor: &mut ArgCursor) -> Result<MarlinCliResult, String> {
    let Some(command) = cursor.next() else {
        return Err(format!("missing graph command\n{}", graph_usage()));
    };

    match command.as_str() {
        "query" => {
            let options = CommonOptions::parse(cursor)?;
            let input = read_input(options.input.as_deref())?;
            let summary = graph_query_output(&input)?;
            Ok(MarlinCliResult::success_json(&summary))
        }
        "propose" => {
            let options = GraphProposeOptions::parse(cursor)?;
            let proposal = propose_graph_policy(options)?;
            Ok(MarlinCliResult::success_json(&proposal))
        }
        "validate" => {
            let options = CommonOptions::parse(cursor)?;
            let proposal: GraphPolicyProposal = read_json_input(options.input.as_deref())?;
            let report = proposal.validate();
            Ok(MarlinCliResult::success_json(&report))
        }
        "run" => {
            let options = GraphRunOptions::parse(cursor)?;
            let request = read_graph_execution_request(options.input.as_deref(), &options.run_id)?;
            let catalog = read_debug_executor_catalog(options.catalog.as_deref())?;
            let result = block_on(run_graph_execution(request, catalog))?;
            Ok(MarlinCliResult::success_json(&result))
        }
        "-h" | "--help" | "help" => Ok(MarlinCliResult::success_text(graph_usage())),
        unknown => Err(format!(
            "unknown graph command `{unknown}`\n{}",
            graph_usage()
        )),
    }
}

fn graph_query_summary(input: &str) -> Result<GraphQuerySummary, String> {
    let graph = parse_graph_input(input)?;
    let executors = graph
        .nodes
        .iter()
        .map(|node| node.executor.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let targeted = graph
        .edges
        .iter()
        .map(|edge| edge.to.as_str())
        .collect::<BTreeSet<_>>();
    let root_nodes = graph
        .nodes
        .iter()
        .filter(|node| !targeted.contains(node.id.as_str()))
        .map(|node| node.id.clone())
        .collect::<Vec<_>>();
    Ok(GraphQuerySummary {
        graph_id: graph.graph_id,
        node_count: graph.nodes.len(),
        edge_count: graph.edges.len(),
        executors,
        root_nodes,
    })
}

fn graph_query_output(input: &str) -> Result<GraphQueryOutput, String> {
    match graph_query_summary(input) {
        Ok(summary) => Ok(GraphQueryOutput::Graph(summary)),
        Err(graph_error) => loop_query_summary(input)
            .map(GraphQueryOutput::Loop)
            .map_err(|loop_error| {
                format!(
                    "expected graph/proposal JSON or loop report JSON: graph parse failed: {graph_error}; loop parse failed: {loop_error}"
                )
            }),
    }
}

fn loop_query_summary(input: &str) -> Result<LoopQuerySummary, String> {
    let reports = parse_loop_report_input(input)?;
    let terminal = reports.last();
    Ok(LoopQuerySummary {
        run_ids: reports
            .iter()
            .map(|report| RunId::new(report.execution_result.snapshot.run_id.clone()))
            .collect(),
        graph_ids: reports
            .iter()
            .map(|report| GraphId::new(report.execution_result.snapshot.graph_id.clone()))
            .collect(),
        iteration_count: reports.len(),
        terminal_status: terminal.map(|report| report.execution_result.status.clone()),
        replayable: reports.iter().all(|report| report.trace.is_some()),
        missing_trace_count: reports
            .iter()
            .filter(|report| report.trace.is_none())
            .count(),
        statuses: reports
            .iter()
            .map(|report| report.execution_result.status.clone())
            .collect(),
        visited_nodes_by_iteration: reports
            .iter()
            .map(|report| report.execution_result.visited_nodes.clone())
            .collect(),
        diagnostic_count: reports
            .iter()
            .map(|report| report.execution_result.diagnostics.len())
            .sum(),
        node_receipt_count: reports
            .iter()
            .map(|report| report.execution_result.node_receipts.len())
            .sum(),
        trace_event_count: reports
            .iter()
            .filter_map(|report| report.trace.as_ref())
            .map(|trace| trace.events.len())
            .sum(),
    })
}

fn parse_loop_report_input(input: &str) -> Result<Vec<GraphLoopIterationReport>, String> {
    serde_json::from_str::<Vec<GraphLoopIterationReport>>(input)
        .or_else(|_| {
            serde_json::from_str::<GraphLoopIterationReport>(input).map(|report| vec![report])
        })
        .or_else(|_| serde_json::from_str::<LoopRunReceipt>(input).map(|receipt| receipt.reports))
        .map_err(|error| {
            format!(
                "expected GraphLoopIterationReport, GraphLoopIterationReport array, or LoopRunReceipt JSON: {error}"
            )
        })
}

fn propose_graph_policy(options: GraphProposeOptions) -> Result<GraphPolicyProposal, String> {
    match options.strategy.as_str() {
        "static" => {
            let graph: LoopGraph = read_json_input(options.input.as_deref())?;
            Ok(GraphPolicyProposal::new(
                GraphLoopStrategy::new(
                    options.strategy_id,
                    options.version,
                    GraphLoopStrategyRuntime::StaticPolicy,
                ),
                graph,
                options.input_digest,
                options.output_digest,
            ))
        }
        "gerbil" => {
            let request: GerbilLoopGraphPolicyCompilationRequest =
                read_json_input(options.input.as_deref())?;
            compile_gerbil_loop_graph_policy(request)
                .map_err(|error| format!("failed to compile Gerbil graph policy: {error:?}"))
        }
        unknown => Err(format!(
            "unsupported graph proposal strategy `{unknown}`; expected static or gerbil"
        )),
    }
}

fn read_graph_execution_request(
    input: Option<&Path>,
    default_run_id: &str,
) -> Result<GraphLoopExecutionRequest, String> {
    let raw = read_input(input)?;
    serde_json::from_str::<GraphLoopExecutionRequest>(&raw).or_else(|_| {
        parse_graph_input(&raw).map(|graph| GraphLoopExecutionRequest::new(default_run_id, graph))
    })
}

pub(super) fn parse_graph_input(input: &str) -> Result<LoopGraph, String> {
    serde_json::from_str::<LoopGraph>(input)
        .or_else(|_| {
            serde_json::from_str::<GraphPolicyProposal>(input)
                .map(|proposal| proposal.proposed_graph)
        })
        .map_err(|error| format!("expected LoopGraph or GraphPolicyProposal JSON: {error}"))
}

async fn run_graph_execution(
    request: GraphLoopExecutionRequest,
    catalog: DebugExecutorCatalog,
) -> Result<GraphLoopExecutionResult, String> {
    let (runtime, _events) = TokioAgentRuntime::new(64);
    let kernel = debug_kernel(&request.run_id, &request.graph, catalog)?;
    kernel
        .spawn_execution(request, &runtime)
        .join()
        .await
        .map_err(|error| format!("graph execution task failed to join: {error}"))
}
