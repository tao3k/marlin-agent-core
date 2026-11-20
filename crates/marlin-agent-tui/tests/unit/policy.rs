// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

// ASP Rust's default severity contract blocks Warning and Error. The TUI
// still carries Info-only owner-boundary advice; do not turn non-blocking
// advice into a test failure while the owner refactor proceeds.
asp_rust::asp_rust_cargo_test_gate!(advice = allow);
