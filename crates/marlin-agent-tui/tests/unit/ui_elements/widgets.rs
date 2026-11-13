// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use crate::ui_elements::{Extent, UiRenderInputs, render_prompt, render_template};
use marlin_agent_ui::{
    UiComponentId, UiElement, UiSpan, UiTemplateSpan, UiTemplateValue, UiTheme, UiTone,
};
use ratatui::{Terminal, backend::TestBackend, layout::Rect, style::Color};

#[test]
fn generic_bar_respects_priority_alignment_and_unicode_tail() {
    use marlin_agent_ui::{UiBackground, UiBarSegment, UiBarSide, UiSize};
    let id = |name| UiComponentId::new(name).unwrap();
    let segment = |side, priority, value| UiBarSegment {
        side,
        priority,
        min_width: 0,
        condition: None,
        child: Box::new(UiElement::Template {
            background: UiBackground::None,
            spans: vec![UiTemplateSpan {
                value,
                tone: UiTone::Normal,
                bold: false,
            }],
        }),
    };
    let tree = UiElement::Bar {
        background: UiBackground::Bar,
        right_divisor: 2,
        segments: vec![
            segment(UiBarSide::Left, 0, UiTemplateValue::Literal("L".into())),
            segment(
                UiBarSide::Left,
                1,
                UiTemplateValue::Literal("abcdef".into()),
            ),
            segment(
                UiBarSide::Right,
                3,
                UiTemplateValue::Tail {
                    id: id("value"),
                    width: UiSize::Cells(5),
                },
            ),
        ],
    };
    let theme = UiTheme::default();
    let inputs = UiRenderInputs {
        theme: &theme,
        offset: &|_| 0,
        facts: &|_| false,
        extent: &|_| Extent::Cells(0),
        bindings: &[("value", "日本/critical")],
    };
    let mut terminal = Terminal::new(TestBackend::new(10, 1)).unwrap();
    terminal
        .draw(|frame| {
            crate::ui_elements::render_with_facts(
                frame,
                frame.area(),
                &tree,
                &mut Vec::new(),
                &inputs,
            )
        })
        .unwrap();
    let row: String = (0..10)
        .map(|x| terminal.backend().buffer()[(x, 0)].symbol())
        .collect();
    assert_eq!(row, "L    …ical");
}

#[test]
fn native_content_renderer_keeps_text_beyond_scheme_projection_limit() {
    let theme = UiTheme::default();
    let text = format!("{}END", "x".repeat(256));
    let template = UiElement::Template {
        background: marlin_agent_ui::UiBackground::None,
        spans: vec![],
    };
    let inputs = UiRenderInputs {
        theme: &theme,
        offset: &|_| 0,
        facts: &|_| false,
        extent: &|_| Extent::Cells(0),
        bindings: &[],
    };
    let mut terminal = Terminal::new(TestBackend::new(260, 1)).unwrap();
    terminal
        .draw(|frame| {
            crate::ui_elements::render_content(
                frame,
                frame.area(),
                &[marlin_agent_ui::UiContent::Text(text)],
                &template,
                &inputs,
                0,
            );
        })
        .unwrap();
    assert_eq!(terminal.backend().buffer()[(256, 0)].symbol(), "E");
    assert_eq!(terminal.backend().buffer()[(258, 0)].symbol(), "D");
}

#[test]
fn prompt_clips_at_graphemes_and_positions_cursor() {
    let tree = UiElement::Template {
        background: marlin_agent_ui::UiBackground::None,
        spans: vec![
            UiTemplateSpan {
                value: UiTemplateValue::Literal("> ".into()),
                tone: UiTone::Normal,
                bold: false,
            },
            UiTemplateSpan {
                value: UiTemplateValue::Binding(UiComponentId::new("query").unwrap()),
                tone: UiTone::Normal,
                bold: false,
            },
        ],
    };
    let theme = UiTheme::default();
    let inputs = UiRenderInputs {
        theme: &theme,
        offset: &|_| 0,
        facts: &|_| false,
        extent: &|_| Extent::Cells(0),
        bindings: &[],
    };
    let mut terminal = Terminal::new(TestBackend::new(6, 1)).unwrap();
    let mut cursor = None;
    terminal
        .draw(|frame| cursor = render_prompt(frame, frame.area(), &tree, &inputs, "query", "a🐟bc"))
        .unwrap();
    assert_eq!(cursor, Some((4, 0)));
    assert_eq!(terminal.backend().buffer()[(2, 0)].symbol(), "b");
    assert_eq!(terminal.backend().buffer()[(3, 0)].symbol(), "c");
}

#[test]
fn list_renders_selected_scheme_template_and_keeps_selection_visible() {
    let text = |label: &str| {
        UiElement::Text(vec![UiSpan {
            text: label.into(),
            tone: UiTone::Normal,
        }])
    };
    let template = UiElement::List {
        id: UiComponentId::new("test-list").unwrap(),
        offset: marlin_agent_ui::UiSize::Cells(2),
        child: Box::new(UiElement::Choice {
            cases: vec![(UiComponentId::new("selected").unwrap(), text("selected"))],
            fallback: Box::new(text("ordinary")),
        }),
    };
    let mut terminal = Terminal::new(TestBackend::new(12, 2)).unwrap();
    let generated = std::cell::Cell::new(0);
    let mut state = crate::ui_elements::UiListState::default();
    terminal
        .draw(|frame| {
            crate::ui_elements::render_list(
                frame,
                frame.area(),
                &template,
                &UiTheme::default(),
                crate::ui_elements::UiListData {
                    count: 4,
                    selected: Some(3),
                    extent: &|_| Extent::Cells(0),
                },
                &mut state,
                |_| {
                    generated.set(generated.get() + 1);
                    Some(crate::ui_elements::UiListRow { bindings: vec![] })
                },
            )
        })
        .unwrap();
    assert_eq!(terminal.backend().buffer()[(0, 0)].symbol(), "o");
    assert_eq!(terminal.backend().buffer()[(0, 1)].symbol(), "s");
    assert_eq!(generated.get(), 2, "only visible rows should be prepared");
}

#[test]
fn declared_list_offsets_are_independent_between_identities() {
    use marlin_agent_ui::UiSize;
    let list = |name| UiElement::List {
        id: UiComponentId::new(name).unwrap(),
        offset: UiSize::Add(
            Box::new(UiSize::Binding(UiComponentId::new("list-offset").unwrap())),
            Box::new(UiSize::Cells(1)),
        ),
        child: Box::new(UiElement::Text(vec![])),
    };
    let left = list("left");
    let right = list("right");
    let mut state = crate::ui_elements::UiListState::default();
    let mut terminal = Terminal::new(TestBackend::new(8, 2)).unwrap();
    for (tree, expected) in [(&left, 1), (&left, 2), (&right, 1), (&left, 3)] {
        let mut visited = Vec::new();
        terminal
            .draw(|frame| {
                crate::ui_elements::render_list(
                    frame,
                    frame.area(),
                    tree,
                    &UiTheme::default(),
                    crate::ui_elements::UiListData {
                        count: 10,
                        selected: None,
                        extent: &|_| Extent::Cells(0),
                    },
                    &mut state,
                    |index| {
                        visited.push(index);
                        Some(crate::ui_elements::UiListRow { bindings: vec![] })
                    },
                );
            })
            .unwrap();
        assert_eq!(visited, vec![expected, expected + 1]);
    }
    // A temporarily hidden list must not advance its stored position.
    terminal
        .draw(|frame| {
            crate::ui_elements::render_list(
                frame,
                Rect::default(),
                &left,
                &UiTheme::default(),
                crate::ui_elements::UiListData {
                    count: 10,
                    selected: None,
                    extent: &|_| Extent::Cells(0),
                },
                &mut state,
                |_| Some(crate::ui_elements::UiListRow { bindings: vec![] }),
            );
        })
        .unwrap();
    let mut visited = Vec::new();
    terminal
        .draw(|frame| {
            crate::ui_elements::render_list(
                frame,
                frame.area(),
                &left,
                &UiTheme::default(),
                crate::ui_elements::UiListData {
                    count: 10,
                    selected: None,
                    extent: &|_| Extent::Cells(0),
                },
                &mut state,
                |index| {
                    visited.push(index);
                    Some(crate::ui_elements::UiListRow { bindings: vec![] })
                },
            );
        })
        .unwrap();
    assert_eq!(visited, vec![4, 5]);
}

#[test]
fn template_binds_untrusted_values_as_literal_cells() {
    let tree = UiElement::Template {
        background: marlin_agent_ui::UiBackground::Bar,
        spans: vec![
            UiTemplateSpan {
                value: UiTemplateValue::Literal("› ".into()),
                tone: UiTone::Accent,
                bold: false,
            },
            UiTemplateSpan {
                value: UiTemplateValue::Binding(UiComponentId::new("name").unwrap()),
                tone: UiTone::Info,
                bold: true,
            },
        ],
    };
    let theme = UiTheme::default();
    let inputs = UiRenderInputs {
        theme: &theme,
        offset: &|_| 0,
        facts: &|_| false,
        extent: &|_| Extent::Cells(0),
        bindings: &[("name", "x\u{1b}y")],
    };
    let mut terminal = Terminal::new(TestBackend::new(12, 1)).unwrap();
    terminal
        .draw(|frame| render_template(frame, frame.area(), &tree, &inputs))
        .unwrap();
    let buffer = terminal.backend().buffer();
    assert_eq!(buffer[(0, 0)].symbol(), "›");
    assert_eq!(buffer[(2, 0)].symbol(), "x");
    assert_eq!(buffer[(3, 0)].symbol(), "�");
    assert_eq!(buffer[(4, 0)].symbol(), "y");
    assert_eq!(
        buffer[(0, 0)].bg,
        Color::Rgb(theme.bar.red, theme.bar.green, theme.bar.blue)
    );
}

#[test]
fn panel_maps_declared_tones_and_sanitizes_host_content() {
    let panel = UiElement::Panel {
        border_tone: UiTone::Accent,
        field_template: Box::new(UiElement::Template {
            background: marlin_agent_ui::UiBackground::Bar,
            spans: vec![
                UiTemplateSpan {
                    value: UiTemplateValue::Binding(UiComponentId::new("label").unwrap()),
                    tone: UiTone::Success,
                    bold: false,
                },
                UiTemplateSpan {
                    value: UiTemplateValue::Literal(" = ".into()),
                    tone: UiTone::Success,
                    bold: false,
                },
                UiTemplateSpan {
                    value: UiTemplateValue::Binding(UiComponentId::new("value").unwrap()),
                    tone: UiTone::Normal,
                    bold: false,
                },
            ],
        }),
        title: Some(Box::new(UiElement::Template {
            background: marlin_agent_ui::UiBackground::None,
            spans: vec![marlin_agent_ui::UiTemplateSpan {
                value: marlin_agent_ui::UiTemplateValue::Binding(
                    UiComponentId::new("title").unwrap(),
                ),
                tone: UiTone::Accent,
                bold: false,
            }],
        })),
        child: Box::new(UiElement::Slot(UiComponentId::new("static").unwrap())),
    };
    let items = [marlin_agent_ui::UiContent::Field {
        label: "K".into(),
        value: "V\u{1b}X".into(),
    }];
    let theme = UiTheme::default();
    let inputs = UiRenderInputs {
        theme: &theme,
        offset: &|_| 0,
        facts: &|_| false,
        extent: &|_| Extent::Cells(0),
        bindings: &[("title", "P\u{1b}nel")],
    };
    let mut slot = crate::ui_elements::UiContentSlot {
        content: &items,
        scroll: 0,
    };
    let mut terminal = Terminal::new(TestBackend::new(24, 4)).unwrap();
    terminal
        .draw(|frame| {
            crate::ui_elements::render_with_slots(
                frame,
                frame.area(),
                &panel,
                &mut Vec::new(),
                &inputs,
                &mut slot,
            )
        })
        .unwrap();
    let buffer = terminal.backend().buffer();
    assert_eq!(buffer[(1, 0)].symbol(), "P");
    assert_eq!(buffer[(2, 0)].symbol(), "�");
    assert_eq!(
        buffer[(0, 0)].fg,
        Color::Rgb(theme.accent.red, theme.accent.green, theme.accent.blue)
    );
    assert_eq!(
        buffer[(1, 1)].fg,
        Color::Rgb(theme.success.red, theme.success.green, theme.success.blue)
    );
    assert_eq!(
        buffer[(1, 1)].bg,
        Color::Rgb(theme.bar.red, theme.bar.green, theme.bar.blue)
    );
    assert_eq!(buffer[(3, 1)].symbol(), "=");
    assert_eq!(buffer[(6, 1)].symbol(), "�");
}

#[test]
fn template_column_pads_by_terminal_cell_width() {
    use marlin_agent_ui::{UiSize, UiTemplateSpan, UiTemplateValue};
    let name = UiComponentId::new("name").unwrap();
    let tree = UiElement::Template {
        background: marlin_agent_ui::UiBackground::None,
        spans: vec![
            UiTemplateSpan {
                value: UiTemplateValue::Column {
                    id: name,
                    width: UiSize::Cells(5),
                },
                tone: UiTone::Normal,
                bold: false,
            },
            UiTemplateSpan {
                value: UiTemplateValue::Literal("|".into()),
                tone: UiTone::Normal,
                bold: false,
            },
        ],
    };
    let theme = UiTheme::default();
    let inputs = UiRenderInputs {
        theme: &theme,
        offset: &|_| 0,
        facts: &|_| false,
        extent: &|_| Extent::Cells(0),
        bindings: &[("name", "鱼")],
    };
    let mut terminal = Terminal::new(TestBackend::new(8, 1)).unwrap();
    terminal
        .draw(|frame| render_template(frame, frame.area(), &tree, &inputs))
        .unwrap();
    assert_eq!(terminal.backend().buffer()[(5, 0)].symbol(), "|");
}

#[test]
fn input_component_uses_selected_presentation_and_literal_bindings() {
    let input = UiElement::Choice {
        cases: vec![(
            UiComponentId::new("command").unwrap(),
            UiElement::Input {
                bordered: false,
                background: marlin_agent_ui::UiBackground::Bar,
                prefix: Some(Box::new(UiElement::Choice {
                    cases: vec![(
                        UiComponentId::new("width-at-least-20").unwrap(),
                        UiElement::Template {
                            background: marlin_agent_ui::UiBackground::None,
                            spans: vec![marlin_agent_ui::UiTemplateSpan {
                                value: marlin_agent_ui::UiTemplateValue::Binding(
                                    UiComponentId::new("prompt").unwrap(),
                                ),
                                tone: UiTone::Accent,
                                bold: false,
                            }],
                        },
                    )],
                    fallback: Box::new(UiElement::Template {
                        background: marlin_agent_ui::UiBackground::None,
                        spans: vec![marlin_agent_ui::UiTemplateSpan {
                            value: marlin_agent_ui::UiTemplateValue::Literal(">".into()),
                            tone: UiTone::Accent,
                            bold: false,
                        }],
                    }),
                })),
                title: None,
                active: UiComponentId::new("focused").unwrap(),
            },
        )],
        fallback: Box::new(UiElement::Input {
            bordered: true,
            background: marlin_agent_ui::UiBackground::None,
            prefix: None,
            title: Some(UiComponentId::new("title").unwrap()),
            active: UiComponentId::new("focused").unwrap(),
        }),
    };
    let theme = UiTheme::default();
    let facts = |id: &UiComponentId| id.as_str() == "command";
    let inputs = UiRenderInputs {
        theme: &theme,
        offset: &|_| 0,
        facts: &facts,
        extent: &|_| Extent::Cells(0),
        bindings: &[("prompt", "M-x "), ("title", "Message")],
    };
    let mut terminal = Terminal::new(TestBackend::new(20, 3)).unwrap();
    let mut measured = None;
    terminal
        .draw(|frame| {
            crate::ui_elements::render_input(
                frame,
                frame.area(),
                &input,
                &inputs,
                true,
                |width, height| {
                    measured = Some((width, height));
                    Some((vec!["help".into()], (4, 0)))
                },
            )
        })
        .unwrap();
    assert_eq!(measured, Some((16, 3)));
    assert_eq!(terminal.backend().buffer()[(0, 0)].symbol(), "M");
    assert_eq!(terminal.backend().buffer()[(4, 0)].symbol(), "h");
    let mut narrow = Terminal::new(TestBackend::new(10, 1)).unwrap();
    narrow
        .draw(|frame| {
            crate::ui_elements::render_input(
                frame,
                frame.area(),
                &input,
                &inputs,
                true,
                |width, _| {
                    assert_eq!(width, 9);
                    Some((vec!["help".into()], (4, 0)))
                },
            );
        })
        .unwrap();
    assert_eq!(narrow.backend().buffer()[(0, 0)].symbol(), ">");
    assert_eq!(
        terminal.backend().buffer()[(4, 0)].bg,
        Color::Rgb(theme.bar.red, theme.bar.green, theme.bar.blue)
    );
}

#[test]
fn input_column_width_can_use_available_width_fact() {
    use marlin_agent_ui::{UiSize, UiTemplateSpan, UiTemplateValue};
    let width = UiSize::Choice {
        cases: vec![(
            UiComponentId::new("width-at-least-12").unwrap(),
            UiSize::Cells(5),
        )],
        fallback: Box::new(UiSize::Cells(2)),
    };
    let input = UiElement::Input {
        bordered: false,
        background: marlin_agent_ui::UiBackground::None,
        prefix: Some(Box::new(UiElement::Template {
            background: marlin_agent_ui::UiBackground::None,
            spans: vec![UiTemplateSpan {
                value: UiTemplateValue::Column {
                    id: UiComponentId::new("name").unwrap(),
                    width,
                },
                tone: UiTone::Normal,
                bold: false,
            }],
        })),
        title: None,
        active: UiComponentId::new("focused").unwrap(),
    };
    let theme = UiTheme::default();
    let inputs = UiRenderInputs {
        theme: &theme,
        offset: &|_| 0,
        facts: &|_| false,
        extent: &|_| Extent::Cells(0),
        bindings: &[("name", "x")],
    };
    for (width, expected) in [(10, 8), (20, 15)] {
        let mut terminal = Terminal::new(TestBackend::new(width, 1)).unwrap();
        terminal
            .draw(|frame| {
                crate::ui_elements::render_input(
                    frame,
                    frame.area(),
                    &input,
                    &inputs,
                    false,
                    |available, _| {
                        assert_eq!(available, expected);
                        None
                    },
                )
            })
            .unwrap();
    }
}
