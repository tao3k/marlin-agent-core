// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Single authoritative `marlin.search.v1.SearchGateway` tonic gRPC service.
//!
//! Owns all three search tiers (L0 RAM brute/outline/read, L1 `:gerbil-ascent`,
//! L2 Dusa finite-choice), Scheme `evidence-chain.ss` M1 harvesting, Turn-1
//! Recency-Peak self-teaching, and Turn-2+ `compact_mode` unloading in one
//! server-side state machine. Clients are purely thin tonic RPC callers.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Write as _;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use marlin_agent_runtime::RuntimeContext;
use marlin_agent_search_engine::{
    SearchDslExecutionMode, SearchDslSessionId, SearchDslSessionRunner, SearchDslStructuredResult,
    SearchEpistemicStatus, SearchEvidenceChainReceipt, WorkspaceEdbPrefilter, WorkspaceSearchIndex,
};
use tonic::{Request, Response, Status};

use crate::brute_search::execute_l0_search;
use crate::search_scheduler::SearchQueryTier;
use crate::search_tool::SharedWorkspaceSearchIndex;

use super::proto::search_gateway_server::SearchGateway;
use super::proto::{
    EvidenceFrontier, GetToolSchemaRequest, GetToolSchemaResponse, ResetSessionRequest,
    ResetSessionResponse, SearchStepRequest, SearchStepResponse, ToolSchemaEntry,
};

const DEFAULT_OUTPUT_LIMIT: usize = 65_536;
const MAX_ACTIVE_SESSIONS: usize = 32;
static NEXT_INTERNAL_SCHEME_SESSION: AtomicU64 = AtomicU64::new(1);

fn allocate_scheme_session_id(logical_id: &str) -> SearchDslSessionId {
    let seq = NEXT_INTERNAL_SCHEME_SESSION.fetch_add(1, Ordering::Relaxed);
    let prefix: String = logical_id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(48)
        .collect();
    let label = if prefix.is_empty() {
        format!("grpc-search-{seq}")
    } else {
        format!("{prefix}-{seq}")
    };
    SearchDslSessionId::new(label).expect("sanitized gRPC session id is valid")
}

struct SessionFrontierState {
    scheme_session_id: SearchDslSessionId,
    step_count: u32,
    teaching_emitted: bool,
    evidence_epoch: u64,
    total_m1_claims: u32,
    conflict_count: u32,
    ambiguous_branches: BTreeSet<String>,
    covered_layers: BTreeSet<String>,
    referenced_layers: BTreeSet<String>,
    layer_sample_paths: BTreeMap<String, String>,
    layer_sample_symbols: BTreeMap<String, Vec<String>>,
    discovered_symbols: Vec<String>,
    seen_symbols: BTreeSet<String>,
    impact_ran: bool,
    dusa_worlds: Option<usize>,
}

impl SessionFrontierState {
    fn new(logical_id: &str) -> Self {
        Self {
            scheme_session_id: allocate_scheme_session_id(logical_id),
            step_count: 0,
            teaching_emitted: false,
            evidence_epoch: 1,
            total_m1_claims: 0,
            conflict_count: 0,
            ambiguous_branches: BTreeSet::new(),
            covered_layers: BTreeSet::new(),
            referenced_layers: BTreeSet::new(),
            layer_sample_paths: BTreeMap::new(),
            layer_sample_symbols: BTreeMap::new(),
            discovered_symbols: Vec::new(),
            seen_symbols: BTreeSet::new(),
            impact_ran: false,
            dusa_worlds: None,
        }
    }

    fn advance(
        &mut self,
        tier: SearchQueryTier,
        raw_query: &str,
        rendered_body: &str,
        structured: Option<&SearchDslStructuredResult>,
        index: &WorkspaceSearchIndex,
    ) -> (EvidenceFrontier, String) {
        self.step_count = self.step_count.saturating_add(1);
        let newly_harvested = self.ingest_evidence(tier, raw_query, rendered_body, structured);
        let had_zero_hits = detect_zero_hits(rendered_body, structured);
        let teaching_kit_active = !self.teaching_emitted || had_zero_hits;
        if teaching_kit_active && !had_zero_hits {
            self.teaching_emitted = true;
        }
        let open_frontiers = self.compute_open_frontiers(structured, index);
        let epistemic_status = self.compute_epistemic_status(structured, &open_frontiers);
        let workspace_layers = if teaching_kit_active {
            summarize_workspace_layers(index)
        } else {
            String::new()
        };
        let layer_distribution =
            summarize_broad_layer_distribution(raw_query, rendered_body, structured);
        let next_dag_templates =
            self.build_next_dag_templates(teaching_kit_active, &open_frontiers, raw_query);

        let frontier = EvidenceFrontier {
            step: self.step_count,
            teaching_kit_active,
            compact_mode: !teaching_kit_active,
            epoch: self.evidence_epoch,
            total_m1_claims: self.total_m1_claims,
            newly_harvested: u32::try_from(newly_harvested).unwrap_or(u32::MAX),
            conflict_count: self.conflict_count,
            epistemic_status,
            workspace_layers,
            covered_layers: self.covered_layers.iter().take(10).cloned().collect(),
            open_frontiers,
            ambiguous_branches: self.ambiguous_branches.iter().take(6).cloned().collect(),
            next_dag_templates,
        };
        let footer = format_frontier_footer(&frontier, &layer_distribution, self.impact_ran);
        (frontier, footer)
    }

    fn ingest_evidence(
        &mut self,
        tier: SearchQueryTier,
        raw_query: &str,
        rendered_body: &str,
        structured: Option<&SearchDslStructuredResult>,
    ) -> usize {
        if raw_query.contains("find-impact") {
            self.impact_ran = true;
        }
        if let Some(res) = structured {
            return self.ingest_structured_result(raw_query, res);
        }
        self.ingest_l0_evidence(tier, raw_query, rendered_body);
        0
    }

    fn ingest_structured_result(
        &mut self,
        raw_query: &str,
        res: &SearchDslStructuredResult,
    ) -> usize {
        let newly_harvested = res
            .evidence_receipt
            .as_ref()
            .map_or(0, |receipt| self.ingest_receipt(receipt));
        if res.mode == SearchDslExecutionMode::Choice && !res.choice_worlds.is_empty() {
            self.dusa_worlds = Some(res.choice_worlds.len());
        }
        let is_broad_truncated_symbols = raw_query.contains("find-symbols")
            && res
                .rendered_text
                .lines()
                .take(16)
                .any(|l| l.trim() == "truncated: true");
        for row in &res.rows {
            let path = row.path.as_deref();
            let sym = row.cells.first().map(String::as_str);
            if let Some(p) = path
                && !is_broad_truncated_symbols
            {
                self.record_referenced_path(p, sym);
            }
            if let Some(s) = sym {
                self.record_discovered_symbol(s, path);
            }
        }
        if res.rendered_text.contains("[DEFINITIONS]") {
            self.impact_ran = true;
            // `find-impact` already returns inline [DEFINITIONS] + [PROD]/[TEST] file:line
            // refs for every affected layer, so mark those layers structurally covered
            // (leaving only unvisited cross-language boundaries such as `scheme/c-abi` open).
            ingest_affected_layers(&res.rendered_text, &mut self.covered_layers);
        }
        newly_harvested
    }

    fn ingest_receipt(&mut self, receipt: &SearchEvidenceChainReceipt) -> usize {
        self.evidence_epoch = u64::try_from(receipt.epoch.max(1)).unwrap_or(1);
        self.total_m1_claims = u32::try_from(receipt.total_m1_claims).unwrap_or(u32::MAX);
        self.conflict_count = u32::try_from(receipt.conflict_count).unwrap_or(u32::MAX);
        self.ambiguous_branches
            .extend(receipt.ambiguous_branches.iter().cloned());
        self.covered_layers
            .extend(receipt.covered_layers.iter().cloned());
        receipt.newly_harvested
    }

    fn ingest_l0_evidence(&mut self, tier: SearchQueryTier, raw_query: &str, rendered_body: &str) {
        if tier != SearchQueryTier::BruteForce {
            return;
        }
        if let Some(target_path) = extract_first_quoted_arg(raw_query)
            && !target_path.is_empty()
            && (raw_query.contains("read-file") || raw_query.contains("file-outline"))
            && let Some(layer) = extract_path_layer(target_path)
        {
            self.covered_layers.insert(layer.clone());
            self.layer_sample_paths
                .entry(layer)
                .or_insert_with(|| target_path.to_owned());
        }
        rendered_body
            .lines()
            .take(80)
            .filter_map(|line| line.strip_prefix("path: "))
            .filter_map(|p| extract_path_layer(p.trim()))
            .for_each(|layer| {
                self.covered_layers.insert(layer);
            });
        if raw_query.contains("file-outline") && !self.impact_ran {
            let outline_syms = extract_outline_anchor_symbols(rendered_body);
            if !outline_syms.is_empty() {
                // Prioritize exact symbols just discovered from an explicit file/dir outline
                // ahead of earlier broad lexical guesses so `find-impact` targets real APIs.
                let mut seen = BTreeSet::new();
                self.discovered_symbols = outline_syms
                    .into_iter()
                    .chain(self.discovered_symbols.iter().cloned())
                    .filter(|sym| seen.insert(sym.clone()))
                    .take(6)
                    .collect();
                self.seen_symbols = seen;
            }
        }
    }

    fn record_referenced_path(&mut self, path: &str, sym: Option<&str>) {
        let Some(layer) = extract_path_layer(path) else {
            return;
        };
        self.referenced_layers.insert(layer.clone());
        self.layer_sample_paths
            .entry(layer.clone())
            .or_insert_with(|| path.to_owned());
        if let Some(symbol) = sym.filter(|s| is_high_signal_anchor_symbol(s, Some(path))) {
            let list = self.layer_sample_symbols.entry(layer).or_default();
            if list.len() < 3 && !list.iter().any(|existing| existing == symbol) {
                list.push(symbol.to_owned());
            }
        }
    }

    fn record_discovered_symbol(&mut self, sym: &str, path: Option<&str>) {
        if self.discovered_symbols.len() >= 6 || !is_high_signal_anchor_symbol(sym, path) {
            return;
        }
        if self.seen_symbols.insert(sym.to_owned()) {
            self.discovered_symbols.push(sym.to_owned());
        }
    }

    fn compute_open_frontiers(
        &self,
        structured: Option<&SearchDslStructuredResult>,
        index: &WorkspaceSearchIndex,
    ) -> Vec<String> {
        // Priority-ordered list: Scheme `evidence-chain.ss` cross-language boundary
        // frontiers ALWAYS come first so alphabetical `crates/*` never evict `scheme/*`.
        let mut ordered = Vec::new();
        let mut seen = BTreeSet::new();
        if let Some(receipt) = structured.and_then(|r| r.evidence_receipt.as_ref()) {
            receipt
                .open_frontiers
                .iter()
                .filter(|f| {
                    let key = extract_path_layer(f).unwrap_or_else(|| (*f).clone());
                    !self.covered_layers.contains(&key)
                })
                .map(|f| resolve_frontier_entry_path(f, index))
                .filter(|resolved| seen.insert(resolved.clone()))
                .for_each(|resolved| ordered.push(resolved));
        }
        let remaining = 4usize.saturating_sub(ordered.len());
        ordered.extend(
            self.referenced_layers
                .iter()
                .filter(|layer| {
                    !self.covered_layers.contains(*layer)
                        && !layer.ends_with("/tests")
                        && seen.insert((*layer).clone())
                })
                .take(remaining)
                .cloned(),
        );
        ordered.truncate(4);
        ordered
    }

    fn compute_epistemic_status(
        &self,
        structured: Option<&SearchDslStructuredResult>,
        open_frontiers: &[String],
    ) -> String {
        if self.conflict_count > 0 {
            return "epistemic-conflict".to_owned();
        }
        if let Some(receipt) = structured.and_then(|r| r.evidence_receipt.as_ref())
            && matches!(
                receipt.epistemic_status,
                SearchEpistemicStatus::UniqueDeduction | SearchEpistemicStatus::CompetingWorlds
            )
        {
            return receipt.epistemic_status.as_str().to_owned();
        }
        if open_frontiers.is_empty()
            && (self.total_m1_claims > 0
                || self.dusa_worlds.is_some()
                || !self.covered_layers.is_empty())
        {
            return "converged-evidence".to_owned();
        }
        "exploring".to_owned()
    }

    fn build_next_dag_templates(
        &self,
        teaching_active: bool,
        open_frontiers: &[String],
        raw_query: &str,
    ) -> Vec<String> {
        let mut out = Vec::new();
        // Always propose one-shot `find-impact` after symbol discovery even if a
        // parallel fast `workspace-map` already flipped `compact_mode: true`!
        if !self.impact_ran && !self.discovered_symbols.is_empty() {
            let quoted: Vec<String> = self
                .discovered_symbols
                .iter()
                .take(3)
                .map(|s| format!("\"{s}\""))
                .collect();
            let arg = if quoted.len() == 1 {
                quoted[0].clone()
            } else {
                format!("({})", quoted.join(" "))
            };
            out.push(format!("workspace_search: (find-impact {arg} limit: 120)"));
        }
        for frontier in open_frontiers.iter().take(2) {
            out.push(format_open_frontier_template(
                frontier,
                &self.layer_sample_paths,
                &self.layer_sample_symbols,
            ));
        }
        if out.is_empty() && teaching_active && raw_query.contains("workspace-map") {
            out.push(
                "workspace_search: (find-symbols name-contains: (\"<stemA>\" \"<stemB>\") lang: \"any\" limit: 60)"
                    .to_owned(),
            );
        }
        out
    }
}

/// Authoritative tonic gRPC implementation of `marlin.search.v1.SearchGateway`.
#[derive(Clone)]
pub struct SearchGatewayService {
    index: SharedWorkspaceSearchIndex,
    tool_schemas: Arc<Vec<ToolSchemaEntry>>,
    dsl_session_runner: SearchDslSessionRunner,
    runtime_context: RuntimeContext,
    sessions: Arc<Mutex<HashMap<String, SessionFrontierState>>>,
}

impl SearchGatewayService {
    /// Construct a new single-owner search gRPC service over `index`,
    /// `tool_schemas`, `dsl_session_runner`, and `runtime_context`.
    #[must_use]
    pub fn new(
        index: SharedWorkspaceSearchIndex,
        tool_schemas: Vec<ToolSchemaEntry>,
        dsl_session_runner: SearchDslSessionRunner,
        runtime_context: RuntimeContext,
    ) -> Self {
        Self {
            index,
            tool_schemas: Arc::new(tool_schemas),
            dsl_session_runner,
            runtime_context,
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn resolve_scheme_session_id(&self, logical_id: &str) -> SearchDslSessionId {
        let mut sessions = self
            .sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if !sessions.contains_key(logical_id) && sessions.len() >= MAX_ACTIVE_SESSIONS {
            sessions.clear();
        }
        sessions
            .entry(logical_id.to_owned())
            .or_insert_with(|| SessionFrontierState::new(logical_id))
            .scheme_session_id
            .clone()
    }
}

#[tonic::async_trait]
impl SearchGateway for SearchGatewayService {
    async fn get_tool_schema(
        &self,
        _request: Request<GetToolSchemaRequest>,
    ) -> Result<Response<GetToolSchemaResponse>, Status> {
        Ok(Response::new(GetToolSchemaResponse {
            tools: (*self.tool_schemas).clone(),
        }))
    }

    async fn reset_session(
        &self,
        request: Request<ResetSessionRequest>,
    ) -> Result<Response<ResetSessionResponse>, Status> {
        let session_id = normalize_session_id(&request.into_inner().session_id);
        let cleared = {
            let mut sessions = self
                .sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            sessions.remove(&session_id).is_some()
        };
        Ok(Response::new(ResetSessionResponse {
            session_id,
            cleared,
        }))
    }

    async fn search_step(
        &self,
        request: Request<SearchStepRequest>,
    ) -> Result<Response<SearchStepResponse>, Status> {
        let req = request.into_inner();
        let query = req.query.trim().to_owned();
        if query.is_empty() {
            return Err(Status::invalid_argument("search query must not be empty"));
        }
        let session_id = normalize_session_id(&req.session_id);
        let output_limit = usize::try_from(req.output_limit)
            .ok()
            .filter(|&v| v > 0)
            .unwrap_or(DEFAULT_OUTPUT_LIMIT);
        let tier = SearchQueryTier::classify(&query);
        let started = Instant::now();

        let (raw_body, structured, tier_label, generation_digest) = match tier {
            SearchQueryTier::BruteForce => {
                let index = Arc::clone(&self.index);
                let q = query.clone();
                let (body, gen_digest) = self
                    .runtime_context
                    .spawn_blocking(move || {
                        let guard = index
                            .read()
                            .map_err(|_| "resident search index lock poisoned".to_owned())?;
                        let gen_str = guard.generation_digest().as_str().to_owned();
                        let rendered = execute_l0_search(&guard, &q, output_limit)?;
                        Ok::<(String, String), String>((rendered, gen_str))
                    })
                    .join()
                    .await
                    .map_err(|e| Status::internal(format!("L0 search worker failed: {e}")))?
                    .map_err(Status::invalid_argument)?;
                (body, None, "L0-Brute".to_owned(), gen_digest)
            }
            SearchQueryTier::FastAscent | SearchQueryTier::DusaSolver => {
                let scheme_sid = self.resolve_scheme_session_id(&session_id);
                let runner = Arc::clone(&self.dsl_session_runner);
                let index = Arc::clone(&self.index);
                let q = query.clone();
                let structured = self
                    .runtime_context
                    .spawn_blocking(move || {
                        runner(index, &q, output_limit, &scheme_sid).map_err(|e| e.to_string())
                    })
                    .join()
                    .await
                    .map_err(|e| Status::internal(format!("Search DSL worker failed: {e}")))?
                    .map_err(Status::invalid_argument)?;
                let tier_name = match structured.mode {
                    SearchDslExecutionMode::Ascent => "L1-Ascent",
                    SearchDslExecutionMode::Choice => "L2-Dusa",
                    SearchDslExecutionMode::GerbilParser => "Parser",
                };
                let body = structured.rendered_text.clone();
                let gen_digest = structured.generation_digest.clone();
                (body, Some(structured), tier_name.to_owned(), gen_digest)
            }
        };

        let (frontier, footer) = {
            let guard = self
                .index
                .read()
                .map_err(|_| Status::internal("resident search index lock poisoned"))?;
            let mut sessions = self
                .sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let state = sessions
                .entry(session_id.clone())
                .or_insert_with(|| SessionFrontierState::new(&session_id));
            state.advance(tier, &query, &raw_body, structured.as_ref(), &guard)
        };

        let elapsed_micros = u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);
        let (total_results, returned_results, truncated) =
            summarize_counts(&raw_body, structured.as_ref());
        let rendered_text = append_bounded_footer(raw_body, &footer, output_limit);

        Ok(Response::new(SearchStepResponse {
            session_id,
            tier: tier_label,
            generation_digest,
            elapsed_micros,
            total_results,
            returned_results,
            truncated,
            rendered_text,
            frontier: Some(frontier),
        }))
    }
}

fn normalize_session_id(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        "default".to_owned()
    } else {
        trimmed.to_owned()
    }
}

fn ingest_affected_layers(rendered_text: &str, referenced_layers: &mut BTreeSet<String>) {
    rendered_text
        .lines()
        .take(120)
        .filter_map(|line| line.strip_prefix("affected_layers:"))
        .flat_map(|rest| rest.split(','))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .for_each(|layer| {
            referenced_layers.insert(layer.to_owned());
        });
}

fn format_frontier_footer(
    frontier: &EvidenceFrontier,
    layer_distribution: &str,
    impact_ran: bool,
) -> String {
    let mut out = String::new();
    let mode_tag = if frontier.teaching_kit_active {
        "teaching_kit: active"
    } else {
        "compact_mode: true"
    };
    let _ = writeln!(
        out,
        "\n[evidence_frontier | step: {} | {mode_tag} | epoch: {} | m1_claims: {} (+{}) | status: {}]",
        frontier.step,
        frontier.epoch,
        frontier.total_m1_claims,
        frontier.newly_harvested,
        frontier.epistemic_status
    );
    if !frontier.workspace_layers.is_empty() {
        let _ = writeln!(out, "workspace_layers: {}", frontier.workspace_layers);
    }
    if frontier.teaching_kit_active {
        let _ = writeln!(
            out,
            "dsl_tips: for cross-layer change impact, run ONE (find-impact (\"SymA\" \"SymB\" \"SymC\") limit: 120) — returns [DEFINITIONS], affected_layers, [PROD] & [TEST] across all crates; read exact files via (read-file \"<path>\" symbols: (\"SymA\" \"SymB\")); Turn 2+ auto-unloads this kit"
        );
    }
    if !layer_distribution.is_empty() {
        let _ = writeln!(out, "matched_layer_distribution: {layer_distribution}");
    }
    if !frontier.covered_layers.is_empty() {
        let _ = writeln!(
            out,
            "covered_layers: {}",
            frontier.covered_layers.join(", ")
        );
    }
    if !frontier.ambiguous_branches.is_empty() {
        let _ = writeln!(
            out,
            "open_branches: {}",
            frontier.ambiguous_branches.join(", ")
        );
    }
    if frontier.open_frontiers.is_empty() {
        if matches!(
            frontier.epistemic_status.as_str(),
            "converged-evidence" | "unique-deduction"
        ) {
            let stop_hint = if impact_ran {
                "all cross-layer definitions and prod/test call sites verified; stop calling tools and synthesize final answer now"
            } else {
                "referenced cross-layer boundaries covered; synthesize once task requirements are met"
            };
            let _ = writeln!(out, "open_frontiers: none ({stop_hint})");
        }
    } else {
        let _ = writeln!(
            out,
            "open_frontiers: {}",
            frontier.open_frontiers.join(", ")
        );
    }
    if !frontier.next_dag_templates.is_empty() {
        let label = if frontier.teaching_kit_active {
            "next_dag_templates"
        } else {
            "next_dag"
        };
        let _ = writeln!(out, "{label}: {}", frontier.next_dag_templates.join(" | "));
    }
    out
}

fn summarize_broad_layer_distribution(
    raw_query: &str,
    rendered_body: &str,
    structured: Option<&SearchDslStructuredResult>,
) -> String {
    let Some(res) = structured else {
        return String::new();
    };
    if !raw_query.contains("find-symbols") || res.rows.len() < 12 {
        return String::new();
    }
    let is_broad = rendered_body
        .lines()
        .take(16)
        .any(|l| l.trim() == "truncated: true");
    if !is_broad {
        return String::new();
    }
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    res.rows
        .iter()
        .filter_map(|r| r.path.as_deref())
        .filter_map(extract_path_layer)
        .for_each(|layer| {
            *counts.entry(layer).or_insert(0) += 1;
        });
    let mut ranked: Vec<(String, usize)> = counts.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    ranked
        .into_iter()
        .take(6)
        .map(|(layer, n)| format!("{layer}({n})"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn resolve_frontier_entry_path(frontier: &str, index: &WorkspaceSearchIndex) -> String {
    if frontier.contains('.') {
        return frontier.to_owned();
    }
    let prefix = if frontier.ends_with('/') {
        frontier.to_owned()
    } else {
        format!("{frontier}/")
    };
    let prefilter = WorkspaceEdbPrefilter {
        languages: &[],
        documents: &[],
        path_prefix: Some(&prefix),
        extension: None,
        required_tokens: &[],
        max_files: 16,
        max_facts: 1,
    };
    let snapshot = index.project_prefiltered_edb(&prefilter);
    if let Some(preferred) = snapshot
        .files
        .iter()
        .find(|f| f.path.contains("search-dsl") || f.path.ends_with("lib.rs"))
    {
        return preferred.path.clone();
    }
    if snapshot.files.len() == 1 {
        return snapshot.files[0].path.clone();
    }
    frontier.to_owned()
}

fn format_open_frontier_template(
    frontier: &str,
    layer_sample_paths: &BTreeMap<String, String>,
    layer_sample_symbols: &BTreeMap<String, Vec<String>>,
) -> String {
    if let Some(path) = layer_sample_paths.get(frontier) {
        if let Some(sym_list) = layer_sample_symbols.get(frontier).filter(|v| !v.is_empty()) {
            let quoted: Vec<String> = sym_list.iter().map(|s| format!("\"{s}\"")).collect();
            return format!(
                "workspace_search: (read-file \"{path}\" symbols: ({}))",
                quoted.join(" ")
            );
        }
        return format!("workspace_search: (read-file \"{path}\" cursor: 1 max-lines: 220)");
    }
    if frontier.contains('.') {
        return format!("workspace_search: (read-file \"{frontier}\" cursor: 1 max-lines: 220)");
    }
    format!(
        "workspace_search: (file-outline \"{frontier}/\" projection: \"kind-summary\" limit: 60)"
    )
}

fn extract_outline_anchor_symbols(rendered_body: &str) -> Vec<String> {
    let mut seen = BTreeSet::new();
    rendered_body
        .lines()
        .take(120)
        .filter_map(parse_outline_symbol_from_line)
        .filter(|sym| is_high_signal_anchor_symbol(sym, None))
        .filter(|sym| seen.insert((*sym).to_owned()))
        .take(4)
        .map(str::to_owned)
        .collect()
}

fn parse_outline_symbol_from_line(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    ["[struct] ", "[enum] ", "[trait] ", "[function] ", "[type] "]
        .iter()
        .find_map(|tag| trimmed.split_once(tag))
        .map(|(_, rest)| {
            rest.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'))
                .next()
                .unwrap_or("")
        })
        .filter(|ident| !ident.is_empty())
}

fn is_high_signal_anchor_symbol(sym: &str, path: Option<&str>) -> bool {
    if sym.len() < 6
        || sym.contains('/')
        || sym.contains(' ')
        || sym.contains(':')
        || is_generic_symbol(sym)
    {
        return false;
    }
    if let Some(p) = path {
        let lc = p.to_ascii_lowercase();
        if lc.contains("/tests/") || lc.starts_with("tests/") || lc.contains("/t/") {
            return false;
        }
    }
    let has_upper = sym.bytes().any(|b| b.is_ascii_uppercase());
    let has_sep = sym.contains('_') || sym.contains('-');
    has_upper || has_sep
}

fn append_bounded_footer(body: String, footer: &str, output_limit: usize) -> String {
    if footer.is_empty() || output_limit < footer.len() + 64 {
        return body;
    }
    if body.len() + footer.len() <= output_limit {
        let mut combined = body;
        combined.push_str(footer);
        return combined;
    }
    let keep = output_limit.saturating_sub(footer.len() + 4);
    let mut cut = keep.min(body.len());
    while cut > 0 && !body.is_char_boundary(cut) {
        cut -= 1;
    }
    let mut trimmed = body[..cut].to_owned();
    if !trimmed.ends_with('\n') {
        trimmed.push('\n');
    }
    trimmed.push_str(footer);
    trimmed
}

fn summarize_counts(
    raw_body: &str,
    structured: Option<&SearchDslStructuredResult>,
) -> (u32, u32, bool) {
    let mut total = 0u32;
    let mut returned = 0u32;
    let mut truncated = false;
    for line in raw_body.lines().take(20) {
        if let Some(v) = line
            .strip_prefix("results:")
            .or_else(|| line.strip_prefix("hits:"))
            && let Ok(n) = v.trim().parse::<u32>()
        {
            total = n;
        } else if let Some(v) = line.strip_prefix("worlds:")
            && let Some(n) = v
                .split_whitespace()
                .next()
                .and_then(|s| s.parse::<u32>().ok())
        {
            total = n;
        } else if let Some(v) = line.strip_prefix("returned:")
            && let Ok(n) = v.trim().parse::<u32>()
        {
            returned = n;
        } else if let Some(v) = line.strip_prefix("truncated:") {
            truncated = v.trim() == "true";
        }
    }
    if let Some(res) = structured {
        if total == 0 {
            total = u32::try_from(res.rows.len().max(res.choice_worlds.len())).unwrap_or(u32::MAX);
        }
        if returned == 0 {
            returned =
                u32::try_from(res.rows.len().max(res.choice_worlds.len())).unwrap_or(u32::MAX);
        }
    }
    (total, returned, truncated)
}

fn detect_zero_hits(rendered_body: &str, structured: Option<&SearchDslStructuredResult>) -> bool {
    if let Some(res) = structured {
        return res.rows.is_empty() && res.choice_worlds.is_empty();
    }
    rendered_body
        .lines()
        .take(12)
        .any(|line| line.trim() == "hits: 0" || line.trim() == "results: 0")
}

fn extract_first_quoted_arg(raw_query: &str) -> Option<&str> {
    let start = raw_query.find('"')? + 1;
    let rest = raw_query.get(start..)?;
    let end = rest.find('"')?;
    Some(&rest[..end])
}

fn is_generic_symbol(sym: &str) -> bool {
    matches!(
        sym,
        "new" | "default" | "from" | "into" | "clone" | "fmt" | "build" | "run" | "init" | "main"
    )
}

fn extract_path_layer(path: &str) -> Option<String> {
    let segs: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if segs.is_empty() {
        return None;
    }
    if segs[0] == "crates" && segs.len() >= 3 && matches!(segs[2], "tests" | "benches") {
        return Some(format!("crates/{}/tests", segs[1]));
    }
    if matches!(segs[0], "crates" | "scheme" | "packages") && segs.len() >= 2 {
        return Some(format!("{}/{}", segs[0], segs[1]));
    }
    (segs.len() >= 2).then(|| segs[0].to_owned())
}

fn summarize_workspace_layers(index: &WorkspaceSearchIndex) -> String {
    let prefilter = WorkspaceEdbPrefilter {
        languages: &[],
        documents: &[],
        path_prefix: None,
        extension: None,
        required_tokens: &[],
        max_files: 4096,
        max_facts: 1,
    };
    let snapshot = index.project_prefiltered_edb(&prefilter);
    if snapshot.files.is_empty() {
        return String::new();
    }
    let mut groups: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    for file in &snapshot.files {
        let mut segs = file.path.split('/').filter(|s| !s.is_empty());
        let (Some(root), Some(second), Some(_)) = (segs.next(), segs.next(), segs.next()) else {
            continue;
        };
        *groups
            .entry(root.to_owned())
            .or_default()
            .entry(second.to_owned())
            .or_insert(0) += 1;
    }
    let mut root_ranks: Vec<(String, usize, BTreeMap<String, usize>)> = groups
        .into_iter()
        .map(|(root, subs)| {
            let total: usize = subs.values().sum();
            (root, total, subs)
        })
        .collect();
    root_ranks.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    root_ranks
        .into_iter()
        .take(4)
        .map(|(root, _, subs)| format_root_layer_group(&root, subs))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_root_layer_group(root: &str, subs: BTreeMap<String, usize>) -> String {
    let mut sub_ranks: Vec<(String, usize)> = subs.into_iter().collect();
    sub_ranks.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let names: Vec<String> = sub_ranks.into_iter().take(10).map(|(n, _)| n).collect();
    format!("{root}/({})", names.join(", "))
}
