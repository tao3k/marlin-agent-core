// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use marlin_agent_ui::{
    UiBufferId, UiComponentId, UiFrame, UiFrameSpec, UiLayoutPolicy, UiTranscriptPolicy, UiWindow,
};

pub(super) fn test_layout() -> UiLayoutPolicy {
    use marlin_agent_ui::{UiElement, UiTemplateSpan, UiTemplateValue, UiTone};
    let candidate = |active| UiElement::Template {
        background: if active {
            marlin_agent_ui::UiBackground::Bar
        } else {
            marlin_agent_ui::UiBackground::None
        },
        spans: vec![
            UiTemplateSpan {
                value: UiTemplateValue::Literal(if active { " › " } else { "   " }.into()),
                tone: UiTone::Accent,
                bold: false,
            },
            UiTemplateSpan {
                value: UiTemplateValue::Binding(UiComponentId::new("name").unwrap()),
                tone: UiTone::Info,
                bold: active,
            },
            UiTemplateSpan {
                value: UiTemplateValue::Literal("  ".into()),
                tone: UiTone::Normal,
                bold: false,
            },
            UiTemplateSpan {
                value: UiTemplateValue::Binding(UiComponentId::new("description").unwrap()),
                tone: UiTone::Muted,
                bold: false,
            },
        ],
    };
    let status = |label: &str| UiElement::Template {
        background: marlin_agent_ui::UiBackground::None,
        spans: vec![
            UiTemplateSpan {
                value: UiTemplateValue::Binding(UiComponentId::new("prefix").unwrap()),
                tone: UiTone::Normal,
                bold: false,
            },
            UiTemplateSpan {
                value: UiTemplateValue::Literal(label.into()),
                tone: UiTone::Normal,
                bold: false,
            },
        ],
    };
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
        frame_tree: marlin_agent_ui::UiElement::Rows(
            [
                "transcript",
                "extensions",
                "modeline",
                "editor",
                "candidates",
                "minibuffer",
            ]
            .into_iter()
            .map(|name| marlin_agent_ui::UiElement::Sized {
                size: if name == "transcript" {
                    marlin_agent_ui::UiSize::Flex(1)
                } else {
                    marlin_agent_ui::UiSize::Binding(UiComponentId::new(name).unwrap())
                },
                child: Box::new(marlin_agent_ui::UiElement::Region {
                    id: UiComponentId::new(name).unwrap(),
                    scrollable: matches!(name, "transcript" | "candidates"),
                    child: Box::new(if name == "candidates" {
                        UiElement::Rows(vec![candidate(true), candidate(false)])
                    } else if name == "minibuffer" {
                        UiElement::Choice {
                            cases: ["searching", "search-match", "search-no-match"]
                                .into_iter()
                                .map(|name| {
                                    (
                                        UiComponentId::new(name).unwrap(),
                                        UiElement::Template {
                                            background: marlin_agent_ui::UiBackground::None,
                                            spans: vec![
                                                UiTemplateSpan {
                                                    value: UiTemplateValue::Literal(
                                                        "fixture search ".into(),
                                                    ),
                                                    tone: UiTone::Normal,
                                                    bold: false,
                                                },
                                                UiTemplateSpan {
                                                    value: UiTemplateValue::Binding(
                                                        UiComponentId::new("query").unwrap(),
                                                    ),
                                                    tone: UiTone::Normal,
                                                    bold: false,
                                                },
                                            ],
                                        },
                                    )
                                })
                                .chain(
                                    [
                                        "draft-limit",
                                        "runtime",
                                        "transcript-focus",
                                        "extension-focus",
                                    ]
                                    .into_iter()
                                    .map(|name| (UiComponentId::new(name).unwrap(), status(name))),
                                )
                                .collect(),
                            fallback: Box::new(status("fixture status")),
                        }
                    } else {
                        UiElement::Text(vec![])
                    }),
                }),
            })
            .collect(),
        ),
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
