// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Zero-regex Open EDB hydrator and BLAKE3 `content_hash`-verified cache bridge
//! for [`WorkspaceSearchIndex`].
//!
//! Hydrates live workspace files automatically with zero regular expressions:
//! 1. Restores matching `(path, content_hash)` Open EDB facts from disk cache
//!    (skipping modified/deleted files fail-closed).
//! 2. Extracts canonical Org AST facts via Marlin's [`parse_live_org_nodes`]
//!    (`marlin-context-org-model`).
//! 3. Extracts native Gerbil Scheme (`.ss`), SysML v2 (`.sysml`), and Quint (`.qnt`)
//!    AST facts via `scheme/search-dsl/extract.ss` (`marlin-search-dsl-extract-facts`)
//!    over the zero-JSON `RootedSchemeObject` C-ABI.
//! 4. Persists the updated content-hash-verified Open EDB cache atomically.

use std::path::Path;

use agent_semantic_search::{
    SEMANTIC_OPEN_EDB_SCHEMA_ID, SEMANTIC_OPEN_EDB_SCHEMA_VERSION, WorkspaceEdbAttr,
    WorkspaceEdbEdgeFact, WorkspaceEdbEntityFact, WorkspaceOpenEdbIngestBatch,
    WorkspaceOpenEdbPendingFile, WorkspaceSearchIndex,
};
use gerbil_scheme::{CollectionKind, GerbilRuntime, RootedSchemeObject};
use marlin_context_org_model::OrgNodeKind;

use crate::SearchPlanProjectionError;
use crate::edb_bridge::{decode_string, decode_usize, make_string, make_vector, native};
use crate::org_port::parse_live_org_nodes;

const MAX_EXTRACTED_FACTS: usize = 32_768;
const MAX_TEXT_BYTES: usize = 65_536;
const MAX_IDENTIFIER_BYTES: usize = 256;
const SCHEME_HYDRATE_LANGUAGES: [&str; 3] = ["scheme", "sysml", "quint"];

/// Typed domain identifier for a fact-producing Open EDB provider.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct OpenEdbProviderId(String);

impl OpenEdbProviderId {
    /// Validate and wrap a non-empty Open EDB provider identifier.
    pub fn new(raw: impl Into<String>) -> Result<Self, SearchPlanProjectionError> {
        let value = raw.into();
        if value.trim().is_empty() || value.len() > MAX_IDENTIFIER_BYTES {
            return Err(SearchPlanProjectionError::Invalid(
                "invalid Open EDB provider identifier",
            ));
        }
        Ok(Self(value))
    }

    /// Borrow the canonical provider identifier string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Deterministic receipt returned by workspace Open EDB hydration passes.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WorkspaceOpenEdbHydrationReceipt {
    pub restored_files: usize,
    pub restored_entities: usize,
    pub org_files_hydrated: usize,
    pub scheme_files_hydrated: usize,
    pub total_ingested_entities: usize,
    pub total_ingested_edges: usize,
    pub cache_saved: bool,
}

fn collect_org_heading_lines(body: &str) -> Vec<usize> {
    body.lines()
        .enumerate()
        .filter_map(|(idx, line)| {
            let stars = line.bytes().take_while(|b| *b == b'*').count();
            (stars > 0 && line.get(stars..).is_some_and(|r| r.starts_with(' '))).then_some(idx + 1)
        })
        .collect()
}

/// Extract canonical Open EDB facts from `.org` files using Marlin's zero-regex
/// [`parse_live_org_nodes`] AST parser.
#[must_use]
pub fn extract_org_open_edb_batch(
    files: &[WorkspaceOpenEdbPendingFile],
) -> WorkspaceOpenEdbIngestBatch {
    let mut entities = Vec::new();
    for file in files {
        let nodes = parse_live_org_nodes(&file.body);
        let heading_lines = collect_org_heading_lines(&file.body);
        let mut heading_cursor = 0usize;
        for node in nodes {
            if node.kind != OrgNodeKind::Heading {
                continue;
            }
            let line = heading_lines.get(heading_cursor).copied().unwrap_or(1);
            heading_cursor += 1;
            let Some(name) = node.title.filter(|t| !t.trim().is_empty()) else {
                continue;
            };
            let attrs: Vec<WorkspaceEdbAttr> = node
                .properties
                .into_iter()
                .map(|(key, value)| WorkspaceEdbAttr { key, value })
                .collect();
            entities.push(WorkspaceEdbEntityFact {
                entity_id: String::new(),
                file_id: 0,
                path: file.path.clone(),
                line,
                name,
                language: "org".to_owned(),
                kind: "heading".to_owned(),
                language_kind: "org_heading".to_owned(),
                source: "gerbil-parser".to_owned(),
                attrs,
            });
        }
    }
    WorkspaceOpenEdbIngestBatch {
        schema_id: Some(SEMANTIC_OPEN_EDB_SCHEMA_ID.to_owned()),
        schema_version: Some(SEMANTIC_OPEN_EDB_SCHEMA_VERSION.to_owned()),
        language_id: Some("org".to_owned()),
        provider_id: Some("marlin-org-model".to_owned()),
        entities,
        edges: Vec::new(),
    }
}

fn decode_extracted_attrs(
    value: RootedSchemeObject<'_>,
) -> Result<Vec<WorkspaceEdbAttr>, SearchPlanProjectionError> {
    let pairs = native(value.into_collection(CollectionKind::Vector))?;
    let mut attrs = Vec::with_capacity(pairs.len());
    for i in 0..pairs.len() {
        let pair = native(native(pairs.element(i))?.into_collection(CollectionKind::Vector))?;
        if pair.len() != 2 {
            return Err(SearchPlanProjectionError::Invalid(
                "extracted attr tuple arity",
            ));
        }
        attrs.push(WorkspaceEdbAttr {
            key: decode_string(native(pair.element(0))?, MAX_IDENTIFIER_BYTES)?,
            value: decode_string(native(pair.element(1))?, MAX_TEXT_BYTES)?,
        });
    }
    Ok(attrs)
}

fn decode_extracted_entities(
    value: RootedSchemeObject<'_>,
) -> Result<Vec<WorkspaceEdbEntityFact>, SearchPlanProjectionError> {
    let rows = native(value.into_collection(CollectionKind::Vector))?;
    if rows.len() > MAX_EXTRACTED_FACTS {
        return Err(SearchPlanProjectionError::Invalid(
            "extracted entities exceed bound",
        ));
    }
    let mut out = Vec::with_capacity(rows.len());
    for i in 0..rows.len() {
        let row = native(native(rows.element(i))?.into_collection(CollectionKind::Vector))?;
        if row.len() != 8 {
            return Err(SearchPlanProjectionError::Invalid(
                "extracted entity tuple arity",
            ));
        }
        out.push(WorkspaceEdbEntityFact {
            entity_id: String::new(),
            file_id: 0,
            path: decode_string(native(row.element(0))?, MAX_TEXT_BYTES)?,
            line: decode_usize(native(row.element(1))?)?.max(1),
            name: decode_string(native(row.element(2))?, MAX_TEXT_BYTES)?,
            language: decode_string(native(row.element(3))?, MAX_IDENTIFIER_BYTES)?,
            kind: decode_string(native(row.element(4))?, MAX_IDENTIFIER_BYTES)?,
            language_kind: decode_string(native(row.element(5))?, MAX_IDENTIFIER_BYTES)?,
            source: decode_string(native(row.element(6))?, MAX_IDENTIFIER_BYTES)?,
            attrs: decode_extracted_attrs(native(row.element(7))?)?,
        });
    }
    Ok(out)
}

fn decode_extracted_edges(
    value: RootedSchemeObject<'_>,
) -> Result<Vec<WorkspaceEdbEdgeFact>, SearchPlanProjectionError> {
    let rows = native(value.into_collection(CollectionKind::Vector))?;
    if rows.len() > MAX_EXTRACTED_FACTS {
        return Err(SearchPlanProjectionError::Invalid(
            "extracted edges exceed bound",
        ));
    }
    let mut out = Vec::with_capacity(rows.len());
    for i in 0..rows.len() {
        let row = native(native(rows.element(i))?.into_collection(CollectionKind::Vector))?;
        if row.len() != 5 {
            return Err(SearchPlanProjectionError::Invalid(
                "extracted edge tuple arity",
            ));
        }
        out.push(WorkspaceEdbEdgeFact {
            src: decode_string(native(row.element(0))?, MAX_TEXT_BYTES)?,
            relation: decode_string(native(row.element(1))?, MAX_IDENTIFIER_BYTES)?,
            dst: decode_string(native(row.element(2))?, MAX_TEXT_BYTES)?,
            path: decode_string(native(row.element(3))?, MAX_TEXT_BYTES)?,
            line: decode_usize(native(row.element(4))?)?,
        });
    }
    Ok(out)
}

/// Decode Scheme's `marlin-search-dsl-extract-facts` rooted 2-vector
/// `#(entities-vec edges-vec)` into a [`WorkspaceOpenEdbIngestBatch`].
pub fn decode_extracted_open_edb_batch(
    value: RootedSchemeObject<'_>,
    provider_id: &OpenEdbProviderId,
) -> Result<WorkspaceOpenEdbIngestBatch, SearchPlanProjectionError> {
    let pair = native(value.into_collection(CollectionKind::Vector))?;
    if pair.len() != 2 {
        return Err(SearchPlanProjectionError::Invalid(
            "extract-facts result must be a 2-vector",
        ));
    }
    Ok(WorkspaceOpenEdbIngestBatch {
        schema_id: Some(SEMANTIC_OPEN_EDB_SCHEMA_ID.to_owned()),
        schema_version: Some(SEMANTIC_OPEN_EDB_SCHEMA_VERSION.to_owned()),
        language_id: None,
        provider_id: Some(provider_id.as_str().to_owned()),
        entities: decode_extracted_entities(native(pair.element(0))?)?,
        edges: decode_extracted_edges(native(pair.element(1))?)?,
    })
}

fn encode_pending_files<'a>(
    runtime: &'a GerbilRuntime,
    files: &[WorkspaceOpenEdbPendingFile],
) -> Result<RootedSchemeObject<'a>, SearchPlanProjectionError> {
    let mut rows = Vec::with_capacity(files.len());
    for file in files {
        rows.push(make_vector(
            runtime,
            &[
                make_string(runtime, &file.path)?,
                make_string(runtime, &file.language)?,
                make_string(runtime, &file.body)?,
            ],
        )?);
    }
    make_vector(runtime, &rows)
}

fn ingest_extracted_scheme_batch(
    index: &mut WorkspaceSearchIndex,
    cache_path: &Path,
    raw: RootedSchemeObject<'_>,
    receipt: &mut WorkspaceOpenEdbHydrationReceipt,
) -> Result<(), SearchPlanProjectionError> {
    let provider = OpenEdbProviderId::new("marlin-gerbil-parser")?;
    let batch = decode_extracted_open_edb_batch(raw, &provider)?;
    let ing = index.ingest_open_edb_facts(batch);
    receipt.scheme_files_hydrated = ing.updated_files;
    receipt.total_ingested_entities += ing.ingested_entities;
    receipt.total_ingested_edges += ing.ingested_edges;
    if ing.updated_files > 0 {
        receipt.cache_saved = index.save_open_edb_cache(cache_path).is_ok();
    }
    Ok(())
}

/// Restore matching BLAKE3-verified Open EDB cache entries and hydrate any
/// unhydrated `.org` files via Marlin's zero-regex Org AST parser.
pub fn hydrate_workspace_open_edb_org_and_cache(
    index: &mut WorkspaceSearchIndex,
    cache_path: &Path,
) -> WorkspaceOpenEdbHydrationReceipt {
    let mut receipt = WorkspaceOpenEdbHydrationReceipt::default();
    if cache_path.is_file()
        && let Ok(restored) = index.load_and_restore_open_edb_cache(cache_path)
    {
        receipt.restored_files = restored.restored_files;
        receipt.restored_entities = restored.restored_entities;
    }

    let pending_org = index.live_files_needing_open_edb(&["org"], 512);
    if !pending_org.is_empty() {
        let batch = extract_org_open_edb_batch(&pending_org);
        if !batch.entities.is_empty() {
            let ing = index.ingest_open_edb_facts(batch);
            receipt.org_files_hydrated = ing.updated_files;
            receipt.total_ingested_entities += ing.ingested_entities;
            receipt.total_ingested_edges += ing.ingested_edges;
            receipt.cache_saved = index.save_open_edb_cache(cache_path).is_ok();
        }
    }
    receipt
}

#[cfg(not(feature = "linked-ui"))]
fn resolve_package_extract_export<'a>(
    runtime: &'a GerbilRuntime,
    scheme_dir: &str,
) -> Result<RootedSchemeObject<'a>, SearchPlanProjectionError> {
    let module = native(runtime.intern_identifier(
        gerbil_scheme::IdentifierKind::Symbol,
        ":marlin-agent-core/c-abi/search-dsl",
    ))?;
    let sym = native(runtime.intern_identifier(
        gerbil_scheme::IdentifierKind::Symbol,
        "marlin-search-dsl-extract-facts",
    ))?;
    native(runtime.resolve_package_export(scheme_dir, &module, &sym))
}

/// Restore cache, hydrate `.org` AST facts, and hydrate pending `.ss`/`.sysml`/`.qnt`
/// files through the compiled Scheme package `marlin-search-dsl-extract-facts`.
#[cfg(not(feature = "linked-ui"))]
pub fn hydrate_package_workspace_open_edb(
    runtime: &GerbilRuntime,
    scheme_dir: &str,
    index: &mut WorkspaceSearchIndex,
    cache_path: &Path,
) -> Result<WorkspaceOpenEdbHydrationReceipt, SearchPlanProjectionError> {
    let mut receipt = hydrate_workspace_open_edb_org_and_cache(index, cache_path);
    let pending = index.live_files_needing_open_edb(&SCHEME_HYDRATE_LANGUAGES, 512);
    if pending.is_empty() {
        return Ok(receipt);
    }
    let extract_fn = resolve_package_extract_export(runtime, scheme_dir)?;
    let payload = encode_pending_files(runtime, &pending)?;
    let raw = native(runtime.call_rooted(&extract_fn, &payload))?;
    ingest_extracted_scheme_batch(index, cache_path, raw, &mut receipt)?;
    Ok(receipt)
}

/// Restore cache, hydrate `.org` AST facts, and hydrate pending `.ss`/`.sysml`/`.qnt`
/// files through an AOT-linked `marlin_search_dsl_extract_facts` export.
///
/// # Safety
/// `extract_export` must originate from `runtime`'s live Marlin programme.
pub unsafe fn hydrate_linked_workspace_open_edb(
    runtime: &GerbilRuntime,
    extract_export: unsafe extern "C" fn(i64) -> i64,
    index: &mut WorkspaceSearchIndex,
    cache_path: &Path,
) -> Result<WorkspaceOpenEdbHydrationReceipt, SearchPlanProjectionError> {
    let mut receipt = hydrate_workspace_open_edb_org_and_cache(index, cache_path);
    let pending = index.live_files_needing_open_edb(&SCHEME_HYDRATE_LANGUAGES, 512);
    if pending.is_empty() {
        return Ok(receipt);
    }
    let extract_fn = native(unsafe { runtime.bind_object_export(extract_export) })?;
    let payload = encode_pending_files(runtime, &pending)?;
    let raw = native(extract_fn.call(&payload).into_result())?;
    ingest_extracted_scheme_batch(index, cache_path, raw, &mut receipt)?;
    Ok(receipt)
}
