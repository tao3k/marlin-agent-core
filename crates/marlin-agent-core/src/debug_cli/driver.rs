//! Public `marlin` debug CLI facade and command dispatcher.

use std::env;

use super::{args::ArgCursor, graph, loop_cmd};

/// Process-shaped result returned by the `marlin` debug CLI facade.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarlinCliResult {
    /// Process exit status code.
    pub status: i32,
    /// Standard output payload.
    pub stdout: String,
    /// Standard error payload.
    pub stderr: String,
}

impl MarlinCliResult {
    pub fn success_json<T>(value: &T) -> Self
    where
        T: serde::Serialize,
    {
        match serde_json::to_string_pretty(value) {
            Ok(stdout) => Self {
                status: 0,
                stdout: format!("{stdout}\n"),
                stderr: String::new(),
            },
            Err(error) => Self::error(format!("failed to encode JSON output: {error}")),
        }
    }

    pub fn success_text(stdout: impl Into<String>) -> Self {
        Self {
            status: 0,
            stdout: stdout.into(),
            stderr: String::new(),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            status: 2,
            stdout: String::new(),
            stderr: format!("{}\n", message.into()),
        }
    }
}

/// Runs the `marlin` harness/debug CLI from process arguments.
pub fn run_marlin_cli() -> MarlinCliResult {
    run_marlin_cli_from_args(env::args().skip(1))
}

/// Runs the `marlin` harness/debug CLI from an explicit argv tail.
pub fn run_marlin_cli_from_args<I, S>(args: I) -> MarlinCliResult
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let args = args.into_iter().map(Into::into).collect::<Vec<_>>();
    match dispatch(args) {
        Ok(result) => result,
        Err(error) => MarlinCliResult::error(error),
    }
}

fn dispatch(args: Vec<String>) -> Result<MarlinCliResult, String> {
    let mut cursor = ArgCursor::new(args);
    let Some(command) = cursor.next() else {
        return Ok(MarlinCliResult::success_text(usage()));
    };

    match command.as_str() {
        "graph" => graph::dispatch_graph(&mut cursor),
        "loop" => loop_cmd::dispatch_loop(&mut cursor),
        "-h" | "--help" | "help" => Ok(MarlinCliResult::success_text(usage())),
        unknown => Err(format!("unknown marlin command `{unknown}`\n{}", usage())),
    }
}

pub(in crate::debug_cli) fn usage() -> String {
    format!(
        "Usage:\n  marlin graph <query|propose|validate|run> [options]\n  marlin loop <run|replay|inspect> [options]\n\n{}\n{}",
        graph_usage(),
        loop_usage()
    )
}

pub(in crate::debug_cli) fn graph_usage() -> &'static str {
    "Graph commands:\n  marlin graph query --input <graph-or-proposal-or-receipt.json>\n  marlin graph query --input <graph-query-request.json> --org-memory-fixture <memory.org> [--org-memory-fixture <memory.org> ...] [--receipt-id <id>]\n  marlin graph query --input <graph-query-request.json> --org-memory-store-root <dir> --org-memory-root <relative-memory.org> [--org-memory-root <relative-memory.org> ...] [--receipt-id <id>]\n  marlin graph query --input <graph-query-request.json> --org-tool-store-root <dir> --org-tool-root <relative-tools.org> [--org-tool-root <relative-tools.org> ...] [--receipt-id <id>]\n  marlin graph propose --strategy static|gerbil --input <graph-or-gerbil-request.json>\n  marlin graph validate --input <proposal.json>\n  marlin graph run --input <graph-or-request.json> [--run-id <id>] [--catalog <catalog.toml|json>]"
}

pub(in crate::debug_cli) fn loop_usage() -> &'static str {
    "Loop commands:\n  marlin loop run --input <graph-or-run-request.json> [--max-iterations N] [--store <dir>|--no-store] [--catalog <catalog.toml|json>]\n  marlin loop replay <trace-or-report.json>\n  marlin loop inspect <run-id> [--store <dir>]"
}
