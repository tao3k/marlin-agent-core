// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

#[path = "unit/kernel/mod.rs"]
mod kernel;

asp_rust::asp_rust_cargo_test_gate!(mode = warn);
