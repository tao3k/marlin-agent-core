// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! L0 pure-Rust zero-I/O RAM `(file-outline ...)` and `(read-file ...)` executors.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use marlin_agent_search_engine::{
    WorkspaceEdbPrefilter, WorkspaceSearchIndex, WorkspaceSearchQuery,
};

use super::parser::{
    DEFAULT_READ_LINES, is_path_excluded, parse_file_outline_form, parse_read_file_form,
};

struct OutlineFileEntry<'a> {
    language: &'a str,
    symbols: Vec<(usize, &'a str, &'a str)>,
}

fn outline_kind_matches(kind: &str, language_kind: &str, requested: &[String]) -> bool {
    if requested.is_empty() {
        return true;
    }
    let kind_lc = kind.to_ascii_lowercase();
    let lk_lc = language_kind.to_ascii_lowercase();
    requested.iter().any(|want| match want.as_str() {
        "any" | "all" | "*" => true,
        "fn" | "func" | "function" => {
            kind_lc == "function" || kind_lc == "method" || lk_lc.contains("fn")
        }
        "type" | "struct" | "enum" | "trait" | "impl" | "macro" | "method" | "module" => {
            kind_lc == want.as_str()
                || lk_lc == want.as_str()
                || lk_lc.starts_with(want.as_str())
                || (want == "type"
                    && matches!(kind_lc.as_str(), "struct" | "enum" | "trait" | "type"))
        }
        other => kind_lc == other || lk_lc.contains(other),
    })
}

fn top_two_segments(path: &str) -> String {
    let mut parts = path.split('/');
    match (parts.next(), parts.next()) {
        (Some(a), Some(b)) => format!("{a}/{b}"),
        (Some(a), None) => a.to_owned(),
        _ => path.to_owned(),
    }
}

fn suggest_nearby_paths(index: &WorkspaceSearchIndex, query_path: &str) -> Vec<String> {
    let probe = index.query(&WorkspaceSearchQuery {
        pattern: query_path,
        languages: &[],
        documents: &[],
        path_prefix: None,
        extension: None,
        symbol: None,
        graph_target: None,
        max_hits: 5,
        context_radius: 0,
    });
    probe.hits.into_iter().map(|h| h.path).collect()
}

pub(super) fn execute_file_outline(
    index: &WorkspaceSearchIndex,
    raw_query: &str,
    output_limit: usize,
) -> Result<String, String> {
    let req = parse_file_outline_form(raw_query)?;
    let prefilter = WorkspaceEdbPrefilter {
        languages: &req.languages,
        documents: &[],
        path_prefix: req.path_prefix.as_deref(),
        extension: None,
        required_tokens: &[],
        max_files: 256,
        max_facts: 32_768,
    };
    let snapshot = index.project_prefiltered_edb(&prefilter);

    let mut by_file: BTreeMap<&str, OutlineFileEntry<'_>> = BTreeMap::new();
    for f in &snapshot.files {
        if is_path_excluded(&f.path, &req.exclude_path_prefixes, req.exclude_tests) {
            continue;
        }
        by_file
            .entry(f.path.as_str())
            .or_insert_with(|| OutlineFileEntry {
                language: f.language.as_deref().unwrap_or("text"),
                symbols: Vec::new(),
            });
    }
    let mut scoped_kind_counts: BTreeMap<&str, usize> = BTreeMap::new();
    let mut native_ast_file_ids: BTreeSet<u32> = BTreeSet::new();
    for ent in &snapshot.entities {
        if ent.kind == "import"
            || is_path_excluded(&ent.path, &req.exclude_path_prefixes, req.exclude_tests)
        {
            continue;
        }
        if by_file.contains_key(ent.path.as_str()) {
            *scoped_kind_counts.entry(ent.kind.as_str()).or_insert(0) += 1;
            if ent.source != "resident-index" {
                native_ast_file_ids.insert(ent.file_id);
            }
        }
        if !outline_kind_matches(&ent.kind, &ent.language_kind, &req.kinds) {
            continue;
        }
        if let Some(entry) = by_file.get_mut(ent.path.as_str()) {
            entry
                .symbols
                .push((ent.line, ent.kind.as_str(), ent.name.as_str()));
        }
    }
    let matched_files_raw = by_file.len();
    let mut omitted_empty_files = 0usize;
    if by_file.len() > 1 {
        let before = by_file.len();
        by_file.retain(|_, entry| !entry.symbols.is_empty());
        omitted_empty_files = before.saturating_sub(by_file.len());
    }
    let total_files = by_file.len();
    let total_symbols: usize = by_file.values().map(|e| e.symbols.len()).sum();
    let single_file_path = (total_files == 1)
        .then(|| by_file.keys().next().copied())
        .flatten()
        .map(str::to_owned);
    let mut dir_symbol_counts: BTreeMap<String, usize> = BTreeMap::new();
    for (path, entry) in &by_file {
        *dir_symbol_counts.entry(top_two_segments(path)).or_insert(0) += entry.symbols.len();
    }

    let mut out = String::new();
    let _ = writeln!(
        out,
        "generation: {}\nelapsed_us: {}\nindexed_files: {}\nmode: file-outline (L0 pure-Rust zero-I/O AST symbol skeleton)\nfiles: {total_files}\nhits: {total_symbols}",
        snapshot.generation_digest.as_str(),
        snapshot.elapsed_micros,
        snapshot.indexed_files,
    );
    if omitted_empty_files > 0 {
        let _ = writeln!(out, "omitted_empty_files: {omitted_empty_files}");
    }
    if req.wrapped_bare_form {
        let _ = writeln!(out, "repairs: wrap-outline-parens");
    }
    if matched_files_raw == 0 {
        if let Some(prefix) = req.path_prefix.as_deref() {
            let suggestions = suggest_nearby_paths(index, prefix);
            if suggestions.is_empty() {
                let _ = writeln!(
                    out,
                    "diagnostic: path-prefix \"{prefix}\" matched 0 indexed files; list directories with (brute-search \"\" path-prefix: \"crates/\" limit: 40)"
                );
            } else {
                let _ = writeln!(
                    out,
                    "diagnostic: path-prefix \"{prefix}\" matched 0 indexed files; did you mean: {}",
                    suggestions.join(", ")
                );
            }
        }
        return Ok(out);
    }
    if total_symbols == 0 && !req.kinds.is_empty() && matched_files_raw > 0 {
        let mut ranked_kinds: Vec<(&str, usize)> = scoped_kind_counts.into_iter().collect();
        ranked_kinds.sort_unstable_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
        let parsed_kinds = if ranked_kinds.is_empty() {
            "none".to_owned()
        } else {
            ranked_kinds
                .iter()
                .take(6)
                .map(|(k, c)| format!("{k}={c}"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let native_ast_files = native_ast_file_ids.len();
        let scope_arg = req.path_prefix.as_deref().unwrap_or("crates/");
        let coverage_hint = if native_ast_files < matched_files_raw {
            format!(
                "partial native_ast coverage ({native_ast_files}/{matched_files_raw} files; coarse resident-index groups structs/enums/traits under `type`); next_query: (file-outline \"{scope_arg}\" kind: \"type\" limit: 60) or (file-outline \"{scope_arg}\" kind: \"any\" limit: 60)"
            )
        } else {
            format!(
                "complete native_ast coverage ({native_ast_files}/{matched_files_raw} files); next_query: (file-outline \"{scope_arg}\" kind: \"any\" limit: 60)"
            )
        };
        let _ = writeln!(
            out,
            "returned: 0\ntruncated: false\ndiagnostic: kind filter ({}) matched 0 symbols across {matched_files_raw} file(s) (parsed_kinds: {parsed_kinds}; {coverage_hint})",
            req.kinds.join(", ")
        );
        return Ok(out);
    }

    let per_file_cap = req.max_per_file.or_else(|| {
        (total_files > 1 && total_symbols > req.max_symbols)
            .then(|| (req.max_symbols / total_files).max(6))
    });
    let body_budget = output_limit.saturating_sub(out.len() + 220);
    let mut emitted_symbols = 0usize;
    let mut body = String::new();
    let mut omitted_dirs: BTreeSet<String> = BTreeSet::new();
    let mut stopped = false;
    for (path, mut entry) in by_file {
        if stopped {
            omitted_dirs.insert(top_two_segments(path));
            continue;
        }
        let lang = entry.language;
        entry
            .symbols
            .sort_unstable_by_key(|(line, _, name)| (*line, *name));
        entry
            .symbols
            .dedup_by(|a, b| a.0 == b.0 && a.1 == b.1 && a.2 == b.2);
        let file_total = entry.symbols.len();
        let file_hdr = format!("  - @{path}  [{lang} · {file_total} symbol(s)]\n");
        if body.len() + file_hdr.len() > body_budget {
            omitted_dirs.insert(top_two_segments(path));
            stopped = true;
            continue;
        }
        body.push_str(&file_hdr);
        let mut emitted_in_file = 0usize;
        for (line, kind, name) in entry.symbols {
            if emitted_symbols >= req.max_symbols
                || per_file_cap.is_some_and(|cap| emitted_in_file >= cap)
            {
                break;
            }
            let row = format!("      L{line:<4} [{kind}] {name}\n");
            if body.len() + row.len() > body_budget {
                stopped = true;
                break;
            }
            body.push_str(&row);
            emitted_symbols += 1;
            emitted_in_file += 1;
        }
        if emitted_in_file < file_total {
            let note = format!(
                "      ... (+{} more in {path}; call (file-outline \"{path}\") for full file)\n",
                file_total - emitted_in_file
            );
            if body.len() + note.len() <= body_budget {
                body.push_str(&note);
            }
        }
        if emitted_symbols >= req.max_symbols {
            stopped = true;
        }
    }

    let truncated = emitted_symbols < total_symbols;
    let _ = writeln!(out, "returned: {emitted_symbols}\ntruncated: {truncated}");
    if truncated && total_files > 1 && !dir_symbol_counts.is_empty() {
        let mut ranked_dirs: Vec<(String, usize)> = dir_symbol_counts.into_iter().collect();
        ranked_dirs.sort_unstable_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        let top_dirs = ranked_dirs
            .iter()
            .take(5)
            .map(|(d, c)| format!("{d}({c})"))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(out, "top_dirs: {top_dirs}");
    }
    if !omitted_dirs.is_empty() {
        let sample: Vec<String> = omitted_dirs.into_iter().take(12).collect();
        let _ = writeln!(out, "omitted_dirs: {}", sample.join(", "));
    }
    out.push_str(&body);
    if truncated {
        if let Some(exact_path) = single_file_path.as_deref() {
            let _ = writeln!(
                out,
                "  ... (+{} symbols omitted; next_query: (file-outline \"{exact_path}\" limit: {total_symbols}) — omit max-per-file: on exact files to inspect all {total_symbols} symbols)",
                total_symbols.saturating_sub(emitted_symbols),
            );
        } else {
            let _ = writeln!(
                out,
                "  ... (+{} symbols omitted; next_query: narrow path-prefix: to one crate from top_dirs, add kind: \"struct|enum|fn\", or use `structural_query` (find-collisions ...))",
                total_symbols.saturating_sub(emitted_symbols),
            );
        }
    }
    Ok(out)
}

fn resolve_symbol_anchor_in_file(
    index: &WorkspaceSearchIndex,
    path: &str,
    symbol: &str,
) -> Result<usize, Vec<String>> {
    let prefilter = WorkspaceEdbPrefilter {
        languages: &[],
        documents: &[],
        path_prefix: Some(path),
        extension: None,
        required_tokens: &[],
        max_files: 4,
        max_facts: 4096,
    };
    let snapshot = index.project_prefiltered_edb(&prefilter);
    let sym_lc = symbol.to_ascii_lowercase();
    let mut exact_ast_line: Option<usize> = None;
    let mut fuzzy_ast_line: Option<usize> = None;
    let mut file_symbols: Vec<String> = Vec::new();
    for ent in &snapshot.entities {
        if ent.path != path || ent.kind == "import" {
            continue;
        }
        if file_symbols.len() < 12 {
            file_symbols.push(format!("{}@L{}", ent.name, ent.line));
        }
        if ent.name == symbol && exact_ast_line.is_none() {
            exact_ast_line = Some(ent.line);
        } else if ent.name.to_ascii_lowercase().contains(&sym_lc) && fuzzy_ast_line.is_none() {
            fuzzy_ast_line = Some(ent.line);
        }
    }
    if let Some(line) = exact_ast_line.or(fuzzy_ast_line) {
        return Ok(line);
    }
    let text_probe = index.query(&WorkspaceSearchQuery {
        pattern: symbol,
        languages: &[],
        documents: &[],
        path_prefix: Some(path),
        extension: None,
        symbol: None,
        graph_target: None,
        max_hits: 4,
        context_radius: 0,
    });
    for hit in text_probe.hits {
        if hit.path == path
            && let Some(line) = hit.line_matches.first().map(|m| m.line_number).or(hit.first_line)
        {
            return Ok(line);
        }
    }
    Err(file_symbols)
}

pub(super) fn execute_read_file(
    index: &WorkspaceSearchIndex,
    raw_query: &str,
    output_limit: usize,
) -> Result<String, String> {
    let req = parse_read_file_form(raw_query)?;
    let (eff_start, eff_end, symbol_anchor_line) = if let Some(sym) = req.symbol.as_deref() {
        match resolve_symbol_anchor_in_file(index, &req.path, sym) {
            Ok(anchor_line) => {
                let before = (req.symbol_context / 3).max(5);
                let after = req.symbol_context.saturating_mul(2).clamp(25, 160);
                let start = anchor_line.saturating_sub(before).max(1);
                let end = anchor_line.saturating_add(after);
                (start, end, Some(anchor_line))
            }
            Err(available) => {
                let probe_exists = !index.live_file_context(&req.path, 1, 0).is_empty();
                if probe_exists {
                    let sym_hint = if available.is_empty() {
                        format!("call (file-outline \"{}\") to inspect symbols", req.path)
                    } else {
                        format!("available symbols in file: {}", available.join(", "))
                    };
                    return Err(format!(
                        "read-file: symbol `{sym}` not found in `{}` ({sym_hint})",
                        req.path
                    ));
                }
                (req.start_line, req.end_line, None)
            }
        }
    } else {
        (req.start_line, req.end_line, None)
    };
    let span = eff_end.saturating_sub(eff_start);
    let mid = eff_start.saturating_add(span / 2);
    let radius = (span / 2).saturating_add(1);
    let raw_lines = index.live_file_context(&req.path, mid, radius);

    if raw_lines.is_empty() {
        let suggestions = suggest_nearby_paths(index, &req.path);
        let hint = if suggestions.is_empty() {
            "list workspace files with (brute-search \"\" path-prefix: \"crates/\" limit: 40)"
                .to_owned()
        } else {
            format!("did you mean: {}", suggestions.join(", "))
        };
        return Err(format!(
            "read-file: path `{}` not found in resident workspace index ({hint})",
            req.path
        ));
    }

    let slice: Vec<_> = raw_lines
        .into_iter()
        .filter(|l| l.line_number >= eff_start && l.line_number <= eff_end)
        .collect();
    let actual_end = slice
        .last()
        .map_or(eff_start, |line| line.line_number);
    let has_more = index
        .live_file_context(&req.path, actual_end.saturating_add(1), 0)
        .iter()
        .any(|l| l.line_number > actual_end);

    let mut out = String::new();
    let _ = writeln!(
        out,
        "generation: {}\nindexed_files: {}\nmode: read-file (L0 pure-Rust zero-I/O RAM reader)\npath: {}\nrange: L{}..=L{}\nhits: {}\nhas_more: {has_more}",
        index.generation_digest().as_str(),
        index.file_count(),
        req.path,
        eff_start,
        actual_end,
        slice.len(),
    );
    if let (Some(sym), Some(anchor_line)) = (req.symbol.as_deref(), symbol_anchor_line) {
        let _ = writeln!(out, "symbol_anchor: {sym} @ L{anchor_line}");
    }
    if has_more {
        let _ = writeln!(out, "next_start: {}", actual_end.saturating_add(1));
    }
    if req.wrapped_bare_form {
        let _ = writeln!(out, "repairs: wrap-read-file-parens");
    }

    let body_budget = output_limit.saturating_sub(out.len() + 128);
    let mut body_bytes = 0usize;
    let formatted_lines: Vec<String> = slice
        .iter()
        .map(|line| format!("L{}: {}\n", line.line_number, line.text.trim_end()))
        .take_while(|formatted| {
            if body_bytes + formatted.len() <= body_budget {
                body_bytes += formatted.len();
                true
            } else {
                false
            }
        })
        .collect();
    let returned = formatted_lines.len();
    let truncated = returned < slice.len() || has_more;
    let _ = writeln!(out, "returned: {returned}\ntruncated: {truncated}");
    out.extend(formatted_lines);
    if has_more {
        let next_start = actual_end.saturating_add(1);
        let next_end = next_start.saturating_add(DEFAULT_READ_LINES - 1);
        let _ = writeln!(
            out,
            "  ... (more lines available; continue with (read-file \"{}\" start: {next_start} end: {next_end}))",
            req.path
        );
    }
    Ok(out)
}
