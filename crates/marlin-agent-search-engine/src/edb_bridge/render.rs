// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Bounded LLM-friendly text formatting, coverage telemetry, and actionable
//! follow-up hint rendering for decoded Search DSL results.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt::Write as _;

use agent_semantic_search::{WorkspaceEdbPrefilter, WorkspaceEdbSnapshot, WorkspaceSearchIndex};

use super::render_groups::{
    is_collision_row_shape, is_impact_row_shape, render_grouped_collision_lines,
    render_grouped_impact_lines, split_identifier_stems,
};
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
                || !pf.required_tokens.is_empty()
                || !pf.any_tokens.is_empty() =>
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
            } else if !pf.any_tokens.is_empty() {
                parts.push(format!("any_tokens={}", pf.any_tokens.len()));
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

fn infer_sample_path_prefix(snapshot: &WorkspaceEdbSnapshot) -> Option<String> {
    snapshot.files.iter().find_map(|f| {
        let mut segs = f.path.split('/').filter(|s| !s.is_empty());
        match (segs.next(), segs.next(), segs.next()) {
            (Some(a), Some(b), Some(_)) => Some(format!("{a}/{b}/")),
            (Some(a), Some(_), None) => Some(format!("{a}/")),
            _ => None,
        }
    })
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

    let inferred_prefix = infer_sample_path_prefix(snapshot);
    let scope_arg = scope_prefix.or(inferred_prefix.as_deref()).unwrap_or("");
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

fn is_test_like_recovery_path(path: &str) -> bool {
    let lc = path.to_ascii_lowercase();
    lc.contains("/tests/")
        || lc.starts_with("tests/")
        || lc.contains("/t/")
        || lc.contains("_test.")
        || lc.contains("-test.")
        || lc.contains("mock")
}

type ScoredRecoverySymbol = (usize, bool, String, String, String, usize);

fn collect_recovery_stems(pf: &OwnedWorkspaceEdbPrefilter) -> Vec<String> {
    let mut seen: BTreeSet<String> = BTreeSet::new();
    pf.required_tokens
        .iter()
        .chain(pf.any_tokens.iter())
        .flat_map(|raw| split_identifier_stems(raw))
        .filter(|st| seen.insert(st.to_ascii_lowercase()))
        .take(6)
        .collect()
}

fn score_entities_against_stems(
    entities: &[agent_semantic_search::WorkspaceEdbEntityFact],
    stems_lc: &[String],
    seen_names: &mut BTreeSet<(String, String)>,
    matched_stem_indices: &mut BTreeSet<usize>,
    scored: &mut Vec<ScoredRecoverySymbol>,
) {
    entities
        .iter()
        .filter(|ent| ent.kind != "import")
        .for_each(|ent| {
            let name_lc = ent.name.to_ascii_lowercase();
            let hits = stems_lc
                .iter()
                .enumerate()
                .filter(|(_, st)| name_lc.contains(st.as_str()))
                .inspect(|(idx, _)| {
                    matched_stem_indices.insert(*idx);
                })
                .count();
            if hits > 0 && seen_names.insert((ent.name.clone(), ent.path.clone())) {
                scored.push((
                    hits,
                    !is_test_like_recovery_path(&ent.path),
                    ent.name.clone(),
                    ent.kind.clone(),
                    ent.path.clone(),
                    ent.line,
                ));
            }
        });
}

fn score_edges_against_stems(
    edges: &[agent_semantic_search::WorkspaceEdbEdgeFact],
    stems_lc: &[String],
    seen_names: &mut BTreeSet<(String, String)>,
    matched_stem_indices: &mut BTreeSet<usize>,
    scored: &mut Vec<ScoredRecoverySymbol>,
) {
    edges
        .iter()
        .filter(|edge| edge.relation != "imports" && !edge.dst.is_empty())
        .for_each(|edge| {
            let dst_lc = edge.dst.to_ascii_lowercase();
            let hits = stems_lc
                .iter()
                .enumerate()
                .filter(|(_, st)| dst_lc.contains(st.as_str()))
                .inspect(|(idx, _)| {
                    matched_stem_indices.insert(*idx);
                })
                .count();
            if hits > 0 && seen_names.insert((edge.dst.clone(), edge.path.clone())) {
                scored.push((
                    hits,
                    !is_test_like_recovery_path(&edge.path),
                    edge.dst.clone(),
                    edge.relation.clone(),
                    edge.path.clone(),
                    edge.line,
                ));
            }
        });
}

fn probe_recovery_symbols_from_index(
    idx: &WorkspaceSearchIndex,
    pf: &OwnedWorkspaceEdbPrefilter,
    stems: &[String],
    stems_lc: &[String],
    seen_names: &mut BTreeSet<(String, String)>,
    matched_stem_indices: &mut BTreeSet<usize>,
    scored: &mut Vec<ScoredRecoverySymbol>,
) {
    for stem in stems.iter().take(4) {
        let probe = WorkspaceEdbPrefilter {
            languages: &pf.languages,
            documents: &[],
            path_prefix: pf.path_prefix.as_deref(),
            extension: None,
            required_tokens: std::slice::from_ref(stem),
            max_files: 24,
            max_facts: 4096,
        };
        let sub = idx.project_prefiltered_edb(&probe);
        score_entities_against_stems(
            &sub.entities,
            stems_lc,
            seen_names,
            matched_stem_indices,
            scored,
        );
        score_edges_against_stems(
            &sub.edges,
            stems_lc,
            seen_names,
            matched_stem_indices,
            scored,
        );
    }
}

fn append_nearest_symbol_recovery(
    rendered_text: &mut String,
    snapshot: &WorkspaceEdbSnapshot,
    index: Option<&WorkspaceSearchIndex>,
    prefilter: Option<&OwnedWorkspaceEdbPrefilter>,
) {
    let Some(pf) = prefilter else {
        return;
    };
    let stems = collect_recovery_stems(pf);
    if stems.is_empty() {
        return;
    }
    let stems_lc: Vec<String> = stems.iter().map(|s| s.to_ascii_lowercase()).collect();
    let mut scored: Vec<ScoredRecoverySymbol> = Vec::new();
    let mut seen_names: BTreeSet<(String, String)> = BTreeSet::new();
    let mut matched_stem_indices: BTreeSet<usize> = BTreeSet::new();

    score_entities_against_stems(
        &snapshot.entities,
        &stems_lc,
        &mut seen_names,
        &mut matched_stem_indices,
        &mut scored,
    );
    score_edges_against_stems(
        &snapshot.edges,
        &stems_lc,
        &mut seen_names,
        &mut matched_stem_indices,
        &mut scored,
    );
    if scored.is_empty()
        && let Some(idx) = index
    {
        probe_recovery_symbols_from_index(
            idx,
            pf,
            &stems,
            &stems_lc,
            &mut seen_names,
            &mut matched_stem_indices,
            &mut scored,
        );
    }
    if scored.is_empty() {
        return;
    }
    scored.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| b.1.cmp(&a.1))
            .then_with(|| a.2.len().cmp(&b.2.len()))
            .then_with(|| a.2.cmp(&b.2))
    });
    let preview: Vec<String> = scored
        .iter()
        .take(8)
        .map(|(_, _, name, kind, path, line)| format!("{name} [{kind}] ({path}:L{line})"))
        .collect();
    let _ = writeln!(rendered_text, "nearest_symbols: {}", preview.join(", "));
    let top_name = &scored[0].2;
    let scope_arg = pf
        .path_prefix
        .as_deref()
        .map_or_else(String::new, |p| format!(" path-prefix: \"{p}\""));
    let matched_stems: Vec<&str> = matched_stem_indices
        .into_iter()
        .filter_map(|i| stems.get(i).map(String::as_str))
        .collect();
    let stem_suggestion = if matched_stems.len() >= 2 {
        format!(
            " or (find-symbols name-contains: (\"{}\" \"{}\"){scope_arg} limit: 60)",
            matched_stems[0], matched_stems[1]
        )
    } else if let Some(st) = matched_stems.first() {
        format!(" or (find-symbols name-contains: \"{st}\"{scope_arg} limit: 60)")
    } else {
        String::new()
    };
    let _ = writeln!(
        rendered_text,
        "next_query: (find-impact \"{top_name}\"{scope_arg} limit: 60){stem_suggestion}"
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
        append_nearest_symbol_recovery(rendered_text, snapshot, index, prefilter);
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
        append_nearest_symbol_recovery(rendered_text, snapshot, index, prefilter);
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
    append_nearest_symbol_recovery(rendered_text, snapshot, index, prefilter);
}

fn format_truncation_next_query_hint(
    snapshot: &WorkspaceEdbSnapshot,
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
        hints.push("exclude-tests?: #t".to_owned());
    }
    if prefilter.and_then(|pf| pf.path_prefix.as_deref()).is_none() {
        let sample_prefix = infer_sample_path_prefix(snapshot)
            .unwrap_or_else(|| "<dir>/".to_owned());
        hints.push(format!("path-prefix: \"{sample_prefix}\""));
    }
    hints.push("name-all: (\"stem1\" \"stem2\")".to_owned());
    hints.push("or raise limit:".to_owned());
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

    let header_reserve = 512usize;
    let footer_reserve = 160usize;
    let body_budget = ctx.output_limit.saturating_sub(header_reserve + footer_reserve);

    let grouped_collision_mode = is_collision_row_shape(rows) && explicit_context.is_none();
    let grouped_impact_mode =
        !grouped_collision_mode && is_impact_row_shape(rows) && explicit_context.is_none();
    let (body_lines, grouped_distinct_symbols, impact_summary) = if grouped_collision_mode {
        let (lines, distinct) = render_grouped_collision_lines(rows, ctx.index, body_budget);
        (lines, Some(distinct), None)
    } else if grouped_impact_mode {
        let (summary, lines) = render_grouped_impact_lines(rows, ctx.index, body_budget);
        (lines, None, Some(summary))
    } else {
        // Attach zero-I/O RAM source snippets automatically for concise L1 Ascent results
        // (or whenever the query explicitly requests `(context N)`), eliminating follow-up grep rounds.
        let effective_context = match explicit_context {
            Some(n) => Some(n.min(8)),
            None if ctx.index.is_some() && !rows.is_empty() && rows.len() <= 8 => Some(1),
            None if ctx.index.is_some() && !rows.is_empty() && rows.len() <= 20 => Some(0),
            None => None,
        };
        let mut body_bytes = 0usize;
        let lines: Vec<String> = rows
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
        (lines, None, None)
    };

    let returned_results = if grouped_collision_mode || grouped_impact_mode {
        rows.len()
    } else {
        body_lines.len()
    };
    let truncated = returned_results < total_results;
    let mut rendered_text = format_snapshot_header(snapshot, ctx.prefilter);
    let _ = writeln!(
        rendered_text,
        "mode: {}\nrelation: {relation}\nresults: {total_results}\nreturned: {returned_results}\ntruncated: {truncated}",
        mode.as_str(),
    );
    if let Some(distinct_symbols) = grouped_distinct_symbols {
        let _ = writeln!(
            rendered_text,
            "grouped_collisions: {distinct_symbols} distinct symbol(s) (deduplicated from {} sampled pair(s) out of {total_results} raw pairs)",
            rows.len()
        );
        if total_results > rows.len().saturating_mul(4) {
            let _ = writeln!(
                rendered_text,
                "high_cardinality: true (narrow via exclude-tests?: #t, name-all: (\"stem\"), or tighter path-prefix:)"
            );
        }
    }
    if let Some(summary) = impact_summary {
        let _ = writeln!(rendered_text, "impact_summary: {summary}");
    }
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
        let hint = format_truncation_next_query_hint(snapshot, &rows[..returned_results], ctx.prefilter);
        let _ = writeln!(
            rendered_text,
            "... (+{omitted} omitted; next_query: narrow via {hint})"
        );
    }
    rendered_text
}
