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
            prompt: " Message ".into(),
        },
        command_frame: UiFrameSpec {
            min_transcript_rows: 0,
            fixed_editor_rows: 1,
            footer_rows: 0,
            extensions: false,
            prompt: " M-x ".into(),
        },
        max_chatbox_rows: 10,
        max_command_rows: 32,
        frame_tree: {
            let mut slots: Vec<_> = [
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
                    child: Box::new(if name == "transcript" {
                        UiElement::Choice {
                            cases: [
                                "review",
                                "messages",
                                "static",
                                "runtime",
                                "extension",
                                "workspace",
                            ]
                            .into_iter()
                            .map(|name| {
                                let slot = UiElement::Slot(UiComponentId::new(name).unwrap());
                                (
                                    UiComponentId::new(name).unwrap(),
                                    if matches!(name, "static" | "runtime" | "extension") {
                                        UiElement::Panel {
                                            border_tone: UiTone::Info,
                                            field_tone: UiTone::Info,
                                            child: Box::new(slot),
                                        }
                                    } else {
                                        slot
                                    },
                                )
                            })
                            .collect(),
                            fallback: Box::new(UiElement::Slot(
                                UiComponentId::new("conversation").unwrap(),
                            )),
                        }
                    } else if name == "candidates" {
                        UiElement::Choice {
                            cases: vec![(UiComponentId::new("selected").unwrap(), candidate(true))],
                            fallback: Box::new(candidate(false)),
                        }
                    } else if name == "editor" {
                        let input =
                            |bordered, background, prefix: Option<&str>, title: Option<&str>| {
                                UiElement::Input {
                                    bordered,
                                    background,
                                    prefix: prefix.map(|name| UiComponentId::new(name).unwrap()),
                                    title: title.map(|name| UiComponentId::new(name).unwrap()),
                                    active: UiComponentId::new("focused").unwrap(),
                                }
                            };
                        UiElement::Choice {
                            cases: vec![(
                                UiComponentId::new("command").unwrap(),
                                input(
                                    false,
                                    marlin_agent_ui::UiBackground::Bar,
                                    Some("prompt"),
                                    None,
                                ),
                            )],
                            fallback: Box::new(input(
                                true,
                                marlin_agent_ui::UiBackground::None,
                                None,
                                Some("title"),
                            )),
                        }
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
            .collect();
            let minibuffer = slots.split_off(3);
            slots.push(UiElement::Region {
                id: UiComponentId::new("minibuffer-hover").unwrap(),
                scrollable: false,
                child: Box::new(UiElement::Rows(minibuffer)),
            });
            UiElement::Rows(slots)
        },
        // These are synthetic host values, not copies of Scheme presentation.
        transcript_policy: UiTranscriptPolicy {
            default_collapsed: true,
            user_prefix: "u: ".into(),
            assistant_role: String::new(),
            thinking_collapsed_role: "closed".into(),
            thinking_expanded_role: "open".into(),
            role_tone: marlin_agent_ui::UiTone::Info,
            role_bold: true,
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
