// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! L0 pure-Rust zero-I/O RAM `(read-file ...)` executor with multi-symbol slicing,
//! cursor continuation, clamp telemetry, and remaining-symbol lookahead anchors.

use std::fmt::Write as _;

use marlin_agent_search_engine::{
    WorkspaceEdbPrefilter, WorkspaceSearchIndex, WorkspaceSearchQuery,
};

use super::outline_read::{
    find_symbol_definition_end_line, refine_coarse_entity_kind, suggest_nearby_paths,
};
use super::parser::{DEFAULT_READ_LINES, MAX_READ_LINES, ReadFileRequest, parse_read_file_form};

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
            && let Some(line) = hit.line_matches.first().map(|m| m.line_number)
        {
            return Ok(line);
        }
    }
    Err(file_symbols)
}

struct SymbolSliceBudget {
    symbol_context: usize,
    body_budget: usize,
}

fn discover_crate_companion_tests(index: &WorkspaceSearchIndex, path: &str) -> Vec<String> {
    let Some((crate_root, rel_in_src)) = path.split_once("/src/") else {
        return Vec::new();
    };
    let tests_prefix = format!("{crate_root}/tests/");
    let prefilter = WorkspaceEdbPrefilter {
        languages: &[],
        documents: &[],
        path_prefix: Some(tests_prefix.as_str()),
        extension: None,
        required_tokens: &[],
        max_files: 32,
        max_facts: 1,
    };
    let snapshot = index.project_prefiltered_edb(&prefilter);
    if snapshot.files.is_empty() {
        return Vec::new();
    }
    let stem = rel_in_src
        .rsplit('/')
        .next()
        .unwrap_or(rel_in_src)
        .split('.')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    let mut ranked: Vec<(bool, String)> = snapshot
        .files
        .into_iter()
        .filter(|f| f.path.starts_with(&tests_prefix))
        .map(|f| {
            let matches_stem = !stem.is_empty() && f.path.to_ascii_lowercase().contains(&stem);
            (matches_stem, f.path)
        })
        .collect();
    ranked.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    ranked.into_iter().take(4).map(|(_, p)| p).collect()
}

fn append_symbol_slice(
    index: &WorkspaceSearchIndex,
    path: &str,
    sym: &str,
    anchor_line: usize,
    budget: &SymbolSliceBudget,
    body: &mut String,
    total_lines: &mut usize,
) -> bool {
    let before = (budget.symbol_context / 4).max(3);
    let ast_end = find_symbol_definition_end_line(index, path, anchor_line, 80);
    let after = budget.symbol_context.clamp(16, 80);
    let start = anchor_line.saturating_sub(before).max(1);
    let end = match ast_end {
        Some(end_line) => end_line.saturating_add(1).min(anchor_line.saturating_add(80)),
        None => anchor_line.saturating_add(after),
    };
    let span = end.saturating_sub(start);
    let slice: Vec<_> = index
        .live_file_context(
            path,
            start.saturating_add(span / 2),
            (span / 2).saturating_add(1),
        )
        .into_iter()
        .filter(|l| l.line_number >= start && l.line_number <= end)
        .collect();
    let actual_end = slice.last().map_or(start, |l| l.line_number);
    let span_note = ast_end.map_or_else(String::new, |e| format!(" [ast_span: L{anchor_line}..=L{e}]"));
    let header = format!(
        "=== symbol: {sym} @ {path}:L{anchor_line} (L{start}..=L{actual_end}){span_note} ===\n"
    );
    if body.len() + header.len() > budget.body_budget {
        return true;
    }
    body.push_str(&header);
    let mut cur_len = body.len();
    let mut overflow = false;
    let formatted: Vec<String> = slice
        .iter()
        .map(|line| format!("L{}: {}\n", line.line_number, line.text.trim_end()))
        .take_while(|row| {
            if cur_len + row.len() <= budget.body_budget {
                cur_len += row.len();
                true
            } else {
                overflow = true;
                false
            }
        })
        .collect();
    *total_lines += formatted.len();
    formatted.iter().for_each(|row| body.push_str(row));
    overflow
}

fn execute_read_file_multi_symbols(
    index: &WorkspaceSearchIndex,
    req: &ReadFileRequest,
    output_limit: usize,
) -> Result<String, String> {
    if index.live_file_context(&req.path, 1, 0).is_empty() {
        let suggestions = suggest_nearby_paths(index, &req.path);
        let hint = if suggestions.is_empty() {
            "list workspace structure with (workspace-map \"\" depth: 2)".to_owned()
        } else {
            format!("did you mean: {}", suggestions.join(", "))
        };
        return Err(format!(
            "read-file: path `{}` not found in resident workspace index ({hint})",
            req.path
        ));
    }

    let mut out = String::new();
    let _ = writeln!(
        out,
        "generation: {}\nindexed_files: {}\nmode: read-file (L0 pure-Rust zero-I/O RAM multi-symbol reader)\npath: {}\nsymbols_requested: {}",
        index.generation_digest().as_str(),
        index.file_count(),
        req.path,
        req.symbols.len(),
    );
    if req.wrapped_bare_form {
        let _ = writeln!(out, "repairs: wrap-read-file-parens");
    }
    let companion_tests = discover_crate_companion_tests(index, &req.path);
    if !companion_tests.is_empty() {
        let _ = writeln!(out, "crate_tests: {}", companion_tests.join(", "));
    }

    let slice_budget = SymbolSliceBudget {
        symbol_context: req.symbol_context,
        body_budget: output_limit.saturating_sub(out.len() + 192),
    };
    let mut body = String::new();
    let mut total_lines = 0usize;
    let mut resolved_count = 0usize;
    let mut missing = Vec::new();
    let mut truncated = false;

    for sym in &req.symbols {
        match resolve_symbol_anchor_in_file(index, &req.path, sym) {
            Ok(anchor_line) => {
                resolved_count += 1;
                if append_symbol_slice(
                    index,
                    &req.path,
                    sym,
                    anchor_line,
                    &slice_budget,
                    &mut body,
                    &mut total_lines,
                ) {
                    truncated = true;
                    break;
                }
            }
            Err(_) => missing.push(sym.as_str()),
        }
    }
    let _ = writeln!(
        out,
        "hits: {total_lines}\nresolved_symbols: {resolved_count}/{}\nreturned: {total_lines}\ntruncated: {truncated}",
        req.symbols.len()
    );
    if !missing.is_empty() {
        let _ = writeln!(
            out,
            "missing_symbols: {} (inspect with (file-outline \"{}\" projection: \"callable-skeleton\"))",
            missing.join(", "),
            req.path
        );
    }
    out.push_str(&body);
    Ok(out)
}

fn collect_remaining_file_symbols(
    index: &WorkspaceSearchIndex,
    path: &str,
    after_line: usize,
) -> Vec<(usize, String, String)> {
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
    let mut syms: Vec<(usize, String, String)> = snapshot
        .entities
        .iter()
        .filter(|ent| ent.path == path && ent.line > after_line && ent.kind != "import")
        .map(|ent| {
            let kind = refine_coarse_entity_kind(index, path, ent.line, &ent.kind);
            (ent.line, kind.to_owned(), ent.name.clone())
        })
        .collect();
    syms.sort_unstable_by(|a, b| a.0.cmp(&b.0).then_with(|| a.2.cmp(&b.2)));
    syms.dedup_by(|a, b| a.0 == b.0 && a.2 == b.2);
    syms.truncate(8);
    syms
}

pub(super) fn execute_read_file(
    index: &WorkspaceSearchIndex,
    raw_query: &str,
    output_limit: usize,
) -> Result<String, String> {
    let req = parse_read_file_form(raw_query)?;
    if req.symbols.len() > 1 {
        return execute_read_file_multi_symbols(index, &req, output_limit);
    }
    let single_symbol = req.symbols.first().map(String::as_str);
    let (eff_start, eff_end, symbol_anchor_line, symbol_ast_end) = if let Some(sym) = single_symbol {
        match resolve_symbol_anchor_in_file(index, &req.path, sym) {
            Ok(anchor_line) => {
                let before = (req.symbol_context / 3).clamp(3, 8);
                let ast_end = find_symbol_definition_end_line(index, &req.path, anchor_line, 120);
                let fallback_after = req.symbol_context.saturating_mul(2).clamp(25, 160);
                let start = anchor_line.saturating_sub(before).max(1);
                let end = match ast_end {
                    Some(end_line) => end_line
                        .saturating_add(1)
                        .max(anchor_line.saturating_add(req.symbol_context.min(12)))
                        .min(anchor_line.saturating_add(120)),
                    None => anchor_line.saturating_add(fallback_after),
                };
                (start, end, Some(anchor_line), ast_end)
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
                (req.start_line, req.end_line, None, None)
            }
        }
    } else {
        (req.start_line, req.end_line, None, None)
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
    let actual_end = slice.last().map_or(eff_start, |line| line.line_number);
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
    if let (Some(sym), Some(anchor_line)) = (single_symbol, symbol_anchor_line) {
        if let Some(end_line) = symbol_ast_end {
            let _ = writeln!(
                out,
                "symbol_anchor: {sym} @ L{anchor_line}\nsymbol_span: L{anchor_line}..=L{end_line} (complete AST body)"
            );
        } else {
            let _ = writeln!(out, "symbol_anchor: {sym} @ L{anchor_line}");
        }
    }
    if req.requested_end_clamped {
        let _ = writeln!(out, "range_clamped: true (max {MAX_READ_LINES} lines per call)");
    }
    let remaining = if has_more {
        collect_remaining_file_symbols(index, &req.path, actual_end)
    } else {
        Vec::new()
    };
    if has_more {
        let next_start = actual_end.saturating_add(1);
        let _ = writeln!(out, "next_start: {next_start}");
        let _ = writeln!(
            out,
            "next_query: (read-file \"{}\" cursor: {next_start} max-lines: {DEFAULT_READ_LINES})",
            req.path
        );
        if !remaining.is_empty() {
            let jump_syms: Vec<String> = remaining
                .iter()
                .take(4)
                .map(|(_, _, name)| format!("\"{name}\""))
                .collect();
            let _ = writeln!(
                out,
                "next_symbol_query: (read-file \"{}\" symbols: ({}))",
                req.path,
                jump_syms.join(" ")
            );
        }
    }
    if req.wrapped_bare_form {
        let _ = writeln!(out, "repairs: wrap-read-file-parens");
    }
    let companion_tests = discover_crate_companion_tests(index, &req.path);
    if !companion_tests.is_empty() {
        let _ = writeln!(out, "crate_tests: {}", companion_tests.join(", "));
    }

    let body_budget = output_limit.saturating_sub(out.len() + 192);
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
        if !remaining.is_empty() {
            let preview: Vec<String> = remaining
                .iter()
                .map(|(line, kind, name)| format!("{name}@L{line}[{kind}]"))
                .collect();
            let jump_syms: Vec<String> = remaining
                .iter()
                .take(3)
                .map(|(_, _, name)| format!("\"{name}\""))
                .collect();
            let _ = writeln!(
                out,
                "remaining_symbols: {} (jump via (read-file \"{}\" symbols: ({})))",
                preview.join(", "),
                req.path,
                jump_syms.join(" ")
            );
        }
        let outline_tip = if single_symbol.is_none() && actual_end >= DEFAULT_READ_LINES {
            format!(
                " or inspect compactly with (file-outline \"{}\" projection: \"callable-skeleton\") / (read-file \"{}\" symbols: (...))",
                req.path, req.path
            )
        } else {
            String::new()
        };
        let _ = writeln!(
            out,
            "  ... (more lines available; continue with (read-file \"{}\" cursor: {next_start} max-lines: {DEFAULT_READ_LINES}){outline_tip})",
            req.path
        );
    }
    Ok(out)
}
