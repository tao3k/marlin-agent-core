//! Debug executor catalog for harness-only graph execution.

use crate::{LoopGraph, TokioGraphLoopKernel};

use super::catalog::DebugExecutorCatalog;

pub(super) fn read_debug_executor_catalog(
    catalog: Option<&std::path::Path>,
) -> Result<DebugExecutorCatalog, String> {
    catalog
        .map(DebugExecutorCatalog::from_path)
        .unwrap_or_else(|| Ok(DebugExecutorCatalog::with_builtin_debug_executors()))
}

pub(super) fn debug_kernel(
    run_id: &str,
    graph: &LoopGraph,
    catalog: DebugExecutorCatalog,
) -> Result<TokioGraphLoopKernel, String> {
    catalog.install_into(TokioGraphLoopKernel::new(run_id, graph.graph_id.clone()))
}
