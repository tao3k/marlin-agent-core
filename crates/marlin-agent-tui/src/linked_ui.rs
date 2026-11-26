// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Process-owned Gerbil VM and its linked, Yuanxu-selected module graph.

use gerbil_scheme::{
    GerbilRuntime, LinkedGerbilProgram, NativeError, RootedCallError, RootedSchemeObject,
};
use gerbil_scheme_sys::{GerbilGlobalState, GerbilModuleOrLink};
use marlin_agent_ui::UiLinkedExports;

#[cfg(test)]
use super::ui_host::UiHost as TestUiHost;

unsafe extern "C" {
    fn ___LNK_marlin__tui(gs: *mut GerbilGlobalState) -> *mut GerbilModuleOrLink;
    fn marlin_ui_publication(root: i64) -> i64;
    fn marlin_ui_package_event(root: i64) -> i64;
    fn marlin_ui_open(root: i64) -> i64;
    fn marlin_ui_project_workbench(root: i64) -> i64;
    fn marlin_ui_dispatch_event(root: i64) -> i64;
    fn marlin_ui_browse_command(root: i64) -> i64;
    fn marlin_ui_command_candidates(root: i64) -> i64;
    fn marlin_ui_review_menu(root: i64) -> i64;
    fn marlin_ui_refresh_labels(root: i64) -> i64;
    fn marlin_ui_helm_index_config(root: i64) -> i64;
    fn marlin_search_dsl_compile_prefilter(root: i64) -> i64;
    fn marlin_search_dsl_eval_edb(root: i64) -> i64;
    fn marlin_search_dsl_extract_facts(root: i64) -> i64;
    fn marlin_search_dsl_completions(root: i64) -> i64;
    fn marlin_tooling_project(root: i64) -> i64;
    fn marlin_envrc_plan(root: i64) -> i64;
    fn marlin_worktree_context_prepare(root: i64) -> i64;
    fn marlin_worktree_context_deliver(root: i64) -> i64;
    fn marlin_worktree_context_revoke(root: i64) -> i64;
    fn marlin_worktree_provider(root: i64) -> i64;
    fn marlin_context_artifacts_exchange(root: i64) -> i64;
    fn gerbil_scheme_rust_take_call_error_raw() -> i64;
}

pub(crate) enum WorktreeOperation {
    Prepare,
    Deliver,
    Revoke,
    Provider,
}

pub(crate) fn worktree_call<'a>(
    runtime: &'a GerbilRuntime,
    operation: WorktreeOperation,
    request: &RootedSchemeObject<'_>,
) -> Result<RootedSchemeObject<'a>, RootedCallError> {
    let export = match operation {
        WorktreeOperation::Prepare => marlin_worktree_context_prepare,
        WorktreeOperation::Deliver => marlin_worktree_context_deliver,
        WorktreeOperation::Revoke => marlin_worktree_context_revoke,
        WorktreeOperation::Provider => marlin_worktree_provider,
    };
    call_context(runtime, export, request)
}

pub(crate) fn artifacts_exchange<'a>(
    runtime: &'a GerbilRuntime,
    request: &RootedSchemeObject<'_>,
) -> Result<RootedSchemeObject<'a>, RootedCallError> {
    call_context(runtime, marlin_context_artifacts_exchange, request)
}

fn call_context<'a>(
    runtime: &'a GerbilRuntime,
    export: unsafe extern "C" fn(i64) -> i64,
    request: &RootedSchemeObject<'_>,
) -> Result<RootedSchemeObject<'a>, RootedCallError> {
    // SAFETY: the private callers select C exports from this linked program.
    unsafe { runtime.bind_object_export(export) }?
        .call(request)
        .into_result()
        .map_err(|source| RootedCallError {
            source,
            diagnostic: context_diagnostic(runtime, request),
        })
}

fn context_diagnostic(runtime: &GerbilRuntime, request: &RootedSchemeObject<'_>) -> Option<String> {
    unsafe extern "C" fn take_error(_: i64) -> i64 {
        // SAFETY: the VM owner consumes its last diagnostic.
        unsafe { gerbil_scheme_rust_take_call_error_raw() }
    }
    // SAFETY: the getter belongs to this initialized VM.
    let export = unsafe { runtime.bind_object_export(take_error) }.ok()?;
    let bytes = export
        .call(request)
        .into_result()
        .ok()?
        .into_bytevector()
        .ok()?;
    String::from_utf8(bytes.to_vec().into_result().ok()?).ok()
}

pub(crate) fn initialize() -> Result<GerbilRuntime, NativeError> {
    // SAFETY: build.rs links one archive produced from the selected programme
    // manifest with the same SDK revision as this runtime.
    let program = unsafe { LinkedGerbilProgram::from_linker(___LNK_marlin__tui) };
    GerbilRuntime::initialize_program(program)
}

pub(crate) fn exports() -> UiLinkedExports {
    UiLinkedExports {
        publication: marlin_ui_publication,
        package_event: marlin_ui_package_event,
        open: marlin_ui_open,
        project: marlin_ui_project_workbench,
        dispatch: marlin_ui_dispatch_event,
        browse_command: marlin_ui_browse_command,
        command_candidates: marlin_ui_command_candidates,
        review_menu: marlin_ui_review_menu,
        refresh_labels: marlin_ui_refresh_labels,
    }
}

pub(crate) fn envrc_plan<'a>(
    runtime: &'a GerbilRuntime,
    request: &RootedSchemeObject<'_>,
) -> Result<RootedSchemeObject<'a>, NativeError> {
    // SAFETY: this strong-root export belongs to the initialized linked VM.
    let export = unsafe { runtime.bind_object_export(marlin_envrc_plan) }?;
    export.call(request).into_result()
}

pub(crate) fn tooling_declarations(
    runtime: &GerbilRuntime,
) -> Result<
    Vec<marlin_agent_runtime::ModelToolDeclaration>,
    marlin_agent_tooling::ToolingProjectionError,
> {
    // SAFETY: this export belongs to the same linked programme initialized above.
    unsafe { marlin_agent_tooling::project_linked(runtime, marlin_tooling_project) }
}

pub(crate) fn helm_index_config(
    runtime: &GerbilRuntime,
) -> Result<
    marlin_agent_search_engine::WorkspaceIndexConfig,
    marlin_agent_search_engine::SearchPlanProjectionError,
> {
    // SAFETY: `marlin_ui_helm_index_config` belongs to the linked Marlin TUI programme.
    unsafe {
        marlin_agent_search_engine::project_linked_helm_index_config(
            runtime,
            marlin_ui_helm_index_config,
        )
    }
}

pub(crate) fn search_dsl_completions(
    runtime: &GerbilRuntime,
    prefix: &str,
) -> Result<
    Vec<marlin_agent_search_engine::SearchDslCompletionEntry>,
    marlin_agent_search_engine::SearchPlanProjectionError,
> {
    // SAFETY: `marlin_search_dsl_completions` belongs to the linked Marlin TUI programme.
    unsafe {
        marlin_agent_search_engine::project_linked_search_dsl_completions(
            runtime,
            marlin_search_dsl_completions,
            prefix,
        )
    }
}

pub(crate) fn execute_search_dsl_structured(
    runtime: &GerbilRuntime,
    index: &marlin_agent_search_engine::WorkspaceSearchIndex,
    query: &str,
    output_limit: usize,
) -> Result<
    marlin_agent_search_engine::SearchDslStructuredResult,
    marlin_agent_search_engine::SearchPlanProjectionError,
> {
    let exports = marlin_agent_search_engine::SearchDslLinkedExports {
        compile_prefilter: marlin_search_dsl_compile_prefilter,
        eval_edb: marlin_search_dsl_eval_edb,
        extract_facts: marlin_search_dsl_extract_facts,
        completions: marlin_search_dsl_completions,
    };
    let request = marlin_agent_search_engine::SearchDslExecutionRequest {
        index,
        query,
        output_limit,
    };
    // SAFETY: `exports` belong to the linked Marlin TUI programme initialized in `runtime`.
    unsafe {
        marlin_agent_search_engine::execute_linked_search_dsl_structured(runtime, exports, &request)
    }
}

pub(crate) fn execute_search_dsl(
    runtime: &GerbilRuntime,
    index: &marlin_agent_search_engine::WorkspaceSearchIndex,
    query: &str,
    output_limit: usize,
) -> Result<String, marlin_agent_search_engine::SearchPlanProjectionError> {
    Ok(execute_search_dsl_structured(runtime, index, query, output_limit)?.rendered_text)
}

/// Service any pending Helm `:gerbil-ascent` / Dusa query directly on the TUI's
/// owning Gerbil VM thread (zero channel deadlock, sub-millisecond evaluation).
pub(crate) fn service_helm_dsl(runtime: &GerbilRuntime, app: &mut super::app::App) -> bool {
    let Some((shared_index, query)) = app
        .helm
        .as_mut()
        .and_then(super::helm::HelmOverlay::take_pending_dsl_query)
    else {
        return false;
    };
    let outcome = match shared_index.read() {
        Ok(index) => execute_search_dsl_structured(runtime, &index, &query, 16_384)
            .map_err(|err| err.to_string()),
        Err(_) => Err("resident search index lock poisoned".to_owned()),
    };
    if let Some(helm) = app.helm.as_mut() {
        helm.apply_dsl_result(outcome);
    }
    true
}

struct LinkedSearchDslJob {
    index: std::sync::Arc<std::sync::RwLock<marlin_agent_search_engine::WorkspaceSearchIndex>>,
    query: String,
    output_limit: usize,
    reply: std::sync::mpsc::Sender<
        Result<String, marlin_agent_search_engine::SearchPlanProjectionError>,
    >,
}

/// Thread-affine mailbox delivering `workspace_search` tool calls onto the TUI's
/// linked Gerbil Scheme VM thread.
pub(crate) struct LinkedSearchDslMailbox {
    rx: std::sync::mpsc::Receiver<LinkedSearchDslJob>,
}

impl LinkedSearchDslMailbox {
    /// Drain any pending `workspace_search` requests on the owning Gerbil VM thread.
    pub(crate) fn drain(&self, runtime: &gerbil_scheme::GerbilRuntime) {
        while let Ok(job) = self.rx.try_recv() {
            let outcome = match job.index.read() {
                Ok(index) => execute_search_dsl(runtime, &index, &job.query, job.output_limit),
                Err(_) => Err(
                    marlin_agent_search_engine::SearchPlanProjectionError::Invalid(
                        "resident search index lock poisoned",
                    ),
                ),
            };
            let _ = job.reply.send(outcome);
        }
    }
}

/// Create a linked `SearchDslRunner` handle paired with its VM-thread mailbox.
pub(crate) fn search_dsl_channel() -> (
    marlin_agent_search_engine::SearchDslRunner,
    LinkedSearchDslMailbox,
) {
    let (tx, rx) = std::sync::mpsc::channel::<LinkedSearchDslJob>();
    let runner: marlin_agent_search_engine::SearchDslRunner =
        std::sync::Arc::new(move |index, query, output_limit| {
            let (reply_tx, reply_rx) = std::sync::mpsc::channel();
            tx.send(LinkedSearchDslJob {
                index,
                query: query.to_owned(),
                output_limit,
                reply: reply_tx,
            })
            .map_err(|_| {
                marlin_agent_search_engine::SearchPlanProjectionError::Invalid(
                    "TUI Scheme VM loop closed",
                )
            })?;
            reply_rx.recv().map_err(|_| {
                marlin_agent_search_engine::SearchPlanProjectionError::Invalid(
                    "TUI Scheme VM response channel closed",
                )
            })?
        });
    (runner, LinkedSearchDslMailbox { rx })
}

#[cfg(test)]
#[path = "../tests/unit/linked_ui_source.rs"]
mod tests;
