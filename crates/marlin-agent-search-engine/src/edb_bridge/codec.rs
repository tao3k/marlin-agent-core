// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Zero-JSON `RootedSchemeObject` codecs between ASP Open EDB snapshots and
//! Marlin's `:gerbil-ascent` / `:gerbil-parser` Scheme runtime.

use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Write as _};

use agent_semantic_search::WorkspaceEdbSnapshot;
use gerbil_scheme::{
    CollectionKind, GerbilRuntime, IdentifierBuffer, IdentifierKind, ObjectKind,
    RootedSchemeObject, SchemeScalar,
};

use super::types::{DEFAULT_MAX_EDB_FACTS, SearchDslDecodeContext};
use super::{
    EdbHydrationMask, OwnedWorkspaceEdbPrefilter, SearchDslCompletionEntry,
    SearchDslCompletionKind, SearchDslExecutionMode, SearchDslStructuredResult,
    SearchDslStructuredRow, SearchEpistemicStatus, SearchEvidenceChainReceipt,
};
use crate::SearchPlanProjectionError;

const MAX_TOKENS: usize = 64;
const MAX_COMPLETIONS: usize = 1024;
const MAX_TEXT_BYTES: usize = 65_536;
const MAX_IDENTIFIER_BYTES: usize = 1024;

pub(crate) fn native<T>(result: Result<T, impl Display>) -> Result<T, SearchPlanProjectionError> {
    result.map_err(|error| SearchPlanProjectionError::Native(error.to_string()))
}

pub(crate) fn decode_symbol(
    value: RootedSchemeObject<'_>,
) -> Result<String, SearchPlanProjectionError> {
    if native(value.as_ref().kind())? != ObjectKind::Symbol {
        return Err(SearchPlanProjectionError::Invalid("expected a symbol"));
    }
    let mut buffer = IdentifierBuffer::default();
    native(value.as_ref().copy_identifier_names(&mut buffer))?;
    let Some((IdentifierKind::Symbol, name)) = buffer.get(0) else {
        return Err(SearchPlanProjectionError::Invalid("invalid symbol"));
    };
    if buffer.len() != 1 || name.is_empty() || name.len() > MAX_IDENTIFIER_BYTES {
        return Err(SearchPlanProjectionError::Invalid("invalid symbol length"));
    }
    Ok(name.to_owned())
}

pub(crate) fn decode_string(
    value: RootedSchemeObject<'_>,
    max_bytes: usize,
) -> Result<String, SearchPlanProjectionError> {
    let string_obj = native(value.into_string())?;
    let text = native(string_obj.to_string().into_result())?;
    if text.len() > max_bytes {
        return Err(SearchPlanProjectionError::Invalid(
            "string exceeds byte limit",
        ));
    }
    Ok(text)
}

fn decode_optional_string(
    value: RootedSchemeObject<'_>,
    max_bytes: usize,
) -> Result<Option<String>, SearchPlanProjectionError> {
    match native(value.as_ref().kind())? {
        ObjectKind::Boolean
            if matches!(
                native(value.as_ref().scalar())?,
                SchemeScalar::Boolean(false)
            ) =>
        {
            Ok(None)
        }
        ObjectKind::String => Ok(Some(decode_string(value, max_bytes)?)),
        _ => Err(SearchPlanProjectionError::Invalid(
            "expected #f or string value",
        )),
    }
}

fn decode_string_vector(
    value: RootedSchemeObject<'_>,
    limit: usize,
) -> Result<Vec<String>, SearchPlanProjectionError> {
    let items = native(value.into_collection(CollectionKind::Vector))?;
    if items.len() > limit {
        return Err(SearchPlanProjectionError::Invalid(
            "string vector exceeds limit",
        ));
    }
    let mut out = Vec::with_capacity(items.len());
    for i in 0..items.len() {
        out.push(decode_string(native(items.element(i))?, MAX_TEXT_BYTES)?);
    }
    Ok(out)
}

fn decode_symbol_vector(
    value: RootedSchemeObject<'_>,
    limit: usize,
) -> Result<Vec<String>, SearchPlanProjectionError> {
    let items = native(value.into_collection(CollectionKind::Vector))?;
    if items.len() > limit {
        return Err(SearchPlanProjectionError::Invalid(
            "symbol vector exceeds limit",
        ));
    }
    let mut out = Vec::with_capacity(items.len());
    for i in 0..items.len() {
        out.push(decode_symbol(native(items.element(i))?)?);
    }
    Ok(out)
}

pub(crate) fn decode_usize(
    value: RootedSchemeObject<'_>,
) -> Result<usize, SearchPlanProjectionError> {
    match native(value.as_ref().scalar())? {
        SchemeScalar::Fixnum(v) if v >= 0 => {
            usize::try_from(v).map_err(|_| SearchPlanProjectionError::Invalid("fixnum overflow"))
        }
        _ => Err(SearchPlanProjectionError::Invalid(
            "expected non-negative fixnum",
        )),
    }
}

pub(crate) fn make_vector<'a>(
    runtime: &'a GerbilRuntime,
    items: &[RootedSchemeObject<'a>],
) -> Result<RootedSchemeObject<'a>, SearchPlanProjectionError> {
    let refs: Vec<&RootedSchemeObject<'a>> = items.iter().collect();
    Ok(native(runtime.collection_from_objects(CollectionKind::Vector, &refs))?.into_object())
}

pub(crate) fn make_string<'a>(
    runtime: &'a GerbilRuntime,
    value: &str,
) -> Result<RootedSchemeObject<'a>, SearchPlanProjectionError> {
    Ok(native(runtime.string_from_utf8(value))?.into_object())
}

fn make_fixnum<'a>(
    runtime: &'a GerbilRuntime,
    value: usize,
) -> Result<RootedSchemeObject<'a>, SearchPlanProjectionError> {
    let signed = isize::try_from(value)
        .map_err(|_| SearchPlanProjectionError::Invalid("fixnum overflow"))?;
    native(runtime.scalar_object(SchemeScalar::Fixnum(signed)))
}

/// Decode Scheme's zero-regex
/// `#(search-edb-prefilter languages path-prefix tokens max-files active-edb any-tokens)`
/// rooted 7-vector (or structured `#(search-error message)` 2-vector) into an owned [`OwnedWorkspaceEdbPrefilter`].
pub fn decode_edb_prefilter(
    value: RootedSchemeObject<'_>,
) -> Result<OwnedWorkspaceEdbPrefilter, SearchPlanProjectionError> {
    let tuple = native(value.into_collection(CollectionKind::Vector))?;
    if tuple.len() == 2 && decode_symbol(native(tuple.element(0))?)? == "search-error" {
        let msg = decode_string(native(tuple.element(1))?, MAX_TEXT_BYTES)?;
        return Err(SearchPlanProjectionError::Validation(msg.trim().to_owned()));
    }
    if tuple.len() != 7 {
        return Err(SearchPlanProjectionError::Invalid(
            "search-edb-prefilter arity",
        ));
    }
    if decode_symbol(native(tuple.element(0))?)? != "search-edb-prefilter" {
        return Err(SearchPlanProjectionError::Invalid(
            "unexpected search-edb-prefilter tag",
        ));
    }
    let languages = decode_string_vector(native(tuple.element(1))?, MAX_TOKENS)?;
    let path_prefix = decode_optional_string(native(tuple.element(2))?, MAX_TEXT_BYTES)?
        .and_then(|raw| {
            let mut normalized = raw.trim().replace('\\', "/");
            while let Some(stripped) = normalized.strip_prefix("./") {
                normalized = stripped.to_owned();
            }
            let trimmed = normalized.trim_start_matches('/');
            (!trimmed.is_empty() && trimmed != ".").then(|| trimmed.to_owned())
        });
    let required_tokens = decode_string_vector(native(tuple.element(3))?, MAX_TOKENS)?;
    let max_files = decode_usize(native(tuple.element(4))?)?.max(1);
    let active_edb = decode_symbol_vector(native(tuple.element(5))?, 16)?;
    let any_tokens = decode_string_vector(native(tuple.element(6))?, MAX_TOKENS)?;
    let hydration = EdbHydrationMask::from_active_relations(&active_edb);
    Ok(OwnedWorkspaceEdbPrefilter {
        languages,
        documents: Vec::new(),
        path_prefix,
        extension: None,
        required_tokens,
        any_tokens,
        max_files,
        max_facts: DEFAULT_MAX_EDB_FACTS,
        hydration,
    })
}

fn is_ident_char(ch: char, allow_hyphen: bool) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || (allow_hyphen && ch == '-')
}

fn line_contains_ident_token(line_text: &str, token: &str) -> bool {
    let trimmed = line_text.trim_start();
    if trimmed.starts_with("//") || trimmed.starts_with(";;") || trimmed.starts_with('#') {
        return false;
    }
    let allow_hyphen = token.contains('-');
    let mut offset = 0usize;
    while let Some(rel) = line_text[offset..].find(token) {
        let start = offset + rel;
        let end = start + token.len();
        let before_ok = line_text[..start]
            .chars()
            .next_back()
            .is_none_or(|c| !is_ident_char(c, allow_hyphen));
        let after_ok = line_text[end..]
            .chars()
            .next()
            .is_none_or(|c| !is_ident_char(c, allow_hyphen));
        if before_ok && after_ok {
            return true;
        }
        offset = start + 1;
    }
    false
}

fn matching_line_numbers<'a>(
    hit: &'a agent_semantic_search::WorkspaceSearchHit,
    token: &'a str,
) -> impl Iterator<Item = (String, String, usize)> + 'a {
    hit.line_matches
        .iter()
        .filter(move |lm| line_contains_ident_token(&lm.line_text, token))
        .map(move |lm| (token.to_owned(), hit.path.clone(), lm.line_number))
}

fn query_token_symbol_refs(
    index: &agent_semantic_search::WorkspaceSearchIndex,
    prefilter: &OwnedWorkspaceEdbPrefilter,
    allowed_paths: &HashSet<&str>,
    max_hits: usize,
    token: &str,
    remaining: usize,
) -> Vec<(String, String, usize)> {
    let report = index.query(&agent_semantic_search::WorkspaceSearchQuery {
        pattern: token,
        languages: &prefilter.languages,
        documents: &prefilter.documents,
        path_prefix: prefilter.path_prefix.as_deref(),
        extension: prefilter.extension.as_deref(),
        symbol: None,
        graph_target: None,
        max_hits,
        context_radius: 0,
    });
    report
        .hits
        .iter()
        .filter(|hit| allowed_paths.contains(hit.path.as_str()))
        .flat_map(|hit| matching_line_numbers(hit, token))
        .take(remaining)
        .collect()
}

/// Collect exact word-boundary lexical symbol references `(symbol, path, line)`
/// across all files in `snapshot` from the pure-Rust resident RAM index (<1ms,
/// zero disk I/O), providing 100% file coverage even where native AST edges are
/// not yet hydrated.
pub(crate) fn collect_symbol_ref_rows(
    index: &agent_semantic_search::WorkspaceSearchIndex,
    snapshot: &WorkspaceEdbSnapshot,
    prefilter: &OwnedWorkspaceEdbPrefilter,
) -> Vec<(String, String, usize)> {
    const MAX_SYMBOL_REF_ROWS: usize = 2048;
    if (prefilter.required_tokens.is_empty() && prefilter.any_tokens.is_empty())
        || snapshot.files.is_empty()
    {
        return Vec::new();
    }
    let allowed_paths: HashSet<&str> = snapshot.files.iter().map(|f| f.path.as_str()).collect();
    let max_hits = snapshot.files.len().clamp(32, 512);
    let mut rows = Vec::new();
    for token in prefilter
        .required_tokens
        .iter()
        .chain(prefilter.any_tokens.iter())
        .filter(|t| t.len() >= 3)
    {
        let remaining = MAX_SYMBOL_REF_ROWS.saturating_sub(rows.len());
        if remaining == 0 {
            break;
        }
        rows.extend(query_token_symbol_refs(
            index,
            prefilter,
            &allowed_paths,
            max_hits,
            token,
            remaining,
        ));
    }
    rows
}

/// Encode an ASP [`WorkspaceEdbSnapshot`] into a rooted Scheme 5-vector
/// `#(files-vec entities-vec edges-vec attrs-vec symbol-refs-vec)` matching
/// `semantic-open-edb.v1` with zero JSON serialization.
pub fn encode_edb_snapshot<'a>(
    runtime: &'a GerbilRuntime,
    snapshot: &WorkspaceEdbSnapshot,
) -> Result<RootedSchemeObject<'a>, SearchPlanProjectionError> {
    encode_edb_snapshot_with_refs(runtime, snapshot, EdbHydrationMask::DEFAULT_RELATIONS, &[])
}

/// Encode an ASP [`WorkspaceEdbSnapshot`] skipping unreferenced `edge`, `attr`,
/// or `symbol_ref` tables when outside the query's dependency cone.
pub(crate) fn encode_edb_snapshot_with_refs<'a>(
    runtime: &'a GerbilRuntime,
    snapshot: &WorkspaceEdbSnapshot,
    hydration: EdbHydrationMask,
    symbol_refs: &[(String, String, usize)],
) -> Result<RootedSchemeObject<'a>, SearchPlanProjectionError> {
    let include_edges = hydration.includes_edges();
    let include_attrs = hydration.includes_attrs();
    let mut file_rows = Vec::with_capacity(snapshot.files.len());
    for f in &snapshot.files {
        let file_id = usize::try_from(f.file_id)
            .map_err(|_| SearchPlanProjectionError::Invalid("negative file_id in EDB file row"))?;
        file_rows.push(make_vector(
            runtime,
            &[
                make_fixnum(runtime, file_id)?,
                make_string(runtime, f.language.as_deref().unwrap_or("unknown"))?,
                make_string(runtime, &f.path)?,
            ],
        )?);
    }

    let mut entity_rows = Vec::with_capacity(snapshot.entities.len());
    let mut attr_rows = Vec::new();
    for e in &snapshot.entities {
        let entity_file_id = usize::try_from(e.file_id).map_err(|_| {
            SearchPlanProjectionError::Invalid("negative file_id in EDB entity row")
        })?;
        entity_rows.push(make_vector(
            runtime,
            &[
                make_string(runtime, &e.entity_id)?,
                make_fixnum(runtime, entity_file_id)?,
                make_string(runtime, &e.language)?,
                make_string(runtime, &e.kind)?,
                make_string(runtime, &e.language_kind)?,
                make_string(runtime, &e.name)?,
                make_fixnum(runtime, e.line)?,
            ],
        )?);
        if include_attrs {
            for attr in &e.attrs {
                attr_rows.push(make_vector(
                    runtime,
                    &[
                        make_string(runtime, &e.entity_id)?,
                        make_string(runtime, &attr.key)?,
                        make_string(runtime, &attr.value)?,
                    ],
                )?);
            }
        }
    }

    let mut edge_rows = Vec::with_capacity(if include_edges {
        snapshot.edges.len()
    } else {
        0
    });
    if include_edges {
        for edge in &snapshot.edges {
            edge_rows.push(make_vector(
                runtime,
                &[
                    make_string(runtime, &edge.src)?,
                    make_string(runtime, &edge.dst)?,
                    make_string(runtime, &edge.relation)?,
                    make_string(runtime, &edge.path)?,
                    make_fixnum(runtime, edge.line)?,
                ],
            )?);
        }
    }

    let mut ref_rows = Vec::with_capacity(symbol_refs.len());
    for (sym, path, line) in symbol_refs {
        ref_rows.push(make_vector(
            runtime,
            &[
                make_string(runtime, sym)?,
                make_string(runtime, path)?,
                make_fixnum(runtime, *line)?,
            ],
        )?);
    }

    make_vector(
        runtime,
        &[
            make_vector(runtime, &file_rows)?,
            make_vector(runtime, &entity_rows)?,
            make_vector(runtime, &edge_rows)?,
            make_vector(runtime, &attr_rows)?,
            make_vector(runtime, &ref_rows)?,
        ],
    )
}

/// Decode Scheme's `#(#(form kind signature lang doc insert-text) ...)` into
/// typed [`SearchDslCompletionEntry`] values for Helm's interactive TAB completion table.
pub fn decode_search_dsl_completions(
    value: RootedSchemeObject<'_>,
) -> Result<Vec<SearchDslCompletionEntry>, SearchPlanProjectionError> {
    let rows = native(value.into_collection(CollectionKind::Vector))?;
    if rows.len() > MAX_COMPLETIONS {
        return Err(SearchPlanProjectionError::Invalid(
            "completion table exceeds limit",
        ));
    }
    let mut out = Vec::with_capacity(rows.len());
    for i in 0..rows.len() {
        let row = native(native(rows.element(i))?.into_collection(CollectionKind::Vector))?;
        if row.len() != 6 {
            return Err(SearchPlanProjectionError::Invalid(
                "completion entry must have 6 fields",
            ));
        }
        let kind_raw = decode_string(native(row.element(1))?, MAX_IDENTIFIER_BYTES)?;
        out.push(SearchDslCompletionEntry {
            form: decode_string(native(row.element(0))?, MAX_IDENTIFIER_BYTES)?,
            kind: SearchDslCompletionKind::parse(&kind_raw)?,
            signature: decode_string(native(row.element(2))?, MAX_TEXT_BYTES)?,
            language: decode_string(native(row.element(3))?, MAX_IDENTIFIER_BYTES)?,
            doc: decode_string(native(row.element(4))?, MAX_TEXT_BYTES)?,
            insert_text: decode_string(native(row.element(5))?, MAX_TEXT_BYTES)?,
        });
    }
    Ok(out)
}

pub(super) fn format_scalar_cell(
    value: RootedSchemeObject<'_>,
) -> Result<String, SearchPlanProjectionError> {
    match native(value.as_ref().kind())? {
        ObjectKind::String => decode_string(value, MAX_TEXT_BYTES),
        ObjectKind::Symbol => decode_symbol(value),
        ObjectKind::ExactInteger => match native(value.as_ref().scalar())? {
            SchemeScalar::Fixnum(v) => Ok(v.to_string()),
            _ => Ok("?".to_owned()),
        },
        ObjectKind::Boolean => match native(value.as_ref().scalar())? {
            SchemeScalar::Boolean(b) => Ok(if b { "#t" } else { "#f" }.to_owned()),
            _ => Ok("#f".to_owned()),
        },
        ObjectKind::Vector => {
            let items = native(value.into_collection(CollectionKind::Vector))?;
            let mut parts = Vec::with_capacity(items.len());
            for i in 0..items.len() {
                parts.push(format_scalar_cell(native(items.element(i))?)?);
            }
            Ok(format!("({})", parts.join(" ")))
        }
        _ => Ok("_".to_owned()),
    }
}

fn decode_row_cells(
    value: RootedSchemeObject<'_>,
) -> Result<(Vec<String>, String), SearchPlanProjectionError> {
    if native(value.as_ref().kind())? == ObjectKind::Vector {
        let items = native(value.into_collection(CollectionKind::Vector))?;
        let mut cells = Vec::with_capacity(items.len());
        for i in 0..items.len() {
            cells.push(format_scalar_cell(native(items.element(i))?)?);
        }
        let display = format!("({})", cells.join(" "));
        Ok((cells, display))
    } else {
        let scalar = format_scalar_cell(value)?;
        Ok((vec![scalar.clone()], scalar))
    }
}

fn extract_locator_path_and_line(
    cell: &str,
    known_paths: &HashSet<&str>,
) -> (Option<String>, Option<usize>) {
    if known_paths.contains(cell) {
        return (Some(cell.to_owned()), None);
    }
    let Some((prefix, tail)) = cell.split_once('#') else {
        return (None, None);
    };
    let path = if known_paths.contains(prefix) || prefix.contains('/') || prefix.contains('.') {
        Some(prefix.to_owned())
    } else {
        None
    };
    let line = tail
        .split("@L")
        .nth(1)
        .and_then(|rest| rest.split(':').next())
        .and_then(|digits| digits.parse::<usize>().ok());
    (path, line)
}

pub(super) fn infer_path_and_line(
    cells: &[String],
    known_paths: &HashSet<&str>,
    file_id_map: &HashMap<usize, &str>,
) -> (Option<String>, Option<usize>) {
    let mut path = None;
    let mut line = None;
    for cell in cells {
        if path.is_none() || line.is_none() {
            let (loc_path, loc_line) = extract_locator_path_and_line(cell, known_paths);
            if path.is_none() {
                path = loc_path;
            }
            if line.is_none() {
                line = loc_line;
            }
        }
        if let Ok(num) = cell.parse::<usize>()
            && num > 0
        {
            if path.is_none()
                && let Some(mapped_path) = file_id_map.get(&num)
            {
                path = Some((*mapped_path).to_owned());
            } else if line.is_none() {
                line = Some(num);
            }
        }
    }
    (path, line)
}

fn decode_optional_usize(
    value: RootedSchemeObject<'_>,
) -> Result<Option<usize>, SearchPlanProjectionError> {
    match native(value.as_ref().kind())? {
        ObjectKind::Boolean
            if matches!(
                native(value.as_ref().scalar())?,
                SchemeScalar::Boolean(false)
            ) =>
        {
            Ok(None)
        }
        ObjectKind::ExactInteger => Ok(Some(decode_usize(value)?)),
        _ => Err(SearchPlanProjectionError::Invalid(
            "expected #f or non-negative fixnum",
        )),
    }
}

pub(super) fn decode_optional_evidence_receipt(
    value: RootedSchemeObject<'_>,
) -> Result<Option<SearchEvidenceChainReceipt>, SearchPlanProjectionError> {
    if native(value.as_ref().kind())? == ObjectKind::Boolean
        && matches!(
            native(value.as_ref().scalar())?,
            SchemeScalar::Boolean(false)
        )
    {
        return Ok(None);
    }
    let tuple = native(value.into_collection(CollectionKind::Vector))?;
    if tuple.len() != 9 || decode_symbol(native(tuple.element(0))?)? != "evidence-receipt" {
        return Err(SearchPlanProjectionError::Invalid(
            "unexpected evidence-receipt vector shape",
        ));
    }
    Ok(Some(SearchEvidenceChainReceipt {
        epoch: decode_usize(native(tuple.element(1))?)?,
        total_m1_claims: decode_usize(native(tuple.element(2))?)?,
        newly_harvested: decode_usize(native(tuple.element(3))?)?,
        ambiguous_branches: decode_string_vector(native(tuple.element(4))?, MAX_TOKENS)?,
        conflict_count: decode_usize(native(tuple.element(5))?)?,
        covered_layers: decode_string_vector(native(tuple.element(6))?, MAX_TOKENS)?,
        open_frontiers: decode_string_vector(native(tuple.element(7))?, MAX_TOKENS)?,
        epistemic_status: SearchEpistemicStatus::parse(&decode_symbol(native(tuple.element(8))?)?)?,
    }))
}

use super::render::{AscentRenderInputs, format_snapshot_header, render_ascent_search_text};

fn decode_parse_artifact_result(
    mut elems: std::vec::IntoIter<RootedSchemeObject<'_>>,
    snapshot: &WorkspaceEdbSnapshot,
    ctx: SearchDslDecodeContext<'_>,
) -> Result<SearchDslStructuredResult, SearchPlanProjectionError> {
    let status = decode_symbol(
        elems
            .next()
            .ok_or(SearchPlanProjectionError::Invalid("status"))?,
    )?;
    let language = decode_string(
        elems
            .next()
            .ok_or(SearchPlanProjectionError::Invalid("language"))?,
        MAX_IDENTIFIER_BYTES,
    )?;
    let root_kind = format_scalar_cell(
        elems
            .next()
            .ok_or(SearchPlanProjectionError::Invalid("root"))?,
    )?;
    let diag_count = decode_usize(
        elems
            .next()
            .ok_or(SearchPlanProjectionError::Invalid("diagnostics"))?,
    )?;
    let mut rendered_text = format_snapshot_header(snapshot, ctx.prefilter);
    let _ = writeln!(
        rendered_text,
        "mode: gerbil-parser\nlanguage: {language}\nstatus: {status}\nroot: {root_kind}\ndiagnostics: {diag_count}"
    );
    let row = SearchDslStructuredRow {
        cells: vec![status.clone(), root_kind.clone(), diag_count.to_string()],
        display: format!("{language}: {status} ({root_kind}, diagnostics={diag_count})"),
        path: None,
        line: None,
    };
    Ok(SearchDslStructuredResult {
        generation_digest: snapshot.generation_digest.as_str().to_owned(),
        elapsed_micros: snapshot.elapsed_micros,
        indexed_files: snapshot.indexed_files,
        matched_files: snapshot.files.len(),
        mode: SearchDslExecutionMode::GerbilParser,
        relation: language,
        repairs: Vec::new(),
        canonical_query: None,
        rows: vec![row],
        choice_worlds: Vec::new(),
        budget_exhausted: None,
        evidence_receipt: None,
        rendered_text,
    })
}

fn decode_ascent_search_result(
    mut elems: std::vec::IntoIter<RootedSchemeObject<'_>>,
    snapshot: &WorkspaceEdbSnapshot,
    ctx: SearchDslDecodeContext<'_>,
) -> Result<SearchDslStructuredResult, SearchPlanProjectionError> {
    let mode_raw = decode_symbol(
        elems
            .next()
            .ok_or(SearchPlanProjectionError::Invalid("mode"))?,
    )?;
    let mode = SearchDslExecutionMode::parse(&mode_raw)?;
    let relation = decode_symbol(
        elems
            .next()
            .ok_or(SearchPlanProjectionError::Invalid("relation"))?,
    )?;
    let payload_obj = elems
        .next()
        .ok_or(SearchPlanProjectionError::Invalid("payload"))?;
    let repairs_obj = elems
        .next()
        .ok_or(SearchPlanProjectionError::Invalid("repairs"))?;
    let repairs = decode_symbol_vector(repairs_obj, MAX_TOKENS)?;
    if mode == SearchDslExecutionMode::Choice {
        let canonical_query = elems
            .next()
            .map(|obj| decode_optional_string(obj, MAX_TEXT_BYTES))
            .transpose()?
            .flatten();
        let evidence_receipt = elems
            .next()
            .map(decode_optional_evidence_receipt)
            .transpose()?
            .flatten();
        let mut choice_res = super::choice_codec::decode_choice_payload(
            relation,
            payload_obj,
            repairs,
            canonical_query,
            snapshot,
            ctx,
        )?;
        choice_res.evidence_receipt = evidence_receipt;
        return Ok(choice_res);
    }
    let total_results_override = elems
        .next()
        .map(decode_usize)
        .transpose()?;
    let canonical_query = elems
        .next()
        .map(|obj| decode_optional_string(obj, MAX_TEXT_BYTES))
        .transpose()?
        .flatten();
    let explicit_context = elems
        .next()
        .map(decode_optional_usize)
        .transpose()?
        .flatten();
    let evidence_receipt = elems
        .next()
        .map(decode_optional_evidence_receipt)
        .transpose()?
        .flatten();
    let payload = native(payload_obj.into_collection(CollectionKind::Vector))?;
    let total_results = total_results_override.unwrap_or(payload.len());

    let known_paths: HashSet<&str> = snapshot.files.iter().map(|f| f.path.as_str()).collect();
    let file_id_map: HashMap<usize, &str> = snapshot
        .files
        .iter()
        .filter_map(|f| {
            usize::try_from(f.file_id)
                .ok()
                .map(|id| (id, f.path.as_str()))
        })
        .collect();

    let single_file_path = (snapshot.files.len() == 1).then(|| snapshot.files[0].path.clone());
    let rows: Vec<SearchDslStructuredRow> = (0..payload.len())
        .map(|i| {
            let (cells, display) = decode_row_cells(native(payload.element(i))?)?;
            let (mut path, line) = infer_path_and_line(&cells, &known_paths, &file_id_map);
            if path.is_none() && line.is_some() {
                path.clone_from(&single_file_path);
            }
            Ok(SearchDslStructuredRow {
                cells,
                display,
                path,
                line,
            })
        })
        .collect::<Result<Vec<_>, SearchPlanProjectionError>>()?;

    let rendered_text = render_ascent_search_text(AscentRenderInputs {
        snapshot,
        ctx,
        mode,
        relation: &relation,
        rows: &rows,
        total_results,
        repairs: &repairs,
        canonical_query: canonical_query.as_deref(),
        explicit_context,
    });

    Ok(SearchDslStructuredResult {
        generation_digest: snapshot.generation_digest.as_str().to_owned(),
        elapsed_micros: snapshot.elapsed_micros,
        indexed_files: snapshot.indexed_files,
        matched_files: snapshot.files.len(),
        mode,
        relation,
        repairs,
        canonical_query,
        rows,
        choice_worlds: Vec::new(),
        budget_exhausted: None,
        evidence_receipt,
        rendered_text,
    })
}

/// Decode a rooted Scheme `marlin-search-dsl-eval-edb` answer vector with host
/// prefilter & resident-index context for scoped coverage and RAM source snippets.
pub(crate) fn decode_rooted_search_dsl_result_with_context(
    value: RootedSchemeObject<'_>,
    snapshot: &WorkspaceEdbSnapshot,
    ctx: SearchDslDecodeContext<'_>,
) -> Result<SearchDslStructuredResult, SearchPlanProjectionError> {
    let tuple = native(value.into_collection(CollectionKind::Vector))?;
    if tuple.len() == 2 && decode_symbol(native(tuple.element(0))?)? == "search-error" {
        let msg = decode_string(native(tuple.element(1))?, MAX_TEXT_BYTES)?;
        return Err(SearchPlanProjectionError::Validation(msg.trim().to_owned()));
    }
    if !(5..=9).contains(&tuple.len()) {
        return Err(SearchPlanProjectionError::Invalid(
            "unexpected search DSL result arity",
        ));
    }
    let mut elems = Vec::with_capacity(tuple.len());
    for i in 0..tuple.len() {
        elems.push(native(tuple.element(i))?);
    }
    let mut iter = elems.into_iter();
    let tag = decode_symbol(
        iter.next()
            .ok_or(SearchPlanProjectionError::Invalid("tag"))?,
    )?;
    match tag.as_str() {
        "parse-artifact-summary" => decode_parse_artifact_result(iter, snapshot, ctx),
        "search-result" => decode_ascent_search_result(iter, snapshot, ctx),
        _ => Err(SearchPlanProjectionError::Invalid(
            "unexpected search DSL result shape",
        )),
    }
}

/// Decode a rooted Scheme `marlin-search-dsl-eval-edb` answer vector into a
/// structured [`SearchDslStructuredResult`] carrying typed rows, path/line
/// navigation coordinates, repair tags, and bounded rendered text.
pub fn decode_rooted_search_dsl_result(
    value: RootedSchemeObject<'_>,
    snapshot: &WorkspaceEdbSnapshot,
    output_limit: usize,
) -> Result<SearchDslStructuredResult, SearchPlanProjectionError> {
    decode_rooted_search_dsl_result_with_context(
        value,
        snapshot,
        SearchDslDecodeContext {
            index: None,
            prefilter: None,
            output_limit,
        },
    )
}

/// Format a rooted Scheme `marlin-search-dsl-eval-edb` answer vector alongside
/// ASP generation evidence and any applied AST repair hook receipts.
pub fn format_rooted_search_dsl_result(
    value: RootedSchemeObject<'_>,
    snapshot: &WorkspaceEdbSnapshot,
    output_limit: usize,
) -> Result<String, SearchPlanProjectionError> {
    Ok(decode_rooted_search_dsl_result(value, snapshot, output_limit)?.rendered_text)
}
