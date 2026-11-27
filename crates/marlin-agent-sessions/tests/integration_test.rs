// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

#[path = "integration/conversation_journal_gate.rs"]
mod conversation_journal_gate;
#[path = "integration/session_storage_gate.rs"]
mod session_storage_gate;

asp_rust::asp_rust_cargo_test_gate!(mode = warn);
