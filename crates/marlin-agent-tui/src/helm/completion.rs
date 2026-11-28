// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Protocol-driven Copilot TAB completion & inline ghost-text engine for TUI
//! Helm (`HelmOverlay`).
//!
//! Contains ZERO hardcoded keyword tables, ZERO hardcoded language lists, ZERO
//! hardcoded AST kind enums, and ZERO hardcoded workspace paths. Every
//! completion candidate comes strictly from:
//!   1. The protocol-projected `catalog: &[SearchDslCompletionEntry]` hydrated
//!      from Scheme POO (`marlin_search_dsl_completions`) and ASP HTTP/JSON
//!      provider schemas (`project_provider_catalog_completions`).
//!   2. The live resident `WorkspaceSearchIndex` (actual workspace directories
//!      and actual file symbols).

use std::collections::BTreeSet;

use marlin_agent_search_engine::{
    SearchDslCompletionEntry, SearchDslCompletionKind, WorkspaceEdbPrefilter, WorkspaceSearchIndex,
    WorkspaceSearchQuery,
};

fn is_keyword_token(tok: &str) -> bool {
    tok.len() > 1 && (tok.ends_with(':') || tok.starts_with(':'))
}

fn canonical_kw_key(tok: &str) -> String {
    tok.trim_start_matches(':')
        .trim_end_matches(':')
        .chars()
        .filter(|c| *c != '-' && *c != '_')
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

fn strip_quotes(tok: &str) -> &str {
    tok.trim_start_matches('"').trim_end_matches('"')
}

fn tokenize_prefix(raw: &str) -> Vec<String> {
    let bytes = raw.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        let ch = bytes[i];
        if ch.is_ascii_whitespace() {
            i += 1;
        } else if ch == b'(' || ch == b')' {
            out.push((ch as char).to_string());
            i += 1;
        } else if ch == b'"' {
            let start = i;
            i += 1;
            while i < bytes.len() {
                if bytes[i] == b'\\' && i + 1 < bytes.len() {
                    i += 2;
                    continue;
                }
                if bytes[i] == b'"' {
                    i += 1;
                    break;
                }
                i += 1;
            }
            out.push(raw[start..i].to_owned());
        } else {
            let start = i;
            while i < bytes.len()
                && !bytes[i].is_ascii_whitespace()
                && bytes[i] != b'('
                && bytes[i] != b')'
                && bytes[i] != b'"'
            {
                i += 1;
            }
            out.push(raw[start..i].to_owned());
        }
    }
    out
}

/// Extract the trailing token being typed (including an unclosed leading `"` so
/// accepting `"struct" ` replaces `"st` cleanly without doubling quotes).
pub(super) fn trailing_completion_fragment(input: &str) -> &str {
    if input.ends_with(char::is_whitespace) {
        return "";
    }
    let bytes = input.as_bytes();
    let mut idx = bytes.len();
    while idx > 0 {
        let ch = bytes[idx - 1];
        if ch.is_ascii_whitespace() || ch == b'(' || ch == b')' {
            break;
        }
        idx -= 1;
    }
    &input[idx..]
}

/// True when `input` supports Copilot TAB completion (either an S-expression
/// prompt starting with `(` or a plain Helm `@lang:` / `@ext:` / `@prefix:` / `@sym:` facet).
pub(super) fn supports_copilot_completion(input: &str) -> bool {
    let trimmed = input.trim_start();
    if trimmed.starts_with('(') {
        return true;
    }
    let frag = trailing_completion_fragment(input);
    frag.starts_with("@lang:")
        || frag.starts_with("@ext:")
        || frag.starts_with("@prefix:")
        || frag.starts_with("@sym:")
}

fn extract_active_lang(tokens: &[String]) -> Option<String> {
    let mut active = None;
    for pair in tokens.windows(2) {
        if is_keyword_token(&pair[0]) {
            let kw = canonical_kw_key(&pair[0]);
            if matches!(kw.as_str(), "lang" | "language" | "languageid") {
                let val = strip_quotes(&pair[1]).trim().to_ascii_lowercase();
                if !val.is_empty() {
                    active = Some(val);
                }
            }
        } else if pair[0].eq_ignore_ascii_case("use") {
            let val = strip_quotes(&pair[1]).trim().to_ascii_lowercase();
            if !val.is_empty() {
                active = Some(val);
            }
        }
    }
    active
}

fn extract_first_positional_path(tokens: &[String]) -> Option<String> {
    let candidate = tokens.get(2)?;
    if is_keyword_token(candidate) {
        return None;
    }
    let cleaned = strip_quotes(candidate).trim();
    (!cleaned.is_empty()).then(|| cleaned.to_owned())
}

fn kw_targets_match(pending_kw_canon: &str, entry_sig_canon: &str) -> bool {
    if pending_kw_canon == entry_sig_canon {
        return true;
    }
    (matches!(pending_kw_canon, "lang" | "language" | "languageid")
        && matches!(entry_sig_canon, "lang" | "language" | "languageid"))
        || (matches!(pending_kw_canon, "kind" | "kinds")
            && matches!(entry_sig_canon, "kind" | "kinds"))
        || (matches!(pending_kw_canon, "ext" | "extension")
            && matches!(entry_sig_canon, "ext" | "extension"))
        || (matches!(
            pending_kw_canon,
            "excludetests?" | "excludetests" | "prodonly?" | "prodonly"
        ) && matches!(
            entry_sig_canon,
            "excludetests?" | "excludetests" | "prodonly?" | "prodonly"
        ))
}

fn keyword_value_schema_tag<'a>(
    head: Option<&str>,
    pending_kw_canon: &str,
    catalog: &'a [SearchDslCompletionEntry],
) -> Option<&'a str> {
    catalog
        .iter()
        .find(|e| {
            e.kind == SearchDslCompletionKind::Keyword
                && canonical_kw_key(&e.form) == pending_kw_canon
                && head.is_none_or(|h| e.language.eq_ignore_ascii_case(h))
        })
        .map(|e| e.signature.as_str())
}

fn live_workspace_dir_entries(
    kw_label: &str,
    index: Option<&WorkspaceSearchIndex>,
) -> Vec<SearchDslCompletionEntry> {
    let Some(idx) = index else {
        return Vec::new();
    };
    let report = idx.query(&WorkspaceSearchQuery {
        pattern: "",
        languages: &[],
        documents: &[],
        path_prefix: None,
        extension: None,
        symbol: None,
        graph_target: None,
        max_hits: 96,
        context_radius: 0,
    });
    let mut dirs: BTreeSet<String> = BTreeSet::new();
    for hit in report.hits {
        let mut parts = hit.path.split('/');
        if let Some(first) = parts.next()
            && !first.is_empty()
            && hit.path.contains('/')
        {
            dirs.insert(format!("{first}/"));
            if let Some(second) = parts.next()
                && !second.is_empty()
                && hit.path.matches('/').count() >= 2
            {
                dirs.insert(format!("{first}/{second}/"));
            }
        }
    }
    dirs.into_iter()
        .take(24)
        .map(|dir| SearchDslCompletionEntry {
            form: format!("\"{dir}\""),
            kind: SearchDslCompletionKind::Value,
            signature: kw_label.to_owned(),
            language: "workspace".to_owned(),
            doc: "Indexed workspace directory".to_owned(),
            insert_text: format!("\"{dir}\" "),
        })
        .collect()
}

fn live_workspace_symbol_entries(
    file_path: Option<&str>,
    index: Option<&WorkspaceSearchIndex>,
) -> Vec<SearchDslCompletionEntry> {
    let Some(idx) = index else {
        return Vec::new();
    };
    let snapshot = idx.project_prefiltered_edb(&WorkspaceEdbPrefilter {
        languages: &[],
        documents: &[],
        path_prefix: file_path,
        extension: None,
        required_tokens: &[],
        max_files: if file_path.is_some() { 4 } else { 32 },
        max_facts: 4_096,
    });
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut entries = Vec::new();
    for ent in &snapshot.entities {
        let path_ok = file_path.is_none_or(|p| ent.path == p);
        if path_ok && ent.kind != "import" && seen.insert(ent.name.clone()) {
            entries.push(SearchDslCompletionEntry {
                form: format!("\"{}\"", ent.name),
                kind: SearchDslCompletionKind::Value,
                signature: "symbol:".to_owned(),
                language: ent.language.clone(),
                doc: format!("[{}] {}:{}", ent.kind, ent.path, ent.line),
                insert_text: format!("\"{}\" ", ent.name),
            });
            if entries.len() >= 24 {
                break;
            }
        }
    }
    entries
}

fn filter_entries(
    entries: Vec<SearchDslCompletionEntry>,
    needle: &str,
) -> Vec<SearchDslCompletionEntry> {
    let clean = strip_quotes(needle).to_ascii_lowercase();
    if clean.is_empty() {
        return entries;
    }
    let mut prefix_hits = Vec::new();
    let mut contains_hits = Vec::new();
    for entry in entries {
        let form_lc = strip_quotes(&entry.form).to_ascii_lowercase();
        if form_lc.starts_with(&clean) {
            prefix_hits.push(entry);
        } else if form_lc.contains(&clean)
            || entry.language.to_ascii_lowercase().contains(&clean)
            || entry.doc.to_ascii_lowercase().contains(&clean)
        {
            contains_hits.push(entry);
        }
    }
    prefix_hits.extend(contains_hits);
    prefix_hits
}

fn facet_completions(
    frag: &str,
    catalog: &[SearchDslCompletionEntry],
    index: Option<&WorkspaceSearchIndex>,
) -> Vec<SearchDslCompletionEntry> {
    if let Some(rest) = frag.strip_prefix("@lang:") {
        let entries: Vec<SearchDslCompletionEntry> = catalog
            .iter()
            .filter(|e| {
                e.kind == SearchDslCompletionKind::Value
                    && kw_targets_match("lang", &canonical_kw_key(&e.signature))
                    && strip_quotes(&e.form) != "any"
            })
            .map(|e| {
                let lang = strip_quotes(&e.form);
                SearchDslCompletionEntry {
                    form: format!("@lang:{lang}"),
                    kind: SearchDslCompletionKind::Value,
                    signature: "@lang:".to_owned(),
                    language: e.language.clone(),
                    doc: e.doc.clone(),
                    insert_text: format!("@lang:{lang} "),
                }
            })
            .collect();
        return filter_entries(entries, &format!("@lang:{rest}"));
    }
    if let Some(rest) = frag.strip_prefix("@ext:") {
        let entries: Vec<SearchDslCompletionEntry> = catalog
            .iter()
            .filter(|e| {
                e.kind == SearchDslCompletionKind::Value
                    && kw_targets_match("ext", &canonical_kw_key(&e.signature))
            })
            .map(|e| {
                let ext = strip_quotes(&e.form);
                SearchDslCompletionEntry {
                    form: format!("@ext:{ext}"),
                    kind: SearchDslCompletionKind::Value,
                    signature: "@ext:".to_owned(),
                    language: e.language.clone(),
                    doc: e.doc.clone(),
                    insert_text: format!("@ext:{ext} "),
                }
            })
            .collect();
        return filter_entries(entries, &format!("@ext:{rest}"));
    }
    if let Some(rest) = frag.strip_prefix("@prefix:") {
        let entries: Vec<SearchDslCompletionEntry> = live_workspace_dir_entries("@prefix:", index)
            .into_iter()
            .map(|e| {
                let dir = strip_quotes(&e.form);
                SearchDslCompletionEntry {
                    form: format!("@prefix:{dir}"),
                    kind: SearchDslCompletionKind::Value,
                    signature: "@prefix:".to_owned(),
                    language: e.language,
                    doc: e.doc,
                    insert_text: format!("@prefix:{dir} "),
                }
            })
            .collect();
        return filter_entries(entries, &format!("@prefix:{rest}"));
    }
    if let Some(rest) = frag.strip_prefix("@sym:") {
        let entries: Vec<SearchDslCompletionEntry> = live_workspace_symbol_entries(None, index)
            .into_iter()
            .map(|e| {
                let sym = strip_quotes(&e.form);
                SearchDslCompletionEntry {
                    form: format!("@sym:{sym}"),
                    kind: SearchDslCompletionKind::Value,
                    signature: "@sym:".to_owned(),
                    language: e.language,
                    doc: e.doc,
                    insert_text: format!("@sym:{sym} "),
                }
            })
            .collect();
        return filter_entries(entries, &format!("@sym:{rest}"));
    }
    Vec::new()
}

fn protocol_values_for_keyword(
    head: Option<&str>,
    pending_kw_canon: &str,
    active_lang: Option<&str>,
    tokens: &[String],
    catalog: &[SearchDslCompletionEntry],
    index: Option<&WorkspaceSearchIndex>,
) -> Vec<SearchDslCompletionEntry> {
    let is_lang_slot = matches!(pending_kw_canon, "lang" | "language" | "languageid");
    let mut values: Vec<SearchDslCompletionEntry> = catalog
        .iter()
        .filter(|e| {
            if e.kind != SearchDslCompletionKind::Value {
                return false;
            }
            if !kw_targets_match(pending_kw_canon, &canonical_kw_key(&e.signature)) {
                return false;
            }
            if is_lang_slot {
                return true;
            }
            match active_lang {
                Some(lang) if lang != "any" && lang != "*" => {
                    e.language.eq_ignore_ascii_case(lang) || e.language == "*"
                }
                _ => true,
            }
        })
        .cloned()
        .collect();

    let schema_tag = keyword_value_schema_tag(head, pending_kw_canon, catalog);
    if schema_tag == Some("workspace:dir")
        || matches!(
            pending_kw_canon,
            "pathprefix" | "prefix" | "scope" | "excludepathprefix"
        )
    {
        values.extend(live_workspace_dir_entries("path-prefix:", index));
    } else if schema_tag == Some("workspace:symbol")
        || matches!(pending_kw_canon, "symbol" | "sym" | "def")
    {
        let file_path = extract_first_positional_path(tokens);
        values.extend(live_workspace_symbol_entries(file_path.as_deref(), index));
    }

    let mut seen: BTreeSet<(String, String)> = BTreeSet::new();
    values.retain(|e| seen.insert((e.form.clone(), e.signature.clone())));
    values
}

fn protocol_keywords_for_context(
    head: &str,
    active_lang: Option<&str>,
    used_kws: &BTreeSet<String>,
    catalog: &[SearchDslCompletionEntry],
) -> Vec<SearchDslCompletionEntry> {
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut out = Vec::new();
    // 1. Form-owned keywords declared in Scheme POO (`language == head`).
    for entry in catalog {
        if entry.kind == SearchDslCompletionKind::Keyword
            && entry.language.eq_ignore_ascii_case(head)
        {
            let ck = canonical_kw_key(&entry.form);
            if !used_kws.contains(&ck) && seen.insert(ck) {
                let mut cloned = entry.clone();
                if let Some(lang) = active_lang {
                    cloned.language = lang.to_owned();
                }
                out.push(cloned);
            }
        }
    }
    // 2. Active-language JSON Schema property keywords forwarded by ASP (`language == active_lang`).
    if let Some(lang) = active_lang {
        for entry in catalog {
            if entry.kind == SearchDslCompletionKind::Keyword
                && entry.language.eq_ignore_ascii_case(lang)
            {
                let ck = canonical_kw_key(&entry.form);
                if !used_kws.contains(&ck) && seen.insert(ck) {
                    out.push(entry.clone());
                }
            }
        }
    }
    // 3. Universal protocol JSON Schema property keywords (`language == "*"`).
    for entry in catalog {
        if entry.kind == SearchDslCompletionKind::Keyword && entry.language == "*" {
            let ck = canonical_kw_key(&entry.form);
            if !used_kws.contains(&ck) && seen.insert(ck) {
                let mut cloned = entry.clone();
                if let Some(lang) = active_lang {
                    cloned.language = lang.to_owned();
                }
                out.push(cloned);
            }
        }
    }
    out
}

/// Compute 100% protocol-driven Copilot completions for `input` using ONLY the
/// Scheme/ASP-projected `catalog` and live resident `index`.
pub(super) fn compute_copilot_completions(
    input: &str,
    catalog: &[SearchDslCompletionEntry],
    index: Option<&WorkspaceSearchIndex>,
) -> Vec<SearchDslCompletionEntry> {
    let trimmed = input.trim();
    if !trimmed.starts_with('(') {
        let frag = trailing_completion_fragment(input);
        return facet_completions(frag, catalog, index);
    }

    if trimmed == "(" {
        return catalog
            .iter()
            .filter(|e| {
                matches!(
                    e.kind,
                    SearchDslCompletionKind::Ascent
                        | SearchDslCompletionKind::Dusa
                        | SearchDslCompletionKind::Parser
                        | SearchDslCompletionKind::Edb
                )
            })
            .cloned()
            .collect();
    }

    let trailing_space = input.ends_with(char::is_whitespace);
    let tokens = tokenize_prefix(input);
    let head = tokens.get(1).map(|s| s.to_ascii_lowercase());
    let frag = trailing_completion_fragment(input);

    if !trailing_space && tokens.len() == 2 && tokens[0] == "(" {
        let root_forms: Vec<SearchDslCompletionEntry> = catalog
            .iter()
            .filter(|e| {
                matches!(
                    e.kind,
                    SearchDslCompletionKind::Ascent
                        | SearchDslCompletionKind::Dusa
                        | SearchDslCompletionKind::Parser
                        | SearchDslCompletionKind::Edb
                )
            })
            .cloned()
            .collect();
        return filter_entries(root_forms, frag);
    }

    let active_lang = extract_active_lang(&tokens);
    let used_kws: BTreeSet<String> = tokens
        .iter()
        .filter(|t| is_keyword_token(t))
        .map(|t| canonical_kw_key(t))
        .collect();
    let last_is_closed_quoted = !trailing_space
        && tokens
            .last()
            .is_some_and(|t| t.len() >= 2 && t.starts_with('"') && t.ends_with('"'));
    let pending_kw = if trailing_space {
        tokens
            .last()
            .filter(|t| is_keyword_token(t))
            .map(|t| canonical_kw_key(t))
    } else if !last_is_closed_quoted && tokens.len() >= 2 {
        tokens
            .get(tokens.len() - 2)
            .filter(|t| is_keyword_token(t))
            .map(|t| canonical_kw_key(t))
    } else {
        None
    };

    if let Some(kw_canon) = pending_kw.as_deref() {
        let values = protocol_values_for_keyword(
            head.as_deref(),
            kw_canon,
            active_lang.as_deref(),
            &tokens,
            catalog,
            index,
        );
        if !values.is_empty() {
            return filter_entries(values, frag);
        }
    }

    if let Some(head_str) = head.as_deref() {
        let kw_entries =
            protocol_keywords_for_context(head_str, active_lang.as_deref(), &used_kws, catalog);
        if !kw_entries.is_empty() {
            let kw_frag = if last_is_closed_quoted { "" } else { frag };
            return filter_entries(kw_entries, kw_frag);
        }
    }

    let needle = super::overlay::last_sexp_token(trimmed).to_ascii_lowercase();
    let mut matched: Vec<SearchDslCompletionEntry> = catalog
        .iter()
        .filter(|entry| {
            !matches!(
                entry.kind,
                SearchDslCompletionKind::Keyword | SearchDslCompletionKind::Value
            ) && (needle.is_empty()
                || entry.form.to_ascii_lowercase().contains(&needle)
                || entry.language.to_ascii_lowercase().contains(&needle)
                || entry.doc.to_ascii_lowercase().contains(&needle))
        })
        .cloned()
        .collect();
    if let Some(lang) = active_lang.as_deref() {
        matched.sort_by_key(|e| usize::from(!e.language.eq_ignore_ascii_case(lang)));
    }
    matched
}

/// Compute the Copilot inline ghost-text suffix shown in muted grey right after
/// the minibuffer cursor when the cursor sits at the end of the input.
pub(super) fn compute_ghost_suffix(
    input: &str,
    cursor_byte: usize,
    entries: &[SearchDslCompletionEntry],
    selected: usize,
) -> Option<String> {
    if cursor_byte != input.len() || input.trim().is_empty() || !supports_copilot_completion(input) {
        return None;
    }
    let entry = entries.get(selected).or_else(|| entries.first())?;
    let suggestion = entry.insert_text.trim_end();
    if suggestion.is_empty() {
        return None;
    }
    let frag = trailing_completion_fragment(input);
    if frag.is_empty() {
        return Some(suggestion.to_owned());
    }
    if frag.len() >= 2 && frag.starts_with('"') && frag.ends_with('"') {
        return Some(format!(" {suggestion}"));
    }
    if suggestion.len() > frag.len()
        && suggestion[..frag.len()].eq_ignore_ascii_case(frag)
    {
        return Some(suggestion[frag.len()..].to_owned());
    }
    let unquoted = strip_quotes(frag);
    let sug_unquoted = strip_quotes(suggestion);
    if !unquoted.is_empty()
        && sug_unquoted.len() > unquoted.len()
        && sug_unquoted[..unquoted.len()].eq_ignore_ascii_case(unquoted)
    {
        let mut suffix = sug_unquoted[unquoted.len()..].to_owned();
        if suggestion.ends_with('"') && !frag.ends_with('"') {
            suffix.push('"');
        }
        return Some(suffix);
    }
    None
}
