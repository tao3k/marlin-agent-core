// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Grouped LLM-oriented renderers for high-signal Search DSL result shapes:
//!   - Grouped cross-file naming collisions (`find-collisions`)
//!   - Role- and scope-grouped cross-layer blast radius (`find-impact`)
//!   - Identifier stem splitting for zero-hit nearest-symbol recovery

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Write as _;

use crate::SearchDslStructuredRow;

const IMPACT_ROLE_ORDER: [&str; 6] = ["def", "caller", "impl", "import", "ffi", "ref"];

type ImpactRoleSites<'a> = BTreeMap<&'a str, BTreeSet<(usize, &'a str)>>;
type ImpactFilesByRole<'a> = BTreeMap<&'a str, ImpactRoleSites<'a>>;

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
        && rows.iter().all(|r| {
            r.cells.len() == 5
                && r.cells[2].parse::<usize>().is_ok()
                && r.cells[4].parse::<usize>().is_ok()
        })
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
        )
}

fn format_grouped_collision_entry(name: &str, sites: &BTreeSet<(&str, usize)>) -> String {
    let preview: Vec<String> = sites
        .iter()
        .take(6)
        .map(|(path, line)| format!("{path}:L{line}"))
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

pub(super) fn render_grouped_collision_lines(
    rows: &[SearchDslStructuredRow],
    body_budget: usize,
) -> (Vec<String>, usize) {
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
    let mut body_bytes = 0usize;
    let lines: Vec<String> = order
        .into_iter()
        .filter_map(|name| grouped.get(name).map(|sites| format_grouped_collision_entry(name, sites)))
        .take_while(|line| {
            if body_bytes + line.len() <= body_budget {
                body_bytes += line.len();
                true
            } else {
                false
            }
        })
        .collect();
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

/// Group `find-impact` `(role, scope, symbol, path, line)` rows into a
/// production-first blast-radius summary, affected-crate/layer matrix, and per-file role lines.
pub(super) fn render_grouped_impact_lines(
    rows: &[SearchDslStructuredRow],
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
    for path in prod_files.keys() {
        *prod_layers.entry(impact_layer_key(path)).or_insert(0) += 1;
    }
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
