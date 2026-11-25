// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Rooted Gerbil Scheme value projection into ASP's typed `SearchPlaybookPlanInput`.

use std::fmt::Display;

use agent_semantic_search_playbook::{
    GraphNativeBlock, ProducerNativeBlock, ProgressiveSearchPlaybookError,
    ProgressiveSearchPlaybookRequest, SEARCH_PLAYBOOK_MAX_COMPOSITION_DEPTH,
    SEARCH_PLAYBOOK_MAX_COMPOSITION_NODES, SearchPlaybookComposition, SearchPlaybookLeaf,
    SearchPlaybookPlanInput, TopologyOwnerMembershipBlock, admit_search_playbook_plan,
};
use gerbil_scheme::{
    CollectionKind, GerbilRuntime, IdentifierBuffer, IdentifierKind, ObjectKind,
    RootedSchemeObject, SchemeScalar,
};

const MAX_PRODUCERS: usize = 32;
const MAX_ARGV_ITEMS: usize = 64;
const MAX_IDENTIFIER_BYTES: usize = 128;
const MAX_TEXT_BYTES: usize = 65_536;

/// Rejection while projecting rooted Scheme search plans into ASP Rust types.
#[derive(Debug, thiserror::Error)]
pub enum SearchPlanProjectionError {
    #[error("search plan native projection failed: {0}")]
    Native(String),
    #[error("invalid search plan projection: {0}")]
    Invalid(&'static str),
    #[error("search playbook admission failed: {0}")]
    Playbook(#[from] ProgressiveSearchPlaybookError),
}

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
    if native(string_obj.len().into_result())? > max_bytes {
        return Err(SearchPlanProjectionError::Invalid(
            "string exceeds byte limit",
        ));
    }
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

fn decode_symbol_vector(
    value: RootedSchemeObject<'_>,
) -> Result<Vec<String>, SearchPlanProjectionError> {
    let items = native(value.into_collection(CollectionKind::Vector))?;
    if items.len() > MAX_PRODUCERS {
        return Err(SearchPlanProjectionError::Invalid(
            "producer vector exceeds limit",
        ));
    }
    let mut names = Vec::with_capacity(items.len());
    for index in 0..items.len() {
        let entry = native(items.element(index))?;
        let name = match native(entry.as_ref().kind())? {
            ObjectKind::Symbol => decode_symbol(entry)?,
            ObjectKind::String => decode_string(entry, MAX_IDENTIFIER_BYTES)?,
            _ => {
                return Err(SearchPlanProjectionError::Invalid(
                    "producer entry must be a symbol or string",
                ));
            }
        };
        names.push(name);
    }
    Ok(names)
}

fn decode_string_vector(
    value: RootedSchemeObject<'_>,
) -> Result<Vec<String>, SearchPlanProjectionError> {
    let items = native(value.into_collection(CollectionKind::Vector))?;
    if items.len() > MAX_ARGV_ITEMS {
        return Err(SearchPlanProjectionError::Invalid(
            "argv vector exceeds limit",
        ));
    }
    let mut values = Vec::with_capacity(items.len());
    for index in 0..items.len() {
        values.push(decode_string(
            native(items.element(index))?,
            MAX_TEXT_BYTES,
        )?);
    }
    Ok(values)
}

fn decode_composition(
    value: RootedSchemeObject<'_>,
    depth: usize,
    node_count: &mut usize,
) -> Result<SearchPlaybookComposition, SearchPlanProjectionError> {
    if depth > SEARCH_PLAYBOOK_MAX_COMPOSITION_DEPTH {
        return Err(SearchPlanProjectionError::Invalid(
            "composition exceeds maximum depth",
        ));
    }
    *node_count = node_count.saturating_add(1);
    if *node_count > SEARCH_PLAYBOOK_MAX_COMPOSITION_NODES {
        return Err(SearchPlanProjectionError::Invalid(
            "composition exceeds maximum node count",
        ));
    }
    let tuple = native(value.into_collection(CollectionKind::Vector))?;
    if tuple.is_empty() {
        return Err(SearchPlanProjectionError::Invalid(
            "empty composition vector",
        ));
    }
    let tag = decode_symbol(native(tuple.element(0))?)?;
    match tag.as_str() {
        "chain" | "intersect" => {
            if tuple.len() != 2 {
                return Err(SearchPlanProjectionError::Invalid(
                    "branch composition arity",
                ));
            }
            let children_vec =
                native(native(tuple.element(1))?.into_collection(CollectionKind::Vector))?;
            let mut children = Vec::with_capacity(children_vec.len());
            for index in 0..children_vec.len() {
                children.push(decode_composition(
                    native(children_vec.element(index))?,
                    depth + 1,
                    node_count,
                )?);
            }
            if tag == "chain" {
                Ok(SearchPlaybookComposition::Chain(children))
            } else {
                Ok(SearchPlaybookComposition::Intersect(children))
            }
        }
        "rg" => {
            if tuple.len() != 2 {
                return Err(SearchPlanProjectionError::Invalid("rg leaf arity"));
            }
            Ok(SearchPlaybookComposition::Leaf(SearchPlaybookLeaf::Rg(
                decode_string_vector(native(tuple.element(1))?)?,
            )))
        }
        "tantivy" => {
            if tuple.len() != 2 {
                return Err(SearchPlanProjectionError::Invalid("tantivy leaf arity"));
            }
            Ok(SearchPlaybookComposition::Leaf(
                SearchPlaybookLeaf::Tantivy(vec![decode_string(
                    native(tuple.element(1))?,
                    MAX_TEXT_BYTES,
                )?]),
            ))
        }
        "topology" => {
            if tuple.len() != 6 {
                return Err(SearchPlanProjectionError::Invalid("topology leaf arity"));
            }
            Ok(SearchPlaybookComposition::Leaf(
                SearchPlaybookLeaf::Topology(TopologyOwnerMembershipBlock {
                    kind: decode_string(native(tuple.element(1))?, MAX_IDENTIFIER_BYTES)?,
                    exact_path: decode_optional_string(native(tuple.element(2))?, MAX_TEXT_BYTES)?,
                    path_prefix: decode_optional_string(native(tuple.element(3))?, MAX_TEXT_BYTES)?,
                    extension: decode_optional_string(
                        native(tuple.element(4))?,
                        MAX_IDENTIFIER_BYTES,
                    )?,
                    path_glob: decode_optional_string(native(tuple.element(5))?, MAX_TEXT_BYTES)?,
                }),
            ))
        }
        "syntax" => {
            if tuple.len() != 3 {
                return Err(SearchPlanProjectionError::Invalid("syntax leaf arity"));
            }
            Ok(SearchPlaybookComposition::Leaf(SearchPlaybookLeaf::Syntax(
                ProducerNativeBlock {
                    producer: decode_symbol(native(tuple.element(1))?)?,
                    argv: vec![decode_string(native(tuple.element(2))?, MAX_TEXT_BYTES)?],
                },
            )))
        }
        "native-syntax" => {
            if tuple.len() != 2 {
                return Err(SearchPlanProjectionError::Invalid(
                    "native-syntax leaf arity",
                ));
            }
            Ok(SearchPlaybookComposition::Leaf(
                SearchPlaybookLeaf::NativeSyntax(decode_string(
                    native(tuple.element(1))?,
                    MAX_TEXT_BYTES,
                )?),
            ))
        }
        "graph" => {
            if tuple.len() != 3 {
                return Err(SearchPlanProjectionError::Invalid("graph leaf arity"));
            }
            Ok(SearchPlaybookComposition::Leaf(SearchPlaybookLeaf::Graph(
                GraphNativeBlock {
                    language: decode_symbol(native(tuple.element(1))?)?,
                    argv: decode_string_vector(native(tuple.element(2))?)?,
                },
            )))
        }
        _ => Err(SearchPlanProjectionError::Invalid(
            "unsupported search composition node kind",
        )),
    }
}

/// Decode a bounded rooted Scheme vector from `scheme/search-engine/funcs.ss` into
/// ASP's typed [`SearchPlaybookPlanInput`] without text S-expression parsing.
pub fn decode_search_plan(
    value: RootedSchemeObject<'_>,
) -> Result<SearchPlaybookPlanInput, SearchPlanProjectionError> {
    let envelope = native(value.into_collection(CollectionKind::Vector))?;
    if envelope.len() != 4 {
        return Err(SearchPlanProjectionError::Invalid(
            "search plan envelope arity",
        ));
    }
    let workspace = decode_optional_string(native(envelope.element(0))?, MAX_IDENTIFIER_BYTES)?;
    let languages = decode_symbol_vector(native(envelope.element(1))?)?;
    let documents = decode_symbol_vector(native(envelope.element(2))?)?;
    let mut node_count = 0usize;
    let composition = decode_composition(native(envelope.element(3))?, 1, &mut node_count)?;
    Ok(SearchPlaybookPlanInput {
        workspace,
        languages,
        documents,
        composition,
    })
}

/// Decode and admit a rooted Scheme search plan through ASP's playbook gate.
pub fn admit_rooted_search_plan(
    value: RootedSchemeObject<'_>,
) -> Result<ProgressiveSearchPlaybookRequest, SearchPlanProjectionError> {
    let plan = decode_search_plan(value)?;
    Ok(admit_search_playbook_plan(plan)?)
}

/// Project a search plan through an AOT-linked `marlin_search_engine_plan_project` C ABI export.
///
/// # Safety
/// `export` must be from `runtime`'s live Marlin programme and return a fresh
/// strong root using the Gerbil Scheme Rust ABI.
pub unsafe fn project_linked(
    runtime: &GerbilRuntime,
    export: unsafe extern "C" fn(i64) -> i64,
    input: Option<&RootedSchemeObject<'_>>,
) -> Result<ProgressiveSearchPlaybookRequest, SearchPlanProjectionError> {
    let procedure = native(unsafe { runtime.bind_object_export(export) })?;
    let default_void = native(runtime.void_object())?;
    let arg = input.unwrap_or(&default_void);
    let projected = native(procedure.call(arg).into_result())?;
    admit_rooted_search_plan(projected)
}

/// Project a live Modern Helm prompt through an AOT-linked `marlin_ui_helm_compile_plan`
/// C ABI export without text serialization.
///
/// # Safety
/// `export` must be from `runtime`'s live Marlin programme and return a fresh
/// strong root using the Gerbil Scheme Rust ABI.
pub unsafe fn project_linked_helm_prompt(
    runtime: &GerbilRuntime,
    export: unsafe extern "C" fn(i64) -> i64,
    prompt: &str,
) -> Result<ProgressiveSearchPlaybookRequest, SearchPlanProjectionError> {
    let procedure = native(unsafe { runtime.bind_object_export(export) })?;
    let prompt_obj = native(runtime.string_from_utf8(prompt))?.into_object();
    let projected = native(procedure.call(&prompt_obj).into_result())?;
    admit_rooted_search_plan(projected)
}

/// Resolve and invoke `:marlin-agent-core/c-abi/search-engine#marlin-search-engine-plan-project`
/// directly from a compiled Gerbil Scheme package directory.
#[cfg(not(feature = "linked-ui"))]
pub fn project_from_package(
    runtime: &GerbilRuntime,
    scheme_dir: &str,
) -> Result<ProgressiveSearchPlaybookRequest, SearchPlanProjectionError> {
    let module = native(runtime.intern_identifier(
        IdentifierKind::Symbol,
        ":marlin-agent-core/c-abi/search-engine",
    ))?;
    let name = native(
        runtime.intern_identifier(IdentifierKind::Symbol, "marlin-search-engine-plan-project"),
    )?;
    let procedure = native(runtime.resolve_package_export(scheme_dir, &module, &name))?;
    if native(procedure.as_ref().kind())? != ObjectKind::Procedure {
        return Err(SearchPlanProjectionError::Invalid(
            "search-engine export is not a procedure",
        ));
    }
    let void_input = native(runtime.void_object())?;
    let projected = native(runtime.call_rooted(&procedure, &void_input))?;
    admit_rooted_search_plan(projected)
}

/// Resolve and invoke `:marlin-agent-core/c-abi/ui-helm#marlin-ui-helm-compile-plan`
/// to compile a live Modern Helm prompt string directly into an admitted ASP
/// Search Playbook through native `RootedSchemeObject` ABI (zero pseudo S-expr).
#[cfg(not(feature = "linked-ui"))]
pub fn project_helm_prompt(
    runtime: &GerbilRuntime,
    scheme_dir: &str,
    prompt: &str,
) -> Result<ProgressiveSearchPlaybookRequest, SearchPlanProjectionError> {
    let module = native(
        runtime.intern_identifier(IdentifierKind::Symbol, ":marlin-agent-core/c-abi/ui-helm"),
    )?;
    let name =
        native(runtime.intern_identifier(IdentifierKind::Symbol, "marlin-ui-helm-compile-plan"))?;
    let procedure = native(runtime.resolve_package_export(scheme_dir, &module, &name))?;
    if native(procedure.as_ref().kind())? != ObjectKind::Procedure {
        return Err(SearchPlanProjectionError::Invalid(
            "ui-helm export is not a procedure",
        ));
    }
    let prompt_obj = native(runtime.string_from_utf8(prompt))?.into_object();
    let projected = native(runtime.call_rooted(&procedure, &prompt_obj))?;
    admit_rooted_search_plan(projected)
}

fn decode_u64(value: RootedSchemeObject<'_>) -> Result<u64, SearchPlanProjectionError> {
    match native(value.as_ref().scalar())? {
        SchemeScalar::Fixnum(v) if v >= 0 => Ok(v as u64),
        _ => Err(SearchPlanProjectionError::Invalid(
            "expected non-negative integer scalar",
        )),
    }
}

/// Decode Scheme-owned `HelmIndexPolicy.` rooted vector into [`crate::WorkspaceIndexConfig`]:
/// `#(skip-dirs-vector max-file-bytes max-line-matches max-line-preview-chars)`.
pub fn decode_workspace_index_config(
    value: RootedSchemeObject<'_>,
) -> Result<crate::WorkspaceIndexConfig, SearchPlanProjectionError> {
    let tuple = native(value.into_collection(CollectionKind::Vector))?;
    if tuple.len() != 4 {
        return Err(SearchPlanProjectionError::Invalid(
            "workspace index config arity",
        ));
    }
    let skip_dirs = decode_string_vector(native(tuple.element(0))?)?;
    let max_file_bytes = decode_u64(native(tuple.element(1))?)?;
    let max_line_matches_per_hit =
        usize::try_from(decode_u64(native(tuple.element(2))?)?).unwrap_or(6);
    let max_line_preview_chars =
        usize::try_from(decode_u64(native(tuple.element(3))?)?).unwrap_or(220);
    Ok(crate::WorkspaceIndexConfig {
        skip_dirs,
        max_file_bytes: max_file_bytes.max(1024),
        max_line_matches_per_hit: max_line_matches_per_hit.max(1),
        max_line_preview_chars: max_line_preview_chars.max(32),
    })
}

/// Project Scheme-owned `HelmIndexPolicy.` through an AOT-linked `marlin_ui_helm_index_config` export.
///
/// # Safety
/// `export` must originate from `runtime`'s live Marlin programme and return a fresh strong root.
pub unsafe fn project_linked_helm_index_config(
    runtime: &GerbilRuntime,
    export: unsafe extern "C" fn(i64) -> i64,
) -> Result<crate::WorkspaceIndexConfig, SearchPlanProjectionError> {
    let procedure = native(unsafe { runtime.bind_object_export(export) })?;
    let void_input = native(runtime.void_object())?;
    let projected = native(procedure.call(&void_input).into_result())?;
    decode_workspace_index_config(projected)
}

/// Resolve and project Scheme-owned `HelmIndexPolicy.` from a compiled package directory.
#[cfg(not(feature = "linked-ui"))]
pub fn project_helm_index_config(
    runtime: &GerbilRuntime,
    scheme_dir: &str,
) -> Result<crate::WorkspaceIndexConfig, SearchPlanProjectionError> {
    let module = native(
        runtime.intern_identifier(IdentifierKind::Symbol, ":marlin-agent-core/c-abi/ui-helm"),
    )?;
    let name =
        native(runtime.intern_identifier(IdentifierKind::Symbol, "marlin-ui-helm-index-config"))?;
    let procedure = native(runtime.resolve_package_export(scheme_dir, &module, &name))?;
    if native(procedure.as_ref().kind())? != ObjectKind::Procedure {
        return Err(SearchPlanProjectionError::Invalid(
            "ui-helm index config export is not a procedure",
        ));
    }
    let void_input = native(runtime.void_object())?;
    let projected = native(runtime.call_rooted(&procedure, &void_input))?;
    decode_workspace_index_config(projected)
}
