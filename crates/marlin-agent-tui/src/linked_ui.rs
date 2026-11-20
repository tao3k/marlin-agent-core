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
    fn marlin_tooling_project(root: i64) -> i64;
    fn marlin_envrc_plan(root: i64) -> i64;
    fn marlin_worktree_context_prepare(root: i64) -> i64;
    fn marlin_worktree_context_deliver(root: i64) -> i64;
    fn marlin_worktree_provider(root: i64) -> i64;
    fn gerbil_scheme_rust_take_call_error_raw() -> i64;
}

pub(crate) enum WorktreeOperation {
    Prepare,
    Deliver,
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
        WorktreeOperation::Provider => marlin_worktree_provider,
    };
    // SAFETY: these rooted exports belong to the linked programme above.
    unsafe { runtime.bind_object_export(export) }?
        .call(request)
        .into_result()
        .map_err(|source| RootedCallError {
            source,
            diagnostic: worktree_diagnostic(runtime, request),
        })
}

fn worktree_diagnostic(
    runtime: &GerbilRuntime,
    request: &RootedSchemeObject<'_>,
) -> Option<String> {
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

#[cfg(test)]
#[path = "../tests/unit/linked_ui_source.rs"]
mod tests;
