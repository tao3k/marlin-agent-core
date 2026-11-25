// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Reusable resident workspace-search tool capability backed by ASP's
//! sub-millisecond `WorkspaceSearchIndex` (`open_or_build_cached`, `@sym:`, `@graph:`).

use std::collections::HashSet;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use marlin_agent_protocol::ModelToolCall;
use marlin_agent_runtime::{
    ModelToolDeclaration, ModelToolOutcome, RuntimeContext, RuntimeFuture, ToolRuntime,
};
use marlin_agent_search_engine::{
    WorkspaceIndexConfig, WorkspaceSearchHit, WorkspaceSearchIndex, WorkspaceSearchQuery,
    WorkspaceSearchReport, default_workspace_snapshot_path,
};

const MAX_AGENT_SEARCH_HITS: usize = 32;
const AGENT_PREVIEW_CONTEXT_RADIUS: usize = 2;

/// Shared resident index handle reused across TUI Helm and Agent tool calls.
pub type SharedWorkspaceSearchIndex = Arc<RwLock<WorkspaceSearchIndex>>;

/// Late-bound slot sharing one resident [`WorkspaceSearchIndex`] between the host
/// UI (e.g. TUI Helm warm-up / `C-l` incremental sync) and LLM Agent tool calls.
#[derive(Clone, Default)]
pub struct SharedSearchIndexSlot {
    inner: Arc<RwLock<Option<SharedWorkspaceSearchIndex>>>,
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
        }
    }

    /// Attach or replace the shared resident index (e.g. after background warm-up).
    pub fn attach(&self, index: SharedWorkspaceSearchIndex) {
        if let Ok(mut guard) = self.inner.write() {
            *guard = Some(index);
        }
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

/// Reusable pure-Rust resident workspace search capability for Agent tool calls.
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
            let rendered = execute_resident_search_tool(
                &active_root,
                &slot,
                &raw_query,
                declaration.output_limit(),
            );
            ModelToolOutcome::Completed(rendered)
        })
    }
}

struct ParsedAgentSearchQuery {
    pattern: String,
    local_tokens: Vec<String>,
    languages: Vec<String>,
    documents: Vec<String>,
    path_prefix: Option<String>,
    extension: Option<String>,
    symbol: Option<String>,
    graph_target: Option<String>,
}

impl ParsedAgentSearchQuery {
    fn parse(raw: &str) -> Self {
        let (async_raw, local_raw) = match raw.find('#') {
            Some(idx) => (raw[..idx].trim(), raw[idx + 1..].trim()),
            None => (raw.trim(), ""),
        };
        let mut pattern_parts = Vec::new();
        let mut languages = Vec::new();
        let mut seen_languages = HashSet::new();
        let mut documents = Vec::new();
        let mut seen_documents = HashSet::new();
        let mut path_prefix = None;
        let mut extension = None;
        let mut symbol = None;
        let mut graph_target = None;

        for token in async_raw.split_whitespace() {
            if let Some(val) = token.strip_prefix("@lang:") {
                if !val.is_empty() && seen_languages.insert(val) {
                    languages.push(val.to_owned());
                }
            } else if let Some(val) = token.strip_prefix("@doc:") {
                if !val.is_empty() && seen_documents.insert(val) {
                    documents.push(val.to_owned());
                }
            } else if let Some(val) = token.strip_prefix("@prefix:") {
                if !val.is_empty() {
                    path_prefix = Some(val.to_owned());
                }
            } else if let Some(val) = token.strip_prefix("@ext:") {
                if !val.is_empty() {
                    extension = Some(val.to_owned());
                }
            } else if let Some(val) = token.strip_prefix("@sym:") {
                if !val.is_empty() {
                    symbol = Some(val.to_owned());
                }
            } else if let Some(val) = token.strip_prefix("@graph:") {
                if !val.is_empty() {
                    graph_target = Some(val.to_owned());
                }
            } else {
                pattern_parts.push(token);
            }
        }

        Self {
            pattern: pattern_parts.join(" "),
            local_tokens: local_raw.split_whitespace().map(str::to_owned).collect(),
            languages,
            documents,
            path_prefix,
            extension,
            symbol,
            graph_target,
        }
    }
}

fn execute_resident_search_tool(
    workspace: &Path,
    slot: &SharedSearchIndexSlot,
    raw_query: &str,
    output_limit: usize,
) -> String {
    let parsed = ParsedAgentSearchQuery::parse(raw_query);
    let _ = slot.sync_from_disk_and_persist(workspace);
    let shared = slot.get_or_open(workspace);
    let report = match shared.read() {
        Ok(index) => index.query(&WorkspaceSearchQuery {
            pattern: &parsed.pattern,
            languages: &parsed.languages,
            documents: &parsed.documents,
            path_prefix: parsed.path_prefix.as_deref(),
            extension: parsed.extension.as_deref(),
            symbol: parsed.symbol.as_deref(),
            graph_target: parsed.graph_target.as_deref(),
            max_hits: MAX_AGENT_SEARCH_HITS,
            context_radius: AGENT_PREVIEW_CONTEXT_RADIUS,
        }),
        Err(_) => return "error: resident search index lock poisoned".to_owned(),
    };
    format_search_evidence(&report, &parsed.local_tokens, output_limit)
}

fn hit_matches_local_tokens(hit: &WorkspaceSearchHit, local_tokens: &[String]) -> bool {
    if local_tokens.is_empty() {
        return true;
    }
    let sym_names: String = hit
        .symbols
        .iter()
        .map(|s| s.name.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let haystack = format!(
        "{} {} {} {} {}",
        hit.path.to_ascii_lowercase(),
        hit.first_line_text.to_ascii_lowercase(),
        hit.matched_axes.join(" ").to_ascii_lowercase(),
        sym_names.to_ascii_lowercase(),
        hit.graph_relations.join(" ").to_ascii_lowercase(),
    );
    local_tokens.iter().all(|tok| {
        if let Some(neg) = tok.strip_prefix('!') {
            neg.is_empty() || !haystack.contains(&neg.to_ascii_lowercase())
        } else {
            haystack.contains(&tok.to_ascii_lowercase())
        }
    })
}

fn format_search_evidence(
    report: &WorkspaceSearchReport,
    local_tokens: &[String],
    output_limit: usize,
) -> String {
    let filtered: Vec<&WorkspaceSearchHit> = report
        .hits
        .iter()
        .filter(|hit| hit_matches_local_tokens(hit, local_tokens))
        .collect();
    let mut out = String::new();
    let _ = writeln!(
        out,
        "generation: {}\nelapsed_us: {}\nindexed_files: {}\nmatched_files: {}",
        report.generation_digest.as_str(),
        report.elapsed_micros,
        report.indexed_files,
        filtered.len()
    );
    for hit in filtered {
        let loc = match hit.first_line {
            Some(line) => format!("{}:{line}", hit.path),
            None => hit.path.clone(),
        };
        let axes = hit.matched_axes.join("+");
        let syms = hit
            .symbols
            .iter()
            .take(2)
            .map(|s| format!("{}:{}", s.kind.as_str(), s.name))
            .collect::<Vec<_>>()
            .join(",");
        let rels = hit.graph_relations.join(",");
        let mut line = format!("- {loc} [{axes}]");
        if !syms.is_empty() {
            let _ = write!(line, " sym=({syms})");
        }
        if !rels.is_empty() {
            let _ = write!(line, " graph=({rels})");
        }
        if !hit.first_line_text.is_empty() {
            let _ = write!(line, " | {}", hit.first_line_text.trim());
        }
        line.push('\n');
        if out.len() + line.len() > output_limit {
            break;
        }
        out.push_str(&line);
    }
    out
}
