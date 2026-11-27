// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Protocol-driven projection from ASP's forwarded JSON Schema catalog
//! (`GET /v1/providers` -> [`ForwardedProviderCatalog`]) into typed
//! [`SearchDslCompletionEntry`] rows for TUI Helm Copilot TAB completion.
//!
//! Contains ZERO hardcoded language names, AST kinds, or keyword tables: every
//! property key and enum value flows directly from each provider's forwarded
//! `targetedSchema` JSON Schema Draft 2020-12 document, `manifest`, and `dslPack`.

use std::collections::{BTreeMap, HashSet};

use crate::edb_bridge::{SearchDslCompletionEntry, SearchDslCompletionKind};

use super::domain_types::TargetedJsonSchemaDocument;
use super::protocol_types::{
    ForwardedKindMapping, ForwardedProviderCatalog, ForwardedProviderEnvelope,
};

fn collect_kind_docs<'a>(mappings: &'a [ForwardedKindMapping], docs: &mut BTreeMap<&'a str, &'a str>) {
    for km in mappings {
        if !km.description.is_empty() {
            docs.entry(km.native_language_kind.as_str())
                .or_insert(km.description.as_str());
            docs.entry(km.general_kind.as_str())
                .or_insert(km.description.as_str());
        }
    }
}

fn build_kind_doc_index(env: &ForwardedProviderEnvelope) -> BTreeMap<&str, &str> {
    let mut docs = BTreeMap::new();
    if let Some(special) = env.manifest.special.as_ref() {
        collect_kind_docs(&special.kind_mappings, &mut docs);
    }
    if let Some(pack) = env.dsl_pack.as_ref() {
        collect_kind_docs(&pack.kind_mappings, &mut docs);
    }
    docs
}

fn push_property_schema_entries(
    schema: &TargetedJsonSchemaDocument,
    owner_scope: &str,
    prop_name: &str,
    kind_docs: &BTreeMap<&str, &str>,
    out: &mut Vec<SearchDslCompletionEntry>,
) {
    let prop_type = schema.property_type(prop_name).unwrap_or("any");
    let kw_form = format!("{prop_name}:");
    let prop_doc = schema
        .property_description(prop_name)
        .map(str::to_owned)
        .unwrap_or_else(|| format!("JSON Schema property `{prop_name}` ({prop_type})"));

    out.push(SearchDslCompletionEntry {
        form: kw_form.clone(),
        kind: SearchDslCompletionKind::Keyword,
        signature: format!("schema:{prop_type}"),
        language: owner_scope.to_owned(),
        doc: prop_doc,
        insert_text: format!("{kw_form} "),
    });

    let enums = schema.property_enum(prop_name);
    let values: Vec<&str> = if !enums.is_empty() {
        enums
    } else if let Some(c) = schema.property_const(prop_name) {
        vec![c]
    } else {
        Vec::new()
    };

    for val in values {
        let quoted = format!("\"{val}\"");
        let doc = kind_docs
            .get(val)
            .copied()
            .map(str::to_owned)
            .unwrap_or_else(|| format!("Allowed `{prop_name}` value from {owner_scope} JSON Schema"));
        out.push(SearchDslCompletionEntry {
            form: quoted.clone(),
            kind: SearchDslCompletionKind::Value,
            signature: kw_form.clone(),
            language: owner_scope.to_owned(),
            doc,
            insert_text: format!("{quoted} "),
        });
    }

    if prop_type == "boolean" {
        for (lit, desc) in [("#t", "Boolean true"), ("#f", "Boolean false")] {
            out.push(SearchDslCompletionEntry {
                form: lit.to_owned(),
                kind: SearchDslCompletionKind::Value,
                signature: kw_form.clone(),
                language: owner_scope.to_owned(),
                doc: format!("{desc} for `{prop_name}`"),
                insert_text: format!("{lit} "),
            });
        }
    }
}

/// Project ANY JSON Schema Draft 2020-12 document (universal ASP query schema,
/// gRPC wire JSON Schema, or per-language `targetedSchema`) into typed Copilot
/// completion entries under `owner_scope` (e.g. `"*"` for universal protocol
/// properties, a query head name, or a `languageId`).
#[must_use]
pub fn project_json_schema_completions(
    owner_scope: &str,
    schema: &TargetedJsonSchemaDocument,
) -> Vec<SearchDslCompletionEntry> {
    let empty_docs = BTreeMap::new();
    let mut out = Vec::new();
    for prop_name in schema.property_names() {
        push_property_schema_entries(schema, owner_scope, prop_name, &empty_docs, &mut out);
    }
    out
}

fn push_provider_identity_and_extensions(
    env: &ForwardedProviderEnvelope,
    lang_id: &str,
    provider_id: &str,
    out: &mut Vec<SearchDslCompletionEntry>,
) {
    let quoted_lang = format!("\"{lang_id}\"");
    out.push(SearchDslCompletionEntry {
        form: quoted_lang.clone(),
        kind: SearchDslCompletionKind::Value,
        signature: "lang:".to_owned(),
        language: lang_id.to_owned(),
        doc: format!("ASP provider `{provider_id}`"),
        insert_text: format!("{quoted_lang} "),
    });

    for ext in &env.manifest.extensions {
        let clean = ext.trim_start_matches('.');
        if clean.is_empty() {
            continue;
        }
        let quoted_ext = format!("\"{clean}\"");
        out.push(SearchDslCompletionEntry {
            form: quoted_ext.clone(),
            kind: SearchDslCompletionKind::Value,
            signature: "ext:".to_owned(),
            language: lang_id.to_owned(),
            doc: format!("Extension `{ext}` owned by {lang_id} provider"),
            insert_text: format!("{quoted_ext} "),
        });
    }
}

fn push_provider_schema_properties(
    env: &ForwardedProviderEnvelope,
    lang_id: &str,
    kind_docs: &BTreeMap<&str, &str>,
    out: &mut Vec<SearchDslCompletionEntry>,
) {
    for prop_name in env.targeted_schema.property_names() {
        push_property_schema_entries(&env.targeted_schema, lang_id, prop_name, kind_docs, out);
    }
    if let Some(special_schema) = env.manifest.special.as_ref().and_then(|s| s.schema.as_ref()) {
        for prop_name in special_schema.property_names() {
            push_property_schema_entries(special_schema, lang_id, prop_name, kind_docs, out);
        }
    }
}

fn push_provider_special_relations(
    env: &ForwardedProviderEnvelope,
    lang_id: &str,
    provider_id: &str,
    out: &mut Vec<SearchDslCompletionEntry>,
) {
    let special_rels = env
        .manifest
        .special
        .iter()
        .flat_map(|s| s.special_relations.iter())
        .chain(
            env.dsl_pack
                .iter()
                .flat_map(|pack| pack.special_relations.iter()),
        );
    for rel in special_rels {
        let sig = format!("({rel} id fid name line)");
        out.push(SearchDslCompletionEntry {
            form: rel.clone(),
            kind: SearchDslCompletionKind::Relation,
            signature: sig.clone(),
            language: lang_id.to_owned(),
            doc: format!("Forwarded special relation from `{provider_id}`"),
            insert_text: sig,
        });
    }
}

/// Project one forwarded provider envelope (`manifest` + `targetedSchema` JSON
/// Schema + `dslPack`) into typed Copilot completion entries.
#[must_use]
pub fn project_provider_envelope_completions(
    env: &ForwardedProviderEnvelope,
) -> Vec<SearchDslCompletionEntry> {
    let lang_id = env
        .dsl_pack
        .as_ref()
        .map_or(env.manifest.language_id.as_str(), |p| p.language_id.as_str());
    let provider_id = env.manifest.provider_id.as_str();
    let kind_docs = build_kind_doc_index(env);
    let mut out = Vec::new();
    push_provider_identity_and_extensions(env, lang_id, provider_id, &mut out);
    push_provider_schema_properties(env, lang_id, &kind_docs, &mut out);
    push_provider_special_relations(env, lang_id, provider_id, &mut out);
    out
}

/// Project an entire ASP [`ForwardedProviderCatalog`] (JSON response from
/// `GET /v1/providers`) into deduplicated [`SearchDslCompletionEntry`] rows,
/// unifying ASP's universal protocol JSON Schema (`universalProtocolSchema`)
/// with every language provider's `manifest.special` & `targetedSchema`.
#[must_use]
pub fn project_provider_catalog_completions(
    catalog: &ForwardedProviderCatalog,
) -> Vec<SearchDslCompletionEntry> {
    let mut entries = Vec::new();
    if let Some(universal) = catalog.universal_protocol_schema.as_ref() {
        entries.extend(project_json_schema_completions("*", universal));
    }
    entries.extend(
        catalog
            .providers
            .iter()
            .flat_map(project_provider_envelope_completions),
    );
    merge_completion_catalogs(Vec::new(), entries)
}

/// Merge two completion catalogs deterministically, preserving order and
/// deduplicating by `(form, kind, language, signature)`.
#[must_use]
pub fn merge_completion_catalogs(
    base: Vec<SearchDslCompletionEntry>,
    extra: Vec<SearchDslCompletionEntry>,
) -> Vec<SearchDslCompletionEntry> {
    let mut seen: HashSet<(String, SearchDslCompletionKind, String, String)> = HashSet::new();
    let mut merged = Vec::with_capacity(base.len().saturating_add(extra.len()));
    for entry in base.into_iter().chain(extra) {
        let key = (
            entry.form.clone(),
            entry.kind,
            entry.language.clone(),
            entry.signature.clone(),
        );
        if seen.insert(key) {
            merged.push(entry);
        }
    }
    merged
}
