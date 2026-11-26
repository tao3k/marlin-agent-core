// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Zero-JSON C-ABI dynamic POO descriptor synchronization and workspace Open EDB hydration.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::time::Instant;

use agent_semantic_search::{WorkspaceOpenEdbIngestBatch, WorkspaceSearchIndex};
use gerbil_scheme::{CollectionKind, GerbilRuntime, RootedSchemeObject, SchemeScalar};
#[cfg(not(feature = "linked-ui"))]
use gerbil_scheme::IdentifierKind;
use serde::{Deserialize, Serialize};

use crate::SearchPlanProjectionError;
use crate::edb_bridge::{decode_string, decode_usize, make_string, make_vector, native};

use super::domain_types::{
    AspForwardingEndpoint, GeneralOntologyKindName, NativeAstKindName,
};
use super::protocol_types::{
    AspHttpHydrationReceipt, DynamicLanguageDescriptorRequest, DynamicSpecialRelationSpec,
    ForwardedDslPack, ForwardedKindMapping, ForwardedProviderCatalog,
};
#[cfg(not(feature = "linked-ui"))]
use super::protocol_types::AspHttpHydrationRequest;
use super::transport::{fetch_asp_provider_catalog, http_loopback_request};

const MAX_INSPECT_BATCH_FILES: usize = 512;

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
struct ForwardedOpenEdbWireResponse {
    inspected_files: usize,
    elapsed_micros: u64,
    batches: Vec<WorkspaceOpenEdbIngestBatch>,
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
                        spec.kind_pattern.as_ref().map(GeneralOntologyKindName::as_str),
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

fn decode_register_receipt(receipt: RootedSchemeObject<'_>) -> Result<usize, SearchPlanProjectionError> {
    let tuple = native(receipt.into_collection(CollectionKind::Vector))?;
    if tuple.len() != 3 {
        return Err(SearchPlanProjectionError::Invalid(
            "dynamic register receipt must be a 3-vector",
        ));
    }
    let _lang = decode_string(native(tuple.element(1))?, 256)?;
    decode_usize(native(tuple.element(2))?)
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
    let reg_fn = native(unsafe { runtime.bind_object_export(export) })?;
    let payload = encode_dynamic_descriptor(runtime, request)?;
    let receipt = native(reg_fn.call(&payload).into_result())?;
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

fn canonical_search_language(provider_lang: &str) -> &str {
    match provider_lang {
        "gerbil-scheme" => "scheme",
        other => other,
    }
}

fn is_builtin_search_language(language_id: &str) -> bool {
    matches!(
        language_id,
        "rust"
            | "python"
            | "typescript"
            | "julia"
            | "scheme"
            | "gerbil-scheme"
            | "sysml"
            | "quint"
            | "org"
    )
}

fn supported_languages_from_catalog(catalog: &ForwardedProviderCatalog) -> BTreeSet<String> {
    catalog
        .providers
        .iter()
        .map(|env| canonical_search_language(env.manifest.language_id.as_str()).to_owned())
        .collect()
}

fn dynamic_provider_packs(
    catalog: &ForwardedProviderCatalog,
) -> impl Iterator<Item = (&[String], &ForwardedDslPack)> {
    catalog.providers.iter().filter_map(|env| {
        if is_builtin_search_language(env.manifest.language_id.as_str()) {
            return None;
        }
        env.dsl_pack
            .as_ref()
            .map(|pack| (env.manifest.extensions.as_slice(), pack))
    })
}

/// Fetch ASP's live provider catalog and sync any non-builtin dynamic language
/// descriptors into Marlin's AOT-linked `*active-search-registry*` over zero-JSON C-ABI.
/// Returns the number of dynamically synced provider descriptors.
///
/// # Safety
/// `export` must originate from `runtime`'s live linked Marlin programme.
pub unsafe fn sync_linked_asp_provider_descriptors(
    runtime: &GerbilRuntime,
    export: unsafe extern "C" fn(i64) -> i64,
    endpoint: &AspForwardingEndpoint,
) -> Result<usize, SearchPlanProjectionError> {
    let catalog = fetch_asp_provider_catalog(endpoint)?;
    dynamic_provider_packs(&catalog).try_fold(0usize, |synced, (extensions, pack)| {
        let specs = infer_special_specs_from_dsl_pack(pack);
        let desc_req = DynamicLanguageDescriptorRequest {
            scheme_dir: "",
            language_id: &pack.language_id,
            provider_language_id: &pack.provider_language_id,
            extensions,
            kind_mappings: &pack.kind_mappings,
            special_specs: &specs,
        };
        let _ = unsafe { register_linked_dynamic_language_descriptor(runtime, export, &desc_req) }?;
        Ok(synced + 1)
    })
}

/// Fetch ASP's live provider catalog and sync any non-builtin dynamic language
/// descriptors into Marlin's compiled package `*active-search-registry*`.
#[cfg(not(feature = "linked-ui"))]
pub fn sync_package_asp_provider_descriptors(
    runtime: &GerbilRuntime,
    scheme_dir: &str,
    endpoint: &AspForwardingEndpoint,
) -> Result<usize, SearchPlanProjectionError> {
    let catalog = fetch_asp_provider_catalog(endpoint)?;
    dynamic_provider_packs(&catalog).try_fold(0usize, |synced, (extensions, pack)| {
        let specs = infer_special_specs_from_dsl_pack(pack);
        let desc_req = DynamicLanguageDescriptorRequest {
            scheme_dir,
            language_id: &pack.language_id,
            provider_language_id: &pack.provider_language_id,
            extensions,
            kind_mappings: &pack.kind_mappings,
            special_specs: &specs,
        };
        let _ = register_package_dynamic_language_descriptor(runtime, &desc_req)?;
        Ok(synced + 1)
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

fn inspect_and_ingest_pending_files(
    endpoint: &AspForwardingEndpoint,
    index: &mut WorkspaceSearchIndex,
    supported_langs: &BTreeSet<String>,
    cache_path: &Path,
    receipt: &mut AspHttpHydrationReceipt,
) -> Result<(), SearchPlanProjectionError> {
    let lang_refs: Vec<&str> = supported_langs.iter().map(String::as_str).collect();
    let pending = index.live_files_needing_open_edb(&lang_refs, MAX_INSPECT_BATCH_FILES);
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

    receipt.http_inspected_files = wire.inspected_files;
    receipt.asp_inspect_micros = wire.elapsed_micros;
    for mut batch in wire.batches {
        for ent in &mut batch.entities {
            if ent.language == "gerbil-scheme" {
                ent.language = "scheme".to_owned();
            }
        }
        if batch.language_id.as_deref() == Some("gerbil-scheme") {
            batch.language_id = Some("scheme".to_owned());
        }
        let ing = index.ingest_open_edb_facts(batch);
        receipt.http_ingested_entities += ing.ingested_entities;
    }
    if receipt.http_ingested_entities > 0 {
        receipt.cache_saved = index.save_open_edb_cache(cache_path).is_ok();
    }
    Ok(())
}

/// Hydrate pending uncached multi-language workspace files via ASP's HTTP
/// forwarding gateway without touching the Gerbil VM thread (safe for background
/// TUI warm-up, Helm refresh, and Agent `workspace_search` indexing threads).
pub fn hydrate_workspace_pending_via_asp_http(
    endpoint: &AspForwardingEndpoint,
    index: &mut WorkspaceSearchIndex,
    cache_path: &Path,
) -> Result<AspHttpHydrationReceipt, SearchPlanProjectionError> {
    let started = Instant::now();
    let mut receipt = AspHttpHydrationReceipt::default();
    let catalog = fetch_asp_provider_catalog(endpoint)?;
    receipt.providers_discovered = catalog.providers.len();
    let supported_langs = supported_languages_from_catalog(&catalog);
    inspect_and_ingest_pending_files(endpoint, index, &supported_langs, cache_path, &mut receipt)?;
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
    let supported_langs = supported_languages_from_catalog(&catalog);
    inspect_and_ingest_pending_files(
        request.endpoint,
        index,
        &supported_langs,
        request.cache_path,
        &mut receipt,
    )?;

    receipt.total_elapsed_micros = u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);
    Ok(receipt)
}
