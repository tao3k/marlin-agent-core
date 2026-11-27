// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Bounded LLM-friendly text formatting, coverage telemetry, and actionable
//! follow-up hint rendering for decoded Search DSL results.

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

use agent_semantic_search::{WorkspaceEdbSnapshot, WorkspaceSearchIndex};

use super::types::{
    DEFAULT_MAX_EDB_FACTS, OwnedWorkspaceEdbPrefilter, SearchDslDecodeContext,
    SearchDslExecutionMode, SearchDslStructuredRow,
};

pub(super) struct AscentRenderInputs<'a> {
    pub snapshot: &'a WorkspaceEdbSnapshot,
    pub ctx: SearchDslDecodeContext<'a>,
    pub mode: SearchDslExecutionMode,
    pub relation: &'a str,
    pub rows: &'a [SearchDslStructuredRow],
    pub total_results: usize,
    pub repairs: &'a [String],
    pub canonical_query: Option<&'a str>,
    pub explicit_context: Option<usize>,
}

pub(super) fn format_snapshot_header(
    snapshot: &WorkspaceEdbSnapshot,
    prefilter: Option<&OwnedWorkspaceEdbPrefilter>,
) -> String {
    let native_ast_file_ids: HashSet<u32> = snapshot
        .entities
        .iter()
        .filter(|ent| ent.source != "resident-index")
        .map(|ent| ent.file_id)
        .collect();
    let matched_files = snapshot.files.len();
    let native_ast_files = native_ast_file_ids.len();
    let hydrated_pct = if matched_files == 0 {
        0
    } else {
        native_ast_files
            .saturating_mul(100)
            .checked_div(matched_files)
            .unwrap_or(0)
    };
    let facts_projected = snapshot.entities.len().saturating_add(snapshot.edges.len());
    let facts_truncated = facts_projected >= DEFAULT_MAX_EDB_FACTS;

    let scope_desc = match prefilter {
        Some(pf)
            if pf.path_prefix.is_some()
                || !pf.languages.is_empty()
                || !pf.required_tokens.is_empty() =>
        {
            let mut parts = Vec::new();
            if let Some(prefix) = pf.path_prefix.as_deref() {
                parts.push(format!("path-prefix=\"{prefix}\""));
            }
            if !pf.languages.is_empty() {
                parts.push(format!("langs=[{}]", pf.languages.join(",")));
            }
            if !pf.required_tokens.is_empty() {
                parts.push(format!("tokens={}", pf.required_tokens.len()));
            }
            format!(
                "scoped ({}; scoped_files: {matched_files}, native_ast_files: {native_ast_files}/{matched_files} [{hydrated_pct}%], facts: {facts_projected}, facts_truncated: {facts_truncated})",
                parts.join(", ")
            )
        }
        _ => format!(
            "workspace-wide (scoped_files: {matched_files}/{}, native_ast_files: {native_ast_files}/{matched_files} [{hydrated_pct}%], facts: {facts_projected}, facts_truncated: {facts_truncated})",
            snapshot.indexed_files
        ),
    };

    let mut out = String::new();
    let _ = writeln!(
        out,
        "generation: {}\nelapsed_us: {}\nindexed_files: {}\nmatched_files: {}\nscope_coverage: {scope_desc}",
        snapshot.generation_digest.as_str(),
        snapshot.elapsed_micros,
        snapshot.indexed_files,
        matched_files,
    );
    out
}

fn summarize_top_keys(rows: &[SearchDslStructuredRow], max_keys: usize) -> Option<String> {
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for row in rows {
        if let Some(first) = row.cells.first().filter(|k| !k.is_empty()) {
            *counts.entry(first.as_str()).or_insert(0) += 1;
        }
    }
    if counts.is_empty() {
        return None;
    }
    let mut ranked: Vec<(&str, usize)> = counts.into_iter().collect();
    ranked.sort_unstable_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
    let summary = ranked
        .into_iter()
        .take(max_keys)
        .map(|(k, c)| format!("\"{k}\"({c})"))
        .collect::<Vec<_>>()
        .join(", ");
    Some(summary)
}

fn summarize_parsed_entity_kinds(snapshot: &WorkspaceEdbSnapshot, max_kinds: usize) -> String {
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for ent in &snapshot.entities {
        if ent.kind != "import" {
            *counts.entry(ent.kind.as_str()).or_insert(0) += 1;
        }
    }
    let mut ranked: Vec<(&str, usize)> = counts.into_iter().collect();
    ranked.sort_unstable_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
    if ranked.is_empty() {
        return "none".to_owned();
    }
    ranked
        .into_iter()
        .take(max_kinds)
        .map(|(k, c)| format!("{k}={c}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_ascent_row_with_context(
    row: &SearchDslStructuredRow,
    index: Option<&WorkspaceSearchIndex>,
    context_radius: Option<usize>,
) -> String {
    let mut block = format!("- {}\n", row.display);
    let (Some(idx), Some(radius), Some(path), Some(line)) = (
        index,
        context_radius,
        row.path.as_deref(),
        row.line,
    ) else {
        return block;
    };
    let ctx_lines = idx.live_file_context(path, line, radius);
    for ctx in ctx_lines {
        let marker = if ctx.line_number == line { '>' } else { ' ' };
        let _ = writeln!(
            block,
            "   {marker}L{}: {}",
            ctx.line_number,
            ctx.text.trim_end()
        );
    }
    block
}

fn append_ram_witness_fallback(
    rendered_text: &mut String,
    snapshot: &WorkspaceEdbSnapshot,
    index: Option<&WorkspaceSearchIndex>,
    prefilter: Option<&OwnedWorkspaceEdbPrefilter>,
) {
    let scope_prefix = prefilter.and_then(|pf| pf.path_prefix.as_deref());
    let first_token = prefilter
        .and_then(|pf| pf.required_tokens.first())
        .map(String::as_str);

    if let (Some(idx), Some(tok)) = (index, first_token) {
        let report = idx.query(&agent_semantic_search::WorkspaceSearchQuery {
            pattern: tok,
            languages: prefilter.map_or(&[], |pf| pf.languages.as_slice()),
            documents: &[],
            path_prefix: scope_prefix,
            extension: None,
            symbol: None,
            graph_target: None,
            max_hits: 6,
            context_radius: 0,
        });
        if !report.hits.is_empty() {
            let witnesses: Vec<String> = report
                .hits
                .iter()
                .take(6)
                .map(|h| {
                    let line = h.line_matches.first().map_or(1, |m| m.line_number);
                    format!("{}:{line}", h.path)
                })
                .collect();
            let scope_arg = scope_prefix.map_or_else(
                String::new,
                |p| format!(" path-prefix: \"{p}\""),
            );
            let _ = writeln!(
                rendered_text,
                "l0_ram_witness: {} file(s) matched \"{tok}\" in RAM ({})\nnext_query: (brute-search \"{tok}\"{scope_arg} limit: 24)",
                report.hits.len(),
                witnesses.join(", ")
            );
            return;
        }
    }

    let scope_arg = scope_prefix.unwrap_or("crates/");
    let lang_arg = prefilter
        .and_then(|pf| pf.languages.first())
        .map_or_else(String::new, |l| format!(" lang: \"{l}\""));
    let sample_symbols: Vec<String> = snapshot
        .entities
        .iter()
        .filter(|e| e.kind != "import")
        .take(6)
        .map(|e| format!("{} [{}] ({}:{})", e.name, e.kind, e.path, e.line))
        .collect();
    if !sample_symbols.is_empty() {
        let _ = writeln!(
            rendered_text,
            "l0_ram_witness: sample resident symbols in scope: {}",
            sample_symbols.join(", ")
        );
    }
    let _ = writeln!(
        rendered_text,
        "next_query: (file-outline \"{scope_arg}\" projection: \"kind-summary\"{lang_arg} exclude-tests?: #t limit: 60)"
    );
}

fn append_zero_hit_diagnostic(
    rendered_text: &mut String,
    snapshot: &WorkspaceEdbSnapshot,
    index: Option<&WorkspaceSearchIndex>,
    prefilter: Option<&OwnedWorkspaceEdbPrefilter>,
) {
    if snapshot.files.is_empty() {
        if let Some(prefix) = prefilter.and_then(|pf| pf.path_prefix.as_deref()) {
            let _ = writeln!(
                rendered_text,
                "diagnostic: scope path-prefix=\"{prefix}\" matched 0 indexed files; orient first with (workspace-map \"\" depth: 2)"
            );
        } else {
            let _ = writeln!(
                rendered_text,
                "diagnostic: prefilter matched 0 indexed files; check language/token filters or verify with (brute-search \"...\" path-prefix: \"...\")"
            );
        }
        return;
    }

    let native_ast_files = snapshot
        .entities
        .iter()
        .filter(|ent| ent.source != "resident-index")
        .map(|ent| ent.file_id)
        .collect::<HashSet<u32>>()
        .len();
    let parsed_kinds = summarize_parsed_entity_kinds(snapshot, 6);
    if native_ast_files == 0 {
        let _ = writeln!(
            rendered_text,
            "diagnostic: matched {} scoped file(s) with 0 native AST facts (only coarse resident-index `type_decl`/`callable` symbols available; parsed_kinds: {parsed_kinds})",
            snapshot.files.len(),
        );
        append_ram_witness_fallback(rendered_text, snapshot, index, prefilter);
        return;
    }
    let coverage_note = if native_ast_files < snapshot.files.len() {
        format!(
            "partial native_ast coverage ({native_ast_files}/{} files); unhydrated files can still be inspected via (file-outline ... projection: \"kind-summary\") or (brute-search ...)",
            snapshot.files.len()
        )
    } else {
        "complete native_ast coverage across all scoped files (true 0 matches)".to_owned()
    };
    let _ = writeln!(
        rendered_text,
        "diagnostic: 0 rows matched across {} scoped file(s) ({} facts; parsed_kinds: {parsed_kinds}; {coverage_note})",
        snapshot.files.len(),
        snapshot.entities.len().saturating_add(snapshot.edges.len()),
    );
    if native_ast_files < snapshot.files.len() {
        append_ram_witness_fallback(rendered_text, snapshot, index, prefilter);
    }
}

fn format_truncation_next_query_hint(
    rows: &[SearchDslStructuredRow],
    prefilter: Option<&OwnedWorkspaceEdbPrefilter>,
) -> String {
    let looks_like_test = |p: &str| {
        let lc = p.to_ascii_lowercase();
        lc.contains("/tests/") || lc.starts_with("tests/") || lc.contains("_test.") || lc.contains("/t/")
    };
    let test_rows = rows
        .iter()
        .filter(|r| r.cells.iter().any(|c| looks_like_test(c)))
        .count();
    let mut hints = Vec::new();
    if test_rows > 0 {
        hints.push("exclude-tests?: #t");
    }
    if prefilter.and_then(|pf| pf.path_prefix.as_deref()).is_none() {
        hints.push("path-prefix: \"crates/<crate>/\"");
    }
    hints.push("name-all: (\"stem1\" \"stem2\")");
    hints.push("or raise limit:");
    hints.join(", ")
}

pub(super) fn render_ascent_search_text(inputs: AscentRenderInputs<'_>) -> String {
    let AscentRenderInputs {
        snapshot,
        ctx,
        mode,
        relation,
        rows,
        total_results,
        repairs,
        canonical_query,
        explicit_context,
    } = inputs;

    let header_reserve = 384usize;
    let footer_reserve = 128usize;
    let body_budget = ctx.output_limit.saturating_sub(header_reserve + footer_reserve);

    // Attach zero-I/O RAM source snippets automatically for concise L1 Ascent results
    // (or whenever the query explicitly requests `(context N)`), eliminating follow-up grep rounds.
    let effective_context = match explicit_context {
        Some(n) => Some(n.min(8)),
        None if ctx.index.is_some() && !rows.is_empty() && rows.len() <= 8 => Some(1),
        None if ctx.index.is_some() && !rows.is_empty() && rows.len() <= 20 => Some(0),
        None => None,
    };

    let mut body_bytes = 0usize;
    let body_lines: Vec<String> = rows
        .iter()
        .map(|row| format_ascent_row_with_context(row, ctx.index, effective_context))
        .take_while(|line| {
            if body_bytes + line.len() <= body_budget {
                body_bytes += line.len();
                true
            } else {
                false
            }
        })
        .collect();

    let returned_results = body_lines.len();
    let truncated = returned_results < total_results;
    let mut rendered_text = format_snapshot_header(snapshot, ctx.prefilter);
    let _ = writeln!(
        rendered_text,
        "mode: {}\nrelation: {relation}\nresults: {total_results}\nreturned: {returned_results}\ntruncated: {truncated}",
        mode.as_str(),
    );
    if ctx
        .prefilter
        .is_some_and(|pf| pf.hydration.includes_symbol_refs())
    {
        let _ = writeln!(
            rendered_text,
            "symbol_refs_hydrated: {} file(s) (100% RAM lexical ref coverage)",
            snapshot.files.len()
        );
    }
    if truncated && returned_results > 1 && rows.first().is_some_and(|r| r.cells.len() >= 2) {
        let distinct_keys: HashSet<&str> = rows
            .iter()
            .take(returned_results)
            .filter_map(|r| r.cells.first().map(String::as_str))
            .collect();
        let _ = writeln!(
            rendered_text,
            "sampling: diverse-by-key ({} distinct keys across {returned_results} returned rows)",
            distinct_keys.len()
        );
        if let Some(top_keys) = summarize_top_keys(&rows[..returned_results], 5) {
            let _ = writeln!(rendered_text, "top_keys: {top_keys}");
        }
    }
    if !repairs.is_empty() {
        let _ = writeln!(rendered_text, "repairs: {}", repairs.join(", "));
        if let Some(cq) = canonical_query {
            let _ = writeln!(rendered_text, "canonical_query: {cq}");
        }
    }
    if total_results == 0 {
        append_zero_hit_diagnostic(&mut rendered_text, snapshot, ctx.index, ctx.prefilter);
    }
    rendered_text.extend(body_lines);
    if truncated {
        let omitted = total_results.saturating_sub(returned_results);
        let hint = format_truncation_next_query_hint(&rows[..returned_results], ctx.prefilter);
        let _ = writeln!(
            rendered_text,
            "... (+{omitted} omitted; next_query: narrow via {hint})"
        );
    }
    rendered_text
}
