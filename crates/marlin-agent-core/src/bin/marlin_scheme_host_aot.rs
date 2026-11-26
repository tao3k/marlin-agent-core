// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Linked-AOT Marlin Scheme Authority Host daemon (`marlin-scheme-host-aot`).
//!
//! Statically links the Yuanxu-selected `:marlin-agent-core/graph-engine`
//! Gambit/Gerbil C-ABI archive into one native binary so startup needs ZERO
//! runtime `.o1` `dlopen` and ZERO external `scheme/.gerbil` package tree.

// Pull in `libmarlin_agent_core` so Cargo attaches `build.rs`'s statically linked
// `libmarlin_graph_engine.a` archive and Gambit runtime libraries to this binary.
extern crate marlin_agent_core as _;

use gerbil_scheme_sys::{GerbilGlobalState, GerbilModuleOrLink};
use marlin_gerbil_scheme::{
    GerbilRuntime, LinkedGerbilProgram,
    scheme_host_runtime::{LinkedSchemeHostExports, run_linked_scheme_host},
};

unsafe extern "C" {
    fn ___LNK_marlin__graph__engine(gs: *mut GerbilGlobalState) -> *mut GerbilModuleOrLink;
    fn marlin_graph_engine_dispatch_vector(root: i64) -> i64;
    fn marlin_graph_engine_reconcile_receipt_vector(root: i64) -> i64;
    fn marlin_graph_engine_workbench_tabs_vector(root: i64) -> i64;
    fn marlin_graph_engine_governed_context_vector(root: i64) -> i64;
    fn marlin_graph_engine_make_demo_fleet_vector(root: i64) -> i64;
    fn marlin_graph_engine_fleet_restore_vector(root: i64) -> i64;
    fn marlin_graph_engine_fleet_controller_open_vector(root: i64) -> i64;
    fn marlin_graph_engine_fleet_controller_dispatch_vector(root: i64) -> i64;
    fn marlin_graph_engine_fleet_controller_audit_capsule_vector(root: i64) -> i64;
    fn marlin_governance_project_vector(root: i64) -> i64;
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // SAFETY: build.rs statically links `libmarlin_graph_engine.a` built from the matching
    // Gerbil SDK + Yuanxu `graph-engine` program manifest.
    let program = unsafe { LinkedGerbilProgram::from_linker(___LNK_marlin__graph__engine) };
    let runtime = GerbilRuntime::initialize_program_single_processor(program)?;
    let exports = LinkedSchemeHostExports {
        dispatch_vector: marlin_graph_engine_dispatch_vector,
        reconcile_receipt_vector: marlin_graph_engine_reconcile_receipt_vector,
        workbench_tabs_vector: marlin_graph_engine_workbench_tabs_vector,
        governed_context_vector: marlin_graph_engine_governed_context_vector,
        make_demo_fleet_vector: marlin_graph_engine_make_demo_fleet_vector,
        fleet_restore_vector: marlin_graph_engine_fleet_restore_vector,
        fleet_open_vector: marlin_graph_engine_fleet_controller_open_vector,
        fleet_dispatch_vector: marlin_graph_engine_fleet_controller_dispatch_vector,
        fleet_audit_capsule_vector: marlin_graph_engine_fleet_controller_audit_capsule_vector,
        governance_project_vector: marlin_governance_project_vector,
    };
    // SAFETY: all 10 C-ABI exports belong to `runtime`'s statically linked program.
    unsafe { run_linked_scheme_host(runtime, exports) }
}
