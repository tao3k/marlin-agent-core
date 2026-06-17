//! `marlin state ...` command implementations.

use marlin_agent_environment::RuntimeStateStorageInitializer;
use marlin_agent_protocol::{
    RuntimeHome, RuntimeStateDirectoryKind, RuntimeStateObjectPath, RuntimeStateStorageReceipt,
};
use serde::{Deserialize, Serialize};

use super::{
    MarlinCliResult,
    args::{ArgCursor, StateHomeOptions},
    state_home::resolve_runtime_state_layout,
    state_usage,
};

pub(super) fn dispatch_state(cursor: &mut ArgCursor) -> Result<MarlinCliResult, String> {
    let Some(command) = cursor.next() else {
        return Err(format!("missing state command\n{}", state_usage()));
    };

    match command.as_str() {
        "init" => {
            let options = StateHomeOptions::parse(cursor)?;
            let receipt = initialize_state_home(options)?;
            Ok(MarlinCliResult::success_json(&receipt))
        }
        "inspect" => {
            let options = StateHomeOptions::parse(cursor)?;
            let receipt = inspect_state_home(options)?;
            Ok(MarlinCliResult::success_json(&receipt))
        }
        "-h" | "--help" | "help" => Ok(MarlinCliResult::success_text(state_usage())),
        unknown => Err(format!(
            "unknown state command `{unknown}`\n{}",
            state_usage()
        )),
    }
}

fn initialize_state_home(options: StateHomeOptions) -> Result<RuntimeStateStorageReceipt, String> {
    let layout = resolve_runtime_state_layout(options.home)?;
    Ok(RuntimeStateStorageInitializer::new().ensure_layout(&layout))
}

fn inspect_state_home(options: StateHomeOptions) -> Result<StateInspectReceipt, String> {
    let layout = resolve_runtime_state_layout(options.home)?;

    Ok(StateInspectReceipt {
        home: layout.home.clone(),
        directories: layout
            .directories
            .iter()
            .map(|directory| StateDirectoryInspection {
                kind: directory.kind,
                path: directory.path.clone(),
                exists: directory.path.exists(),
                is_dir: directory.path.is_dir(),
            })
            .collect(),
        object_paths: StateObjectPathPreview {
            session: layout
                .session_path("default-session")
                .expect("standard layout should include sessions directory"),
            memory_shard: layout
                .memory_shard_path("default-memory-shard")
                .expect("standard layout should include memory directory"),
            receipt: layout
                .receipt_path("default-receipt")
                .expect("standard layout should include receipts directory"),
            graph_cache: layout
                .graph_cache_path("default-graph-cache")
                .expect("standard layout should include graph cache directory"),
        },
    })
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct StateInspectReceipt {
    home: RuntimeHome,
    directories: Vec<StateDirectoryInspection>,
    object_paths: StateObjectPathPreview,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct StateDirectoryInspection {
    kind: RuntimeStateDirectoryKind,
    path: std::path::PathBuf,
    exists: bool,
    is_dir: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct StateObjectPathPreview {
    session: RuntimeStateObjectPath,
    memory_shard: RuntimeStateObjectPath,
    receipt: RuntimeStateObjectPath,
    graph_cache: RuntimeStateObjectPath,
}
