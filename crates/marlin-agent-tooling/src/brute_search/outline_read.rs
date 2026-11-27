// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! L0 pure-Rust zero-I/O RAM `(file-outline ...)` executor.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use marlin_agent_search_engine::{
    WorkspaceEdbPrefilter, WorkspaceSearchIndex, WorkspaceSearchQuery,
};

use super::parser::{is_path_excluded, parse_file_outline_form};

struct OutlineFileEntry<'a> {
    language: &'a str,
    symbols: Vec<(usize, &'a str, &'a str)>,
}

pub(super) fn refine_coarse_entity_kind<'a>(
    index: &WorkspaceSearchIndex,
    path: &str,
    line: usize,
    kind: &'a str,
) -> &'a str {
    if !kind.eq_ignore_ascii_case("type") {
        return kind;
    }
    let Some(row) = index
        .live_file_context(path, line, 0)
        .into_iter()
        .find(|r| r.line_number == line)
    else {
        return kind;
    };
    let trimmed = row.text.trim_start();
    if trimmed.contains("enum ") {
        "enum"
    } else if trimmed.contains("struct ") {
        "struct"
    } else if trimmed.contains("trait ") || trimmed.contains("interface ") {
        "trait"
    } else {
        kind
    }
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

fn format_kind_summary_file_row(path: &str, entry: &OutlineFileEntry<'_>) -> String {
    let mut by_kind: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    entry.symbols.iter().for_each(|(_, kind, name)| {
        let names = by_kind.entry(*kind).or_default();
        if !names.contains(name) {
            names.push(*name);
        }
    });
    let parts: Vec<String> = by_kind
        .into_iter()
        .map(|(kind, names)| {
            let preview: Vec<&str> = names.iter().copied().take(8).collect();
            if names.len() > 8 {
                format!("{kind}({}): {}, +{}", names.len(), preview.join(", "), names.len() - 8)
            } else {
                format!("{kind}({}): {}", names.len(), preview.join(", "))
            }
        })
        .collect();
    format!(
        "  - @{path} [{} · {} sym] {}\n",
        entry.language,
        entry.symbols.len(),
        parts.join(" | ")
    )
}

fn top_two_segments(path: &str) -> String {
    let mut parts = path.split('/');
    match (parts.next(), parts.next()) {
        (Some(a), Some(b)) => format!("{a}/{b}"),
        (Some(a), None) => a.to_owned(),
        _ => path.to_owned(),
    }
}

pub(super) fn suggest_nearby_paths(index: &WorkspaceSearchIndex, query_path: &str) -> Vec<String> {
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

#[derive(Default)]
struct DelimScanState {
    brace_depth: i32,
    paren_depth: i32,
    saw_delim: bool,
}

fn scan_line_delimiters(line: &str, state: &mut DelimScanState) {
    let mut in_str = false;
    let mut esc = false;
    for ch in line.chars() {
        if esc {
            esc = false;
            continue;
        }
        if ch == '\\' && in_str {
            esc = true;
            continue;
        }
        if ch == '"' {
            in_str = !in_str;
            continue;
        }
        if in_str {
            continue;
        }
        match ch {
            '{' => {
                state.brace_depth += 1;
                state.saw_delim = true;
            }
            '}' => state.brace_depth -= 1,
            '(' => {
                state.paren_depth += 1;
                if state.brace_depth == 0 && line.starts_with('(') {
                    state.saw_delim = true;
                }
            }
            ')' => state.paren_depth -= 1,
            _ => {}
        }
    }
}

fn format_skeleton_signature(
    index: &WorkspaceSearchIndex,
    path: &str,
    line: usize,
) -> (Option<usize>, Option<String>) {
    let window = index.live_file_context(path, line.saturating_add(40), 42);
    if window.is_empty() {
        return (None, None);
    }
    let mut sig = String::new();
    let mut sig_lines = 0usize;
    let mut delim = DelimScanState::default();
    let mut end_line: Option<usize> = None;

    for row in window.iter().filter(|r| r.line_number >= line) {
        let trimmed = row.text.trim();
        if sig_lines < 3 && sig.len() < 140 {
            if !sig.is_empty() {
                sig.push(' ');
            }
            sig.push_str(trimmed);
            sig_lines += 1;
            if trimmed.contains('{') || trimmed.ends_with(';') || trimmed.ends_with(':') {
                sig_lines = 3;
            }
        }
        scan_line_delimiters(trimmed, &mut delim);
        if (delim.saw_delim && delim.brace_depth <= 0 && delim.paren_depth <= 0)
            || (!delim.saw_delim && trimmed.ends_with(';'))
        {
            end_line = Some(row.line_number);
            break;
        }
    }
    let clean_sig = (!sig.is_empty()).then(|| {
        let collapsed = sig.split_whitespace().collect::<Vec<_>>().join(" ");
        if collapsed.len() > 140 {
            format!("{}…", &collapsed[..collapsed.floor_char_boundary(139)])
        } else {
            collapsed
        }
    });
    (end_line, clean_sig)
}

fn extract_type_member_preview(
    index: &WorkspaceSearchIndex,
    path: &str,
    line: usize,
) -> (Option<usize>, Option<String>) {
    let window = index.live_file_context(path, line.saturating_add(24), 26);
    if window.is_empty() {
        return (None, None);
    }
    let mut delim = DelimScanState::default();
    let mut end_line: Option<usize> = None;
    let mut members: Vec<String> = Vec::new();
    let mut total_members = 0usize;

    for row in window.iter().filter(|r| r.line_number >= line) {
        let trimmed = row.text.trim();
        let in_body = delim.brace_depth == 1
            && !trimmed.is_empty()
            && !trimmed.starts_with("//")
            && !trimmed.starts_with('#')
            && !trimmed.starts_with('}');
        if in_body {
            total_members += 1;
            if members.len() < 6 {
                let compact = trimmed.trim_end_matches([',', '{']).trim();
                let clipped = if compact.len() > 42 {
                    format!("{}…", &compact[..compact.floor_char_boundary(41)])
                } else {
                    compact.to_owned()
                };
                if !clipped.is_empty() {
                    members.push(clipped);
                }
            }
        }
        scan_line_delimiters(trimmed, &mut delim);
        if (delim.saw_delim && delim.brace_depth <= 0 && delim.paren_depth <= 0)
            || (!delim.saw_delim && trimmed.ends_with(';'))
        {
            end_line = Some(row.line_number);
            break;
        }
    }
    let preview = if members.is_empty() {
        format_skeleton_signature(index, path, line).1
    } else {
        let suffix = total_members
            .checked_sub(members.len())
            .filter(|&n| n > 0)
            .map_or_else(String::new, |n| format!(", +{n} more"));
        Some(format!("{{ {}{suffix} }}", members.join(", ")))
    };
    (end_line, preview)
}

fn format_outline_symbol_row(
    index: &WorkspaceSearchIndex,
    path: &str,
    line: usize,
    kind: &str,
    name: &str,
    cfg: &OutlineRenderConfig,
) -> String {
    if cfg.use_type_detail && matches!(kind, "struct" | "enum" | "trait" | "type") {
        let (end_line, preview) = extract_type_member_preview(index, path, line);
        let span_label = match end_line {
            Some(end) if end > line => format!("L{line}..L{end}"),
            _ => format!("L{line}"),
        };
        return match preview {
            Some(p) => format!("      {span_label:<10} [{kind}] {name} :: {p}\n"),
            None => format!("      {span_label:<10} [{kind}] {name}\n"),
        };
    }
    if !cfg.use_skeleton && !cfg.use_type_detail {
        return format!("      L{line:<4} [{kind}] {name}\n");
    }
    let (end_line, sig) = format_skeleton_signature(index, path, line);
    let span_label = match end_line {
        Some(end) if end > line => format!("L{line}..L{end}"),
        _ => format!("L{line}"),
    };
    match sig {
        Some(s) => format!("      {span_label:<10} [{kind}] {name} :: {s}\n"),
        None => format!("      {span_label:<10} [{kind}] {name}\n"),
    }
}

struct OutlineRenderConfig {
    max_symbols: usize,
    per_file_cap: Option<usize>,
    use_skeleton: bool,
    use_type_detail: bool,
    body_budget: usize,
}

fn append_outline_file_block(
    index: &WorkspaceSearchIndex,
    path: &str,
    mut entry: OutlineFileEntry<'_>,
    cfg: &OutlineRenderConfig,
    emitted_symbols: &mut usize,
    body: &mut String,
) -> bool {
    let lang = entry.language;
    entry
        .symbols
        .sort_unstable_by_key(|(line, _, name)| (*line, *name));
    entry
        .symbols
        .dedup_by(|a, b| a.0 == b.0 && a.1 == b.1 && a.2 == b.2);
    let file_total = entry.symbols.len();
    let file_hdr = format!("  - @{path}  [{lang} · {file_total} symbol(s)]\n");
    if body.len() + file_hdr.len() > cfg.body_budget {
        return true;
    }
    body.push_str(&file_hdr);
    let remaining_global = cfg.max_symbols.saturating_sub(*emitted_symbols);
    let allowed = cfg
        .per_file_cap
        .map_or(remaining_global, |cap| cap.min(remaining_global));
    let mut cur_len = body.len();
    let mut budget_hit = false;
    let rows: Vec<String> = entry
        .symbols
        .into_iter()
        .take(allowed)
        .map(|(line, kind, name)| {
            format_outline_symbol_row(index, path, line, kind, name, cfg)
        })
        .take_while(|row| {
            if cur_len + row.len() <= cfg.body_budget {
                cur_len += row.len();
                true
            } else {
                budget_hit = true;
                false
            }
        })
        .collect();
    let emitted_in_file = rows.len();
    rows.iter().for_each(|row| body.push_str(row));
    *emitted_symbols += emitted_in_file;
    if emitted_in_file < file_total {
        let note = format!(
            "      ... (+{} more in {path}; call (file-outline \"{path}\") for full file)\n",
            file_total - emitted_in_file
        );
        if body.len() + note.len() <= cfg.body_budget {
            body.push_str(&note);
        }
    }
    budget_hit || *emitted_symbols >= cfg.max_symbols
}

fn outline_name_matches(name: &str, name_contains: &[String], name_all: &[String]) -> bool {
    if name_contains.is_empty() && name_all.is_empty() {
        return true;
    }
    let lc = name.to_ascii_lowercase();
    let any_ok = name_contains.is_empty()
        || name_contains
            .iter()
            .any(|needle| lc.contains(&needle.to_ascii_lowercase()));
    let all_ok = name_all
        .iter()
        .all(|needle| lc.contains(&needle.to_ascii_lowercase()));
    any_ok && all_ok
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
        max_facts: 131_072,
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
        let refined_kind = refine_coarse_entity_kind(index, &ent.path, ent.line, &ent.kind);
        if by_file.contains_key(ent.path.as_str()) {
            *scoped_kind_counts.entry(refined_kind).or_insert(0) += 1;
            if ent.source != "resident-index" {
                native_ast_file_ids.insert(ent.file_id);
            }
        }
        if !outline_kind_matches(refined_kind, &ent.language_kind, &req.kinds)
            || !outline_name_matches(&ent.name, &req.name_contains, &req.name_all)
        {
            continue;
        }
        if let Some(entry) = by_file.get_mut(ent.path.as_str()) {
            entry
                .symbols
                .push((ent.line, refined_kind, ent.name.as_str()));
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

    if req.kind_summary {
        let mut ranked_kinds: Vec<(&str, usize)> = scoped_kind_counts.into_iter().collect();
        ranked_kinds.sort_unstable_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
        let kind_totals: Vec<String> = ranked_kinds
            .iter()
            .take(8)
            .map(|(k, c)| format!("{k}={c}"))
            .collect();
        let body_budget = output_limit.min(12_288).saturating_sub(out.len() + 256);
        let mut used = 0usize;
        let rows: Vec<String> = by_file
            .iter()
            .map(|(path, entry)| format_kind_summary_file_row(path, entry))
            .take_while(|row| {
                if used + row.len() <= body_budget {
                    used += row.len();
                    true
                } else {
                    false
                }
            })
            .collect();
        let returned_files = rows.len();
        let truncated = returned_files < total_files;
        let _ = writeln!(
            out,
            "projection: kind-summary\nkind_totals: {}\nreturned: {returned_files}\ntruncated: {truncated}",
            if kind_totals.is_empty() {
                "none".to_owned()
            } else {
                kind_totals.join(", ")
            }
        );
        rows.iter().for_each(|r| out.push_str(r));
        if truncated {
            let _ = writeln!(
                out,
                "  ... (+{} more file(s); narrow path-prefix: or call (file-outline \"<path>\" projection: \"callable-skeleton\"))",
                total_files.saturating_sub(returned_files)
            );
        }
        return Ok(out);
    }

    let per_file_cap = req.max_per_file.or_else(|| {
        (total_files > 1 && total_symbols > req.max_symbols)
            .then(|| (req.max_symbols / total_files).max(6))
    });
    let render_cfg = OutlineRenderConfig {
        max_symbols: req.max_symbols,
        per_file_cap,
        use_skeleton: req.skeleton || total_files == 1,
        use_type_detail: req.type_detail,
        body_budget: output_limit.saturating_sub(out.len() + 220),
    };
    let mut emitted_symbols = 0usize;
    let mut body = String::new();
    let mut omitted_dirs: BTreeSet<String> = BTreeSet::new();
    let mut stopped = false;
    for (path, entry) in by_file {
        if stopped {
            omitted_dirs.insert(top_two_segments(path));
            continue;
        }
        if append_outline_file_block(
            index,
            path,
            entry,
            &render_cfg,
            &mut emitted_symbols,
            &mut body,
        ) {
            if emitted_symbols == 0 || body.len() >= render_cfg.body_budget {
                omitted_dirs.insert(top_two_segments(path));
            }
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
