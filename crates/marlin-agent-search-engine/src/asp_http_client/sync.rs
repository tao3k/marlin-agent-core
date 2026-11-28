// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Zero-JSON C-ABI dynamic POO descriptor synchronization and workspace Open EDB hydration.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::time::Instant;

use agent_semantic_search::{WorkspaceOpenEdbIngestBatch, WorkspaceSearchIndex};
#[cfg(not(feature = "linked-ui"))]
use gerbil_scheme::IdentifierKind;
use gerbil_scheme::{CollectionKind, GerbilRuntime, RootedSchemeObject, SchemeScalar};
use serde::{Deserialize, Serialize};

use crate::SearchPlanProjectionError;
use crate::edb_bridge::{decode_string, decode_usize, make_string, make_vector, native};
use crate::projection::AspHydrationPolicy;

use super::domain_types::{
    AspForwardingEndpoint, GeneralOntologyKindName, NativeAstKindName, TargetedJsonSchemaDocument,
};
#[cfg(not(feature = "linked-ui"))]
use super::protocol_types::AspHttpHydrationRequest;
use super::protocol_types::{
    AspHttpHydrationReceipt, DynamicLanguageDescriptorRequest, DynamicSpecialRelationSpec,
    ForwardedDslPack, ForwardedKindMapping, ForwardedProviderCatalog,
};
use super::transport::{fetch_asp_provider_catalog, http_loopback_request};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InspectFilePayload {
    path: String,
    source_utf8: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InspectFilesRequestPayload {
    files: Vec<InspectFilePayload>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ForwardedInspectFailureWire {
    #[serde(default)]
    _path: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ForwardedOpenEdbWireResponse {
    inspected_files: usize,
    elapsed_micros: u64,
    batches: Vec<WorkspaceOpenEdbIngestBatch>,
    #[serde(default)]
    failed_paths: Vec<ForwardedInspectFailureWire>,
}

fn encode_optional_str<'a>(
    runtime: &'a GerbilRuntime,
    value: Option<&str>,
) -> Result<RootedSchemeObject<'a>, SearchPlanProjectionError> {
    match value {
        Some(text) => make_string(runtime, text),
        None => native(runtime.scalar_object(SchemeScalar::Boolean(false))),
    }
}

fn encode_dynamic_descriptor<'a>(
    runtime: &'a GerbilRuntime,
    request: &DynamicLanguageDescriptorRequest<'_>,
) -> Result<RootedSchemeObject<'a>, SearchPlanProjectionError> {
    let ext_objs = request
        .extensions
        .iter()
        .map(|ext| make_string(runtime, ext))
        .collect::<Result<Vec<_>, _>>()?;
    let km_objs = request
        .kind_mappings
        .iter()
        .map(|km| {
            make_vector(
                runtime,
                &[
                    make_string(runtime, km.native_language_kind.as_str())?,
                    make_string(runtime, km.general_kind.as_str())?,
                    make_string(runtime, &km.description)?,
                ],
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let spec_objs = request
        .special_specs
        .iter()
        .map(|spec| {
            make_vector(
                runtime,
                &[
                    make_string(runtime, &spec.relation_name)?,
                    encode_optional_str(
                        runtime,
                        spec.kind_pattern
                            .as_ref()
                            .map(GeneralOntologyKindName::as_str),
                    )?,
                    encode_optional_str(
                        runtime,
                        spec.language_kind_pattern
                            .as_ref()
                            .map(NativeAstKindName::as_str),
                    )?,
                    make_string(runtime, &spec.description)?,
                ],
            )
        })
        .collect::<Result<Vec<_>, _>>()?;

    make_vector(
        runtime,
        &[
            make_string(runtime, request.language_id.as_str())?,
            make_string(runtime, request.provider_language_id.as_str())?,
            make_vector(runtime, &ext_objs)?,
            make_vector(runtime, &km_objs)?,
            make_vector(runtime, &spec_objs)?,
        ],
    )
}

struct RegisterDescriptorOutcome {
    total_relations: usize,
    added_entries: usize,
}

fn decode_register_receipt(
    receipt: RootedSchemeObject<'_>,
) -> Result<RegisterDescriptorOutcome, SearchPlanProjectionError> {
    let tuple = native(receipt.into_collection(CollectionKind::Vector))?;
    if tuple.len() != 3 && tuple.len() != 4 {
        return Err(SearchPlanProjectionError::Invalid(
            "dynamic register receipt must be a 3- or 4-vector",
        ));
    }
    let _lang = decode_string(native(tuple.element(1))?, 256)?;
    let total_relations = decode_usize(native(tuple.element(2))?)?;
    let added_entries = if tuple.len() == 4 {
        decode_usize(native(tuple.element(3))?)?
    } else {
        1
    };
    Ok(RegisterDescriptorOutcome {
        total_relations,
        added_entries,
    })
}

unsafe fn register_linked_descriptor_outcome(
    runtime: &GerbilRuntime,
    export: unsafe extern "C" fn(i64) -> i64,
    request: &DynamicLanguageDescriptorRequest<'_>,
) -> Result<RegisterDescriptorOutcome, SearchPlanProjectionError> {
    let reg_fn = native(unsafe { runtime.bind_object_export(export) })?;
    let payload = encode_dynamic_descriptor(runtime, request)?;
    let receipt = native(reg_fn.call(&payload).into_result())?;
    decode_register_receipt(receipt)
}

/// Register a dynamically forwarded language descriptor through an AOT-linked
/// `marlin_search_dsl_register_provider_descriptor` C-ABI export (zero JSON).
///
/// # Safety
/// `export` must originate from `runtime`'s live linked Marlin programme.
pub unsafe fn register_linked_dynamic_language_descriptor(
    runtime: &GerbilRuntime,
    export: unsafe extern "C" fn(i64) -> i64,
    request: &DynamicLanguageDescriptorRequest<'_>,
) -> Result<usize, SearchPlanProjectionError> {
    Ok(unsafe { register_linked_descriptor_outcome(runtime, export, request) }?.total_relations)
}

#[cfg(not(feature = "linked-ui"))]
fn register_package_descriptor_outcome(
    runtime: &GerbilRuntime,
    request: &DynamicLanguageDescriptorRequest<'_>,
) -> Result<RegisterDescriptorOutcome, SearchPlanProjectionError> {
    let module = native(runtime.intern_identifier(
        IdentifierKind::Symbol,
        ":marlin-agent-core/c-abi/search-dsl",
    ))?;
    let sym = native(runtime.intern_identifier(
        IdentifierKind::Symbol,
        "marlin-search-dsl-register-provider-descriptor",
    ))?;
    let reg_fn = native(runtime.resolve_package_export(request.scheme_dir, &module, &sym))?;
    let payload = encode_dynamic_descriptor(runtime, request)?;
    let receipt = native(runtime.call_rooted(&reg_fn, &payload))?;
    decode_register_receipt(receipt)
}

/// Register a dynamically forwarded language descriptor directly in Marlin's
/// compiled Scheme package over the zero-JSON `RootedSchemeObject` C-ABI.
/// Returns the updated total relation count in `*active-search-registry*`.
#[cfg(not(feature = "linked-ui"))]
pub fn register_package_dynamic_language_descriptor(
    runtime: &GerbilRuntime,
    request: &DynamicLanguageDescriptorRequest<'_>,
) -> Result<usize, SearchPlanProjectionError> {
    Ok(register_package_descriptor_outcome(runtime, request)?.total_relations)
}

pub(crate) fn infer_special_specs_from_dsl_pack(
    pack: &ForwardedDslPack,
) -> Vec<DynamicSpecialRelationSpec> {
    let mut by_native_kind: BTreeMap<&str, &ForwardedKindMapping> = BTreeMap::new();
    let mut by_general_kind: BTreeMap<&str, &ForwardedKindMapping> = BTreeMap::new();
    for km in &pack.kind_mappings {
        by_native_kind
            .entry(km.native_language_kind.as_str())
            .or_insert(km);
        by_general_kind
            .entry(km.general_kind.as_str())
            .or_insert(km);
    }

    pack.special_relations
        .iter()
        .map(|rel_name| {
            let suffix = rel_name
                .split_once('_')
                .map_or(rel_name.as_str(), |(_, tail)| tail);
            let matched = by_native_kind
                .get(suffix)
                .or_else(|| by_general_kind.get(suffix))
                .copied();
            match matched {
                Some(km) => DynamicSpecialRelationSpec {
                    relation_name: rel_name.clone(),
                    kind_pattern: None,
                    language_kind_pattern: Some(km.native_language_kind.clone()),
                    description: km.description.clone(),
                },
                None => DynamicSpecialRelationSpec {
                    relation_name: rel_name.clone(),
                    kind_pattern: None,
                    language_kind_pattern: None,
                    description: format!(
                        "Dynamically forwarded {} relation",
                        pack.language_id.as_str()
                    ),
                },
            }
        })
        .collect()
}

/// Build `providerLanguageId -> canonicalLanguageId` aliases strictly from
/// the live ASP provider catalog (`dslPack.providerLanguageId -> dslPack.languageId`)
/// with zero hardcoded language names in Rust.
pub(super) fn provider_language_aliases(
    catalog: &ForwardedProviderCatalog,
) -> BTreeMap<String, String> {
    let mut aliases = BTreeMap::new();
    for env in &catalog.providers {
        if let Some(pack) = env.dsl_pack.as_ref() {
            let provider_lang = pack.provider_language_id.as_str();
            let canonical_lang = pack.language_id.as_str();
            if provider_lang != canonical_lang {
                aliases.insert(provider_lang.to_owned(), canonical_lang.to_owned());
            }
        }
    }
    aliases
}

fn resolve_canonical_language<'a>(
    provider_lang: &'a str,
    aliases: &'a BTreeMap<String, String>,
) -> &'a str {
    aliases
        .get(provider_lang)
        .map_or(provider_lang, String::as_str)
}

fn supported_languages_from_catalog(
    catalog: &ForwardedProviderCatalog,
    aliases: &BTreeMap<String, String>,
) -> BTreeSet<String> {
    catalog
        .providers
        .iter()
        .map(|env| {
            resolve_canonical_language(env.manifest.language_id.as_str(), aliases).to_owned()
        })
        .collect()
}

/// Forward every provider's `dslPack` (or fallback `manifest.special`)
/// (`kindMappings` + `specialRelations`) straight to Scheme `*active-search-registry*`,
/// which owns deduplication and mapping onto the General Syntax Ontology with zero
/// hardcoded language tables in Rust.
fn forwarded_provider_descriptors(
    catalog: &ForwardedProviderCatalog,
) -> Vec<(
    &[String],
    ForwardedDslPack,
    Vec<DynamicSpecialRelationSpec>,
)> {
    catalog
        .providers
        .iter()
        .filter_map(|env| {
            let pack = if let Some(pack) = env.dsl_pack.as_ref() {
                pack.clone()
            } else {
                let special = env.manifest.special.as_ref()?;
                if special.kind_mappings.is_empty() && special.special_relations.is_empty() {
                    return None;
                }
                ForwardedDslPack {
                    language_id: env.manifest.language_id.clone(),
                    provider_language_id: env.manifest.language_id.clone(),
                    extensions: env.manifest.extensions.clone(),
                    kind_mappings: special.kind_mappings.clone(),
                    special_relations: special.special_relations.clone(),
                }
            };
            let specs = infer_special_specs_from_dsl_pack(&pack);
            Some((env.manifest.extensions.as_slice(), pack, specs))
        })
        .collect()
}

/// Fetch ASP's live provider catalog and sync any new or additive language
/// descriptors into Marlin's AOT-linked `*active-search-registry*` over zero-JSON C-ABI.
/// Returns the number of descriptors that introduced new languages or relations in Scheme.
///
/// # Safety
/// `export` must originate from `runtime`'s live linked Marlin programme.
pub unsafe fn sync_linked_asp_provider_descriptors(
    runtime: &GerbilRuntime,
    export: unsafe extern "C" fn(i64) -> i64,
    endpoint: &AspForwardingEndpoint,
) -> Result<usize, SearchPlanProjectionError> {
    let catalog = fetch_asp_provider_catalog(endpoint)?;
    forwarded_provider_descriptors(&catalog)
        .into_iter()
        .try_fold(0usize, |synced, (extensions, pack, specs)| {
            let desc_req = DynamicLanguageDescriptorRequest {
                scheme_dir: "",
                language_id: &pack.language_id,
                provider_language_id: &pack.provider_language_id,
                extensions,
                kind_mappings: &pack.kind_mappings,
                special_specs: &specs,
            };
            let outcome =
                unsafe { register_linked_descriptor_outcome(runtime, export, &desc_req) }?;
            Ok(synced + usize::from(outcome.added_entries > 0))
        })
}

/// Fetch ASP's live provider catalog and sync any new or additive language
/// descriptors into Marlin's compiled package `*active-search-registry*`.
#[cfg(not(feature = "linked-ui"))]
pub fn sync_package_asp_provider_descriptors(
    runtime: &GerbilRuntime,
    scheme_dir: &str,
    endpoint: &AspForwardingEndpoint,
) -> Result<usize, SearchPlanProjectionError> {
    let catalog = fetch_asp_provider_catalog(endpoint)?;
    forwarded_provider_descriptors(&catalog)
        .into_iter()
        .try_fold(0usize, |synced, (extensions, pack, specs)| {
            let desc_req = DynamicLanguageDescriptorRequest {
                scheme_dir,
                language_id: &pack.language_id,
                provider_language_id: &pack.provider_language_id,
                extensions,
                kind_mappings: &pack.kind_mappings,
                special_specs: &specs,
            };
            let outcome = register_package_descriptor_outcome(runtime, &desc_req)?;
            Ok(synced + usize::from(outcome.added_entries > 0))
        })
}

#[cfg(not(feature = "linked-ui"))]
fn restore_cached_open_edb(
    index: &mut WorkspaceSearchIndex,
    cache_path: &Path,
    receipt: &mut AspHttpHydrationReceipt,
) {
    if !cache_path.is_file() {
        return;
    }
    if let Ok(restored) = index.load_and_restore_open_edb_cache(cache_path) {
        receipt.restored_from_cache_files = restored.restored_files;
        receipt.restored_from_cache_entities = restored.restored_entities;
    }
}

fn targeted_schemas_by_language<'a>(
    catalog: &'a ForwardedProviderCatalog,
    aliases: &'a BTreeMap<String, String>,
) -> BTreeMap<&'a str, &'a TargetedJsonSchemaDocument> {
    catalog
        .providers
        .iter()
        .map(|env| {
            (
                resolve_canonical_language(env.manifest.language_id.as_str(), aliases),
                &env.targeted_schema,
            )
        })
        .collect()
}

fn normalize_and_validate_batch(
    mut batch: WorkspaceOpenEdbIngestBatch,
    aliases: &BTreeMap<String, String>,
    schemas_by_lang: &BTreeMap<&str, &TargetedJsonSchemaDocument>,
    receipt: &mut AspHttpHydrationReceipt,
) -> WorkspaceOpenEdbIngestBatch {
    for ent in &mut batch.entities {
        if let Some(canonical) = aliases.get(ent.language.as_str()) {
            ent.language.clone_from(canonical);
        }
        if let Some(schema) = schemas_by_lang.get(ent.language.as_str()) {
            if schema.admits_entity_kinds(&ent.kind, &ent.language_kind) {
                receipt.schema_validated_entities += 1;
            } else {
                receipt.schema_drift_entities += 1;
            }
        }
    }
    if let Some(lang) = batch.language_id.as_deref()
        && let Some(canonical) = aliases.get(lang)
    {
        batch.language_id = Some(canonical.clone());
    }
    batch
}

fn inspect_and_ingest_pending_files(
    endpoint: &AspForwardingEndpoint,
    index: &mut WorkspaceSearchIndex,
    catalog: &ForwardedProviderCatalog,
    cache_path: &Path,
    max_batch_files: usize,
    receipt: &mut AspHttpHydrationReceipt,
) -> Result<(), SearchPlanProjectionError> {
    let aliases = provider_language_aliases(catalog);
    let supported_langs = supported_languages_from_catalog(catalog, &aliases);
    let lang_refs: Vec<&str> = supported_langs.iter().map(String::as_str).collect();
    let pending = index.live_files_needing_open_edb(&lang_refs, max_batch_files.max(1));
    if pending.is_empty() {
        return Ok(());
    }

    let req_payload = InspectFilesRequestPayload {
        files: pending
            .into_iter()
            .map(|file| InspectFilePayload {
                path: file.path,
                source_utf8: file.body,
            })
            .collect(),
    };
    let encoded = serde_json::to_vec(&req_payload).map_err(|e| {
        SearchPlanProjectionError::Native(format!("encode inspect-files request: {e}"))
    })?;
    let (status, resp_body) = http_loopback_request(
        endpoint,
        "POST",
        "/v1/open-edb/inspect-files",
        Some(&encoded),
    )?;
    if status != 200 {
        return Err(SearchPlanProjectionError::Native(format!(
            "ASP POST /v1/open-edb/inspect-files returned HTTP {status}: {}",
            String::from_utf8_lossy(&resp_body)
        )));
    }
    let wire: ForwardedOpenEdbWireResponse = serde_json::from_slice(&resp_body).map_err(|e| {
        SearchPlanProjectionError::Native(format!("decode inspect-files response: {e}"))
    })?;

    let schemas_by_lang = targeted_schemas_by_language(catalog, &aliases);
    receipt.http_inspected_files = wire.inspected_files;
    receipt.failed_files = wire.failed_paths.len();
    receipt.asp_inspect_micros = wire.elapsed_micros;
    for batch in wire.batches {
        let normalized = normalize_and_validate_batch(batch, &aliases, &schemas_by_lang, receipt);
        let ing = index.ingest_open_edb_facts(normalized);
        receipt.http_ingested_entities += ing.ingested_entities;
    }
    if receipt.http_ingested_entities > 0
        && crate::open_edb_hydrator::is_non_symlink_cache_path(cache_path)
    {
        receipt.cache_saved = index.save_open_edb_cache(cache_path).is_ok();
    }
    Ok(())
}

/// Hydrate pending uncached multi-language workspace files via ASP's HTTP
/// forwarding gateway, bounded by Scheme-projected [`AspHydrationPolicy`].
pub fn hydrate_workspace_pending_via_asp_http(
    endpoint: &AspForwardingEndpoint,
    index: &mut WorkspaceSearchIndex,
    cache_path: &Path,
    policy: &AspHydrationPolicy,
) -> Result<AspHttpHydrationReceipt, SearchPlanProjectionError> {
    let started = Instant::now();
    let mut receipt = AspHttpHydrationReceipt::default();
    let catalog = fetch_asp_provider_catalog(endpoint)?;
    receipt.providers_discovered = catalog.providers.len();
    inspect_and_ingest_pending_files(
        endpoint,
        index,
        &catalog,
        cache_path,
        policy.inspect_batch_files(),
        &mut receipt,
    )?;
    receipt.total_elapsed_micros = u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);
    Ok(receipt)
}

/// Derive package-manifest anchor directories from the live ASP provider
/// catalog's declared `projectMarkers` against `workspace_root`.
fn discover_package_anchors(
    workspace_root: &Path,
    catalog: &ForwardedProviderCatalog,
    aliases: &BTreeMap<String, String>,
    pending: &[agent_semantic_search::WorkspaceOpenEdbPendingFile],
) -> BTreeMap<String, Vec<String>> {
    let mut anchors_by_lang: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut marker_map: BTreeMap<&str, &[String]> = BTreeMap::new();
    for env in &catalog.providers {
        marker_map.insert(
            resolve_canonical_language(env.manifest.language_id.as_str(), aliases),
            env.manifest.project_markers.as_slice(),
        );
    }
    for file in pending {
        let lang = resolve_canonical_language(&file.language, aliases);
        let Some(markers) = marker_map.get(lang) else {
            continue;
        };
        if markers.is_empty() {
            continue;
        }
        let rel = Path::new(&file.path);
        let mut cursor = rel.parent();
        while let Some(dir) = cursor {
            let abs_dir = if dir.as_os_str().is_empty() {
                workspace_root.to_path_buf()
            } else {
                workspace_root.join(dir)
            };
            if markers.iter().any(|marker| {
                !marker.is_empty()
                    && !marker.contains(['/', '\\'])
                    && marker != "."
                    && marker != ".."
                    && abs_dir.join(marker).is_file()
            }) {
                let norm = dir.to_string_lossy().replace('\\', "/");
                anchors_by_lang
                    .entry(lang.to_owned())
                    .or_default()
                    .insert(norm);
                break;
            }
            if dir.as_os_str().is_empty() {
                break;
            }
            cursor = dir.parent();
        }
    }
    anchors_by_lang
        .into_iter()
        .map(|(lang, dirs)| (lang, dirs.into_iter().collect()))
        .collect()
}

/// Rank a pending file strictly by its ASP provider `projectMarkers` package
/// anchor without any hardcoded repository paths or language names.
fn package_anchor_rank(
    path: &str,
    language: &str,
    aliases: &BTreeMap<String, String>,
    anchors_by_lang: &BTreeMap<String, Vec<String>>,
) -> (u8, usize, bool) {
    let lang = resolve_canonical_language(language, aliases);
    let anchored_depth = anchors_by_lang.get(lang).and_then(|anchors| {
        anchors
            .iter()
            .filter(|anchor| {
                anchor.is_empty()
                    || path == anchor.as_str()
                    || path
                        .strip_prefix(anchor.as_str())
                        .is_some_and(|rest| rest.starts_with('/'))
            })
            .map(|anchor| {
                if anchor.is_empty() {
                    0
                } else {
                    anchor.matches('/').count() + 1
                }
            })
            .max()
    });
    let is_test_like = path.contains("/tests/")
        || path.contains("/benches/")
        || path.contains("/test/")
        || path.contains("/t/");
    match anchored_depth {
        Some(depth) => (0, depth, is_test_like),
        None => (1, usize::MAX, is_test_like),
    }
}

/// Typed request envelope for progressive, lock-free-I/O chunked ASP Open EDB hydration.
pub struct AspChunkedHydrationRequest<'a> {
    pub endpoint: &'a AspForwardingEndpoint,
    pub workspace_root: &'a Path,
    pub cache_path: &'a Path,
    pub policy: &'a AspHydrationPolicy,
}

fn collect_ranked_pending_files(
    shared_index: &std::sync::RwLock<WorkspaceSearchIndex>,
    request: &AspChunkedHydrationRequest<'_>,
    catalog: &ForwardedProviderCatalog,
    aliases: &BTreeMap<String, String>,
) -> Result<Vec<agent_semantic_search::WorkspaceOpenEdbPendingFile>, SearchPlanProjectionError> {
    let supported_langs = supported_languages_from_catalog(catalog, aliases);
    let lang_refs: Vec<&str> = supported_langs.iter().map(String::as_str).collect();
    let mut pending = {
        let guard = shared_index.read().map_err(|_| {
            SearchPlanProjectionError::Native("workspace index lock poisoned".to_owned())
        })?;
        guard.live_files_needing_open_edb(&lang_refs, request.policy.max_files().max(1))
    };
    if !pending.is_empty() {
        let anchors_by_lang =
            discover_package_anchors(request.workspace_root, catalog, aliases, &pending);
        pending.sort_by(|a, b| {
            package_anchor_rank(&a.path, &a.language, aliases, &anchors_by_lang)
                .cmp(&package_anchor_rank(
                    &b.path,
                    &b.language,
                    aliases,
                    &anchors_by_lang,
                ))
                .then_with(|| a.path.cmp(&b.path))
        });
    }
    Ok(pending)
}

struct ChunkHydrationTarget<'a> {
    endpoint: &'a AspForwardingEndpoint,
    cache_path: &'a Path,
    aliases: &'a BTreeMap<String, String>,
    schemas_by_lang: &'a BTreeMap<&'a str, &'a TargetedJsonSchemaDocument>,
}

fn hydrate_single_chunk(
    target: &ChunkHydrationTarget<'_>,
    shared_index: &std::sync::RwLock<WorkspaceSearchIndex>,
    chunk: &[agent_semantic_search::WorkspaceOpenEdbPendingFile],
    persist_cache: bool,
    receipt: &mut AspHttpHydrationReceipt,
) {
    let req_payload = InspectFilesRequestPayload {
        files: chunk
            .iter()
            .map(|file| InspectFilePayload {
                path: file.path.clone(),
                source_utf8: file.body.clone(),
            })
            .collect(),
    };
    let Ok(encoded) = serde_json::to_vec(&req_payload) else {
        receipt.failed_files += chunk.len();
        return;
    };
    let Ok((200, resp_body)) = http_loopback_request(
        target.endpoint,
        "POST",
        "/v1/open-edb/inspect-files",
        Some(&encoded),
    ) else {
        receipt.failed_files += chunk.len();
        return;
    };
    let Ok(wire) = serde_json::from_slice::<ForwardedOpenEdbWireResponse>(&resp_body) else {
        receipt.failed_files += chunk.len();
        return;
    };
    receipt.http_inspected_files += wire.inspected_files;
    receipt.failed_files += wire.failed_paths.len();
    receipt.asp_inspect_micros = receipt
        .asp_inspect_micros
        .saturating_add(wire.elapsed_micros);
    let normalized_batches: Vec<WorkspaceOpenEdbIngestBatch> = wire
        .batches
        .into_iter()
        .map(|batch| {
            normalize_and_validate_batch(batch, target.aliases, target.schemas_by_lang, receipt)
        })
        .collect();
    if let Ok(mut guard) = shared_index.write() {
        for batch in normalized_batches {
            let ing = guard.ingest_open_edb_facts(batch);
            receipt.http_ingested_entities += ing.ingested_entities;
        }
        if persist_cache
            && receipt.http_ingested_entities > 0
            && crate::open_edb_hydrator::is_non_symlink_cache_path(target.cache_path)
        {
            receipt.cache_saved = guard.save_open_edb_cache(target.cache_path).is_ok();
        }
    }
}

fn hydrate_pending_chunks(
    request: &AspChunkedHydrationRequest<'_>,
    shared_index: &std::sync::RwLock<WorkspaceSearchIndex>,
    catalog: &ForwardedProviderCatalog,
    aliases: &BTreeMap<String, String>,
    pending: &[agent_semantic_search::WorkspaceOpenEdbPendingFile],
    receipt: &mut AspHttpHydrationReceipt,
    mut on_progress: impl FnMut(usize, usize),
) {
    let schemas_by_lang = targeted_schemas_by_language(catalog, aliases);
    let target = ChunkHydrationTarget {
        endpoint: request.endpoint,
        cache_path: request.cache_path,
        aliases,
        schemas_by_lang: &schemas_by_lang,
    };
    let step = request.policy.chunk_size().max(1);
    let total = pending.len();
    let mut processed = 0usize;
    for (chunk_idx, chunk) in pending.chunks(step).enumerate() {
        processed = processed.saturating_add(chunk.len());
        let remaining = total.saturating_sub(processed);
        let persist_cache = chunk_idx % 4 == 3 || remaining == 0;
        hydrate_single_chunk(&target, shared_index, chunk, persist_cache, receipt);
        on_progress(receipt.http_ingested_entities, remaining);
    }
}

/// Typed request envelope for on-demand scoped ASP Open EDB hydration targeting
/// one bounded `path_prefix` (e.g. a crate or module slice).
pub struct AspScopedHydrationRequest<'a> {
    pub endpoint: &'a AspForwardingEndpoint,
    pub cache_path: &'a Path,
    pub path_prefix: &'a str,
    pub max_files: usize,
}

/// On-demand scoped ASP Open EDB hydration for a bounded `path_prefix`
/// (e.g. a single crate/module targeted by an LLM L1/L2 structural query).
/// Inspects only unhydrated files matching `path_prefix` (capped at `max_files`)
/// without holding the index write lock across HTTP I/O, then persists the
/// updated BLAKE3 cache.
pub fn hydrate_workspace_scoped_via_asp_http(
    request: &AspScopedHydrationRequest<'_>,
    shared_index: &std::sync::RwLock<WorkspaceSearchIndex>,
) -> Result<AspHttpHydrationReceipt, SearchPlanProjectionError> {
    let started = Instant::now();
    let mut receipt = AspHttpHydrationReceipt::default();
    let trimmed = request.path_prefix.trim();
    if trimmed.is_empty() || request.max_files == 0 {
        return Ok(receipt);
    }
    let catalog = fetch_asp_provider_catalog(request.endpoint)?;
    receipt.providers_discovered = catalog.providers.len();
    let aliases = provider_language_aliases(&catalog);
    let supported_langs = supported_languages_from_catalog(&catalog, &aliases);
    let lang_refs: Vec<&str> = supported_langs.iter().map(String::as_str).collect();
    let scoped_pending: Vec<_> = {
        let guard = shared_index.read().map_err(|_| {
            SearchPlanProjectionError::Native("workspace index lock poisoned".to_owned())
        })?;
        guard
            .live_files_needing_open_edb(&lang_refs, 2048)
            .into_iter()
            .filter(|file| file.path.starts_with(trimmed))
            .take(request.max_files)
            .collect()
    };
    if !scoped_pending.is_empty() {
        let schemas_by_lang = targeted_schemas_by_language(&catalog, &aliases);
        let target = ChunkHydrationTarget {
            endpoint: request.endpoint,
            cache_path: request.cache_path,
            aliases: &aliases,
            schemas_by_lang: &schemas_by_lang,
        };
        hydrate_single_chunk(&target, shared_index, &scoped_pending, true, &mut receipt);
    }
    receipt.total_elapsed_micros = u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);
    Ok(receipt)
}

/// Progressive, lock-free-I/O chunked ASP Open EDB hydration over a shared
/// [`std::sync::RwLock<WorkspaceSearchIndex>`], anchored by each ASP language
/// provider's declared `projectMarkers` and bounded by Scheme-projected
/// [`AspHydrationPolicy`] (`HelmIndexPolicy.`).
pub fn hydrate_workspace_chunked_via_asp_http(
    request: &AspChunkedHydrationRequest<'_>,
    shared_index: &std::sync::RwLock<WorkspaceSearchIndex>,
    on_progress: impl FnMut(usize, usize),
) -> Result<AspHttpHydrationReceipt, SearchPlanProjectionError> {
    let started = Instant::now();
    let mut receipt = AspHttpHydrationReceipt::default();
    let catalog = fetch_asp_provider_catalog(request.endpoint)?;
    receipt.providers_discovered = catalog.providers.len();
    let aliases = provider_language_aliases(&catalog);
    let pending = collect_ranked_pending_files(shared_index, request, &catalog, &aliases)?;
    if !pending.is_empty() {
        hydrate_pending_chunks(
            request,
            shared_index,
            &catalog,
            &aliases,
            &pending,
            &mut receipt,
            on_progress,
        );
    }
    receipt.total_elapsed_micros = u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);
    Ok(receipt)
}

/// Fetch ASP's live provider catalog, restore matching BLAKE3 cache entries,
/// inspect pending multi-language workspace files through ASP's HTTP forwarding
/// gateway (`POST /v1/open-edb/inspect-files`), ingest all General JSON Open EDB
/// batches into `index`, and atomically persist the updated BLAKE3 cache.
#[cfg(not(feature = "linked-ui"))]
pub fn hydrate_workspace_from_asp_http(
    runtime: &GerbilRuntime,
    index: &mut WorkspaceSearchIndex,
    request: &AspHttpHydrationRequest<'_>,
) -> Result<AspHttpHydrationReceipt, SearchPlanProjectionError> {
    let started = Instant::now();
    let mut receipt = AspHttpHydrationReceipt::default();

    restore_cached_open_edb(index, request.cache_path, &mut receipt);
    let catalog = fetch_asp_provider_catalog(request.endpoint)?;
    receipt.providers_discovered = catalog.providers.len();
    if request.sync_new_descriptors {
        receipt.dynamic_descriptors_synced =
            sync_package_asp_provider_descriptors(runtime, request.scheme_dir, request.endpoint)?;
    }
    let policy =
        crate::projection::project_helm_index_policy(runtime, request.scheme_dir)?.hydration;
    inspect_and_ingest_pending_files(
        request.endpoint,
        index,
        &catalog,
        request.cache_path,
        policy.inspect_batch_files(),
        &mut receipt,
    )?;

    receipt.total_elapsed_micros = u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);
    Ok(receipt)
}
