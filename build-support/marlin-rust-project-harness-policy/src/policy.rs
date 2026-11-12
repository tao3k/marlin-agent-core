// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Project-owned policy selection and native ASP Rust gate mounts.

/// The workspace graph and its member ordering are owned by ASP Rust.
#[must_use]
pub fn workspace_policy() -> asp_rust::AspRustWorkspacePolicy {
    asp_rust::AspRustWorkspacePolicy::new("marlin", asp_rust::default_asp_rust_config())
}

/// Mount the package's ASP Rust Dev Gate through its development dependency.
#[macro_export]
macro_rules! marlin_rust_dev_gate {
    () => {
        $crate::asp_rust::asp_rust_cargo_test_gate!(mode = deny);
    };
}

/// Mount one native ASP Rust gate for all Cargo workspace members.
#[macro_export]
macro_rules! marlin_workspace_dev_gate {
    () => {
        $crate::asp_rust::asp_rust_workspace_dev_gate!(
            mode = deny,
            policy = $crate::workspace_policy()
        );
    };
}
