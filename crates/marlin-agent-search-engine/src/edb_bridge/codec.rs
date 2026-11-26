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

use super::{
    OwnedWorkspaceEdbPrefilter, SearchDslCompletionEntry, SearchDslCompletionKind,
    SearchDslExecutionMode, SearchDslStructuredResult, SearchDslStructuredRow,
};
use crate::SearchPlanProjectionError;

const MAX_TOKENS: usize = 64;
const MAX_COMPLETIONS: usize = 128;
const MAX_TEXT_BYTES: usize = 65_536;
const MAX_IDENTIFIER_BYTES: usize = 256;
const DEFAULT_MAX_EDB_FACTS: usize = 4096;

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

/// Decode Scheme's zero-regex `#(search-edb-prefilter languages path-prefix tokens max-files)`
/// rooted 5-vector into an owned [`OwnedWorkspaceEdbPrefilter`].
pub fn decode_edb_prefilter(
    value: RootedSchemeObject<'_>,
) -> Result<OwnedWorkspaceEdbPrefilter, SearchPlanProjectionError> {
    let tuple = native(value.into_collection(CollectionKind::Vector))?;
    if tuple.len() != 5 {
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
    let path_prefix = decode_optional_string(native(tuple.element(2))?, MAX_TEXT_BYTES)?;
    let required_tokens = decode_string_vector(native(tuple.element(3))?, MAX_TOKENS)?;
    let max_files = decode_usize(native(tuple.element(4))?)?.max(1);
    Ok(OwnedWorkspaceEdbPrefilter {
        languages,
        documents: Vec::new(),
        path_prefix,
        extension: None,
        required_tokens,
        max_files,
        max_facts: DEFAULT_MAX_EDB_FACTS,
    })
}

/// Encode an ASP [`WorkspaceEdbSnapshot`] into a rooted Scheme 4-vector
/// `#(files-vec entities-vec edges-vec attrs-vec)` matching `semantic-open-edb.v1`
/// with zero JSON serialization.
pub fn encode_edb_snapshot<'a>(
    runtime: &'a GerbilRuntime,
    snapshot: &WorkspaceEdbSnapshot,
) -> Result<RootedSchemeObject<'a>, SearchPlanProjectionError> {
    let mut file_rows = Vec::with_capacity(snapshot.files.len());
    for f in &snapshot.files {
        file_rows.push(make_vector(
            runtime,
            &[
                make_fixnum(runtime, usize::try_from(f.file_id).unwrap_or(0))?,
                make_string(runtime, f.language.as_deref().unwrap_or("unknown"))?,
                make_string(runtime, &f.path)?,
            ],
        )?);
    }

    let mut entity_rows = Vec::with_capacity(snapshot.entities.len());
    let mut attr_rows = Vec::new();
    for e in &snapshot.entities {
        entity_rows.push(make_vector(
            runtime,
            &[
                make_string(runtime, &e.entity_id)?,
                make_fixnum(runtime, usize::try_from(e.file_id).unwrap_or(0))?,
                make_string(runtime, &e.language)?,
                make_string(runtime, &e.kind)?,
                make_string(runtime, &e.language_kind)?,
                make_string(runtime, &e.name)?,
                make_fixnum(runtime, e.line)?,
            ],
        )?);
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

    let mut edge_rows = Vec::with_capacity(snapshot.edges.len());
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

    make_vector(
        runtime,
        &[
            make_vector(runtime, &file_rows)?,
            make_vector(runtime, &entity_rows)?,
            make_vector(runtime, &edge_rows)?,
            make_vector(runtime, &attr_rows)?,
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

fn format_scalar_cell(value: RootedSchemeObject<'_>) -> Result<String, SearchPlanProjectionError> {
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

fn infer_path_and_line(
    cells: &[String],
    known_paths: &HashSet<&str>,
    file_id_map: &HashMap<usize, &str>,
) -> (Option<String>, Option<usize>) {
    let mut path = cells
        .iter()
        .find(|cell| known_paths.contains(cell.as_str()))
        .cloned();
    let mut line = None;
    for cell in cells {
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

fn format_snapshot_header(snapshot: &WorkspaceEdbSnapshot) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "generation: {}\nelapsed_us: {}\nindexed_files: {}\nmatched_files: {}",
        snapshot.generation_digest.as_str(),
        snapshot.elapsed_micros,
        snapshot.indexed_files,
        snapshot.files.len()
    );
    out
}

fn decode_parse_artifact_result(
    mut elems: std::vec::IntoIter<RootedSchemeObject<'_>>,
    snapshot: &WorkspaceEdbSnapshot,
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
    let mut rendered_text = format_snapshot_header(snapshot);
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
        rows: vec![row],
        rendered_text,
    })
}

fn decode_ascent_search_result(
    mut elems: std::vec::IntoIter<RootedSchemeObject<'_>>,
    snapshot: &WorkspaceEdbSnapshot,
    output_limit: usize,
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
    let payload = native(payload_obj.into_collection(CollectionKind::Vector))?;
    let repairs = decode_symbol_vector(repairs_obj, MAX_TOKENS)?;

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

    let mut rendered_text = format_snapshot_header(snapshot);
    let _ = writeln!(
        rendered_text,
        "mode: {}\nrelation: {relation}\nresults: {}",
        mode.as_str(),
        payload.len()
    );
    if !repairs.is_empty() {
        let _ = writeln!(rendered_text, "repairs: {}", repairs.join(", "));
    }

    let mut rows = Vec::with_capacity(payload.len());
    for i in 0..payload.len() {
        let (cells, display) = decode_row_cells(native(payload.element(i))?)?;
        let line_str = format!("- {display}\n");
        if rendered_text.len() + line_str.len() <= output_limit {
            rendered_text.push_str(&line_str);
        }
        let (path, line) = infer_path_and_line(&cells, &known_paths, &file_id_map);
        rows.push(SearchDslStructuredRow {
            cells,
            display,
            path,
            line,
        });
    }

    Ok(SearchDslStructuredResult {
        generation_digest: snapshot.generation_digest.as_str().to_owned(),
        elapsed_micros: snapshot.elapsed_micros,
        indexed_files: snapshot.indexed_files,
        matched_files: snapshot.files.len(),
        mode,
        relation,
        repairs,
        rows,
        rendered_text,
    })
}

/// Decode a rooted Scheme `marlin-search-dsl-eval-edb` answer vector into a
/// structured [`SearchDslStructuredResult`] carrying typed rows, path/line
/// navigation coordinates, repair tags, and bounded rendered text.
pub fn decode_rooted_search_dsl_result(
    value: RootedSchemeObject<'_>,
    snapshot: &WorkspaceEdbSnapshot,
    output_limit: usize,
) -> Result<SearchDslStructuredResult, SearchPlanProjectionError> {
    let tuple = native(value.into_collection(CollectionKind::Vector))?;
    if tuple.len() != 5 {
        return Err(SearchPlanProjectionError::Invalid(
            "unexpected search DSL result arity",
        ));
    }
    let mut elems = Vec::with_capacity(5);
    for i in 0..tuple.len() {
        elems.push(native(tuple.element(i))?);
    }
    let mut iter = elems.into_iter();
    let tag = decode_symbol(
        iter.next()
            .ok_or(SearchPlanProjectionError::Invalid("tag"))?,
    )?;
    match tag.as_str() {
        "parse-artifact-summary" => decode_parse_artifact_result(iter, snapshot),
        "search-result" => decode_ascent_search_result(iter, snapshot, output_limit),
        _ => Err(SearchPlanProjectionError::Invalid(
            "unexpected search DSL result shape",
        )),
    }
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
