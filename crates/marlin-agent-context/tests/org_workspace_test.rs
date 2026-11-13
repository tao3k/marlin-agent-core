// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later
#![cfg(feature = "org-workspace")]

#[path = "unit/org_workspace/document/mod.rs"]
mod document;

#[test]
fn workspace_documents_share_explicit_startup() {
    // SAFETY: this is the process's only test entry. No document workers or
    // subprocesses exist before startup; all parser calls occur below it.
    unsafe { marlin_agent_context::org::workspace::initialize_org_runtime() }
        .expect("exclusive Org workspace startup");
    let cases: Vec<_> = document::cases().collect();
    assert_eq!(cases.len(), 14, "retain every workspace document case");
    std::thread::scope(|scope| {
        for (name, case) in cases {
            scope.spawn(move || {
                case();
                println!("CASE-OK {name}");
            });
        }
    });
}
