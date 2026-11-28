// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Grouped LLM-oriented renderers for high-signal Search DSL result shapes:
//!   - Grouped cross-file naming collisions (`find-collisions`)
//!   - Role- and scope-grouped cross-layer blast radius (`find-impact`)
//!   - Identifier stem splitting for zero-hit nearest-symbol recovery

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Write as _;

use agent_semantic_search::{WorkspaceEdbPrefilter, WorkspaceSearchIndex, WorkspaceSearchQuery};

use crate::SearchDslStructuredRow;

const IMPACT_ROLE_ORDER: [&str; 6] = ["def", "caller", "impl", "import", "ffi", "ref"];

type ImpactRoleSites<'a> = BTreeMap<&'a str, BTreeSet<(usize, &'a str)>>;
type ImpactFilesByRole<'a> = BTreeMap<&'a str, ImpactRoleSites<'a>>;
type CollisionSites<'a> = BTreeSet<(&'a str, usize)>;
type CollisionBucket<'a> = (Vec<&'a str>, HashMap<&'a str, CollisionSites<'a>>);

/// Split a PascalCase, camelCase, snake_case, or kebab-case identifier into
/// deduplicated lexical stems (`len >= 3`) for zero-hit symbol recovery.
pub(super) fn split_identifier_stems(raw: &str) -> Vec<String> {
    let mut stems = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut current = String::new();
    let mut prev_lower = false;

    let mut flush = |buf: &mut String| {
        if buf.len() >= 3 {
            let key = buf.to_ascii_lowercase();
            if seen.insert(key) {
                stems.push(std::mem::take(buf));
                return;
            }
        }
        buf.clear();
    };

    for ch in raw.chars() {
        if !ch.is_ascii_alphanumeric() {
            flush(&mut current);
            prev_lower = false;
            continue;
        }
        if ch.is_ascii_uppercase() && prev_lower && !current.is_empty() {
            flush(&mut current);
        }
        current.push(ch);
        prev_lower = ch.is_ascii_lowercase() || ch.is_ascii_digit();
    }
    flush(&mut current);
    stems
}

pub(super) fn is_collision_row_shape(rows: &[SearchDslStructuredRow]) -> bool {
    !rows.is_empty()
        && (rows.iter().all(|r| {
            r.cells.len() == 5
                && r.cells[2].parse::<usize>().is_ok()
                && r.cells[4].parse::<usize>().is_ok()
        }) || rows.iter().all(|r| {
            r.cells.len() == 6
                && r.cells[3].parse::<usize>().is_ok()
                && r.cells[5].parse::<usize>().is_ok()
        }))
}

fn is_boilerplate_collision_name(name: &str) -> bool {
    name.len() < 3
        || matches!(
            name,
            "new"
                | "default"
                | "fmt"
                | "from"
                | "into"
                | "drop"
                | "as_str"
                | "clone"
                | "eq"
                | "ne"
                | "cmp"
                | "partial_cmp"
                | "hash"
                | "deref"
                | "deref_mut"
                | "as_ref"
                | "borrow"
                | "len"
                | "is_empty"
                | "id"
                | "name"
                | "value"
                | "build"
                | "run"
                | "parse"
                | "next"
                | "try_from"
                | "try_into"
                | "display"
                | "to_string"
                | "main"
                | "init"
                | "get"
                | "set"
                | "serialize"
                | "deserialize"
                | "write"
                | "read"
                | "flush"
                | "poll"
                | "iter"
                | "into_iter"
                | "clear"
                | "push"
                | "pop"
                | "index"
                | "finish"
                | "with_capacity"
                | "as_bytes"
                | "as_slice"
                | "to_owned"
                | "to_vec"
                | "clone_from"
                | "is_none"
                | "is_some"
        )
}

fn fetch_compact_line_signature(
    index: Option<&WorkspaceSearchIndex>,
    path: &str,
    line: usize,
) -> Option<String> {
    let idx = index?;
    let row = idx
        .live_file_context(path, line, 0)
        .into_iter()
        .find(|r| r.line_number == line)?;
    let compact = row.text.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.is_empty() {
        return None;
    }
    if compact.len() > 78 {
        Some(format!("{}…", &compact[..compact.floor_char_boundary(77)]))
    } else {
        Some(compact)
    }
}

fn format_grouped_collision_entry(
    name: &str,
    sites: &BTreeSet<(&str, usize)>,
    index: Option<&WorkspaceSearchIndex>,
    attach_sigs: bool,
) -> String {
    let preview: Vec<String> = sites
        .iter()
        .take(6)
        .enumerate()
        .map(|(i, (path, line))| {
            if attach_sigs
                && i < 2
                && let Some(sig) = fetch_compact_line_signature(index, path, *line)
            {
                format!("{path}:L{line} [`{sig}`]")
            } else {
                format!("{path}:L{line}")
            }
        })
        .collect();
    let extra = sites
        .len()
        .checked_sub(6)
        .filter(|&n| n > 0)
        .map_or_else(String::new, |n| format!(", +{n} more"));
    format!(
        "- {name} [{} file(s)]: {}{extra}\n",
        sites.len(),
        preview.join(", ")
    )
}

fn render_collision_bucket(
    grouped: &HashMap<&str, CollisionSites<'_>>,
    mut order: Vec<&str>,
    index: Option<&WorkspaceSearchIndex>,
    body_budget: usize,
    body_bytes: &mut usize,
) -> Vec<String> {
    order.sort_by(|a, b| {
        let a_boiler = is_boilerplate_collision_name(a);
        let b_boiler = is_boilerplate_collision_name(b);
        let a_sites = grouped.get(a).map_or(0, BTreeSet::len);
        let b_sites = grouped.get(b).map_or(0, BTreeSet::len);
        a_boiler
            .cmp(&b_boiler)
            .then_with(|| b_sites.cmp(&a_sites))
            .then_with(|| a.cmp(b))
    });
    order
        .into_iter()
        .enumerate()
        .filter_map(|(rank, name)| {
            let sites = grouped.get(name)?;
            let attach_sigs = index.is_some() && rank < 6 && !is_boilerplate_collision_name(name);
            Some(format_grouped_collision_entry(name, sites, index, attach_sigs))
        })
        .take_while(|line| {
            if *body_bytes + line.len() <= body_budget {
                *body_bytes += line.len();
                true
            } else {
                false
            }
        })
        .collect()
}

pub(super) fn render_grouped_collision_lines(
    rows: &[SearchDslStructuredRow],
    index: Option<&WorkspaceSearchIndex>,
    body_budget: usize,
) -> (Vec<String>, usize) {
    if rows.first().is_some_and(|r| r.cells.len() == 6) {
        let mut kind_order: Vec<&str> = Vec::new();
        let mut by_kind: HashMap<&str, CollisionBucket<'_>> = HashMap::new();
        rows.iter().for_each(|row| {
            let kind = row.cells[0].as_str();
            let name = row.cells[1].as_str();
            let (sym_order, grouped) = by_kind.entry(kind).or_insert_with(|| {
                kind_order.push(kind);
                (Vec::new(), HashMap::new())
            });
            let entry = grouped.entry(name).or_insert_with(|| {
                sym_order.push(name);
                BTreeSet::new()
            });
            if let Ok(l1) = row.cells[3].parse::<usize>() {
                entry.insert((row.cells[2].as_str(), l1));
            }
            if let Ok(l2) = row.cells[5].parse::<usize>() {
                entry.insert((row.cells[4].as_str(), l2));
            }
        });
        let distinct_symbols: usize = by_kind.values().map(|(o, _)| o.len()).sum();
        let mut lines = Vec::new();
        let mut body_bytes = 0usize;
        for kind in kind_order {
            let Some((sym_order, grouped)) = by_kind.remove(kind) else {
                continue;
            };
            let hdr = format!("[KIND: {kind}] ({} colliding symbol(s)):\n", sym_order.len());
            if body_bytes + hdr.len() > body_budget {
                break;
            }
            body_bytes += hdr.len();
            lines.push(hdr);
            lines.extend(render_collision_bucket(
                &grouped,
                sym_order,
                index,
                body_budget,
                &mut body_bytes,
            ));
        }
        return (lines, distinct_symbols);
    }

    let mut order: Vec<&str> = Vec::new();
    let mut grouped: HashMap<&str, BTreeSet<(&str, usize)>> = HashMap::new();
    rows.iter().for_each(|row| {
        let name = row.cells[0].as_str();
        let entry = grouped.entry(name).or_insert_with(|| {
            order.push(name);
            BTreeSet::new()
        });
        if let Ok(l1) = row.cells[2].parse::<usize>() {
            entry.insert((row.cells[1].as_str(), l1));
        }
        if let Ok(l2) = row.cells[4].parse::<usize>() {
            entry.insert((row.cells[3].as_str(), l2));
        }
    });
    let distinct_symbols = order.len();
    let mut body_bytes = 0usize;
    let lines = render_collision_bucket(&grouped, order, index, body_budget, &mut body_bytes);
    (lines, distinct_symbols)
}

pub(super) fn is_impact_row_shape(rows: &[SearchDslStructuredRow]) -> bool {
    !rows.is_empty()
        && rows.iter().all(|r| {
            r.cells.len() == 5
                && IMPACT_ROLE_ORDER.contains(&r.cells[0].as_str())
                && matches!(r.cells[1].as_str(), "prod" | "test")
                && r.cells[4].parse::<usize>().is_ok()
        })
}

fn format_scope_role_counts(counts: &HashMap<&str, usize>) -> String {
    let total: usize = counts.values().sum();
    if total == 0 {
        return "0".to_owned();
    }
    let parts: Vec<String> = IMPACT_ROLE_ORDER
        .iter()
        .filter_map(|&role| counts.get(role).filter(|&&c| c > 0).map(|c| format!("{role}={c}")))
        .collect();
    format!("{total}: {}", parts.join(", "))
}

fn format_impact_file_entry(path: &str, by_role: &ImpactRoleSites<'_>) -> String {
    let role_segments: Vec<String> = IMPACT_ROLE_ORDER
        .iter()
        .filter_map(|&role| {
            let sites = by_role.get(role)?;
            if sites.is_empty() {
                return None;
            }
            let rendered: Vec<String> = sites
                .iter()
                .take(8)
                .map(|(line, sym)| {
                    if role == "ref" || sym.is_empty() {
                        format!("L{line}")
                    } else {
                        format!("{sym}@L{line}")
                    }
                })
                .collect();
            let extra = sites
                .len()
                .checked_sub(8)
                .filter(|&n| n > 0)
                .map_or_else(String::new, |n| format!(",+{n}"));
            Some(format!("{role}[{}{extra}]", rendered.join(",")))
        })
        .collect();
    format!("  - {path}: {}\n", role_segments.join("; "))
}

fn impact_layer_key(path: &str) -> String {
    let mut parts = path.split('/');
    match (parts.next(), parts.next()) {
        (Some(a), Some(b)) => format!("{a}/{b}"),
        (Some(a), None) => a.to_owned(),
        _ => path.to_owned(),
    }
}

#[derive(Default)]
struct DefinitionDelimState {
    brace_depth: i32,
    paren_depth: i32,
    saw_delim: bool,
}

fn advance_definition_delims(trimmed: &str, state: &mut DefinitionDelimState) -> bool {
    let mut in_str = false;
    let mut esc = false;
    for ch in trimmed.chars() {
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
                if state.brace_depth == 0 && trimmed.starts_with('(') {
                    state.saw_delim = true;
                }
            }
            ')' => state.paren_depth -= 1,
            _ => {}
        }
    }
    (state.saw_delim && state.brace_depth <= 0 && state.paren_depth <= 0)
        || (!state.saw_delim && trimmed.ends_with(';'))
}

fn extract_compact_definition_block(
    index: &WorkspaceSearchIndex,
    path: &str,
    anchor_line: usize,
    max_lines: usize,
) -> Option<String> {
    let start = anchor_line.saturating_sub(2).max(1);
    let window = index.live_file_context(path, anchor_line.saturating_add(max_lines / 2), max_lines);
    if window.is_empty() {
        return None;
    }
    let mut delim = DefinitionDelimState::default();
    let mut out = String::new();
    for (emitted, row) in window
        .iter()
        .filter(|r| r.line_number >= start)
        .enumerate()
    {
        if emitted >= max_lines {
            let _ = writeln!(out, "    ... (definition continues)");
            break;
        }
        let _ = writeln!(out, "    L{}: {}", row.line_number, row.text.trim_end());
        if row.line_number >= anchor_line && advance_definition_delims(row.text.trim(), &mut delim) {
            break;
        }
    }
    (!out.is_empty()).then_some(out)
}

fn collect_unique_def_sites<'a>(
    prod_files: &'a ImpactFilesByRole<'a>,
    test_files: &'a ImpactFilesByRole<'a>,
) -> Vec<(&'a str, usize, &'a str)> {
    let mut seen: BTreeSet<(&str, usize)> = BTreeSet::new();
    [prod_files, test_files]
        .into_iter()
        .flat_map(|files| files.iter())
        .flat_map(|(&path, by_role)| {
            by_role
                .get("def")
                .into_iter()
                .flat_map(move |sites| sites.iter().map(move |&(line, sym)| (path, line, sym)))
        })
        .filter(|&(path, line, _)| seen.insert((path, line)))
        .take(4)
        .collect()
}

fn append_impact_definition_snippets(
    index: Option<&WorkspaceSearchIndex>,
    prod_files: &ImpactFilesByRole<'_>,
    test_files: &ImpactFilesByRole<'_>,
    snippet_budget: usize,
    lines: &mut Vec<String>,
    body_bytes: &mut usize,
) {
    let Some(idx) = index else {
        return;
    };
    let defs = collect_unique_def_sites(prod_files, test_files);
    if defs.is_empty() {
        return;
    }
    let mut section = String::from("[DEFINITIONS] (inline RAM snippets for target def sites):\n");
    let entries: Vec<String> = defs
        .into_iter()
        .filter_map(|(path, line, sym)| {
            let block = extract_compact_definition_block(idx, path, line, 18)?;
            Some(format!("  - {sym} @ {path}:L{line}\n{block}"))
        })
        .collect();
    let mut emitted_any = false;
    for entry in entries {
        if section.len() + entry.len() > snippet_budget {
            break;
        }
        section.push_str(&entry);
        emitted_any = true;
    }
    if emitted_any {
        *body_bytes += section.len();
        lines.push(section);
    }
}

struct FfiBindingCandidate {
    scheme_fn: String,
    c_sym: String,
    ffi_path: String,
    ffi_bind_line: usize,
}

struct CrossFfiBoundaryHit {
    host_path: String,
    host_line: usize,
    scheme_symbol: String,
    c_symbol: String,
    ffi_path: String,
    ffi_bind_line: usize,
    ffi_def_line: usize,
}

fn parse_c_define_candidate(index: &WorkspaceSearchIndex, path: &str, line: usize) -> Option<FfiBindingCandidate> {
    let window = index.live_file_context(path, line.saturating_add(1), 2);
    let combined: String = window
        .iter()
        .filter(|l| l.line_number >= line && l.line_number <= line.saturating_add(3))
        .map(|l| l.text.trim())
        .collect::<Vec<_>>()
        .join(" ");
    let after_head = combined.split_once("(c-define (")?.1;
    let scheme_fn = after_head
        .split(|c: char| c.is_whitespace() || c == ')')
        .next()
        .filter(|s| !s.is_empty())?
        .to_owned();
    let after_first_q = after_head.split_once('"')?.1;
    let c_sym = after_first_q
        .split_once('"')
        .map(|(s, _)| s)
        .filter(|s| !s.is_empty())?
        .to_owned();
    Some(FfiBindingCandidate {
        scheme_fn,
        c_sym,
        ffi_path: path.to_owned(),
        ffi_bind_line: line,
    })
}

fn collect_ffi_binding_candidates(
    index: &WorkspaceSearchIndex,
) -> (Vec<FfiBindingCandidate>, HashMap<(String, String), usize>) {
    let ffi_tokens = [String::from("c-define")];
    let prefilter = WorkspaceEdbPrefilter {
        languages: &[],
        documents: &[],
        path_prefix: None,
        extension: None,
        required_tokens: &ffi_tokens,
        max_files: 64,
        max_facts: 8192,
    };
    let snapshot = index.project_prefiltered_edb(&prefilter);
    let def_lines: HashMap<(String, String), usize> = snapshot
        .entities
        .iter()
        .filter(|ent| ent.kind != "import")
        .map(|ent| ((ent.path.clone(), ent.name.clone()), ent.line))
        .collect();
    let mut seen: BTreeSet<(String, String)> = BTreeSet::new();
    let mut candidates: Vec<FfiBindingCandidate> = snapshot
        .edges
        .iter()
        .filter(|edge| edge.relation == "ffi_binds")
        .filter(|edge| seen.insert((edge.path.clone(), edge.dst.clone())))
        .map(|edge| FfiBindingCandidate {
            scheme_fn: edge.src.clone(),
            c_sym: edge.dst.clone(),
            ffi_path: edge.path.clone(),
            ffi_bind_line: edge.line,
        })
        .collect();
    let fallback_query = index.query(&WorkspaceSearchQuery {
        pattern: "(c-define (",
        languages: &[],
        documents: &[],
        path_prefix: None,
        extension: None,
        symbol: None,
        graph_target: None,
        max_hits: 32,
        context_radius: 0,
    });
    candidates.extend(
        fallback_query
            .hits
            .iter()
            .flat_map(|hit| {
                hit.line_matches
                    .iter()
                    .filter_map(|m| parse_c_define_candidate(index, &hit.path, m.line_number))
            })
            .filter(|cand| seen.insert((cand.ffi_path.clone(), cand.c_sym.clone()))),
    );
    (candidates, def_lines)
}

fn find_host_reference_in_prod_files(
    index: &WorkspaceSearchIndex,
    prod_files: &ImpactFilesByRole<'_>,
    needle: &str,
) -> Option<(String, usize)> {
    let query = index.query(&WorkspaceSearchQuery {
        pattern: needle,
        languages: &[],
        documents: &[],
        path_prefix: None,
        extension: None,
        symbol: None,
        graph_target: None,
        max_hits: 32,
        context_radius: 0,
    });
    query.hits.into_iter().find_map(|hit| {
        if !prod_files.contains_key(hit.path.as_str()) {
            return None;
        }
        let line = hit.line_matches.first().map_or(1, |m| m.line_number);
        Some((hit.path, line))
    })
}

fn resolve_scheme_def_line(
    index: &WorkspaceSearchIndex,
    def_lines: &HashMap<(String, String), usize>,
    ffi_path: &str,
    kebab_sym: &str,
    scheme_fn: &str,
    fallback_line: usize,
) -> (String, usize) {
    if let Some(&ln) = def_lines.get(&(ffi_path.to_owned(), kebab_sym.to_owned())) {
        return (kebab_sym.to_owned(), ln);
    }
    if let Some(&ln) = def_lines.get(&(ffi_path.to_owned(), scheme_fn.to_owned())) {
        return (scheme_fn.to_owned(), ln);
    }
    let def_pat = format!("(def ({kebab_sym}");
    let probe = index.query(&WorkspaceSearchQuery {
        pattern: &def_pat,
        languages: &[],
        documents: &[],
        path_prefix: Some(ffi_path),
        extension: None,
        symbol: None,
        graph_target: None,
        max_hits: 4,
        context_radius: 0,
    });
    let found = probe
        .hits
        .into_iter()
        .find(|h| h.path == ffi_path)
        .and_then(|h| h.line_matches.first().map(|m| m.line_number));
    match found {
        Some(ln) => (kebab_sym.to_owned(), ln),
        None => (kebab_sym.to_owned(), fallback_line),
    }
}

fn discover_cross_language_ffi_boundaries(
    index: Option<&WorkspaceSearchIndex>,
    prod_files: &ImpactFilesByRole<'_>,
) -> Vec<CrossFfiBoundaryHit> {
    let Some(idx) = index else {
        return Vec::new();
    };
    if prod_files.is_empty() {
        return Vec::new();
    }
    let (candidates, def_lines) = collect_ffi_binding_candidates(idx);
    let mut seen: BTreeSet<(String, String)> = BTreeSet::new();
    candidates
        .into_iter()
        .filter(|cand| !prod_files.contains_key(cand.ffi_path.as_str()))
        .filter_map(|cand| {
            let kebab = cand.c_sym.replace('_', "-");
            let host = find_host_reference_in_prod_files(idx, prod_files, &kebab)
                .or_else(|| find_host_reference_in_prod_files(idx, prod_files, &cand.c_sym))?;
            let (scheme_symbol, ffi_def_line) = resolve_scheme_def_line(
                idx,
                &def_lines,
                &cand.ffi_path,
                &kebab,
                &cand.scheme_fn,
                cand.ffi_bind_line,
            );
            if !seen.insert((cand.ffi_path.clone(), scheme_symbol.clone())) {
                return None;
            }
            Some(CrossFfiBoundaryHit {
                host_path: host.0,
                host_line: host.1,
                scheme_symbol,
                c_symbol: cand.c_sym,
                ffi_path: cand.ffi_path,
                ffi_bind_line: cand.ffi_bind_line,
                ffi_def_line,
            })
        })
        .take(4)
        .collect()
}

fn append_cross_language_ffi_section(
    index: Option<&WorkspaceSearchIndex>,
    ffi_hits: &[CrossFfiBoundaryHit],
    snippet_budget: usize,
    lines: &mut Vec<String>,
    body_bytes: &mut usize,
) {
    if ffi_hits.is_empty() {
        return;
    }
    let mut section =
        String::from("[CROSS-LANGUAGE FFI BOUNDARY] (auto-followed from affected bridge files):\n");
    let entries: Vec<String> = ffi_hits
        .iter()
        .map(|hit| {
            let mut row = format!(
                "  - {}:L{} -> {} (c-abi: {}) @ {}:L{} [def: L{}]\n",
                hit.host_path,
                hit.host_line,
                hit.scheme_symbol,
                hit.c_symbol,
                hit.ffi_path,
                hit.ffi_bind_line,
                hit.ffi_def_line,
            );
            if let Some(snippet) = index
                .and_then(|idx| extract_compact_definition_block(idx, &hit.ffi_path, hit.ffi_def_line, 18))
            {
                row.push_str(&snippet);
            }
            row
        })
        .collect();
    let mut emitted_any = false;
    for entry in entries {
        if section.len() + entry.len() > snippet_budget {
            break;
        }
        section.push_str(&entry);
        emitted_any = true;
    }
    if emitted_any {
        *body_bytes += section.len();
        lines.push(section);
    }
}

/// Group `find-impact` `(role, scope, symbol, path, line)` rows into a
/// production-first blast-radius summary, affected-crate/layer matrix, inline
/// target definition snippets, auto-followed cross-language FFI boundaries, and
/// per-file role lines.
pub(super) fn render_grouped_impact_lines(
    rows: &[SearchDslStructuredRow],
    index: Option<&WorkspaceSearchIndex>,
    body_budget: usize,
) -> (String, Vec<String>) {
    let mut prod_counts: HashMap<&str, usize> = HashMap::new();
    let mut test_counts: HashMap<&str, usize> = HashMap::new();
    let mut prod_files: ImpactFilesByRole<'_> = BTreeMap::new();
    let mut test_files: ImpactFilesByRole<'_> = BTreeMap::new();
    let mut prod_layers: BTreeMap<String, usize> = BTreeMap::new();

    rows.iter().for_each(|row| {
        let role = row.cells[0].as_str();
        let scope = row.cells[1].as_str();
        let sym = row.cells[2].as_str();
        let path = row.cells[3].as_str();
        let line = row.cells[4].parse::<usize>().unwrap_or(0);
        if scope == "prod" {
            *prod_counts.entry(role).or_insert(0) += 1;
            prod_files
                .entry(path)
                .or_default()
                .entry(role)
                .or_default()
                .insert((line, sym));
        } else {
            *test_counts.entry(role).or_insert(0) += 1;
            test_files
                .entry(path)
                .or_default()
                .entry(role)
                .or_default()
                .insert((line, sym));
        }
    });
    let ffi_hits = discover_cross_language_ffi_boundaries(index, &prod_files);
    for path in prod_files.keys() {
        *prod_layers.entry(impact_layer_key(path)).or_insert(0) += 1;
    }
    let mut ffi_layer_files: BTreeSet<&str> = BTreeSet::new();
    ffi_hits
        .iter()
        .filter(|hit| ffi_layer_files.insert(hit.ffi_path.as_str()))
        .for_each(|hit| {
            *prod_layers.entry(impact_layer_key(&hit.ffi_path)).or_insert(0) += 1;
        });
    let layer_rollup = if prod_layers.is_empty() {
        String::new()
    } else {
        let parts: Vec<String> = prod_layers
            .iter()
            .map(|(layer, file_cnt)| format!("{layer}({file_cnt})"))
            .collect();
        format!("\naffected_layers: {}", parts.join(", "))
    };

    let summary = format!(
        "prod({}) | test({}){layer_rollup}",
        format_scope_role_counts(&prod_counts),
        format_scope_role_counts(&test_counts),
    );

    let mut lines = Vec::new();
    let mut body_bytes = 0usize;
    let snippet_budget = (body_budget / 3).min(2_800);
    append_impact_definition_snippets(
        index,
        &prod_files,
        &test_files,
        snippet_budget,
        &mut lines,
        &mut body_bytes,
    );
    append_cross_language_ffi_section(
        index,
        &ffi_hits,
        snippet_budget,
        &mut lines,
        &mut body_bytes,
    );
    let sections = [
        ("PROD", prod_counts.values().sum::<usize>(), prod_files),
        ("TEST", test_counts.values().sum::<usize>(), test_files),
    ];
    for (label, total, files) in sections {
        if files.is_empty() {
            continue;
        }
        let mut hdr = String::new();
        let _ = writeln!(hdr, "[{label}] ({total} hit(s) across {} file(s)):", files.len());
        if body_bytes + hdr.len() > body_budget {
            break;
        }
        body_bytes += hdr.len();
        lines.push(hdr);
        for (path, by_role) in &files {
            let line = format_impact_file_entry(path, by_role);
            if body_bytes + line.len() > body_budget {
                break;
            }
            body_bytes += line.len();
            lines.push(line);
        }
    }
    (summary, lines)
}

#[cfg(test)]
#[path = "../../tests/unit/render_collisions.rs"]
mod render_collisions_tests;
