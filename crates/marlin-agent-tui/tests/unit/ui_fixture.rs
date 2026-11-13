// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use marlin_agent_ui::{
    UiBufferId, UiComponentId, UiFrame, UiFrameSpec, UiLayoutPolicy, UiTranscriptPolicy, UiWindow,
};

pub(super) fn test_layout() -> UiLayoutPolicy {
    UiLayoutPolicy {
        message_frame: UiFrameSpec {
            min_transcript_rows: 2,
            fixed_editor_rows: 0,
            footer_rows: 1,
            extensions: true,
            bordered: true,
            prompt: " Message ".into(),
        },
        command_frame: UiFrameSpec {
            min_transcript_rows: 0,
            fixed_editor_rows: 1,
            footer_rows: 0,
            extensions: false,
            bordered: false,
            prompt: " M-x ".into(),
        },
        max_chatbox_rows: 10,
        max_command_rows: 32,
        // These are synthetic host values, not copies of Scheme presentation.
        transcript_policy: UiTranscriptPolicy {
            default_collapsed: true,
            user_prefix: "u: ".into(),
            assistant_role: String::new(),
            thinking_collapsed_role: "closed".into(),
            thinking_expanded_role: "open".into(),
        },
    }
}

pub(super) fn test_frame() -> UiFrame {
    let id = |name| UiComponentId::new(name).unwrap();
    let buffer = |name| UiBufferId::new(name).unwrap();
    UiFrame {
        id: id("main"),
        windows: vec![UiWindow {
            id: id("main"),
            buffer: buffer("conversation"),
        }],
        selected_window: id("main"),
        minibuffer: id("minibuffer"),
        buffers: vec![buffer("conversation"), buffer("messages")],
    }
}
