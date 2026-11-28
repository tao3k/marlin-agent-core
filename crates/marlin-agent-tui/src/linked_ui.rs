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
    fn marlin_search_dsl_register_provider_descriptor(root: i64) -> i64;
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

pub(crate) fn helm_index_policy(
    runtime: &GerbilRuntime,
) -> Result<
    marlin_agent_search_engine::HelmIndexPolicyProjection,
    marlin_agent_search_engine::SearchPlanProjectionError,
> {
    // SAFETY: `marlin_ui_helm_index_config` belongs to the linked Marlin TUI programme.
    unsafe {
        marlin_agent_search_engine::project_linked_helm_index_policy(
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

pub(crate) fn sync_asp_provider_descriptors(
    runtime: &GerbilRuntime,
    endpoint: &marlin_agent_search_engine::AspForwardingEndpoint,
) -> Result<usize, marlin_agent_search_engine::SearchPlanProjectionError> {
    // SAFETY: `marlin_search_dsl_register_provider_descriptor` belongs to the linked Marlin TUI programme.
    unsafe {
        marlin_agent_search_engine::sync_linked_asp_provider_descriptors(
            runtime,
            marlin_search_dsl_register_provider_descriptor,
            endpoint,
        )
    }
}

#[cfg(test)]
pub(crate) fn register_dynamic_language_descriptor(
    runtime: &GerbilRuntime,
    request: &marlin_agent_search_engine::DynamicLanguageDescriptorRequest<'_>,
) -> Result<usize, marlin_agent_search_engine::SearchPlanProjectionError> {
    // SAFETY: `marlin_search_dsl_register_provider_descriptor` belongs to the linked Marlin TUI programme.
    unsafe {
        marlin_agent_search_engine::register_linked_dynamic_language_descriptor(
            runtime,
            marlin_search_dsl_register_provider_descriptor,
            request,
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
        register_provider_descriptor: marlin_search_dsl_register_provider_descriptor,
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

pub(crate) fn execute_search_dsl_session_structured(
    runtime: &GerbilRuntime,
    index: &marlin_agent_search_engine::WorkspaceSearchIndex,
    query: &str,
    output_limit: usize,
    session_id: &marlin_agent_search_engine::SearchDslSessionId,
) -> Result<
    marlin_agent_search_engine::SearchDslStructuredResult,
    marlin_agent_search_engine::SearchPlanProjectionError,
> {
    let exports = marlin_agent_search_engine::SearchDslLinkedExports {
        compile_prefilter: marlin_search_dsl_compile_prefilter,
        eval_edb: marlin_search_dsl_eval_edb,
        extract_facts: marlin_search_dsl_extract_facts,
        completions: marlin_search_dsl_completions,
        register_provider_descriptor: marlin_search_dsl_register_provider_descriptor,
    };
    let request = marlin_agent_search_engine::SearchDslSessionExecutionRequest {
        execution: marlin_agent_search_engine::SearchDslExecutionRequest {
            index,
            query,
            output_limit,
        },
        session_id,
    };
    // SAFETY: `exports` belong to the linked Marlin TUI programme initialized in `runtime`.
    unsafe {
        marlin_agent_search_engine::execute_linked_search_dsl_session_structured(
            runtime, exports, &request,
        )
    }
}

/// Service any pending Helm `:gerbil-ascent` / Dusa query directly on the TUI's
/// owning Gerbil VM thread (zero channel deadlock, sub-millisecond evaluation).
pub(crate) fn service_helm_dsl(runtime: &GerbilRuntime, app: &mut super::app::App) -> bool {
    let Some((shared_index, query_epoch, query)) = app
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
        helm.apply_dsl_result(query_epoch, outcome);
    }
    true
}

/// Maximum queued L1 (`FastAscent` / `BruteForce`) jobs in the priority-isolated
/// search DSL ingress channel and VM mailbox (SM11).
const MAX_FAST_DSL_JOBS: usize = 64;
/// Maximum queued L2 (`DusaSolver`) jobs in the priority-isolated search DSL
/// ingress channel and VM mailbox (SM11).
const MAX_DUSA_DSL_JOBS: usize = 32;

struct LinkedSearchDslJob {
    index: std::sync::Arc<std::sync::RwLock<marlin_agent_search_engine::WorkspaceSearchIndex>>,
    query: String,
    output_limit: usize,
    session_id: marlin_agent_search_engine::SearchDslSessionId,
    reply: tokio::sync::oneshot::Sender<
        Result<
            marlin_agent_search_engine::SearchDslStructuredResult,
            marlin_agent_search_engine::SearchPlanProjectionError,
        >,
    >,
}

/// Priority-isolated, bounded, time-sliced thread-affine mailbox delivering
/// `workspace_search` tool calls onto the TUI's linked Gerbil Scheme VM thread
/// without ever blocking Tokio worker threads or stalling the 60 FPS TUI render loop.
///
/// - Separate bounded ingress channels (`MAX_FAST_DSL_JOBS` / `MAX_DUSA_DSL_JOBS`)
///   prevent saturated L2 (`DusaSolver`) bursts from ever starving L1 (`FastAscent`).
/// - L1 (`FastAscent`) queries preempt waiting L2 (`DusaSolver`) queries (4:1 ratio
///   for starvation-free weighted fair queueing).
/// - Cancelled or timed-out callers (`reply.is_closed()`) are actively compacted in
///   O(cap) before touching the Gerbil VM, and idle queues reclaim excess capacity.
/// - Draining yields once `budget` (e.g. `8ms`) elapses so keyboard input and
///   rendering remain responsive under heavy multi-agent Dusa workloads.
pub(crate) struct LinkedSearchDslMailbox {
    fast_rx: tokio::sync::mpsc::Receiver<LinkedSearchDslJob>,
    dusa_rx: tokio::sync::mpsc::Receiver<LinkedSearchDslJob>,
    fast_queue: std::collections::VecDeque<LinkedSearchDslJob>,
    dusa_queue: std::collections::VecDeque<LinkedSearchDslJob>,
    fast_burst: u8,
}

impl LinkedSearchDslMailbox {
    /// Drain pending `workspace_search` jobs within `budget`, returning `true` if
    /// queued jobs remain for the next frame slice.
    pub(crate) fn drain_budgeted(
        &mut self,
        runtime: &gerbil_scheme::GerbilRuntime,
        budget: std::time::Duration,
    ) -> bool {
        let started = std::time::Instant::now();
        self.ingest_pending();
        while started.elapsed() < budget {
            let Some(job) = self.pop_next_live_job() else {
                break;
            };
            let outcome = match job.index.read() {
                Ok(index) => execute_search_dsl_session_structured(
                    runtime,
                    &index,
                    &job.query,
                    job.output_limit,
                    &job.session_id,
                ),
                Err(_) => Err(
                    marlin_agent_search_engine::SearchPlanProjectionError::Invalid(
                        "resident search index lock poisoned",
                    ),
                ),
            };
            let _ = job.reply.send(outcome);
            self.ingest_pending();
        }
        if self.fast_queue.is_empty() && self.fast_queue.capacity() > MAX_FAST_DSL_JOBS {
            self.fast_queue.shrink_to(MAX_FAST_DSL_JOBS);
        }
        if self.dusa_queue.is_empty() && self.dusa_queue.capacity() > MAX_DUSA_DSL_JOBS {
            self.dusa_queue.shrink_to(MAX_DUSA_DSL_JOBS);
        }
        !self.fast_queue.is_empty() || !self.dusa_queue.is_empty()
    }

    fn ingest_pending(&mut self) {
        self.fast_queue.retain(|job| !job.reply.is_closed());
        self.dusa_queue.retain(|job| !job.reply.is_closed());
        while let Ok(job) = self.fast_rx.try_recv() {
            if job.reply.is_closed() {
                continue;
            }
            if self.fast_queue.len() < MAX_FAST_DSL_JOBS {
                self.fast_queue.push_back(job);
            } else {
                let _ = job.reply.send(Err(
                    marlin_agent_search_engine::SearchPlanProjectionError::Invalid(
                        "TUI search DSL fast mailbox saturated; shed excess load",
                    ),
                ));
            }
        }
        while let Ok(job) = self.dusa_rx.try_recv() {
            if job.reply.is_closed() {
                continue;
            }
            if self.dusa_queue.len() < MAX_DUSA_DSL_JOBS {
                self.dusa_queue.push_back(job);
            } else {
                let _ = job.reply.send(Err(
                    marlin_agent_search_engine::SearchPlanProjectionError::Invalid(
                        "TUI search DSL solver mailbox saturated; shed excess load",
                    ),
                ));
            }
        }
    }

    fn pop_next_live_job(&mut self) -> Option<LinkedSearchDslJob> {
        loop {
            let take_dusa =
                self.fast_queue.is_empty() || (!self.dusa_queue.is_empty() && self.fast_burst >= 4);
            let next = if take_dusa {
                self.fast_burst = 0;
                self.dusa_queue.pop_front()
            } else {
                self.fast_burst = self.fast_burst.saturating_add(1);
                self.fast_queue.pop_front()
            }?;
            if !next.reply.is_closed() {
                return Some(next);
            }
        }
    }
}

/// Create a non-blocking, priority-isolated bounded `AsyncSearchDslSessionRunner`
/// paired with its time-sliced VM-thread [`LinkedSearchDslMailbox`].
pub(crate) fn search_dsl_async_channel() -> (
    marlin_agent_tooling::AsyncSearchDslSessionRunner,
    LinkedSearchDslMailbox,
) {
    let (fast_tx, fast_rx) = tokio::sync::mpsc::channel::<LinkedSearchDslJob>(MAX_FAST_DSL_JOBS);
    let (dusa_tx, dusa_rx) = tokio::sync::mpsc::channel::<LinkedSearchDslJob>(MAX_DUSA_DSL_JOBS);
    let runner: marlin_agent_tooling::AsyncSearchDslSessionRunner =
        std::sync::Arc::new(move |index, query, output_limit, tier, session_id| {
            let tx = match tier {
                marlin_agent_tooling::SearchQueryTier::DusaSolver => dusa_tx.clone(),
                marlin_agent_tooling::SearchQueryTier::FastAscent
                | marlin_agent_tooling::SearchQueryTier::BruteForce => fast_tx.clone(),
            };
            Box::pin(async move {
                let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
                tx.try_send(LinkedSearchDslJob {
                    index,
                    query,
                    output_limit,
                    session_id,
                    reply: reply_tx,
                })
                .map_err(|err| match err {
                    tokio::sync::mpsc::error::TrySendError::Full(_) => {
                        marlin_agent_search_engine::SearchPlanProjectionError::Invalid(
                            "TUI search DSL mailbox saturated; shed excess load",
                        )
                    }
                    tokio::sync::mpsc::error::TrySendError::Closed(_) => {
                        marlin_agent_search_engine::SearchPlanProjectionError::Invalid(
                            "TUI Scheme VM loop closed",
                        )
                    }
                })?;
                reply_rx.await.map_err(|_| {
                    marlin_agent_search_engine::SearchPlanProjectionError::Invalid(
                        "TUI Scheme VM response channel closed",
                    )
                })?
            })
        });
    (
        runner,
        LinkedSearchDslMailbox {
            fast_rx,
            dusa_rx,
            fast_queue: std::collections::VecDeque::with_capacity(MAX_FAST_DSL_JOBS.min(16)),
            dusa_queue: std::collections::VecDeque::with_capacity(MAX_DUSA_DSL_JOBS.min(16)),
            fast_burst: 0,
        },
    )
}

#[cfg(test)]
#[path = "../tests/unit/linked_ui_source.rs"]
mod tests;

#[cfg(test)]
#[path = "../tests/unit/linked_ui_stability.rs"]
mod stability_tests;
