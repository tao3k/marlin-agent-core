// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Single authoritative `marlin.search.v1.SearchGateway` tonic gRPC service.
//!
//! Owns all three search tiers (L0 RAM brute/outline/read, L1 `:gerbil-ascent`,
//! L2 Dusa finite-choice), Scheme `evidence-chain.ss` M1 harvesting, Turn-1
//! Recency-Peak self-teaching, and Turn-2+ `compact_mode` unloading in one
//! server-side state machine. Clients are purely thin tonic RPC callers.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use marlin_agent_runtime::RuntimeContext;
use marlin_agent_search_engine::{
    SearchDslExecutionMode, SearchDslSessionId, SearchDslSessionRunner, SearchDslStructuredResult,
};
use tonic::{Request, Response, Status};

use crate::brute_search::{execute_l0_search, parse_read_file_form};
use crate::search_scheduler::SearchQueryTier;
use crate::search_tool::SharedWorkspaceSearchIndex;

use super::frontier::SessionFrontierState;
use super::proto::search_gateway_server::SearchGateway;
use super::proto::{
    GetToolSchemaRequest, GetToolSchemaResponse, ResetSessionRequest, ResetSessionResponse,
    SearchStepRequest, SearchStepResponse, ToolSchemaEntry,
};

const DEFAULT_OUTPUT_LIMIT: usize = 65_536;
const MAX_ACTIVE_SESSIONS: usize = 32;

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

    fn dedup_or_advance_read_file_query(
        &self,
        session_id: &str,
        raw_query: &str,
    ) -> (String, Option<String>) {
        if !raw_query.contains("read-file") {
            return (raw_query.to_owned(), None);
        }
        let Ok(rf) = parse_read_file_form(raw_query) else {
            return (raw_query.to_owned(), None);
        };
        if rf.signature_only || !rf.symbols.is_empty() {
            return (raw_query.to_owned(), None);
        }
        let mut sessions = self
            .sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let state = sessions
            .entry(session_id.to_owned())
            .or_insert_with(|| SessionFrontierState::new(session_id));
        let ranges = state.read_ranges.entry(rf.path.clone()).or_default();
        let covered = ranges
            .iter()
            .any(|&(ps, pe)| ps <= rf.start_line && rf.end_line <= pe);
        if !covered {
            ranges.push((rf.start_line, rf.end_line));
            return (raw_query.to_owned(), None);
        }
        let max_end = ranges
            .iter()
            .map(|&(_, pe)| pe)
            .max()
            .unwrap_or(rf.end_line);
        let new_start = max_end.saturating_add(1);
        let has_unread = self
            .index
            .read()
            .ok()
            .is_some_and(|guard| !guard.live_file_context(&rf.path, new_start, 0).is_empty());
        if !has_unread {
            return (
                raw_query.to_owned(),
                Some(format!(
                    "read_overlap_note: L{}..=L{} was already read in this session and file has no unread tail lines\n",
                    rf.start_line, rf.end_line
                )),
            );
        }
        let span = rf.end_line.saturating_sub(rf.start_line).saturating_add(1);
        let new_end = new_start.saturating_add(span.saturating_sub(1));
        ranges.push((new_start, new_end));
        let rewritten = format!(
            "(read-file \"{}\" cursor: {new_start} max-lines: {span})",
            rf.path
        );
        let note = format!(
            "auto_advanced_cursor: L{}..=L{} was already read in this session; auto-advanced to L{new_start}..=L{new_end}\n",
            rf.start_line, rf.end_line
        );
        (rewritten, Some(note))
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
                let (effective_query, overlap_note) =
                    self.dedup_or_advance_read_file_query(&session_id, &query);
                let index = Arc::clone(&self.index);
                let (mut body, gen_digest) = self
                    .runtime_context
                    .spawn_blocking(move || {
                        let guard = index
                            .read()
                            .map_err(|_| "resident search index lock poisoned".to_owned())?;
                        let gen_str = guard.generation_digest().as_str().to_owned();
                        let rendered = execute_l0_search(&guard, &effective_query, output_limit)?;
                        Ok::<(String, String), String>((rendered, gen_str))
                    })
                    .join()
                    .await
                    .map_err(|e| Status::internal(format!("L0 search worker failed: {e}")))?
                    .map_err(Status::invalid_argument)?;
                if let Some(note) = overlap_note {
                    body.insert_str(0, &note);
                }
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
