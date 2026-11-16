// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use marlin_context_org_model::TodoState;

pub(super) fn todo_state(todo: &TodoState) -> String {
    match todo {
        TodoState::Todo => "TODO".to_string(),
        TodoState::Next => "NEXT".to_string(),
        TodoState::Wait => "WAIT".to_string(),
        TodoState::Blocked => "BLOCKED".to_string(),
        TodoState::Done => "DONE".to_string(),
        TodoState::Custom(value) => value.clone(),
    }
}
