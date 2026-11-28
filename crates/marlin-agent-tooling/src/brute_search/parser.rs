// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! S-expression tokenizer, form classifier, and argument parsers for L0
//! pure-Rust RAM search forms (`brute-search`, `file-outline`, `read-file`).

use std::collections::BTreeSet;

pub(crate) const DEFAULT_BRUTE_MAX_HITS: usize = 32;
const MAX_BRUTE_HITS: usize = 128;
const MAX_CONTEXT_LINES: usize = 8;
const DEFAULT_OUTLINE_MAX_SYMBOLS: usize = 80;
pub(crate) const MAX_OUTLINE_SYMBOLS: usize = 240;
pub(crate) const DEFAULT_READ_LINES: usize = 120;
pub(crate) const MAX_READ_LINES: usize = 360;

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct BruteSearchRequest {
    pub(crate) pattern: String,
    pub(crate) languages: Vec<String>,
    pub(crate) documents: Vec<String>,
    pub(crate) path_prefix: Option<String>,
    pub(crate) exclude_path_prefixes: Vec<String>,
    pub(crate) exclude_tests: bool,
    pub(crate) extension: Option<String>,
    pub(crate) symbol: Option<String>,
    pub(crate) graph_target: Option<String>,
    pub(crate) max_hits: usize,
    pub(crate) context_lines: Option<usize>,
    pub(crate) wrapped_bare_form: bool,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct FileOutlineRequest {
    pub(crate) path_prefix: Option<String>,
    pub(crate) languages: Vec<String>,
    pub(crate) kinds: Vec<String>,
    pub(crate) name_contains: Vec<String>,
    pub(crate) name_all: Vec<String>,
    pub(crate) exclude_path_prefixes: Vec<String>,
    pub(crate) exclude_tests: bool,
    pub(crate) max_symbols: usize,
    pub(crate) max_per_file: Option<usize>,
    pub(crate) skeleton: bool,
    pub(crate) kind_summary: bool,
    pub(crate) type_detail: bool,
    pub(crate) wrapped_bare_form: bool,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct WorkspaceMapRequest {
    pub(crate) path_prefix: Option<String>,
    pub(crate) languages: Vec<String>,
    pub(crate) exclude_path_prefixes: Vec<String>,
    pub(crate) exclude_tests: bool,
    pub(crate) depth: usize,
    pub(crate) max_dirs: usize,
    pub(crate) wrapped_bare_form: bool,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct ReadFileRequest {
    pub(crate) path: String,
    pub(crate) start_line: usize,
    pub(crate) end_line: usize,
    pub(crate) requested_end_clamped: bool,
    pub(crate) symbols: Vec<String>,
    pub(crate) symbol_context: usize,
    pub(crate) signature_only: bool,
    pub(crate) wrapped_bare_form: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum L0FormKind {
    BruteSearch,
    FileOutline,
    ReadFile,
    WorkspaceMap,
}

fn normalize_scope_prefix(raw: &str) -> Option<String> {
    let mut normalized = raw.trim().replace('\\', "/");
    while let Some(stripped) = normalized.strip_prefix("./") {
        normalized = stripped.to_owned();
    }
    let trimmed = normalized.trim_start_matches('/');
    if trimmed.is_empty() || trimmed == "." {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

pub(crate) fn classify_l0_form(raw_query: &str) -> Option<L0FormKind> {
    let trimmed = strip_markdown_fence(raw_query);
    let head = trimmed
        .strip_prefix('(')
        .map(str::trim_start)
        .unwrap_or(trimmed);
    let matches_head = |kw: &str| {
        head.strip_prefix(kw).is_some_and(|after| {
            after.is_empty()
                || after.starts_with(char::is_whitespace)
                || after.starts_with(')')
                || after.starts_with('"')
        })
    };
    if matches_head("brute-search")
        || matches_head("workspace_search")
        || matches_head("workspace-search")
    {
        Some(L0FormKind::BruteSearch)
    } else if matches_head("file-outline") || matches_head("outline") {
        Some(L0FormKind::FileOutline)
    } else if matches_head("read-file-range") || matches_head("read-file") {
        Some(L0FormKind::ReadFile)
    } else if matches_head("workspace-map") || matches_head("dir-map") || matches_head("repo-map") {
        Some(L0FormKind::WorkspaceMap)
    } else {
        None
    }
}

/// Returns `true` if `raw_query` targets any L0 pure-Rust RAM fast-path form.
pub(crate) fn is_l0_search_form(raw_query: &str) -> bool {
    classify_l0_form(raw_query).is_some()
}

fn strip_markdown_fence(raw: &str) -> &str {
    let trimmed = raw.trim();
    if let Some(after_open) = trimmed.strip_prefix("```") {
        let body = after_open
            .split_once('\n')
            .map_or(after_open, |(_, rest)| rest);
        return body.trim().trim_end_matches("```").trim();
    }
    trimmed
}

#[derive(Debug, Eq, PartialEq)]
enum SexpAtom {
    Keyword(String),
    Text(String),
    ListStart,
    ListEnd,
}

fn read_quoted_string(
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
) -> Result<String, String> {
    chars.next();
    let mut buf = String::new();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(escaped) = chars.next() {
                match escaped {
                    'n' => buf.push('\n'),
                    't' => buf.push('\t'),
                    'r' => buf.push('\r'),
                    '"' => buf.push('"'),
                    '\\' => buf.push('\\'),
                    other => {
                        buf.push('\\');
                        buf.push(other);
                    }
                }
            } else {
                buf.push('\\');
            }
            continue;
        }
        if c == '"' {
            return Ok(buf);
        }
        buf.push(c);
    }
    Err("unterminated string literal in L0 search form".to_owned())
}

fn read_bare_atom(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> Option<SexpAtom> {
    let mut tok = String::new();
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() || c == '(' || c == ')' {
            break;
        }
        tok.push(c);
        chars.next();
    }
    if tok.is_empty() {
        return None;
    }
    if let Some(kw) = tok.strip_suffix(':').or_else(|| tok.strip_prefix(':')) {
        return Some(SexpAtom::Keyword(kw.to_ascii_lowercase()));
    }
    Some(SexpAtom::Text(tok))
}

fn tokenize_l0_form(raw: &str) -> Result<(Vec<SexpAtom>, bool), String> {
    let trimmed = strip_markdown_fence(raw);
    let (inner, wrapped_bare) = if let Some(after_open) = trimmed.strip_prefix('(') {
        let stripped = after_open
            .strip_suffix(')')
            .ok_or_else(|| "L0 search form must be enclosed in matching parentheses".to_owned())?;
        (stripped.trim(), false)
    } else if is_l0_search_form(trimmed) {
        (trimmed, true)
    } else {
        return Err("L0 search form must be enclosed in parentheses".to_owned());
    };
    let mut atoms = Vec::new();
    let mut chars = inner.chars().peekable();
    while let Some(&ch) = chars.peek() {
        if ch.is_whitespace() {
            chars.next();
            continue;
        }
        if ch == '(' {
            chars.next();
            atoms.push(SexpAtom::ListStart);
            continue;
        }
        if ch == ')' {
            chars.next();
            atoms.push(SexpAtom::ListEnd);
            continue;
        }
        if ch == '"' {
            atoms.push(SexpAtom::Text(read_quoted_string(&mut chars)?));
            continue;
        }
        if let Some(atom) = read_bare_atom(&mut chars) {
            atoms.push(atom);
        }
    }
    Ok((atoms, wrapped_bare))
}

fn consume_string_or_list(
    atoms: &[SexpAtom],
    idx: usize,
    kw: &str,
) -> Result<(Vec<String>, usize), String> {
    match atoms.get(idx + 1) {
        Some(SexpAtom::Text(val)) => Ok((vec![val.clone()], idx + 2)),
        Some(SexpAtom::ListStart) => {
            let mut vals = Vec::new();
            let mut cursor = idx + 2;
            while cursor < atoms.len() {
                match &atoms[cursor] {
                    SexpAtom::ListEnd => return Ok((vals, cursor + 1)),
                    SexpAtom::Text(val) => {
                        if !val.is_empty() {
                            vals.push(val.clone());
                        }
                        cursor += 1;
                    }
                    _ => return Err(format!("expected string items inside `{kw}: (...)` list")),
                }
            }
            Err(format!("unclosed list for `{kw}:`"))
        }
        _ => Err(format!("missing value for `{kw}:`")),
    }
}

fn push_unique_nonempty(dest: &mut Vec<String>, items: Vec<String>) {
    let mut seen: BTreeSet<String> = dest.iter().cloned().collect();
    for val in items {
        if !val.is_empty() && seen.insert(val.clone()) {
            dest.push(val);
        }
    }
}

fn parse_bool_atom(raw: &str) -> Result<bool, String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "#t" | "true" | "yes" | "1" => Ok(true),
        "#f" | "false" | "no" | "0" => Ok(false),
        other => Err(format!("invalid boolean `{other}`; expected #t or #f")),
    }
}

pub(crate) fn is_test_path(path: &str) -> bool {
    let normalized = path.replace('\\', "/").to_ascii_lowercase();
    normalized.starts_with("tests/")
        || normalized.contains("/tests/")
        || normalized.contains("/t/")
        || normalized.contains("_test.")
        || normalized.contains("-test.")
        || normalized.contains("test_")
        || normalized.contains("_mock.")
        || normalized.contains("mock")
        || normalized.contains("fixture")
}

pub(crate) fn is_path_excluded(path: &str, excludes: &[String], exclude_tests: bool) -> bool {
    if exclude_tests && is_test_path(path) {
        return true;
    }
    if excludes.is_empty() {
        return false;
    }
    let normalized = path.replace('\\', "/").to_ascii_lowercase();
    excludes.iter().any(|raw| {
        let ex = raw.trim().replace('\\', "/").to_ascii_lowercase();
        if ex.is_empty() {
            return false;
        }
        let ex_trimmed = ex.trim_start_matches("./");
        let slash_segment = format!("/{ex_trimmed}");
        normalized.starts_with(ex_trimmed)
            || normalized.contains(&slash_segment)
            || (!ex_trimmed.ends_with('/') && normalized.contains(&format!("/{ex_trimmed}/")))
    })
}

pub(crate) fn parse_brute_search_form(raw_query: &str) -> Result<BruteSearchRequest, String> {
    let (atoms, wrapped_bare_form) = tokenize_l0_form(raw_query)?;
    let Some(SexpAtom::Text(head)) = atoms.first() else {
        return Err("expected (brute-search \"pattern\" ...)".to_owned());
    };
    let repaired_wrapper_head = matches!(head.as_str(), "workspace_search" | "workspace-search");
    if head != "brute-search" && !repaired_wrapper_head {
        return Err("expected (brute-search \"pattern\" ...)".to_owned());
    }
    let mut req = BruteSearchRequest {
        pattern: String::new(),
        languages: Vec::new(),
        documents: Vec::new(),
        path_prefix: None,
        exclude_path_prefixes: Vec::new(),
        exclude_tests: false,
        extension: None,
        symbol: None,
        graph_target: None,
        max_hits: DEFAULT_BRUTE_MAX_HITS,
        context_lines: None,
        wrapped_bare_form: wrapped_bare_form || repaired_wrapper_head,
    };
    let mut idx = 1usize;
    while idx < atoms.len() {
        match &atoms[idx] {
            SexpAtom::Keyword(kw) => match kw.as_str() {
                "lang" | "language" => {
                    let (vals, next) = consume_string_or_list(&atoms, idx, kw)?;
                    let concrete: Vec<String> = vals
                        .into_iter()
                        .filter(|v| {
                            !matches!(v.trim().to_ascii_lowercase().as_str(), "any" | "all" | "*")
                        })
                        .collect();
                    push_unique_nonempty(&mut req.languages, concrete);
                    idx = next;
                }
                "doc" | "document" => {
                    let (vals, next) = consume_string_or_list(&atoms, idx, kw)?;
                    push_unique_nonempty(&mut req.documents, vals);
                    idx = next;
                }
                "exclude-path-prefix" | "exclude-prefix" | "exclude" | "not-path" => {
                    let (vals, next) = consume_string_or_list(&atoms, idx, kw)?;
                    push_unique_nonempty(&mut req.exclude_path_prefixes, vals);
                    idx = next;
                }
                _ => {
                    let Some(SexpAtom::Text(val)) = atoms.get(idx + 1) else {
                        return Err(format!("missing value for `{kw}:` in (brute-search ...)"));
                    };
                    match kw.as_str() {
                        "path-prefix" | "prefix" | "scope" | "path" | "file" => {
                            if let Some(norm) = normalize_scope_prefix(val) {
                                req.path_prefix = Some(norm);
                            }
                        }
                        "ext" | "extension" => {
                            if !val.is_empty() {
                                req.extension = Some(val.trim_start_matches('.').to_owned());
                            }
                        }
                        "sym" | "symbol" => {
                            if !val.is_empty() {
                                req.symbol = Some(val.clone());
                            }
                        }
                        "graph" | "graph-target" => {
                            if !val.is_empty() {
                                req.graph_target = Some(val.clone());
                            }
                        }
                        "limit" | "max-hits" => {
                            let parsed = val
                                .parse::<usize>()
                                .map_err(|_| format!("invalid integer `{val}` for `limit:`"))?;
                            req.max_hits = parsed.clamp(1, MAX_BRUTE_HITS);
                        }
                        "context" | "ctx" => {
                            let parsed = val
                                .parse::<usize>()
                                .map_err(|_| format!("invalid integer `{val}` for `context:`"))?;
                            req.context_lines = Some(parsed.min(MAX_CONTEXT_LINES));
                        }
                        "exclude-tests?" | "exclude-tests" | "prod-only?" | "prod-only" => {
                            req.exclude_tests = parse_bool_atom(val)?;
                        }
                        other => {
                            return Err(format!(
                                "unknown keyword `{other}:` in (brute-search ...); allowed: path-prefix:, exclude-tests?:, exclude-path-prefix:, scope:, lang:, doc:, ext:, sym:, graph:, context:, limit:"
                            ));
                        }
                    }
                    idx += 2;
                }
            },
            SexpAtom::Text(text) => {
                if req.pattern.is_empty() {
                    req.pattern = text.clone();
                } else {
                    req.pattern.push(' ');
                    req.pattern.push_str(text);
                }
                idx += 1;
            }
            SexpAtom::ListStart | SexpAtom::ListEnd => idx += 1,
        }
    }
    if req.pattern.is_empty()
        && req.languages.is_empty()
        && req.documents.is_empty()
        && req.path_prefix.is_none()
        && req.extension.is_none()
        && req.symbol.is_none()
        && req.graph_target.is_none()
    {
        return Err(
            "(brute-search ...) requires a non-empty pattern or filter (path-prefix:, lang:, sym:, graph:)"
                .to_owned(),
        );
    }
    Ok(req)
}

pub(crate) fn parse_file_outline_form(raw_query: &str) -> Result<FileOutlineRequest, String> {
    let (atoms, wrapped_bare_form) = tokenize_l0_form(raw_query)?;
    let mut req = FileOutlineRequest {
        path_prefix: None,
        languages: Vec::new(),
        kinds: Vec::new(),
        name_contains: Vec::new(),
        name_all: Vec::new(),
        exclude_path_prefixes: Vec::new(),
        exclude_tests: false,
        max_symbols: DEFAULT_OUTLINE_MAX_SYMBOLS,
        max_per_file: None,
        skeleton: false,
        kind_summary: false,
        type_detail: false,
        wrapped_bare_form,
    };
    let mut idx = 1usize;
    while idx < atoms.len() {
        match &atoms[idx] {
            SexpAtom::Keyword(kw) => match kw.as_str() {
                "lang" | "language" => {
                    let (vals, next) = consume_string_or_list(&atoms, idx, kw)?;
                    let concrete: Vec<String> = vals
                        .into_iter()
                        .filter(|v| {
                            !matches!(v.trim().to_ascii_lowercase().as_str(), "any" | "all" | "*")
                        })
                        .collect();
                    push_unique_nonempty(&mut req.languages, concrete);
                    idx = next;
                }
                "kind" | "kinds" => {
                    let (vals, next) = consume_string_or_list(&atoms, idx, kw)?;
                    let expanded: Vec<String> = vals
                        .into_iter()
                        .flat_map(|v| {
                            v.split([',', '|'])
                                .map(|part| part.trim().to_ascii_lowercase())
                                .filter(|part| !part.is_empty())
                                .collect::<Vec<_>>()
                        })
                        .collect();
                    push_unique_nonempty(&mut req.kinds, expanded);
                    idx = next;
                }
                "exclude-path-prefix" | "exclude-prefix" | "exclude" | "not-path" => {
                    let (vals, next) = consume_string_or_list(&atoms, idx, kw)?;
                    push_unique_nonempty(&mut req.exclude_path_prefixes, vals);
                    idx = next;
                }
                "path-prefix" | "prefix" | "scope" | "path" | "file" => {
                    let Some(SexpAtom::Text(val)) = atoms.get(idx + 1) else {
                        return Err(format!("missing value for `{kw}:` in (file-outline ...)"));
                    };
                    if let Some(norm) = normalize_scope_prefix(val) {
                        req.path_prefix = Some(norm);
                    }
                    idx += 2;
                }
                "limit" | "max-symbols" => {
                    let Some(SexpAtom::Text(val)) = atoms.get(idx + 1) else {
                        return Err(format!("missing value for `{kw}:` in (file-outline ...)"));
                    };
                    let parsed = val
                        .parse::<usize>()
                        .map_err(|_| format!("invalid integer `{val}` for `limit:`"))?;
                    req.max_symbols = parsed.clamp(1, MAX_OUTLINE_SYMBOLS);
                    idx += 2;
                }
                "max-per-file" | "per-file" => {
                    let Some(SexpAtom::Text(val)) = atoms.get(idx + 1) else {
                        return Err(format!("missing value for `{kw}:` in (file-outline ...)"));
                    };
                    let parsed = val
                        .parse::<usize>()
                        .map_err(|_| format!("invalid integer `{val}` for `{kw}:`"))?;
                    req.max_per_file = Some(parsed.clamp(1, MAX_OUTLINE_SYMBOLS));
                    idx += 2;
                }
                "exclude-tests?" | "exclude-tests" | "prod-only?" | "prod-only" => {
                    let Some(SexpAtom::Text(val)) = atoms.get(idx + 1) else {
                        return Err(format!("missing value for `{kw}:` in (file-outline ...)"));
                    };
                    req.exclude_tests = parse_bool_atom(val)?;
                    idx += 2;
                }
                "projection" | "view" => {
                    let Some(SexpAtom::Text(val)) = atoms.get(idx + 1) else {
                        return Err(format!("missing value for `{kw}:` in (file-outline ...)"));
                    };
                    let lc = val.trim().to_ascii_lowercase();
                    req.skeleton = matches!(
                        lc.as_str(),
                        "callable-skeleton" | "skeleton" | "signatures" | "signature"
                    );
                    req.kind_summary = matches!(
                        lc.as_str(),
                        "kind-summary" | "summary" | "inventory" | "kinds"
                    );
                    req.type_detail = matches!(
                        lc.as_str(),
                        "struct-enum-detail" | "type-detail" | "detail" | "fields"
                    );
                    idx += 2;
                }
                "name-contains" | "name-any" | "name" | "symbol" | "filter" => {
                    let (vals, next) = consume_string_or_list(&atoms, idx, kw)?;
                    push_unique_nonempty(&mut req.name_contains, vals);
                    idx = next;
                }
                "name-all" | "all-of" => {
                    let (vals, next) = consume_string_or_list(&atoms, idx, kw)?;
                    push_unique_nonempty(&mut req.name_all, vals);
                    idx = next;
                }
                "skeleton?" | "skeleton" => {
                    let Some(SexpAtom::Text(val)) = atoms.get(idx + 1) else {
                        return Err(format!("missing value for `{kw}:` in (file-outline ...)"));
                    };
                    req.skeleton = parse_bool_atom(val)?;
                    idx += 2;
                }
                other => {
                    return Err(format!(
                        "unknown keyword `{other}:` in (file-outline ...); allowed: path-prefix:, exclude-tests?:, exclude-path-prefix:, lang:, kind:, name-contains:, name-all:, projection:, skeleton?:, limit:, max-per-file:"
                    ));
                }
            },
            SexpAtom::Text(text) => {
                if req.path_prefix.is_none()
                    && let Some(norm) = normalize_scope_prefix(text)
                {
                    req.path_prefix = Some(norm);
                }
                idx += 1;
            }
            SexpAtom::ListStart | SexpAtom::ListEnd => idx += 1,
        }
    }
    if req.type_detail && req.kinds.is_empty() {
        req.kinds = vec!["struct".to_owned(), "enum".to_owned()];
    }
    Ok(req)
}

pub(crate) fn parse_workspace_map_form(raw_query: &str) -> Result<WorkspaceMapRequest, String> {
    let (atoms, wrapped_bare_form) = tokenize_l0_form(raw_query)?;
    let mut req = WorkspaceMapRequest {
        path_prefix: None,
        languages: Vec::new(),
        exclude_path_prefixes: Vec::new(),
        exclude_tests: true,
        depth: 2,
        max_dirs: 32,
        wrapped_bare_form,
    };
    let mut idx = 1usize;
    while idx < atoms.len() {
        match &atoms[idx] {
            SexpAtom::Keyword(kw) => match kw.as_str() {
                "lang" | "language" => {
                    let (vals, next) = consume_string_or_list(&atoms, idx, kw)?;
                    let concrete: Vec<String> = vals
                        .into_iter()
                        .filter(|v| {
                            !matches!(v.trim().to_ascii_lowercase().as_str(), "any" | "all" | "*")
                        })
                        .collect();
                    push_unique_nonempty(&mut req.languages, concrete);
                    idx = next;
                }
                "exclude-path-prefix" | "exclude-prefix" | "exclude" | "not-path" => {
                    let (vals, next) = consume_string_or_list(&atoms, idx, kw)?;
                    push_unique_nonempty(&mut req.exclude_path_prefixes, vals);
                    idx = next;
                }
                "path-prefix" | "prefix" | "scope" | "path" => {
                    let Some(SexpAtom::Text(val)) = atoms.get(idx + 1) else {
                        return Err(format!("missing value for `{kw}:` in (workspace-map ...)"));
                    };
                    if let Some(norm) = normalize_scope_prefix(val) {
                        req.path_prefix = Some(norm);
                    }
                    idx += 2;
                }
                "depth" => {
                    let Some(SexpAtom::Text(val)) = atoms.get(idx + 1) else {
                        return Err("missing value for `depth:` in (workspace-map ...)".to_owned());
                    };
                    let parsed = val
                        .parse::<usize>()
                        .map_err(|_| format!("invalid integer `{val}` for `depth:`"))?;
                    req.depth = parsed.clamp(1, 4);
                    idx += 2;
                }
                "limit" | "max-dirs" => {
                    let Some(SexpAtom::Text(val)) = atoms.get(idx + 1) else {
                        return Err(format!("missing value for `{kw}:` in (workspace-map ...)"));
                    };
                    let parsed = val
                        .parse::<usize>()
                        .map_err(|_| format!("invalid integer `{val}` for `{kw}:`"))?;
                    req.max_dirs = parsed.clamp(1, 128);
                    idx += 2;
                }
                "exclude-tests?" | "exclude-tests" | "prod-only?" | "prod-only" => {
                    let Some(SexpAtom::Text(val)) = atoms.get(idx + 1) else {
                        return Err(format!("missing value for `{kw}:` in (workspace-map ...)"));
                    };
                    req.exclude_tests = parse_bool_atom(val)?;
                    idx += 2;
                }
                other => {
                    return Err(format!(
                        "unknown keyword `{other}:` in (workspace-map ...); allowed: path-prefix:, lang:, exclude-tests?:, exclude-path-prefix:, depth:, limit:"
                    ));
                }
            },
            SexpAtom::Text(text) => {
                if req.path_prefix.is_none()
                    && let Some(norm) = normalize_scope_prefix(text)
                {
                    req.path_prefix = Some(norm);
                }
                idx += 1;
            }
            SexpAtom::ListStart | SexpAtom::ListEnd => idx += 1,
        }
    }
    Ok(req)
}

pub(crate) fn parse_read_file_form(raw_query: &str) -> Result<ReadFileRequest, String> {
    let (atoms, wrapped_bare_form) = tokenize_l0_form(raw_query)?;
    let mut path = String::new();
    let mut start_line: Option<usize> = None;
    let mut end_line: Option<usize> = None;
    let mut line_count: Option<usize> = None;
    let mut around_line: Option<usize> = None;
    let mut around_context: Option<usize> = None;
    let mut symbols: Vec<String> = Vec::new();
    let mut signature_only = false;
    let mut idx = 1usize;
    while idx < atoms.len() {
        match &atoms[idx] {
            SexpAtom::Keyword(kw) => match kw.as_str() {
                "symbol" | "symbols" | "sym" | "def" => {
                    let (vals, next) = consume_string_or_list(&atoms, idx, kw)?;
                    push_unique_nonempty(&mut symbols, vals);
                    idx = next;
                }
                _ => {
                    let Some(SexpAtom::Text(val)) = atoms.get(idx + 1) else {
                        return Err(format!("missing value for `{kw}:` in (read-file ...)"));
                    };
                    match kw.as_str() {
                        "path" | "file" => path = val.clone(),
                        "start" | "start-line" | "from" | "line" | "cursor" | "offset" => {
                            let parsed = val
                                .parse::<usize>()
                                .map_err(|_| format!("invalid integer `{val}` for `{kw}:`"))?;
                            start_line = Some(parsed.max(1));
                        }
                        "end" | "end-line" | "to" => {
                            let parsed = val
                                .parse::<usize>()
                                .map_err(|_| format!("invalid integer `{val}` for `{kw}:`"))?;
                            end_line = Some(parsed.max(1));
                        }
                        "around" | "around-line" | "focus" => {
                            let parsed = val
                                .parse::<usize>()
                                .map_err(|_| format!("invalid integer `{val}` for `{kw}:`"))?;
                            around_line = Some(parsed.max(1));
                        }
                        "context" | "ctx" | "radius" => {
                            let parsed = val
                                .parse::<usize>()
                                .map_err(|_| format!("invalid integer `{val}` for `{kw}:`"))?;
                            around_context = Some(parsed.min(MAX_READ_LINES / 2));
                        }
                        "lines" | "max-lines" | "limit" | "count" => {
                            let parsed = val
                                .parse::<usize>()
                                .map_err(|_| format!("invalid integer `{val}` for `{kw}:`"))?;
                            line_count = Some(parsed.max(1));
                        }
                        "signature-only?" | "signature-only" | "signatures-only?"
                        | "signatures-only" | "signatures?" | "signatures" => {
                            signature_only = parse_bool_atom(val)?;
                        }
                        other => {
                            return Err(format!(
                                "unknown keyword `{other}:` in (read-file ...); allowed: path:, symbol:, symbols:, signature-only?:, start:, cursor:, end:, around:, context:, max-lines:, lines:"
                            ));
                        }
                    }
                    idx += 2;
                }
            },
            SexpAtom::Text(text) => {
                if path.is_empty() {
                    path = text.clone();
                } else if start_line.is_none()
                    && let Ok(n) = text.parse::<usize>()
                {
                    start_line = Some(n.max(1));
                } else if end_line.is_none()
                    && let Ok(n) = text.parse::<usize>()
                {
                    end_line = Some(n.max(1));
                }
                idx += 1;
            }
            SexpAtom::ListStart | SexpAtom::ListEnd => idx += 1,
        }
    }
    if path.is_empty() {
        return Err(
            "(read-file ...) requires a workspace-relative path, e.g. (read-file \"crates/marlin-agent-kernel/src/governance_cedar.rs\" symbol: \"CompiledCedarPolicy\" context: 25)"
                .to_owned(),
        );
    }
    let (start, requested_end) = if let Some(focus) = around_line {
        let radius = around_context.unwrap_or(25);
        (
            focus.saturating_sub(radius).max(1),
            focus.saturating_add(radius),
        )
    } else {
        let start = start_line.unwrap_or(1);
        let req_end = match (end_line, line_count) {
            (Some(end), _) => end.max(start),
            (None, Some(count)) => start.saturating_add(count.saturating_sub(1)),
            (None, None) => start.saturating_add(DEFAULT_READ_LINES - 1),
        };
        (start, req_end)
    };
    let max_end = start.saturating_add(MAX_READ_LINES - 1);
    Ok(ReadFileRequest {
        path: path.replace('\\', "/"),
        start_line: start,
        end_line: requested_end.min(max_end),
        requested_end_clamped: requested_end > max_end,
        symbols,
        symbol_context: around_context.unwrap_or(30),
        signature_only,
        wrapped_bare_form,
    })
}
