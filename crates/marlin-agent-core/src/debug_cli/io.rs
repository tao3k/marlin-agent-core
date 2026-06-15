//! JSON and run-store IO helpers for the `marlin` debug CLI.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use serde::de::DeserializeOwned;

use crate::GraphLoopIterationReport;

use super::receipts::LoopRunReceipt;

pub(super) fn read_json_input<T>(input: Option<&Path>) -> Result<T, String>
where
    T: DeserializeOwned,
{
    let raw = read_input(input)?;
    serde_json::from_str(&raw).map_err(|error| format!("invalid JSON input: {error}"))
}

pub(super) fn read_input(input: Option<&Path>) -> Result<String, String> {
    match input {
        Some(path) if path == Path::new("-") => read_stdin(),
        Some(path) => fs::read_to_string(path)
            .map_err(|error| format!("failed to read input {}: {error}", path.display())),
        None => read_stdin(),
    }
}

pub(super) fn block_on<T>(
    future: impl std::future::Future<Output = Result<T, String>>,
) -> Result<T, String> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| format!("failed to create Tokio runtime: {error}"))?
        .block_on(future)
}

pub(super) fn write_loop_reports(
    store: &Path,
    reports: &[GraphLoopIterationReport],
) -> Result<Option<PathBuf>, String> {
    let Some(first) = reports.first() else {
        return Ok(None);
    };
    fs::create_dir_all(store)
        .map_err(|error| format!("failed to create run store {}: {error}", store.display()))?;
    let path = run_report_path(store, &first.execution_result.snapshot.run_id);
    let json = serde_json::to_string_pretty(reports)
        .map_err(|error| format!("failed to encode run report: {error}"))?;
    fs::write(&path, json)
        .map_err(|error| format!("failed to write run report {}: {error}", path.display()))?;
    Ok(Some(path))
}

pub(super) fn read_reports_from_store(
    store: &Path,
    run_id: &str,
) -> Result<(PathBuf, Vec<GraphLoopIterationReport>), String> {
    let path = run_report_path(store, run_id);
    read_iteration_reports_from_path(&path).map(|reports| (path, reports))
}

pub(super) fn read_iteration_reports_from_path(
    path: &Path,
) -> Result<Vec<GraphLoopIterationReport>, String> {
    let raw = fs::read_to_string(path)
        .map_err(|error| format!("failed to read report {}: {error}", path.display()))?;
    serde_json::from_str::<Vec<GraphLoopIterationReport>>(&raw)
        .or_else(|_| {
            serde_json::from_str::<GraphLoopIterationReport>(&raw).map(|report| vec![report])
        })
        .or_else(|_| serde_json::from_str::<LoopRunReceipt>(&raw).map(|receipt| receipt.reports))
        .map_err(|error| {
            format!(
                "expected GraphLoopIterationReport, GraphLoopIterationReport array, or LoopRunReceipt JSON: {error}"
            )
        })
}

fn read_stdin() -> Result<String, String> {
    let mut input = String::new();
    io::Read::read_to_string(&mut io::stdin(), &mut input)
        .map_err(|error| format!("failed to read stdin: {error}"))?;
    Ok(input)
}

fn sanitize_run_id(run_id: &str) -> String {
    run_id
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

pub(super) fn run_report_path(store: &Path, run_id: &str) -> PathBuf {
    store.join(format!("{}.json", sanitize_run_id(run_id)))
}
