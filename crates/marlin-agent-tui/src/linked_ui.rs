// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Process-owned AOT graph selected by the package's Yuanxu build projection.

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
    fn marlin_ui_refresh_labels(root: i64) -> i64;
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
        refresh_labels: marlin_ui_refresh_labels,
    }
}
