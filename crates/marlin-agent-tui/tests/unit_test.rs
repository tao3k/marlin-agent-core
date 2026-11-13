// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

#[path = "unit/harness.rs"]
mod harness;

// The source-level harness also needs this package's linked AOT archive.
// Referencing the library carries its build-script native link directives.
#[cfg(feature = "linked-ui")]
use marlin_agent_tui as _;
