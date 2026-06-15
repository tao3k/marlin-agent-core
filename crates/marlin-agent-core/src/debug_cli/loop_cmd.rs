//! `marlin loop ...` command implementations.

use std::path::{Path, PathBuf};

use crate::{
    GraphId, GraphLoopController, GraphLoopEvidencePolicy, GraphLoopExecutionRequest,
    GraphLoopIterationReport, GraphLoopRunRequest, GraphLoopStopPolicy, RunId, TokioAgentRuntime,
    TokioGraphLoopController, runtime::GraphLoopRunObservation,
};

use super::{
    MarlinCliResult,
    args::{ArgCursor, LoopInspectOptions, LoopReplayOptions, LoopRunOptions},
    catalog::DebugExecutorCatalog,
    executor::{debug_kernel, read_debug_executor_catalog},
    graph::parse_graph_input,
    io::{
        block_on, read_input, read_iteration_reports_from_path, read_reports_from_store,
        write_loop_reports,
    },
    loop_usage,
    receipts::{LoopInspectReceipt, LoopReplayReceipt, LoopRunReceipt},
};

pub(super) fn dispatch_loop(cursor: &mut ArgCursor) -> Result<MarlinCliResult, String> {
    let Some(command) = cursor.next() else {
        return Err(format!("missing loop command\n{}", loop_usage()));
    };

    match command.as_str() {
        "run" => {
            let options = LoopRunOptions::parse(cursor)?;
            let request = read_loop_run_request(options.input.as_deref(), options.max_iterations)?;
            let catalog = read_debug_executor_catalog(options.catalog.as_deref())?;
            let requested_run_id = request.initial_request.run_id.clone();
            let output = block_on(run_loop_controller(request, catalog))?;
            let report_path = options
                .store
                .as_deref()
                .map(|store| write_loop_reports(store, &output.reports))
                .transpose()?
                .flatten();
            let receipt = loop_run_receipt(
                requested_run_id,
                output.reports,
                report_path,
                output.runtime_observation,
            );
            Ok(MarlinCliResult::success_json(&receipt))
        }
        "replay" => {
            let options = LoopReplayOptions::parse(cursor)?;
            let reports = read_iteration_reports_from_path(&options.trace_or_report)?;
            let receipt = replay_reports(&options.trace_or_report, &reports);
            Ok(MarlinCliResult::success_json(&receipt))
        }
        "inspect" => {
            let options = LoopInspectOptions::parse(cursor)?;
            let (report_path, reports) = read_reports_from_store(&options.store, &options.run_id)?;
            let receipt = inspect_reports(&options.run_id, &report_path, &reports);
            Ok(MarlinCliResult::success_json(&receipt))
        }
        "-h" | "--help" | "help" => Ok(MarlinCliResult::success_text(loop_usage())),
        unknown => Err(format!(
            "unknown loop command `{unknown}`\n{}",
            loop_usage()
        )),
    }
}

fn read_loop_run_request(
    input: Option<&Path>,
    max_iterations: Option<u64>,
) -> Result<GraphLoopRunRequest, String> {
    let raw = read_input(input)?;
    let mut request = serde_json::from_str::<GraphLoopRunRequest>(&raw).or_else(|_| {
        parse_graph_input(&raw).map(|graph| {
            GraphLoopRunRequest::new(GraphLoopExecutionRequest::new("marlin-loop-run", graph))
        })
    })?;
    if let Some(max_iterations) = max_iterations {
        request = request.with_stop_policy(GraphLoopStopPolicy::max_iterations(max_iterations));
    }
    if request.evidence_policy == GraphLoopEvidencePolicy::default() {
        request = request.with_evidence_policy(GraphLoopEvidencePolicy::replayable_runtime());
    }
    Ok(request)
}

struct LoopRunControllerOutput {
    reports: Vec<GraphLoopIterationReport>,
    runtime_observation: Option<GraphLoopRunObservation>,
}

async fn run_loop_controller(
    request: GraphLoopRunRequest,
    catalog: DebugExecutorCatalog,
) -> Result<LoopRunControllerOutput, String> {
    let (runtime, _events) = TokioAgentRuntime::new(64);
    let requested_run_id = request.initial_request.run_id.clone();
    let kernel = debug_kernel(
        &request.initial_request.run_id,
        &request.initial_request.graph,
        catalog,
    )?;
    let controller = TokioGraphLoopController::new(kernel);
    let reports = controller
        .spawn_loop(request, &runtime)
        .join()
        .await
        .map_err(|error| format!("loop controller task failed to join: {error}"))?;
    let runtime_observation = runtime.graph_loop_runs().read_registry(|registry| {
        registry
            .snapshot(0)
            .runs
            .into_iter()
            .find(|observation| observation.run_id.as_str() == requested_run_id)
    });
    Ok(LoopRunControllerOutput {
        reports,
        runtime_observation,
    })
}

fn loop_run_receipt(
    requested_run_id: String,
    reports: Vec<GraphLoopIterationReport>,
    report_path: Option<PathBuf>,
    runtime_observation: Option<GraphLoopRunObservation>,
) -> LoopRunReceipt {
    let terminal = reports.last();
    LoopRunReceipt {
        run_id: RunId::new(
            terminal
                .map(|report| report.execution_result.snapshot.run_id.clone())
                .unwrap_or(requested_run_id),
        ),
        report_path,
        iteration_count: reports.len(),
        terminal_status: terminal.map(|report| report.execution_result.status.clone()),
        runtime_observation,
        replayable: reports_are_replayable(&reports),
        missing_trace_count: missing_trace_count(&reports),
        reports,
    }
}

fn replay_reports(source: &Path, reports: &[GraphLoopIterationReport]) -> LoopReplayReceipt {
    let statuses = reports
        .iter()
        .map(|report| report.execution_result.status.clone())
        .collect::<Vec<_>>();
    let run_ids = reports
        .iter()
        .map(|report| RunId::new(report.execution_result.snapshot.run_id.clone()))
        .collect::<Vec<_>>();
    let graph_ids = reports
        .iter()
        .map(|report| GraphId::new(report.execution_result.snapshot.graph_id.clone()))
        .collect::<Vec<_>>();
    let diagnostics = reports
        .iter()
        .flat_map(|report| report.execution_result.diagnostics.clone())
        .collect::<Vec<_>>();
    LoopReplayReceipt {
        source: source.to_path_buf(),
        replayable: reports_are_replayable(reports),
        iteration_count: reports.len(),
        missing_trace_count: missing_trace_count(reports),
        statuses,
        run_ids,
        graph_ids,
        diagnostics,
    }
}

fn inspect_reports(
    run_id: &str,
    report_path: &Path,
    reports: &[GraphLoopIterationReport],
) -> LoopInspectReceipt {
    let terminal = reports.last();
    LoopInspectReceipt {
        run_id: RunId::new(run_id),
        report_path: report_path.to_path_buf(),
        iteration_count: reports.len(),
        terminal_status: terminal.map(|report| report.execution_result.status.clone()),
        terminal_graph_id: terminal
            .map(|report| GraphId::new(report.execution_result.snapshot.graph_id.clone())),
        replayable: reports_are_replayable(reports),
        missing_trace_count: missing_trace_count(reports),
        diagnostics: reports
            .iter()
            .flat_map(|report| report.execution_result.diagnostics.clone())
            .collect(),
    }
}

fn reports_are_replayable(reports: &[GraphLoopIterationReport]) -> bool {
    reports.iter().all(|report| report.trace.is_some())
}

fn missing_trace_count(reports: &[GraphLoopIterationReport]) -> usize {
    reports
        .iter()
        .filter(|report| report.trace.is_none())
        .count()
}
