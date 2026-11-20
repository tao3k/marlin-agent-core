// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

asp_rust::asp_rust_workspace_dev_gate!(
    mode = deny,
    policy = asp_rust::AspRustWorkspacePolicy::new("marlin", asp_rust::default_asp_rust_config())
);
