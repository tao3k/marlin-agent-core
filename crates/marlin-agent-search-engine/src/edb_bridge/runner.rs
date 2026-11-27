// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Linked AOT and compiled-package execution runners for Marlin's Scheme
//! `:gerbil-ascent` / Dusa / `:gerbil-parser` Search DSL.

use std::path::{Path, PathBuf};
#[cfg(not(feature = "linked-ui"))]
use std::sync::{Arc, RwLock};

#[cfg(not(feature = "linked-ui"))]
use agent_semantic_search::WorkspaceSearchIndex;
use gerbil_scheme::GerbilRuntime;
#[cfg(not(feature = "linked-ui"))]
use gerbil_scheme::{IdentifierKind, RootedSchemeObject};

use super::codec::{
    collect_symbol_ref_rows, decode_edb_prefilter, decode_rooted_search_dsl_result_with_context,
    decode_search_dsl_completions, encode_edb_snapshot_with_refs, make_string, make_vector, native,
};
use super::types::SearchDslDecodeContext;
use super::{
    SearchDslCompletionEntry, SearchDslExecutionRequest, SearchDslLinkedExports, SearchDslRunner,
    SearchDslStructuredResult, SearchDslStructuredRunner,
};
use crate::SearchPlanProjectionError;

/// Execute a native `:gerbil-ascent` query through AOT-linked C-ABI exports
/// and return the full structured [`SearchDslStructuredResult`].
///
/// # Safety
/// `exports` must originate from `runtime`'s live Marlin programme.
pub unsafe fn execute_linked_search_dsl_structured(
    runtime: &GerbilRuntime,
    exports: SearchDslLinkedExports,
    request: &SearchDslExecutionRequest<'_>,
) -> Result<SearchDslStructuredResult, SearchPlanProjectionError> {
    let compile_fn = native(unsafe { runtime.bind_object_export(exports.compile_prefilter) })?;
    let query_obj = make_string(runtime, request.query)?;
    let prefilter_obj = native(compile_fn.call(&query_obj).into_result())?;
    let owned_prefilter = decode_edb_prefilter(prefilter_obj)?;
    let snapshot = request
        .index
        .project_prefiltered_edb(&owned_prefilter.as_prefilter());
    let symbol_refs = if owned_prefilter.hydration.includes_symbol_refs() {
        collect_symbol_ref_rows(request.index, &snapshot, &owned_prefilter)
    } else {
        Vec::new()
    };
    let edb_obj = encode_edb_snapshot_with_refs(
        runtime,
        &snapshot,
        owned_prefilter.hydration,
        &symbol_refs,
    )?;
    let eval_input = make_vector(runtime, &[query_obj, edb_obj])?;

    let eval_fn = native(unsafe { runtime.bind_object_export(exports.eval_edb) })?;
    let result_obj = native(eval_fn.call(&eval_input).into_result())?;
    decode_rooted_search_dsl_result_with_context(
        result_obj,
        &snapshot,
        SearchDslDecodeContext {
            index: Some(request.index),
            prefilter: Some(&owned_prefilter),
            output_limit: request.output_limit,
        },
    )
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
    Ok(unsafe { execute_linked_search_dsl_structured(runtime, exports, request) }?.rendered_text)
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

#[cfg(not(feature = "linked-ui"))]
fn execute_with_bound_package_exports(
    runtime: &GerbilRuntime,
    compile_fn: &RootedSchemeObject<'_>,
    eval_fn: &RootedSchemeObject<'_>,
    request: &SearchDslExecutionRequest<'_>,
) -> Result<SearchDslStructuredResult, SearchPlanProjectionError> {
    let query_obj = make_string(runtime, request.query)?;
    let prefilter_obj = native(runtime.call_rooted(compile_fn, &query_obj))?;
    let owned_prefilter = decode_edb_prefilter(prefilter_obj)?;
    let snapshot = request
        .index
        .project_prefiltered_edb(&owned_prefilter.as_prefilter());
    let symbol_refs = if owned_prefilter.hydration.includes_symbol_refs() {
        collect_symbol_ref_rows(request.index, &snapshot, &owned_prefilter)
    } else {
        Vec::new()
    };
    let edb_obj = encode_edb_snapshot_with_refs(
        runtime,
        &snapshot,
        owned_prefilter.hydration,
        &symbol_refs,
    )?;
    let eval_input = make_vector(runtime, &[query_obj, edb_obj])?;
    let result_obj = native(runtime.call_rooted(eval_fn, &eval_input))?;
    decode_rooted_search_dsl_result_with_context(
        result_obj,
        &snapshot,
        SearchDslDecodeContext {
            index: Some(request.index),
            prefilter: Some(&owned_prefilter),
            output_limit: request.output_limit,
        },
    )
}

/// Execute a native `:gerbil-ascent` query by resolving `:marlin-agent-core/c-abi/search-dsl`
/// and returning a structured [`SearchDslStructuredResult`].
#[cfg(not(feature = "linked-ui"))]
pub fn execute_package_search_dsl_structured(
    runtime: &GerbilRuntime,
    scheme_dir: &str,
    request: &SearchDslExecutionRequest<'_>,
) -> Result<SearchDslStructuredResult, SearchPlanProjectionError> {
    let (compile_fn, eval_fn) = resolve_package_search_dsl_exports(runtime, scheme_dir)?;
    execute_with_bound_package_exports(runtime, &compile_fn, &eval_fn, request)
}

/// Execute a native `:gerbil-ascent` query by resolving
/// `:marlin-agent-core/c-abi/search-dsl` from a compiled Gerbil package tree.
#[cfg(not(feature = "linked-ui"))]
pub fn execute_package_search_dsl(
    runtime: &GerbilRuntime,
    scheme_dir: &str,
    request: &SearchDslExecutionRequest<'_>,
) -> Result<String, SearchPlanProjectionError> {
    Ok(execute_package_search_dsl_structured(runtime, scheme_dir, request)?.rendered_text)
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

/// Return the default package-backed [`SearchDslStructuredRunner`] when `linked-ui`
/// is not active, or `None` under AOT `linked-ui`.
#[must_use]
pub fn default_search_dsl_structured_runner() -> Option<SearchDslStructuredRunner> {
    #[cfg(not(feature = "linked-ui"))]
    {
        Some(default_package_search_dsl_structured_runner())
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
    reply:
        std::sync::mpsc::SyncSender<Result<SearchDslStructuredResult, SearchPlanProjectionError>>,
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
                let mut cached_exports: Option<(
                    String,
                    RootedSchemeObject<'_>,
                    RootedSchemeObject<'_>,
                )> = None;
                let mut synced_asp_endpoint: Option<String> = None;
                while let Ok(job) = rx.recv() {
                    let outcome = match &runtime {
                        Ok(rt) => run_worker_dsl_job(
                            rt,
                            &job,
                            &mut cached_exports,
                            &mut synced_asp_endpoint,
                        ),
                        Err(err) => Err(SearchPlanProjectionError::Native(err.to_string())),
                    };
                    let _ = job.reply.send(outcome);
                }
            })
            .expect("spawn marlin-search-dsl-vm thread");
        tx
    })
}

#[cfg(not(feature = "linked-ui"))]
fn run_worker_dsl_job<'a>(
    rt: &'a GerbilRuntime,
    job: &PackageDslJob,
    cached_exports: &mut Option<(String, RootedSchemeObject<'a>, RootedSchemeObject<'a>)>,
    synced_asp_endpoint: &mut Option<String>,
) -> Result<SearchDslStructuredResult, SearchPlanProjectionError> {
    let needs_resolve = cached_exports
        .as_ref()
        .is_none_or(|(dir, _, _)| dir != &job.scheme_dir);
    if needs_resolve {
        let (compile_fn, eval_fn) = resolve_package_search_dsl_exports(rt, &job.scheme_dir)?;
        *cached_exports = Some((job.scheme_dir.clone(), compile_fn, eval_fn));
    }
    if let Some(endpoint) = crate::asp_http_client::AspForwardingEndpoint::from_env()
        && synced_asp_endpoint.as_deref() != Some(endpoint.as_str())
        && crate::asp_http_client::sync_package_asp_provider_descriptors(
            rt,
            &job.scheme_dir,
            &endpoint,
        )
        .is_ok()
    {
        *synced_asp_endpoint = Some(endpoint.as_str().to_owned());
    }
    let Some((_, compile_fn, eval_fn)) = cached_exports.as_ref() else {
        return Err(SearchPlanProjectionError::Invalid(
            "search DSL exports unavailable",
        ));
    };
    let guard = job
        .index
        .read()
        .map_err(|_| SearchPlanProjectionError::Invalid("search index lock poisoned"))?;
    let req = SearchDslExecutionRequest {
        index: &guard,
        query: &job.query,
        output_limit: job.output_limit,
    };
    execute_with_bound_package_exports(rt, compile_fn, eval_fn, &req)
}

#[cfg(not(feature = "linked-ui"))]
fn dispatch_package_dsl_job(
    shared_index: Arc<RwLock<WorkspaceSearchIndex>>,
    query: &str,
    output_limit: usize,
) -> Result<SearchDslStructuredResult, SearchPlanProjectionError> {
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
}

/// Build a default structured [`SearchDslStructuredRunner`] backed by a dedicated
/// thread-affine [`GerbilRuntime`] worker when `linked-ui` is not active.
#[cfg(not(feature = "linked-ui"))]
#[must_use]
pub fn default_package_search_dsl_structured_runner() -> SearchDslStructuredRunner {
    Arc::new(|shared_index, query, output_limit| {
        dispatch_package_dsl_job(shared_index, query, output_limit)
    })
}

/// Build a default [`SearchDslRunner`] backed by a dedicated thread-affine
/// [`GerbilRuntime`] worker and the compiled Marlin Scheme package when
/// `linked-ui` is not active.
#[cfg(not(feature = "linked-ui"))]
#[must_use]
pub fn default_package_search_dsl_runner() -> SearchDslRunner {
    Arc::new(|shared_index, query, output_limit| {
        Ok(dispatch_package_dsl_job(shared_index, query, output_limit)?.rendered_text)
    })
}
