// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! L0 pure-Rust parallel brute-force executor and L0 dispatcher.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use marlin_agent_search_engine::{
    WorkspaceEdbPrefilter, WorkspaceSearchIndex, WorkspaceSearchQuery,
};

use super::outline_read::execute_file_outline;
use super::parser::{
    BruteSearchRequest, DEFAULT_BRUTE_MAX_HITS, L0FormKind, classify_l0_form, is_path_excluded,
    is_test_path, parse_brute_search_form,
};
use super::read_file::execute_read_file;
use super::workspace_map::execute_workspace_map;

fn has_line_anchor(expr: &str) -> bool {
    expr.chars()
        .scan((false, false), |(escaped, in_class), ch| {
            if *escaped {
                *escaped = false;
                return Some(false);
            }
            let found = match ch {
                '\\' => {
                    *escaped = true;
                    false
                }
                '[' if !*in_class => {
                    *in_class = true;
                    false
                }
                ']' if *in_class => {
                    *in_class = false;
                    false
                }
                '^' | '$' if !*in_class => true,
                _ => false,
            };
            Some(found)
        })
        .any(|found| found)
}

fn normalize_brute_pattern(raw: &str) -> (String, Option<&'static str>) {
    if let Some(expr) = raw.strip_prefix("re:") {
        if expr.is_empty() {
            return ("re:.".to_owned(), Some("normalize-empty-regex"));
        }
        // Enable multiline anchors silently as first-class surface syntax
        // (zero `repairs:` noise) so `^`/`$` always match line boundaries.
        if has_line_anchor(expr)
            && !expr.starts_with("(?m)")
            && !expr.starts_with("(?-m)")
        {
            return (format!("re:(?m){expr}"), None);
        }
        return (raw.to_owned(), None);
    }
    (raw.to_owned(), None)
}

fn format_canonical_brute_query(req: &BruteSearchRequest, effective_pattern: &str) -> String {
    let mut out = format!("(brute-search \"{effective_pattern}\"");
    if let Some(prefix) = req.path_prefix.as_deref() {
        let _ = write!(out, " path-prefix: \"{prefix}\"");
    }
    for ex in &req.exclude_path_prefixes {
        let _ = write!(out, " exclude-path-prefix: \"{ex}\"");
    }
    for lang in &req.languages {
        let _ = write!(out, " lang: \"{lang}\"");
    }
    if let Some(ctx) = req.context_lines {
        let _ = write!(out, " context: {ctx}");
    }
    if req.max_hits != DEFAULT_BRUTE_MAX_HITS {
        let _ = write!(out, " limit: {}", req.max_hits);
    }
    out.push(')');
    out
}

fn looks_like_unprefixed_regex(pattern: &str) -> bool {
    if pattern.starts_with("re:") || pattern.is_empty() {
        return false;
    }
    pattern.starts_with('^')
        || pattern.ends_with('$')
        || pattern.contains("\\s")
        || pattern.contains("\\w")
        || pattern.contains("\\d")
        || pattern.contains(".*")
        || pattern.contains(".+")
        || pattern.contains('|')
}

fn format_hit_block(
    index: &WorkspaceSearchIndex,
    hit: &marlin_agent_search_engine::WorkspaceSearchHit,
    effective_context: usize,
    max_matches_per_file: usize,
) -> String {
    let lang = hit.language.as_deref().unwrap_or("file");
    let axes = hit.matched_axes.join("+");
    let mut block = String::new();

    if hit.line_matches.is_empty() {
        let loc = match hit.first_line {
            Some(line) => format!("@{}:{line}", hit.path),
            None => format!("@{}", hit.path),
        };
        let preview = hit.first_line_text.trim();
        if preview.is_empty() {
            let _ = writeln!(block, "  - {loc}  [{lang} · {axes}]");
        } else {
            let _ = writeln!(block, "  - {loc}  [{lang} · {axes}]  {preview}");
        }
        return block;
    }

    if hit.line_matches.len() == 1 && effective_context == 0 {
        let m = &hit.line_matches[0];
        let preview = m.line_text.trim();
        let _ = writeln!(
            block,
            "  - @{}:{}  [{lang} · {axes}]  {preview}",
            hit.path, m.line_number
        );
        return block;
    }

    let total_file_matches = hit.line_matches.len();
    let header_loc = hit
        .line_matches
        .first()
        .map_or_else(|| format!("@{}", hit.path), |m| format!("@{}:{}", hit.path, m.line_number));
    let _ = writeln!(
        block,
        "  - {header_loc}  [{lang} · {axes} · {total_file_matches} match(es)]"
    );
    if effective_context == 0 {
        for m in hit.line_matches.iter().take(max_matches_per_file) {
            let _ = writeln!(block, "      L{}: {}", m.line_number, m.line_text.trim_end());
        }
        if total_file_matches > max_matches_per_file {
            let _ = writeln!(
                block,
                "      ... (+{} more in {}; inspect via (file-outline \"{}\" projection: \"callable-skeleton\"))",
                total_file_matches - max_matches_per_file,
                hit.path,
                hit.path
            );
        }
        return block;
    }

    let match_lines: BTreeSet<usize> = hit.line_matches.iter().map(|m| m.line_number).collect();
    let shown_match_lines: Vec<usize> = match_lines
        .iter()
        .copied()
        .take(max_matches_per_file)
        .collect();
    let mut emitted: BTreeSet<usize> = BTreeSet::new();
    let mut last_emitted: Option<usize> = None;
    for &line_num in &shown_match_lines {
        let ctx_lines = index.live_file_context(&hit.path, line_num, effective_context);
        for ctx in ctx_lines {
            if !emitted.insert(ctx.line_number) {
                continue;
            }
            if let Some(prev) = last_emitted
                && ctx.line_number > prev + 1
            {
                let _ = writeln!(block, "      ...");
            }
            let marker = if match_lines.contains(&ctx.line_number) {
                '>'
            } else {
                ' '
            };
            let _ = writeln!(
                block,
                "     {marker}L{}: {}",
                ctx.line_number,
                ctx.text.trim_end()
            );
            last_emitted = Some(ctx.line_number);
        }
    }
    if match_lines.len() > shown_match_lines.len() {
        let _ = writeln!(
            block,
            "      ... (+{} more match line(s) in {})",
            match_lines.len() - shown_match_lines.len(),
            hit.path
        );
    }
    block
}

fn extract_diagnostic_stems(raw_pattern: &str) -> Vec<String> {
    let cleaned = raw_pattern
        .strip_prefix("re:")
        .unwrap_or(raw_pattern)
        .strip_prefix("(?m)")
        .unwrap_or(raw_pattern.strip_prefix("re:").unwrap_or(raw_pattern));
    let mut stems = Vec::new();
    let mut seen = BTreeSet::new();
    let mut cur = String::new();
    let mut prev_lower = false;
    let push_cur = |buf: &mut String, out: &mut Vec<String>, seen_set: &mut BTreeSet<String>| {
        if buf.len() >= 4 {
            let lc = buf.to_ascii_lowercase();
            if !matches!(
                lc.as_str(),
                "error" | "tests" | "test" | "from" | "with" | "this" | "that" | "true" | "false"
            ) && seen_set.insert(lc.clone())
            {
                out.push(lc);
            }
        }
        buf.clear();
    };
    for ch in cleaned.chars() {
        if ch.is_ascii_alphabetic() {
            if ch.is_ascii_uppercase() && prev_lower && cur.len() >= 4 {
                push_cur(&mut cur, &mut stems, &mut seen);
            }
            cur.push(ch);
            prev_lower = ch.is_ascii_lowercase();
        } else {
            push_cur(&mut cur, &mut stems, &mut seen);
            prev_lower = false;
        }
    }
    push_cur(&mut cur, &mut stems, &mut seen);
    stems.sort_by(|a, b| b.len().cmp(&a.len()).then_with(|| a.cmp(b)));
    stems.truncate(5);
    stems
}

fn format_brute_zero_hit_diagnostic(
    index: &WorkspaceSearchIndex,
    req: &BruteSearchRequest,
    effective_pattern: &str,
) -> Option<String> {
    let mut hints = Vec::new();
    if req.pattern.contains("path-prefix:")
        || req.pattern.contains("exclude-path-prefix:")
        || req.pattern.contains("lang:")
    {
        hints.push(
            "pattern contains keyword syntax inside quotes; pass path-prefix:, exclude-path-prefix:, or lang: as bare keywords outside the quoted pattern"
                .to_owned(),
        );
    }
    if looks_like_unprefixed_regex(&req.pattern) {
        let (re_pattern, _) = normalize_brute_pattern(&format!("re:{}", req.pattern));
        let re_probe = index.query(&WorkspaceSearchQuery {
            pattern: &re_pattern,
            languages: &req.languages,
            documents: &req.documents,
            path_prefix: req.path_prefix.as_deref(),
            extension: req.extension.as_deref(),
            symbol: req.symbol.as_deref(),
            graph_target: req.graph_target.as_deref(),
            max_hits: 3,
            context_radius: 0,
        });
        if re_probe.matched_files > 0 {
            hints.push(format!(
                "retry with explicit regex prefix (brute-search \"re:{}\" ...) which matched {} file(s)",
                req.pattern, re_probe.matched_files
            ));
        } else {
            hints.push(
                "regex pattern matched 0 files; the exact symbol name(s) may differ — try shorter identifier stems, (file-outline ...), or `structural_query`"
                    .to_owned(),
            );
        }
    }

    if let Some(prefix) = req.path_prefix.as_deref() {
        let scope_probe = index.query(&WorkspaceSearchQuery {
            pattern: "",
            languages: &req.languages,
            documents: &req.documents,
            path_prefix: Some(prefix),
            extension: req.extension.as_deref(),
            symbol: None,
            graph_target: None,
            max_hits: 8,
            context_radius: 0,
        });
        let scope_files = scope_probe.matched_files;
        if scope_files == 0 {
            let parent = prefix.trim_end_matches('/').rsplit_once('/').map(|(p, _)| p);
            let siblings = parent.map(|parent_dir| {
                let sib = index.query(&WorkspaceSearchQuery {
                    pattern: "",
                    languages: &[],
                    documents: &[],
                    path_prefix: Some(parent_dir),
                    extension: None,
                    symbol: None,
                    graph_target: None,
                    max_hits: 5,
                    context_radius: 0,
                });
                sib.hits
                    .into_iter()
                    .map(|h| h.path)
                    .collect::<Vec<_>>()
                    .join(", ")
            });
            match siblings.filter(|s| !s.is_empty()) {
                Some(s) => hints.push(format!(
                    "path-prefix \"{prefix}\" matched 0 indexed files (nearby: {s})"
                )),
                None => hints.push(format!("path-prefix \"{prefix}\" matched 0 indexed files")),
            }
        } else {
            let has_scheme = scope_probe
                .hits
                .iter()
                .any(|h| h.language.as_deref() == Some("scheme") || h.path.ends_with(".ss"));
            if has_scheme && req.pattern.contains("define") {
                hints.push(
                    "scoped file(s) are Gerbil Scheme (.ss), which uses (def ...), (.def ...), (defstruct ...), (defmethod ...) instead of R5RS define; try (file-outline \"...\")".to_owned(),
                );
            }
            if !effective_pattern.is_empty() {
                let global_probe = index.query(&WorkspaceSearchQuery {
                    pattern: effective_pattern,
                    languages: &req.languages,
                    documents: &req.documents,
                    path_prefix: None,
                    extension: req.extension.as_deref(),
                    symbol: req.symbol.as_deref(),
                    graph_target: req.graph_target.as_deref(),
                    max_hits: 4,
                    context_radius: 0,
                });
                if global_probe.matched_files > 0 {
                    let sample_paths = global_probe
                        .hits
                        .iter()
                        .take(3)
                        .map(|h| h.path.as_str())
                        .collect::<Vec<_>>()
                        .join(", ");
                    hints.push(format!(
                        "path-prefix \"{prefix}\" has {scope_files} file(s) with 0 matches, but workspace-wide found {} file(s) (e.g. {sample_paths})",
                        global_probe.matched_files
                    ));
                } else {
                    hints.push(format!(
                        "path-prefix \"{prefix}\" has {scope_files} file(s); 0 matches workspace-wide — try (file-outline \"{prefix}\") or relax pattern"
                    ));
                }
            }
        }
    }

    if !effective_pattern.is_empty() {
        let stems = extract_diagnostic_stems(&req.pattern);
        let mut stem_matches: Vec<(String, usize)> = Vec::new();
        for stem in stems {
            if stem.eq_ignore_ascii_case(effective_pattern) {
                continue;
            }
            let probe = index.query(&WorkspaceSearchQuery {
                pattern: &stem,
                languages: &req.languages,
                documents: &req.documents,
                path_prefix: req.path_prefix.as_deref(),
                extension: req.extension.as_deref(),
                symbol: None,
                graph_target: None,
                max_hits: 8,
                context_radius: 0,
            });
            if probe.matched_files > 0 {
                stem_matches.push((stem, probe.matched_files));
            }
        }
        if !stem_matches.is_empty() {
            stem_matches.sort_unstable_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
            let summary = stem_matches
                .iter()
                .map(|(s, c)| format!("\"{s}\"={c} files"))
                .collect::<Vec<_>>()
                .join(", ");
            let best_stem = &stem_matches[0].0;
            let best_lc = best_stem.to_ascii_lowercase();
            let probe_edb = index.project_prefiltered_edb(&WorkspaceEdbPrefilter {
                languages: &req.languages,
                documents: &[],
                path_prefix: req.path_prefix.as_deref(),
                extension: None,
                required_tokens: std::slice::from_ref(best_stem),
                max_files: 16,
                max_facts: 2048,
            });
            let mut seen_syms = BTreeSet::new();
            let nearest: Vec<String> = probe_edb
                .entities
                .iter()
                .filter(|e| {
                    e.kind != "import"
                        && !is_test_path(&e.path)
                        && e.name.to_ascii_lowercase().contains(&best_lc)
                        && seen_syms.insert(e.name.clone())
                })
                .take(5)
                .map(|e| format!("{} [{}] ({}:L{})", e.name, e.kind, e.path, e.line))
                .collect();
            let nearest_clause = if nearest.is_empty() {
                String::new()
            } else {
                format!("; nearest_symbols: {}", nearest.join(", "))
            };
            hints.push(format!(
                "live stem_hits: {summary}{nearest_clause}; try (brute-search \"{best_stem}\" exclude-tests?: #t) or (find-symbols name-contains: \"{best_stem}\" lang: \"any\" exclude-tests?: #t)"
            ));
        }
    }

    (!hints.is_empty()).then(|| hints.join("; "))
}

fn top_two_path_segments(path: &str) -> String {
    let mut parts = path.split('/');
    match (parts.next(), parts.next()) {
        (Some(a), Some(b)) => format!("{a}/{b}"),
        (Some(a), None) => a.to_owned(),
        _ => path.to_owned(),
    }
}

fn summarize_brute_hit_dirs(
    hits: &[&marlin_agent_search_engine::WorkspaceSearchHit],
) -> Option<String> {
    let mut counts: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for hit in hits {
        *counts.entry(top_two_path_segments(&hit.path)).or_insert(0) += 1;
    }
    if counts.len() <= 1 {
        return None;
    }
    let mut ranked: Vec<(String, usize)> = counts.into_iter().collect();
    ranked.sort_unstable_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    Some(
        ranked
            .into_iter()
            .take(5)
            .map(|(dir, count)| format!("{dir}({count})"))
            .collect::<Vec<_>>()
            .join(", "),
    )
}

fn brute_path_priority(path: &str) -> u8 {
    let lc = path.to_ascii_lowercase();
    if is_test_path(&lc) || lc.contains("test-case-") {
        2
    } else if lc.starts_with("docs/")
        || lc.starts_with("dev/")
        || lc.ends_with(".org")
        || lc.ends_with(".md")
    {
        1
    } else {
        0
    }
}

fn execute_brute_search_only(
    index: &WorkspaceSearchIndex,
    raw_query: &str,
    output_limit: usize,
) -> Result<String, String> {
    let req = parse_brute_search_form(raw_query)?;
    let (effective_pattern, pattern_repair) = normalize_brute_pattern(&req.pattern);
    let mut repairs: Vec<&'static str> = Vec::new();
    if req.wrapped_bare_form {
        repairs.push("wrap-brute-parens");
    }
    if let Some(repair) = pattern_repair {
        repairs.push(repair);
    }

    let query_context = req.context_lines.unwrap_or(1);
    let has_exclusions = !req.exclude_path_prefixes.is_empty() || req.exclude_tests;
    let fetch_limit = if !has_exclusions {
        req.max_hits.saturating_mul(2).clamp(32, 256)
    } else {
        req.max_hits.saturating_mul(4).clamp(64, 512)
    };
    let report = index.query(&WorkspaceSearchQuery {
        pattern: &effective_pattern,
        languages: &req.languages,
        documents: &req.documents,
        path_prefix: req.path_prefix.as_deref(),
        extension: req.extension.as_deref(),
        symbol: req.symbol.as_deref(),
        graph_target: req.graph_target.as_deref(),
        max_hits: fetch_limit,
        context_radius: query_context,
    });

    let excluded_count = if !has_exclusions {
        0
    } else {
        report
            .hits
            .iter()
            .filter(|h| is_path_excluded(&h.path, &req.exclude_path_prefixes, req.exclude_tests))
            .count()
    };
    let mut filtered_hits: Vec<&marlin_agent_search_engine::WorkspaceSearchHit> = report
        .hits
        .iter()
        .filter(|h| !is_path_excluded(&h.path, &req.exclude_path_prefixes, req.exclude_tests))
        .collect();
    // Rank production source files ahead of docs and tests so broad scans never let
    // noisy fixture/test files consume the entire output budget before real code appears.
    filtered_hits.sort_by_key(|h| brute_path_priority(&h.path));
    filtered_hits.truncate(req.max_hits);

    let default_context = usize::from(
        !req.pattern.is_empty() && !filtered_hits.is_empty() && filtered_hits.len() <= 3,
    );
    let effective_context = req.context_lines.unwrap_or(default_context);
    let max_matches_per_file = if filtered_hits.len() > 6 {
        3
    } else if filtered_hits.len() > 2 {
        6
    } else {
        16
    };

    let header_reserve = 384usize;
    let footer_reserve = 192usize;
    // Keep brute-search output compact by default (~24KB max unless caller explicitly
    // asks for large limit/context) so truncated broad scans never flood LLM context.
    let effective_limit = if req.max_hits <= 48 && req.context_lines.unwrap_or(0) <= 4 {
        output_limit.min(24_576)
    } else {
        output_limit
    };
    let body_budget = effective_limit.saturating_sub(header_reserve + footer_reserve);
    let mut body_bytes = 0usize;
    let rendered_blocks: Vec<String> = filtered_hits
        .iter()
        .map(|hit| format_hit_block(index, hit, effective_context, max_matches_per_file))
        .take_while(|block| {
            if body_bytes + block.len() <= body_budget {
                body_bytes += block.len();
                true
            } else {
                false
            }
        })
        .collect();

    let total_hits = if !has_exclusions {
        report.matched_files.max(filtered_hits.len())
    } else {
        report
            .matched_files
            .saturating_sub(excluded_count)
            .max(filtered_hits.len())
    };
    let returned_hits = rendered_blocks.len();
    let truncated = returned_hits < total_hits || filtered_hits.len() < total_hits;
    let mut out = String::new();
    let _ = writeln!(
        out,
        "generation: {}\nelapsed_us: {}\nindexed_files: {}\nmode: brute-force (L0 pure-Rust parallel trigram/regex/bm25)\nhits: {total_hits}\nreturned: {returned_hits}\ntruncated: {truncated}",
        report.generation_digest.as_str(),
        report.elapsed_micros,
        report.indexed_files,
    );
    if excluded_count > 0 {
        let _ = writeln!(
            out,
            "excluded_files: {excluded_count} (exclude_tests={}, exclude_path_prefix={:?})",
            req.exclude_tests, req.exclude_path_prefixes
        );
    }
    let top_dirs_opt = if truncated {
        summarize_brute_hit_dirs(&filtered_hits)
    } else {
        None
    };
    let top_files_ranked: Vec<(&str, usize)> = if truncated {
        let mut ranked: Vec<(&str, usize)> = filtered_hits
            .iter()
            .map(|h| (h.path.as_str(), h.line_matches.len()))
            .collect();
        ranked.sort_unstable_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
        ranked.truncate(5);
        ranked
    } else {
        Vec::new()
    };
    if let Some(top_dirs) = top_dirs_opt.as_deref() {
        let _ = writeln!(out, "top_dirs: {top_dirs}");
    }
    if !top_files_ranked.is_empty() {
        let top_files_str: Vec<String> = top_files_ranked
            .iter()
            .map(|(p, c)| format!("{p}({c})"))
            .collect();
        let _ = writeln!(out, "top_files: {}", top_files_str.join(", "));
    }
    if !repairs.is_empty() {
        let _ = writeln!(out, "repairs: {}", repairs.join(", "));
        let _ = writeln!(
            out,
            "canonical_query: {}",
            format_canonical_brute_query(&req, &effective_pattern)
        );
    }
    if total_hits == 0
        && let Some(diag) = format_brute_zero_hit_diagnostic(index, &req, &effective_pattern)
    {
        let _ = writeln!(out, "diagnostic: {diag}");
    }
    for block in rendered_blocks {
        out.push_str(&block);
    }
    if returned_hits < total_hits {
        let scoped_suggestion = top_dirs_opt
            .as_deref()
            .and_then(|dirs| dirs.split('(').next())
            .map(str::trim)
            .filter(|dir| !dir.is_empty())
            .map(|top_dir| {
                format!(
                    "(brute-search \"{effective_pattern}\" path-prefix: \"{top_dir}/\" exclude-tests?: #t limit: 24)"
                )
            });
        let top_file_outline = top_files_ranked.first().map(|(path, _)| {
            format!("(file-outline \"{path}\" projection: \"callable-skeleton\")")
        });
        match (scoped_suggestion, top_file_outline) {
            (Some(next_q), Some(outline_q)) => {
                let _ = writeln!(
                    out,
                    "  ... (+{} omitted; next_query: {next_q} or inspect top file via {outline_q})",
                    total_hits - returned_hits
                );
            }
            (Some(next_q), None) => {
                let _ = writeln!(
                    out,
                    "  ... (+{} omitted; next_query: {next_q} or use `structural_query` (find-impact ...) / (find-symbols name-all: (...)))",
                    total_hits - returned_hits
                );
            }
            (None, Some(outline_q)) => {
                let _ = writeln!(
                    out,
                    "  ... (+{} omitted; next_query: {outline_q} or narrow path-prefix: / exclude-tests?: #t)",
                    total_hits - returned_hits
                );
            }
            (None, None) => {
                let _ = writeln!(
                    out,
                    "  ... (+{} omitted; next_query: narrow path-prefix: to one crate from top_dirs, add exclude-tests?: #t, or use `structural_query` (find-impact ...) / (find-symbols ...))",
                    total_hits - returned_hits
                );
            }
        }
    }
    Ok(out)
}

/// Execute any L0 pure-Rust RAM fast-path query (`workspace-map`, `brute-search`,
/// `file-outline`, or `read-file`) directly against `index` (zero Gerbil VM lock).
pub(crate) fn execute_l0_search(
    index: &WorkspaceSearchIndex,
    raw_query: &str,
    output_limit: usize,
) -> Result<String, String> {
    match classify_l0_form(raw_query) {
        Some(L0FormKind::WorkspaceMap) => execute_workspace_map(index, raw_query, output_limit),
        Some(L0FormKind::BruteSearch) => execute_brute_search_only(index, raw_query, output_limit),
        Some(L0FormKind::FileOutline) => execute_file_outline(index, raw_query, output_limit),
        Some(L0FormKind::ReadFile) => execute_read_file(index, raw_query, output_limit),
        None => Err(
            "expected an L0 form: (workspace-map ...), (brute-search ...), (file-outline ...), or (read-file ...)"
                .to_owned(),
        ),
    }
}
