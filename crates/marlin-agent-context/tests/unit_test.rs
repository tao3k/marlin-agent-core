// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later
#![cfg(feature = "org-memory")]

#[path = "unit/memory/mod.rs"]
mod memory;

#[test]
fn memory_workspace_shares_explicit_startup() {
    // SAFETY: this is the process's only test entry; no application workers
    // or child processes exist until the explicit startup window closes.
    unsafe { marlin_agent_context::org::workspace::initialize_org_runtime() }
        .expect("exclusive Org memory startup");
    let cases: Vec<_> = memory::cases().collect();
    assert_eq!(cases.len(), 33, "retain every memory workspace case");
    std::thread::scope(|scope| {
        for (name, case) in cases {
            scope.spawn(move || {
                case();
                println!("CASE-OK {name}");
            });
        }
    });
}
