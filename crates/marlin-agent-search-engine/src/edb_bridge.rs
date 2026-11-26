// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Rooted Gerbil Scheme C-ABI bridge for Marlin's native `:gerbil-ascent` +
//! `:gerbil-parser` + POO Search DSL (`scheme/search-dsl` + `scheme/c-abi/search-dsl.ss`).
//!
//! Zero JSON serialization, zero regexes, and zero hardcoded language parsing live here:
//! - Scheme owns all POO language fragments over ASP's universal `semantic-open-edb.v1`
//!   `(lang, kind, language_kind)` protocol, real `:gerbil-ascent` rules, `:gerbil-parser`
//!   sub-syntaxes, Dusa finite-choice reasoning, and the Safe Pre-Tool-Call Model AST
//!   Repair Hook (`scheme/search-dsl/repair.ss`).
//! - ASP Rust runs sub-millisecond trigram prefiltering and streams generation-scoped
//!   Open EDB facts via [`WorkspaceSearchIndex::project_prefiltered_edb`].

use std::fmt::{Display, Write as _};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use agent_semantic_search::{WorkspaceEdbPrefilter, WorkspaceEdbSnapshot, WorkspaceSearchIndex};
use gerbil_scheme::{
    CollectionKind, GerbilRuntime, IdentifierBuffer, IdentifierKind, ObjectKind,
    RootedSchemeObject, SchemeScalar,
};

use crate::SearchPlanProjectionError;

const MAX_TOKENS: usize = 64;
const MAX_COMPLETIONS: usize = 128;
const MAX_TEXT_BYTES: usize = 65_536;
const MAX_IDENTIFIER_BYTES: usize = 256;
const DEFAULT_MAX_EDB_FACTS: usize = 4096;

/// Owned storage backing ASP's zero-regex [`WorkspaceEdbPrefilter`].
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OwnedWorkspaceEdbPrefilter {
    pub languages: Vec<String>,
    pub documents: Vec<String>,
    pub path_prefix: Option<String>,
    pub extension: Option<String>,
    pub required_tokens: Vec<String>,
    pub max_files: usize,
    pub max_facts: usize,
}

impl OwnedWorkspaceEdbPrefilter {
    /// Borrow as ASP's [`WorkspaceEdbPrefilter`].
    #[must_use]
    pub fn as_prefilter(&self) -> WorkspaceEdbPrefilter<'_> {
        WorkspaceEdbPrefilter {
            languages: &self.languages,
            documents: &self.documents,
            path_prefix: self.path_prefix.as_deref(),
            extension: self.extension.as_deref(),
            required_tokens: &self.required_tokens,
            max_files: self.max_files,
            max_facts: self.max_facts,
        }
    }
}

/// Semantic category of an interactive Helm TAB completion row.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum SearchDslCompletionKind {
    Ascent,
    Dusa,
    Parser,
    Edb,
    Relation,
}

impl SearchDslCompletionKind {
    /// Canonical lowercase label for UI columns and protocol matching.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ascent => "ascent",
            Self::Dusa => "dusa",
            Self::Parser => "parser",
            Self::Edb => "edb",
            Self::Relation => "relation",
        }
    }

    fn parse(raw: &str) -> Result<Self, SearchPlanProjectionError> {
        match raw {
            "ascent" => Ok(Self::Ascent),
            "dusa" => Ok(Self::Dusa),
            "parser" => Ok(Self::Parser),
            "edb" => Ok(Self::Edb),
            "relation" => Ok(Self::Relation),
            _ => Err(SearchPlanProjectionError::Invalid(
                "unknown search DSL completion kind",
            )),
        }
    }
}

/// One interactive Helm TAB completion row introspected from Scheme's POO
/// `SearchLanguageRegistry.` and real `:gerbil-ascent` relation declarations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SearchDslCompletionEntry {
    pub form: String,
    pub kind: SearchDslCompletionKind,
    pub signature: String,
    pub language: String,
    pub doc: String,
    pub insert_text: String,
}

/// Typed request executing a native `:gerbil-ascent` query against a resident
/// [`WorkspaceSearchIndex`].
#[derive(Clone, Copy)]
pub struct SearchDslExecutionRequest<'a> {
    pub index: &'a WorkspaceSearchIndex,
    pub query: &'a str,
    pub output_limit: usize,
}

/// AOT-linked C-ABI exports from `scheme/c-abi/search-dsl.ss`.
#[derive(Clone, Copy)]
pub struct SearchDslLinkedExports {
    pub compile_prefilter: unsafe extern "C" fn(i64) -> i64,
    pub eval_edb: unsafe extern "C" fn(i64) -> i64,
    pub completions: unsafe extern "C" fn(i64) -> i64,
}

/// Host-supplied runner executing a native `:gerbil-ascent` query against a
/// shared resident [`WorkspaceSearchIndex`] and returning formatted evidence.
pub type SearchDslRunner = Arc<
    dyn Fn(
            Arc<RwLock<WorkspaceSearchIndex>>,
            &str,
            usize,
        ) -> Result<String, SearchPlanProjectionError>
        + Send
        + Sync,
>;

fn native<T>(result: Result<T, impl Display>) -> Result<T, SearchPlanProjectionError> {
    result.map_err(|error| SearchPlanProjectionError::Native(error.to_string()))
}

fn decode_symbol(value: RootedSchemeObject<'_>) -> Result<String, SearchPlanProjectionError> {
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

fn decode_string(
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

fn decode_usize(value: RootedSchemeObject<'_>) -> Result<usize, SearchPlanProjectionError> {
    match native(value.as_ref().scalar())? {
        SchemeScalar::Fixnum(v) if v >= 0 => {
            usize::try_from(v).map_err(|_| SearchPlanProjectionError::Invalid("fixnum overflow"))
        }
        _ => Err(SearchPlanProjectionError::Invalid(
            "expected non-negative fixnum",
        )),
    }
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

fn make_vector<'a>(
    runtime: &'a GerbilRuntime,
    items: &[RootedSchemeObject<'a>],
) -> Result<RootedSchemeObject<'a>, SearchPlanProjectionError> {
    let refs: Vec<&RootedSchemeObject<'a>> = items.iter().collect();
    Ok(native(runtime.collection_from_objects(CollectionKind::Vector, &refs))?.into_object())
}

fn make_string<'a>(
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

/// Encode an ASP [`WorkspaceEdbSnapshot`] into a rooted Scheme 4-vector
/// `#(files-vec entities-vec edges-vec attrs-vec)` matching `semantic-open-edb.v1`
/// with zero JSON serialization:
/// - `file`: `#(file_id lang path)` (3 cols)
/// - `entity`: `#(entity_id file_id language kind language_kind name line)` (7 cols)
/// - `edge`: `#(src dst relation path line)` (5 cols)
/// - `attr`: `#(entity_id key value)` (3 cols)
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

fn format_parse_artifact_summary(
    mut elems: std::vec::IntoIter<RootedSchemeObject<'_>>,
    out: &mut String,
) -> Result<(), SearchPlanProjectionError> {
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
    let _ = writeln!(
        out,
        "mode: gerbil-parser\nlanguage: {language}\nstatus: {status}\nroot: {root_kind}\ndiagnostics: {diag_count}"
    );
    Ok(())
}

fn format_ascent_search_result(
    mut elems: std::vec::IntoIter<RootedSchemeObject<'_>>,
    output_limit: usize,
    out: &mut String,
) -> Result<(), SearchPlanProjectionError> {
    let mode = decode_symbol(
        elems
            .next()
            .ok_or(SearchPlanProjectionError::Invalid("mode"))?,
    )?;
    let rel = decode_symbol(
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
    let _ = writeln!(
        out,
        "mode: {mode}\nrelation: {rel}\nresults: {}",
        payload.len()
    );
    if !repairs.is_empty() {
        let _ = writeln!(out, "repairs: {}", repairs.join(", "));
    }
    for i in 0..payload.len() {
        let row = format_scalar_cell(native(payload.element(i))?)?;
        let line = format!("- {row}\n");
        if out.len() + line.len() > output_limit {
            break;
        }
        out.push_str(&line);
    }
    Ok(())
}

/// Format a rooted Scheme `marlin-search-dsl-eval-edb` answer vector alongside
/// ASP generation evidence and any applied AST repair hook receipts.
pub fn format_rooted_search_dsl_result(
    value: RootedSchemeObject<'_>,
    snapshot: &WorkspaceEdbSnapshot,
    output_limit: usize,
) -> Result<String, SearchPlanProjectionError> {
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
    let mut out = format_snapshot_header(snapshot);
    match tag.as_str() {
        "parse-artifact-summary" => format_parse_artifact_summary(iter, &mut out)?,
        "search-result" => format_ascent_search_result(iter, output_limit, &mut out)?,
        _ => {
            return Err(SearchPlanProjectionError::Invalid(
                "unexpected search DSL result shape",
            ));
        }
    }
    Ok(out)
}

/// Execute a native `:gerbil-ascent` query through AOT-linked C-ABI exports
/// over a resident [`WorkspaceSearchIndex`].
///
/// # Safety
/// `exports` must originate from `runtime`'s live Marlin programme.
pub unsafe fn execute_linked_search_dsl(
    runtime: &GerbilRuntime,
    exports: SearchDslLinkedExports,
    request: &SearchDslExecutionRequest<'_>,
) -> Result<String, SearchPlanProjectionError> {
    let compile_fn = native(unsafe { runtime.bind_object_export(exports.compile_prefilter) })?;
    let query_obj = make_string(runtime, request.query)?;
    let prefilter_obj = native(compile_fn.call(&query_obj).into_result())?;
    let owned_prefilter = decode_edb_prefilter(prefilter_obj)?;
    let prefilter = owned_prefilter.as_prefilter();

    let snapshot = request.index.project_prefiltered_edb(&prefilter);
    let edb_obj = encode_edb_snapshot(runtime, &snapshot)?;
    let eval_input = make_vector(runtime, &[query_obj, edb_obj])?;

    let eval_fn = native(unsafe { runtime.bind_object_export(exports.eval_edb) })?;
    let result_obj = native(eval_fn.call(&eval_input).into_result())?;
    format_rooted_search_dsl_result(result_obj, &snapshot, request.output_limit)
}

/// Project context-aware Helm TAB completion rows through an AOT-linked
/// `marlin_search_dsl_completions` export.
///
/// # Safety
/// `export` must originate from `runtime`'s live Marlin programme.
pub unsafe fn project_linked_search_dsl_completions(
    runtime: &GerbilRuntime,
    export: unsafe extern "C" fn(i64) -> i64,
    prefix: &str,
) -> Result<Vec<SearchDslCompletionEntry>, SearchPlanProjectionError> {
    let proc = native(unsafe { runtime.bind_object_export(export) })?;
    let prefix_obj = make_string(runtime, prefix)?;
    let rows = native(proc.call(&prefix_obj).into_result())?;
    decode_search_dsl_completions(rows)
}

/// Locate the compiled Marlin `scheme/` directory for non-AOT package execution.
#[must_use]
pub fn default_marlin_scheme_dir() -> Option<PathBuf> {
    if let Ok(env_dir) = std::env::var("MARLIN_SCHEME_DIR") {
        let candidate = PathBuf::from(env_dir);
        if candidate.join("gerbil.pkg").is_file() {
            return Some(candidate);
        }
    }
    let manifest_scheme = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("scheme");
    if manifest_scheme.join("gerbil.pkg").is_file() {
        return Some(manifest_scheme);
    }
    let cwd_scheme = std::env::current_dir().ok()?.join("scheme");
    cwd_scheme
        .join("gerbil.pkg")
        .is_file()
        .then_some(cwd_scheme)
}

#[cfg(not(feature = "linked-ui"))]
fn resolve_package_search_dsl_exports<'a>(
    runtime: &'a GerbilRuntime,
    scheme_dir: &str,
) -> Result<(RootedSchemeObject<'a>, RootedSchemeObject<'a>), SearchPlanProjectionError> {
    let module = native(runtime.intern_identifier(
        IdentifierKind::Symbol,
        ":marlin-agent-core/c-abi/search-dsl",
    ))?;
    let compile_sym = native(runtime.intern_identifier(
        IdentifierKind::Symbol,
        "marlin-search-dsl-compile-prefilter",
    ))?;
    let eval_sym =
        native(runtime.intern_identifier(IdentifierKind::Symbol, "marlin-search-dsl-eval-edb"))?;
    let compile_fn = native(runtime.resolve_package_export(scheme_dir, &module, &compile_sym))?;
    let eval_fn = native(runtime.resolve_package_export(scheme_dir, &module, &eval_sym))?;
    Ok((compile_fn, eval_fn))
}

/// Execute a native `:gerbil-ascent` query by resolving
/// `:marlin-agent-core/c-abi/search-dsl` from a compiled Gerbil package tree.
#[cfg(not(feature = "linked-ui"))]
pub fn execute_package_search_dsl(
    runtime: &GerbilRuntime,
    scheme_dir: &str,
    request: &SearchDslExecutionRequest<'_>,
) -> Result<String, SearchPlanProjectionError> {
    let (compile_fn, eval_fn) = resolve_package_search_dsl_exports(runtime, scheme_dir)?;
    let query_obj = make_string(runtime, request.query)?;
    let prefilter_obj = native(runtime.call_rooted(&compile_fn, &query_obj))?;
    let owned_prefilter = decode_edb_prefilter(prefilter_obj)?;
    let snapshot = request
        .index
        .project_prefiltered_edb(&owned_prefilter.as_prefilter());
    let edb_obj = encode_edb_snapshot(runtime, &snapshot)?;
    let eval_input = make_vector(runtime, &[query_obj, edb_obj])?;
    let result_obj = native(runtime.call_rooted(&eval_fn, &eval_input))?;
    format_rooted_search_dsl_result(result_obj, &snapshot, request.output_limit)
}

/// Project context-aware Helm TAB completion rows from a compiled package directory.
#[cfg(not(feature = "linked-ui"))]
pub fn project_package_search_dsl_completions(
    runtime: &GerbilRuntime,
    scheme_dir: &str,
    prefix: &str,
) -> Result<Vec<SearchDslCompletionEntry>, SearchPlanProjectionError> {
    let module = native(runtime.intern_identifier(
        IdentifierKind::Symbol,
        ":marlin-agent-core/c-abi/search-dsl",
    ))?;
    let comp_sym =
        native(runtime.intern_identifier(IdentifierKind::Symbol, "marlin-search-dsl-completions"))?;
    let comp_fn = native(runtime.resolve_package_export(scheme_dir, &module, &comp_sym))?;
    let prefix_obj = make_string(runtime, prefix)?;
    let rows = native(runtime.call_rooted(&comp_fn, &prefix_obj))?;
    decode_search_dsl_completions(rows)
}

/// Return the default package-backed [`SearchDslRunner`] when `linked-ui` is not
/// active, or `None` under AOT `linked-ui` (where the host attaches the linked runner).
#[must_use]
pub fn default_search_dsl_runner() -> Option<SearchDslRunner> {
    #[cfg(not(feature = "linked-ui"))]
    {
        Some(default_package_search_dsl_runner())
    }
    #[cfg(feature = "linked-ui")]
    {
        None
    }
}

#[cfg(not(feature = "linked-ui"))]
struct PackageDslJob {
    scheme_dir: String,
    index: Arc<RwLock<WorkspaceSearchIndex>>,
    query: String,
    output_limit: usize,
    reply: std::sync::mpsc::SyncSender<Result<String, SearchPlanProjectionError>>,
}

#[cfg(not(feature = "linked-ui"))]
fn package_search_dsl_worker() -> &'static std::sync::mpsc::Sender<PackageDslJob> {
    use std::sync::{OnceLock, mpsc};

    static WORKER: OnceLock<mpsc::Sender<PackageDslJob>> = OnceLock::new();
    WORKER.get_or_init(|| {
        let (tx, rx) = mpsc::channel::<PackageDslJob>();
        std::thread::Builder::new()
            .name("marlin-search-dsl-vm".to_owned())
            .spawn(move || {
                let runtime = GerbilRuntime::initialize();
                while let Ok(job) = rx.recv() {
                    let outcome = match &runtime {
                        Ok(rt) => match job.index.read() {
                            Ok(guard) => {
                                let req = SearchDslExecutionRequest {
                                    index: &guard,
                                    query: &job.query,
                                    output_limit: job.output_limit,
                                };
                                execute_package_search_dsl(rt, &job.scheme_dir, &req)
                            }
                            Err(_) => Err(SearchPlanProjectionError::Invalid(
                                "search index lock poisoned",
                            )),
                        },
                        Err(err) => Err(SearchPlanProjectionError::Native(err.to_string())),
                    };
                    let _ = job.reply.send(outcome);
                }
            })
            .expect("spawn marlin-search-dsl-vm thread");
        tx
    })
}

/// Build a default [`SearchDslRunner`] backed by a dedicated thread-affine
/// [`GerbilRuntime`] worker and the compiled Marlin Scheme package when
/// `linked-ui` is not active.
#[cfg(not(feature = "linked-ui"))]
#[must_use]
pub fn default_package_search_dsl_runner() -> SearchDslRunner {
    Arc::new(|shared_index, query, output_limit| {
        let scheme_dir = default_marlin_scheme_dir().ok_or(SearchPlanProjectionError::Invalid(
            "compiled Marlin scheme/ directory not found (set MARLIN_SCHEME_DIR)",
        ))?;
        let scheme_dir_str = scheme_dir
            .to_str()
            .ok_or(SearchPlanProjectionError::Invalid(
                "non-UTF-8 Marlin scheme directory path",
            ))?
            .to_owned();
        let (reply_tx, reply_rx) = std::sync::mpsc::sync_channel(1);
        package_search_dsl_worker()
            .send(PackageDslJob {
                scheme_dir: scheme_dir_str,
                index: shared_index,
                query: query.to_owned(),
                output_limit,
                reply: reply_tx,
            })
            .map_err(|_| SearchPlanProjectionError::Invalid("search DSL worker thread stopped"))?;
        reply_rx
            .recv()
            .map_err(|_| SearchPlanProjectionError::Invalid("search DSL worker reply dropped"))?
    })
}
