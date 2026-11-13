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
    use marlin_agent_ui::UiSize;
    let id = |name| UiComponentId::new(name).unwrap();
    let bind = |name| UiSize::Binding(id(name));
    let binary = |f: fn(Box<UiSize>, Box<UiSize>) -> UiSize, a, b| f(Box::new(a), Box::new(b));
    let choice = |cases| UiSize::Choice {
        cases,
        fallback: Box::new(UiSize::Cells(0)),
    };
    let bounded = |value, limit| UiSize::Bounded {
        value: id(value),
        limit: id(limit),
    };
    let transcript = binary(
        UiSize::Min,
        bind("transcript-min"),
        binary(UiSize::Subtract, bind("terminal-height"), UiSize::Cells(2)),
    );
    let footer = binary(UiSize::Max, bind("footer-min"), bind("hint-rows"));
    let editor_reserved = binary(
        UiSize::Add,
        binary(UiSize::Add, footer.clone(), UiSize::Cells(1)),
        transcript.clone(),
    );
    let editor = choice(vec![
        (
            id("editor-fixed"),
            bounded("fixed-editor", "terminal-height"),
        ),
        (
            id("editor-expanded"),
            binary(
                UiSize::Min,
                bind("measured-editor"),
                binary(
                    UiSize::Min,
                    bind("chatbox-limit"),
                    binary(UiSize::Subtract, bind("terminal-height"), editor_reserved),
                ),
            ),
        ),
    ]);
    let footer_size = binary(
        UiSize::Max,
        binary(
            UiSize::Min,
            footer,
            binary(
                UiSize::Subtract,
                bind("terminal-height"),
                binary(
                    UiSize::Add,
                    binary(UiSize::Add, transcript.clone(), bind("region/editor")),
                    UiSize::Cells(1),
                ),
            ),
        ),
        binary(UiSize::Min, bind("footer-min"), bind("terminal-height")),
    );
    let spare = binary(
        UiSize::Subtract,
        bind("terminal-height"),
        binary(
            UiSize::Add,
            binary(UiSize::Add, bind("region/minibuffer"), transcript),
            bind("region/editor"),
        ),
    );
    let modeline = choice(vec![(
        id("modeline-present"),
        binary(UiSize::Min, UiSize::Cells(1), spare.clone()),
    )]);
    let candidate_budget = binary(UiSize::Subtract, spare, bind("region/modeline"));
    let candidates = choice(vec![
        (
            id("review"),
            binary(UiSize::Min, bind("review-count"), candidate_budget.clone()),
        ),
        (
            id("command"),
            binary(UiSize::Min, bind("command-limit"), candidate_budget.clone()),
        ),
    ]);
    let extensions = choice(vec![(
        id("extensions-enabled"),
        binary(
            UiSize::Min,
            bind("extension-count"),
            binary(
                UiSize::Subtract,
                candidate_budget,
                bind("region/candidates"),
            ),
        ),
    )]);
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
                size: match name {
                    "transcript" => marlin_agent_ui::UiSize::Flex(1),
                    "editor" => editor.clone(),
                    "minibuffer" => footer_size.clone(),
                    "modeline" => modeline.clone(),
                    "candidates" => candidates.clone(),
                    "extensions" => extensions.clone(),
                    _ => marlin_agent_ui::UiSize::Binding(UiComponentId::new(name).unwrap()),
                },
                child: Box::new(marlin_agent_ui::UiElement::Region {
                    id: UiComponentId::new(name).unwrap(),
                    scroll_target: matches!(name, "transcript" | "candidates")
                        .then(|| marlin_agent_ui::UiScrollTarget::new(name).unwrap()),
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
                    } else if name == "extensions" {
                        UiElement::Template {
                            background: marlin_agent_ui::UiBackground::None,
                            spans: vec![
                                UiTemplateSpan {
                                    value: UiTemplateValue::Literal("fixture: ".into()),
                                    tone: UiTone::Normal,
                                    bold: false,
                                },
                                UiTemplateSpan {
                                    value: UiTemplateValue::Binding(
                                        UiComponentId::new("label").unwrap(),
                                    ),
                                    tone: UiTone::Normal,
                                    bold: false,
                                },
                            ],
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
                scroll_target: None,
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
