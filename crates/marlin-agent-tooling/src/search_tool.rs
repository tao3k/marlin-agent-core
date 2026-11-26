// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Reusable resident workspace-search tool capability executing pure Gerbil Scheme
//! `:gerbil-ascent` + `:gerbil-parser` + POO S-expression queries over ASP's
//! zero-regex protocol-native Open EDB (`WorkspaceSearchIndex::project_prefiltered_edb`).
//!
//! All query parsing, safe pre-tool-call AST repair (`scheme/search-dsl/repair.ss`),
//! and fail-closed rejection of retired `@lang:` / `@sym:` syntax live in Scheme;
//! Rust remains a thin preset mapping and index-custody framework.

use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use marlin_agent_protocol::ModelToolCall;
use marlin_agent_runtime::{
    ModelToolDeclaration, ModelToolOutcome, RuntimeContext, RuntimeFuture, ToolRuntime,
};
use marlin_agent_search_engine::{
    SearchDslRunner, WorkspaceIndexConfig, WorkspaceSearchIndex, default_workspace_snapshot_path,
};

/// Shared resident index handle reused across TUI Helm and Agent tool calls.
pub type SharedWorkspaceSearchIndex = Arc<RwLock<WorkspaceSearchIndex>>;

/// Late-bound slot sharing one resident [`WorkspaceSearchIndex`] between the host
/// UI (e.g. TUI Helm warm-up / `C-l` incremental sync) and LLM Agent tool calls.
#[derive(Clone, Default)]
pub struct SharedSearchIndexSlot {
    inner: Arc<RwLock<Option<SharedWorkspaceSearchIndex>>>,
    dsl_runner: Arc<RwLock<Option<SearchDslRunner>>>,
}

impl SharedSearchIndexSlot {
    /// Create an empty slot that lazily opens a cached `mmap` snapshot if queried
    /// before the host attaches a warm index.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Create a slot pre-populated with an existing warm resident index.
    #[must_use]
    pub fn from_index(index: SharedWorkspaceSearchIndex) -> Self {
        Self {
            inner: Arc::new(RwLock::new(Some(index))),
            dsl_runner: Arc::new(RwLock::new(None)),
        }
    }

    /// Attach or replace the shared resident index (e.g. after background warm-up).
    pub fn attach(&self, index: SharedWorkspaceSearchIndex) {
        if let Ok(mut guard) = self.inner.write() {
            *guard = Some(index);
        }
    }

    /// Attach an AOT-linked or custom Scheme Search DSL runner.
    pub fn attach_dsl_runner(&self, runner: SearchDslRunner) {
        if let Ok(mut guard) = self.dsl_runner.write() {
            *guard = Some(runner);
        }
    }

    /// Resolve the active [`SearchDslRunner`], falling back to the compiled package
    /// runner when `linked-ui` is not active.
    #[must_use]
    pub fn dsl_runner(&self) -> Option<SearchDslRunner> {
        if let Ok(guard) = self.dsl_runner.read()
            && let Some(existing) = guard.as_ref()
        {
            return Some(existing.clone());
        }
        marlin_agent_search_engine::default_search_dsl_runner()
    }

    /// Return the attached resident index or lazily open/build a cached `mmap` index
    /// for `workspace` and store it in the slot for subsequent calls.
    #[must_use]
    pub fn get_or_open(&self, workspace: &Path) -> SharedWorkspaceSearchIndex {
        if let Ok(guard) = self.inner.read()
            && let Some(existing) = guard.as_ref()
        {
            return existing.clone();
        }
        let snapshot_path = default_workspace_snapshot_path(workspace);
        let opened = Arc::new(RwLock::new(WorkspaceSearchIndex::open_or_build_cached(
            workspace,
            &snapshot_path,
            WorkspaceIndexConfig::default(),
        )));
        self.attach(opened.clone());
        opened
    }

    /// Incrementally reconcile the attached resident index with on-disk workspace
    /// changes and persist the cached `mmap` snapshot when files changed.
    #[must_use]
    pub fn sync_from_disk_and_persist(&self, workspace: &Path) -> bool {
        let shared = self.get_or_open(workspace);
        let Ok(mut guard) = shared.write() else {
            return false;
        };
        let receipt = guard.sync_workspace_from_disk();
        let changed = receipt.upserted_files > 0 || receipt.deleted_files > 0;
        if changed {
            let snapshot_path = default_workspace_snapshot_path(workspace);
            let _ = guard.save_snapshot(&snapshot_path);
        }
        changed
    }
}

/// Reusable resident workspace search capability for Agent tool calls.
/// Accepts exclusively canonical Scheme S-expression search DSL forms.
pub struct ResidentSearchToolHandler {
    workspace: PathBuf,
    declaration: ModelToolDeclaration,
    slot: SharedSearchIndexSlot,
}

impl ResidentSearchToolHandler {
    /// Construct a handler bound to `workspace`, `declaration`, and `slot`.
    #[must_use]
    pub fn new(
        workspace: PathBuf,
        declaration: ModelToolDeclaration,
        slot: SharedSearchIndexSlot,
    ) -> Self {
        Self {
            workspace,
            declaration,
            slot,
        }
    }
}

impl ToolRuntime for ResidentSearchToolHandler {
    type Invocation = ModelToolCall;
    type Output = ModelToolOutcome;

    fn run_tool(
        &self,
        call: ModelToolCall,
        context: RuntimeContext,
    ) -> RuntimeFuture<Self::Output> {
        let workspace = self.workspace.clone();
        let declaration = self.declaration.clone();
        let slot = self.slot.clone();
        Box::pin(async move {
            if !declaration.admits(&call) {
                return ModelToolOutcome::Failed("invalid search tool arguments".into());
            }
            let raw_query = call.arguments[declaration.argument()]
                .as_str()
                .expect("admitted string query")
                .to_owned();
            let active_root = context
                .active_working_copy()
                .map_or(workspace, |binding| binding.working_copy.path.clone());
            match execute_resident_search_tool(
                &active_root,
                &slot,
                &raw_query,
                declaration.output_limit(),
            ) {
                Ok(rendered) => ModelToolOutcome::Completed(rendered),
                Err(message) => ModelToolOutcome::Failed(message),
            }
        })
    }
}

fn execute_resident_search_tool(
    workspace: &Path,
    slot: &SharedSearchIndexSlot,
    raw_query: &str,
    output_limit: usize,
) -> Result<String, String> {
    let trimmed = raw_query.trim();
    if trimmed.is_empty() {
        return Err(
            "workspace_search requires a non-empty native :gerbil-ascent S-expression query"
                .to_owned(),
        );
    }
    let Some(runner) = slot.dsl_runner() else {
        return Err(
            "search DSL Scheme runtime is not attached to SharedSearchIndexSlot".to_owned(),
        );
    };
    let _ = slot.sync_from_disk_and_persist(workspace);
    let shared = slot.get_or_open(workspace);
    runner(shared, trimmed, output_limit).map_err(|err| err.to_string())
}
