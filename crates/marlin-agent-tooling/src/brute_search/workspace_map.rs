// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! L0 pure-Rust zero-I/O RAM `(workspace-map ...)` topology bird's-eye executor.
//!
//! Gives LLMs a compact (<2.5KB, <1ms) crate/package/directory overview on Turn 1
//! with file counts, languages, native AST hydration coverage, entry files, and
//! representative type/module symbols — eliminating blind 2000-symbol `file-outline`
//! dumps and forbidden shell `ls` attempts.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use marlin_agent_search_engine::{WorkspaceEdbPrefilter, WorkspaceSearchIndex};

use super::parser::{is_path_excluded, parse_workspace_map_form};

struct DirBucket<'a> {
    files: usize,
    native_ast_files: BTreeSet<u32>,
    languages: BTreeMap<&'a str, usize>,
    entry_files: BTreeSet<&'a str>,
    symbol_count: usize,
    highlight_symbols: Vec<&'a str>,
    seen_highlights: BTreeSet<&'a str>,
}

impl<'a> DirBucket<'a> {
    fn new() -> Self {
        Self {
            files: 0,
            native_ast_files: BTreeSet::new(),
            languages: BTreeMap::new(),
            entry_files: BTreeSet::new(),
            symbol_count: 0,
            highlight_symbols: Vec::new(),
            seen_highlights: BTreeSet::new(),
        }
    }
}

fn bucket_key_at_depth(path: &str, depth: usize) -> String {
    let segs: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if segs.len() <= 1 {
        return ".".to_owned();
    }
    // Always exclude the filename itself when bucketing into directories.
    let dir_segs = &segs[..segs.len() - 1];
    let take = depth.clamp(1, 4).min(dir_segs.len());
    dir_segs[..take].join("/")
}

fn is_entry_or_manifest_file(path: &str) -> bool {
    let base = path.rsplit('/').next().unwrap_or(path);
    matches!(
        base,
        "Cargo.toml"
            | "lib.rs"
            | "main.rs"
            | "mod.rs"
            | "init.ss"
            | "interface.ss"
            | "build.ss"
            | "gerbil.pkg"
            | "package.json"
            | "pyproject.toml"
            | "Project.toml"
            | "index.ts"
            | "__init__.py"
    )
}

fn is_highlight_kind(kind: &str) -> bool {
    matches!(
        kind,
        "struct" | "enum" | "trait" | "interface" | "class" | "type" | "module" | "item"
    )
}

fn format_bucket_row(dir: &str, bucket: &DirBucket<'_>) -> String {
    let mut lang_pairs: Vec<(&str, usize)> =
        bucket.languages.iter().map(|(k, v)| (*k, *v)).collect();
    lang_pairs.sort_unstable_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
    let langs_str = lang_pairs
        .iter()
        .take(3)
        .map(|(l, c)| format!("{l}:{c}"))
        .collect::<Vec<_>>()
        .join(",");
    let pct = bucket
        .native_ast_files
        .len()
        .saturating_mul(100)
        .checked_div(bucket.files)
        .unwrap_or(0);
    let mut row = format!(
        "  - {dir}/  [files={} · {langs_str} · symbols={} · ast={pct}%]",
        bucket.files, bucket.symbol_count
    );
    if !bucket.entry_files.is_empty() {
        let entries: Vec<&str> = bucket.entry_files.iter().copied().collect();
        let _ = write!(row, "  entries=[{}]", entries.join(", "));
    }
    if !bucket.highlight_symbols.is_empty() {
        let _ = write!(row, "  key=[{}]", bucket.highlight_symbols.join(", "));
    }
    row.push('\n');
    row
}

fn insert_file_into_buckets<'a>(
    buckets: &mut BTreeMap<String, DirBucket<'a>>,
    file_to_bucket: &mut BTreeMap<u32, String>,
    depth: usize,
    file: &'a marlin_agent_search_engine::WorkspaceEdbFileFact,
) {
    let key = bucket_key_at_depth(&file.path, depth);
    file_to_bucket.insert(file.file_id, key.clone());
    let bucket = buckets.entry(key).or_insert_with(DirBucket::new);
    bucket.files += 1;
    let lang = file.language.as_deref().unwrap_or("text");
    *bucket.languages.entry(lang).or_insert(0) += 1;
    if is_entry_or_manifest_file(&file.path) && bucket.entry_files.len() < 4 {
        let short = file.path.rsplit('/').next().unwrap_or(file.path.as_str());
        bucket.entry_files.insert(short);
    }
}

fn record_entity_in_bucket<'a>(
    buckets: &mut BTreeMap<String, DirBucket<'a>>,
    file_to_bucket: &BTreeMap<u32, String>,
    total_native_ast: &mut BTreeSet<u32>,
    ent: &'a marlin_agent_search_engine::WorkspaceEdbEntityFact,
) {
    let Some(bucket) = file_to_bucket
        .get(&ent.file_id)
        .and_then(|key| buckets.get_mut(key))
    else {
        return;
    };
    bucket.symbol_count += 1;
    if ent.source != "resident-index" {
        bucket.native_ast_files.insert(ent.file_id);
        total_native_ast.insert(ent.file_id);
    }
    if is_highlight_kind(&ent.kind)
        && bucket.highlight_symbols.len() < 5
        && !ent.name.is_empty()
        && ent.name.len() <= 48
        && bucket.seen_highlights.insert(ent.name.as_str())
    {
        bucket.highlight_symbols.push(ent.name.as_str());
    }
}

fn effective_map_depth(req: &super::parser::WorkspaceMapRequest) -> usize {
    let prefix_segs = req
        .path_prefix
        .as_deref()
        .map_or(0, |p| p.split('/').filter(|s| !s.is_empty()).count());
    if prefix_segs > 0 && req.depth <= prefix_segs {
        (prefix_segs + 1).min(4)
    } else {
        req.depth
    }
}

fn collect_workspace_buckets<'a>(
    snapshot: &'a marlin_agent_search_engine::WorkspaceEdbSnapshot,
    req: &super::parser::WorkspaceMapRequest,
) -> (Vec<(String, DirBucket<'a>)>, usize, usize) {
    let mut buckets: BTreeMap<String, DirBucket<'a>> = BTreeMap::new();
    let mut file_to_bucket: BTreeMap<u32, String> = BTreeMap::new();
    let mut total_native_ast: BTreeSet<u32> = BTreeSet::new();
    let eff_depth = effective_map_depth(req);

    snapshot
        .files
        .iter()
        .filter(|f| !is_path_excluded(&f.path, &req.exclude_path_prefixes, req.exclude_tests))
        .for_each(|file| {
            insert_file_into_buckets(&mut buckets, &mut file_to_bucket, eff_depth, file);
        });
    let total_files = file_to_bucket.len();

    snapshot
        .entities
        .iter()
        .filter(|e| e.kind != "import")
        .for_each(|ent| {
            record_entity_in_bucket(&mut buckets, &file_to_bucket, &mut total_native_ast, ent);
        });

    let mut ranked: Vec<(String, DirBucket<'a>)> = buckets.into_iter().collect();
    ranked.sort_unstable_by(|a, b| {
        b.1.files
            .cmp(&a.1.files)
            .then_with(|| b.1.symbol_count.cmp(&a.1.symbol_count))
            .then_with(|| a.0.cmp(&b.0))
    });
    (ranked, total_files, total_native_ast.len())
}

pub(super) fn execute_workspace_map(
    index: &WorkspaceSearchIndex,
    raw_query: &str,
    output_limit: usize,
) -> Result<String, String> {
    let req = parse_workspace_map_form(raw_query)?;
    const MAX_WORKSPACE_MAP_FACTS: usize = 1_000_000;
    let prefilter = WorkspaceEdbPrefilter {
        languages: &req.languages,
        documents: &[],
        path_prefix: req.path_prefix.as_deref(),
        extension: None,
        required_tokens: &[],
        max_files: 16_384,
        max_facts: MAX_WORKSPACE_MAP_FACTS,
    };
    let snapshot = index.project_prefiltered_edb(&prefilter);
    let facts_projected = snapshot.entities.len().saturating_add(snapshot.edges.len());
    let facts_truncated = facts_projected >= MAX_WORKSPACE_MAP_FACTS;
    let (ranked, total_files, native_ast_count) = collect_workspace_buckets(&snapshot, &req);

    let total_dirs = ranked.len();
    let total_hydrated_pct = native_ast_count
        .saturating_mul(100)
        .checked_div(total_files)
        .unwrap_or(0);

    let mut out = String::new();
    let _ = writeln!(
        out,
        "generation: {}\nelapsed_us: {}\nindexed_files: {}\nmode: workspace-map (L0 pure-Rust zero-I/O topology bird's-eye)\nfiles: {total_files}\ndirs: {total_dirs}\nnative_ast_files: {native_ast_count}/{total_files} [{total_hydrated_pct}%]\nfacts_truncated: {facts_truncated}",
        snapshot.generation_digest.as_str(),
        snapshot.elapsed_micros,
        snapshot.indexed_files,
    );
    if req.wrapped_bare_form {
        let _ = writeln!(out, "repairs: wrap-workspace-map-parens");
    }

    let body_budget = output_limit.min(8_192).saturating_sub(out.len() + 256);
    let mut used = 0usize;
    let rows: Vec<String> = ranked
        .iter()
        .take(req.max_dirs)
        .map(|(dir, bucket)| format_bucket_row(dir, bucket))
        .take_while(|row| {
            if used + row.len() <= body_budget {
                used += row.len();
                true
            } else {
                false
            }
        })
        .collect();
    let returned = rows.len();
    let truncated = returned < total_dirs;
    let _ = writeln!(out, "returned: {returned}\ntruncated: {truncated}");
    if total_dirs == 0 {
        let scope = req.path_prefix.as_deref().unwrap_or("");
        let _ = writeln!(
            out,
            "diagnostic: workspace-map scope \"{scope}\" matched 0 indexed files; retry with (workspace-map \"\" depth: 2)"
        );
        return Ok(out);
    }
    rows.iter().for_each(|row| out.push_str(row));
    if let Some((top_dir, _)) = ranked.first() {
        let _ = writeln!(
            out,
            "next_query: (file-outline \"{top_dir}/\" projection: \"callable-skeleton\" exclude-tests?: #t limit: 40) or (find-symbols name-all: (\"...\") path-prefix: \"{top_dir}/\")"
        );
    }
    Ok(out)
}
