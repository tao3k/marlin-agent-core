// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Process-owned Gerbil VM and its linked, Yuanxu-selected module graph.

use gerbil_scheme::{GerbilRuntime, LinkedGerbilProgram, NativeError};
use gerbil_scheme_sys::{GerbilGlobalState, GerbilModuleOrLink};
use marlin_agent_ui::UiLinkedExports;

unsafe extern "C" {
    fn ___LNK_marlin__tui(gs: *mut GerbilGlobalState) -> *mut GerbilModuleOrLink;
    fn marlin_ui_publication(root: i64) -> i64;
    fn marlin_ui_open(root: i64) -> i64;
    fn marlin_ui_project_workbench(root: i64) -> i64;
    fn marlin_ui_dispatch_event(root: i64) -> i64;
    fn marlin_ui_browse_command(root: i64) -> i64;
    fn marlin_ui_command_candidates(root: i64) -> i64;
    fn marlin_ui_review_menu(root: i64) -> i64;
    fn marlin_ui_refresh_labels(root: i64) -> i64;
    fn marlin_tooling_project(root: i64) -> i64;
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
        open: marlin_ui_open,
        project: marlin_ui_project_workbench,
        dispatch: marlin_ui_dispatch_event,
        browse_command: marlin_ui_browse_command,
        command_candidates: marlin_ui_command_candidates,
        review_menu: marlin_ui_review_menu,
        refresh_labels: marlin_ui_refresh_labels,
    }
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
