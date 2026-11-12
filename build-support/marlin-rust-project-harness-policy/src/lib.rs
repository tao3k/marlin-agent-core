// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Shared, test-only ASP Rust policy for the Marlin Cargo workspace.

#[doc(hidden)]
pub use asp_rust;

mod policy;

pub use policy::workspace_policy;
