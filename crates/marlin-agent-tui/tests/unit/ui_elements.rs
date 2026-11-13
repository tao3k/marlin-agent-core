// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::ui_elements::{
    Extent, UiRenderInputs, hit_test, hit_test_scroll, layout_regions_with_facts, render_element,
    render_prompt, render_template, render_with_facts, render_with_regions, render_with_scroll,
    resolve_choice,
};
use marlin_agent_ui::{
    UiComponentId, UiElement, UiSpan, UiTemplateSpan, UiTemplateValue, UiTheme, UiTone,
};
use proptest::{prop_assert, prop_assert_eq, proptest};
use ratatui::{Terminal, backend::TestBackend, layout::Rect, style::Color};

#[test]
fn native_content_renderer_keeps_text_beyond_scheme_projection_limit() {
    let theme = UiTheme::default();
    let text = format!("{}END", "x".repeat(256));
    let mut terminal = Terminal::new(TestBackend::new(260, 1)).unwrap();
    terminal
        .draw(|frame| {
            super::ui_elements::render_content(
                frame,
                frame.area(),
                &[marlin_agent_ui::UiContent::Text(text)],
                &theme,
                UiTone::Normal,
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
fn choice_intrinsic_rows_follow_the_selected_branch() {
    let text = |value: &str| {
        UiElement::Text(vec![UiSpan {
            text: value.into(),
            tone: UiTone::Normal,
        }])
    };
    let nested = UiElement::Rows(vec![UiElement::Choice {
        cases: vec![(UiComponentId::new("tall").unwrap(), text("one\ntwo"))],
        fallback: Box::new(text("one")),
    }]);
    assert_eq!(nested.intrinsic_rows_with(&|_| true), 2);
    assert_eq!(nested.intrinsic_rows_with(&|_| false), 1);
}

#[test]
fn bounded_size_uses_scheme_choice_and_host_cell_metrics() {
    use marlin_agent_ui::UiSize;
    let id = |name| UiComponentId::new(name).unwrap();
    let size = UiSize::Choice {
        cases: vec![(
            id("active"),
            UiSize::Bounded {
                value: id("requested"),
                limit: id("available"),
            },
        )],
        fallback: Box::new(UiSize::Cells(0)),
    };
    let metrics = |id: &UiComponentId| match id.as_str() {
        "requested" => Extent::Cells(20),
        "available" => Extent::Cells(7),
        _ => Extent::Cells(0),
    };
    assert_eq!(
        super::ui_elements::resolve_size(&size, &metrics, &|_| true),
        Extent::Cells(7)
    );
    assert_eq!(
        super::ui_elements::resolve_size(&size, &metrics, &|_| false),
        Extent::Cells(0)
    );
    assert_eq!(
        super::ui_elements::resolve_size(&size, &|_| Extent::Flex(1), &|_| true),
        Extent::Cells(0)
    );
}

#[test]
fn size_arithmetic_saturates_and_rejects_flexible_operands() {
    use marlin_agent_ui::UiSize;
    let binary = |f: fn(Box<UiSize>, Box<UiSize>) -> UiSize, a, b| f(Box::new(a), Box::new(b));
    let cases = [
        (
            binary(UiSize::Add, UiSize::Cells(u16::MAX), UiSize::Cells(1)),
            u16::MAX,
        ),
        (
            binary(UiSize::Subtract, UiSize::Cells(1), UiSize::Cells(2)),
            0,
        ),
        (binary(UiSize::Max, UiSize::Cells(3), UiSize::Cells(5)), 5),
        (binary(UiSize::Min, UiSize::Cells(3), UiSize::Cells(5)), 3),
        (binary(UiSize::Min, UiSize::Flex(1), UiSize::Cells(5)), 0),
    ];
    for (size, expected) in cases {
        assert_eq!(
            super::ui_elements::resolve_size(&size, &|_| Extent::Cells(0), &|_| false),
            Extent::Cells(expected)
        );
    }
}

#[test]
fn region_extents_follow_dependencies_and_bound_cycles() {
    use marlin_agent_ui::UiSize;
    let id = |name| UiComponentId::new(name).unwrap();
    let region = |name, size| UiElement::Sized {
        size,
        child: Box::new(UiElement::Region {
            id: id(name),
            scroll_target: None,
            child: Box::new(UiElement::Text(vec![])),
        }),
    };
    let tree = UiElement::Rows(vec![
        region("a", UiSize::Binding(id("region/b"))),
        region("b", UiSize::Binding(id("raw"))),
        region("cycle", UiSize::Binding(id("region/cycle"))),
    ]);
    let calls = std::cell::Cell::new(0);
    let raw = |_: &UiComponentId| {
        calls.set(calls.get() + 1);
        Extent::Cells(7)
    };
    let facts = |_: &UiComponentId| false;
    let resolver = super::ui_elements::RegionExtents::new(&tree, &raw, &facts);
    assert_eq!(resolver.extent(&id("region/a")), Extent::Cells(7));
    assert_eq!(resolver.extent(&id("region/a")), Extent::Cells(7));
    assert_eq!(calls.get(), 1);
    assert_eq!(resolver.extent(&id("region/cycle")), Extent::Cells(0));
    assert_eq!(resolver.extent(&id("region/missing")), Extent::Cells(0));
}

#[test]
fn choice_uses_same_facts_for_layout_and_paint() {
    let region = |id: &str| UiElement::Region {
        id: UiComponentId::new(id).unwrap(),
        scroll_target: Some(marlin_agent_ui::UiScrollTarget::new("transcript").unwrap()),
        child: Box::new(UiElement::Text(vec![UiSpan {
            text: id.into(),
            tone: UiTone::Normal,
        }])),
    };
    let tree = UiElement::Choice {
        cases: vec![(UiComponentId::new("active").unwrap(), region("selected"))],
        fallback: Box::new(region("fallback")),
    };
    let facts = |id: &UiComponentId| id.as_str() == "active";
    let area = Rect::new(0, 0, 12, 1);
    let mut layout = Vec::new();
    layout_regions_with_facts(&tree, area, &|_| Extent::Flex(1), &mut layout, &facts);
    let theme = UiTheme::default();
    let inputs = UiRenderInputs {
        theme: &theme,
        offset: &|_| 0,
        facts: &facts,
        extent: &|_| Extent::Flex(1),
        bindings: &[],
    };
    let mut painted = Vec::new();
    let mut terminal = Terminal::new(TestBackend::new(12, 1)).unwrap();
    terminal
        .draw(|frame| render_with_facts(frame, area, &tree, &mut painted, &inputs))
        .unwrap();
    assert_eq!(layout[0].id, painted[0].id);
    assert_eq!(layout[0].area, painted[0].area);
    assert_eq!(terminal.backend().buffer()[(0, 0)].symbol(), "s");
}

#[test]
fn nested_slots_follow_declared_layout_and_share_hit_regions() {
    struct Labels;
    impl super::ui_elements::UiSlotRenderer for Labels {
        fn render(
            &mut self,
            frame: &mut ratatui::Frame<'_>,
            area: Rect,
            id: &UiComponentId,
            _: Option<UiTone>,
            _: usize,
        ) {
            frame.render_widget(ratatui::widgets::Paragraph::new(id.as_str()), area);
        }
    }
    let region = |name: &str| UiElement::Region {
        id: UiComponentId::new(name).unwrap(),
        scroll_target: Some(marlin_agent_ui::UiScrollTarget::new("transcript").unwrap()),
        child: Box::new(UiElement::Slot(UiComponentId::new(name).unwrap())),
    };
    let tree = UiElement::Columns(vec![region("left"), region("right")]);
    let theme = UiTheme::default();
    let inputs = UiRenderInputs {
        theme: &theme,
        offset: &|_| 0,
        facts: &|_| false,
        extent: &|_| Extent::Flex(1),
        bindings: &[],
    };
    let mut regions = Vec::new();
    let mut terminal = Terminal::new(TestBackend::new(20, 1)).unwrap();
    terminal
        .draw(|frame| {
            super::ui_elements::render_with_slots(
                frame,
                frame.area(),
                &tree,
                &mut regions,
                &inputs,
                &mut Labels,
            )
        })
        .unwrap();
    assert_eq!(terminal.backend().buffer()[(0, 0)].symbol(), "l");
    assert_eq!(terminal.backend().buffer()[(10, 0)].symbol(), "r");
    assert_eq!(
        hit_test_scroll(&regions, 10, 0).unwrap().id.as_str(),
        "right"
    );
}

#[test]
fn scroll_target_preserves_same_and_distinct_region_bindings() {
    let target = |name| Some(marlin_agent_ui::UiScrollTarget::new(name).unwrap());
    let region = |id, scroll_target| UiElement::Region {
        id: UiComponentId::new(id).unwrap(),
        scroll_target,
        child: Box::new(UiElement::Text(vec![])),
    };
    let tree = UiElement::Columns(vec![
        region("transcript", target("transcript")),
        region("nested", target("candidates")),
    ]);
    let mut regions = Vec::new();
    super::ui_elements::layout_regions(
        &tree,
        Rect::new(0, 0, 20, 1),
        &|_| Extent::Flex(1),
        &mut regions,
    );
    assert_eq!(
        hit_test_scroll(&regions, 0, 0).unwrap().scroll_target(),
        Some("transcript")
    );
    assert_eq!(
        hit_test_scroll(&regions, 10, 0).unwrap().scroll_target(),
        Some("candidates")
    );
}

#[test]
fn list_renders_selected_scheme_template_and_keeps_selection_visible() {
    let text = |label: &str| {
        UiElement::Text(vec![UiSpan {
            text: label.into(),
            tone: UiTone::Normal,
        }])
    };
    let template = UiElement::Choice {
        cases: vec![(UiComponentId::new("selected").unwrap(), text("selected"))],
        fallback: Box::new(text("ordinary")),
    };
    let mut terminal = Terminal::new(TestBackend::new(12, 2)).unwrap();
    let generated = std::cell::Cell::new(0);
    terminal
        .draw(|frame| {
            super::ui_elements::render_list(
                frame,
                frame.area(),
                &template,
                &UiTheme::default(),
                4,
                Some(3),
                |_| {
                    generated.set(generated.get() + 1);
                    Some(super::ui_elements::UiListRow { bindings: vec![] })
                },
            )
        })
        .unwrap();
    assert_eq!(terminal.backend().buffer()[(0, 0)].symbol(), "o");
    assert_eq!(terminal.backend().buffer()[(0, 1)].symbol(), "s");
    assert_eq!(generated.get(), 2, "only visible rows should be prepared");
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
    let mut terminal = Terminal::new(TestBackend::new(12, 1)).unwrap();
    terminal
        .draw(|frame| render_template(frame, frame.area(), &tree, &theme, &[("name", "x\u{1b}y")]))
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
fn scrolling_moves_painted_rows_and_hit_regions_together() {
    let row = |id, text: &str| UiElement::Region {
        id: UiComponentId::new(id).unwrap(),
        scroll_target: None,
        child: Box::new(UiElement::Text(vec![UiSpan {
            text: text.into(),
            tone: UiTone::Normal,
        }])),
    };
    let tree = UiElement::Region {
        id: UiComponentId::new("messages").unwrap(),
        scroll_target: Some(marlin_agent_ui::UiScrollTarget::new("transcript").unwrap()),
        child: Box::new(UiElement::Rows(vec![
            row("first", "A"),
            row("second", "B"),
            row("third", "C"),
        ])),
    };
    let mut terminal = Terminal::new(TestBackend::new(8, 2)).unwrap();
    let mut regions = Vec::new();
    terminal
        .draw(|frame| {
            render_with_scroll(
                frame,
                frame.area(),
                &tree,
                &UiTheme::default(),
                &mut regions,
                &|id| usize::from(id.as_str() == "messages"),
            )
        })
        .unwrap();
    assert_eq!(terminal.backend().buffer()[(0, 0)].symbol(), "B");
    assert_eq!(terminal.backend().buffer()[(0, 1)].symbol(), "C");
    assert_eq!(hit_test(&regions, 0, 0).unwrap().id.as_str(), "second");
    assert_eq!(
        hit_test_scroll(&regions, 0, 0).unwrap().id.as_str(),
        "messages"
    );
    assert!(!regions.iter().any(|region| region.id.as_str() == "first"));
}

#[test]
fn maps_nested_elements_and_tones_to_terminal_cells() {
    let text = |value: &str, tone| {
        UiElement::Text(vec![UiSpan {
            text: value.into(),
            tone,
        }])
    };
    let tree = UiElement::Rows(vec![
        UiElement::Columns(vec![
            text("左", UiTone::Info),
            text("right", UiTone::Warning),
        ]),
        text("second", UiTone::Normal),
    ]);
    let theme = UiTheme::default();
    let mut terminal = Terminal::new(TestBackend::new(20, 2)).unwrap();
    terminal
        .draw(|frame| render_element(frame, frame.area(), &tree, &theme))
        .unwrap();
    let buffer = terminal.backend().buffer();
    assert_eq!(buffer[(0, 0)].symbol(), "左");
    assert_eq!(buffer[(10, 0)].symbol(), "r");
    assert_eq!(buffer[(0, 1)].symbol(), "s");
    assert_eq!(
        buffer[(0, 0)].fg,
        Color::Rgb(theme.info.red, theme.info.green, theme.info.blue)
    );
}

#[test]
fn hit_regions_follow_painted_layout_and_clip_at_terminal_edges() {
    let tree = UiElement::Region {
        id: UiComponentId::new("frame").unwrap(),
        scroll_target: None,
        child: Box::new(UiElement::Columns(vec![
            UiElement::Text(vec![]),
            UiElement::Region {
                id: UiComponentId::new("messages").unwrap(),
                scroll_target: Some(marlin_agent_ui::UiScrollTarget::new("transcript").unwrap()),
                child: Box::new(UiElement::Text(vec![UiSpan {
                    text: "abc".into(),
                    tone: UiTone::Normal,
                }])),
            },
        ])),
    };
    let mut terminal = Terminal::new(TestBackend::new(10, 2)).unwrap();
    let mut regions = Vec::new();
    terminal
        .draw(|frame| {
            render_with_regions(
                frame,
                Rect::new(2, 0, 12, 2),
                &tree,
                &UiTheme::default(),
                &mut regions,
            );
        })
        .unwrap();
    assert_eq!(hit_test(&regions, 6, 0).unwrap().id.as_str(), "messages");
    assert_eq!(
        hit_test_scroll(&regions, 6, 0).unwrap().id.as_str(),
        "messages"
    );
    assert!(hit_test_scroll(&regions, 2, 0).is_none());
    assert_eq!(terminal.backend().buffer()[(6, 0)].symbol(), "a");
    assert_eq!(hit_test(&regions, 2, 0).unwrap().id.as_str(), "frame");
    assert!(hit_test(&regions, 10, 0).is_none());
}

#[test]
fn panel_maps_declared_tones_and_sanitizes_host_content() {
    let panel = UiElement::Panel {
        border_tone: UiTone::Accent,
        field_tone: UiTone::Success,
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
        bindings: &[],
    };
    let mut slot = super::ui_elements::UiContentSlot {
        title: "Panel",
        content: &items,
        theme: &theme,
        scroll: 0,
    };
    let mut terminal = Terminal::new(TestBackend::new(24, 4)).unwrap();
    terminal
        .draw(|frame| {
            super::ui_elements::render_with_slots(
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
    assert_eq!(
        buffer[(0, 0)].fg,
        Color::Rgb(theme.accent.red, theme.accent.green, theme.accent.blue)
    );
    assert_eq!(
        buffer[(1, 1)].fg,
        Color::Rgb(theme.success.red, theme.success.green, theme.success.blue)
    );
    assert_eq!(buffer[(5, 1)].symbol(), "�");
}

#[test]
fn input_component_uses_selected_presentation_and_literal_bindings() {
    let input = UiElement::Choice {
        cases: vec![(
            UiComponentId::new("command").unwrap(),
            UiElement::Input {
                bordered: false,
                background: marlin_agent_ui::UiBackground::Bar,
                prefix: Some(UiComponentId::new("prompt").unwrap()),
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
            super::ui_elements::render_input(
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
    assert_eq!(
        terminal.backend().buffer()[(4, 0)].bg,
        Color::Rgb(theme.bar.red, theme.bar.green, theme.bar.blue)
    );
}

#[test]
fn input_measurement_counts_selected_nested_borders() {
    let input = |bordered| UiElement::Input {
        bordered,
        background: marlin_agent_ui::UiBackground::None,
        prefix: None,
        title: None,
        active: UiComponentId::new("focused").unwrap(),
    };
    let tree = UiElement::Panel {
        border_tone: UiTone::Normal,
        field_tone: UiTone::Normal,
        child: Box::new(UiElement::Rows(vec![UiElement::Choice {
            cases: vec![(UiComponentId::new("compact").unwrap(), input(false))],
            fallback: Box::new(input(true)),
        }])),
    };
    assert_eq!(super::ui_elements::input_border(&tree, &|_| false), Some(4));
    assert_eq!(super::ui_elements::input_border(&tree, &|_| true), Some(2));
}

#[test]
fn scheme_size_bindings_are_independent_of_region_names() {
    let region = |name: &str, size| UiElement::Sized {
        size,
        child: Box::new(UiElement::Region {
            id: UiComponentId::new(name).unwrap(),
            scroll_target: None,
            child: Box::new(UiElement::Text(vec![])),
        }),
    };
    let tree = UiElement::Rows(vec![
        region("body", marlin_agent_ui::UiSize::Flex(1)),
        region(
            "prompt",
            marlin_agent_ui::UiSize::Binding(UiComponentId::new("prompt-height").unwrap()),
        ),
        region("status", marlin_agent_ui::UiSize::Cells(1)),
    ]);
    let mut regions = Vec::new();
    super::ui_elements::layout_regions(
        &tree,
        Rect::new(0, 0, 20, 10),
        &|id| {
            assert_eq!(id.as_str(), "prompt-height");
            Extent::Cells(3)
        },
        &mut regions,
    );
    assert_eq!(
        regions
            .iter()
            .map(|region| region.area.height)
            .collect::<Vec<_>>(),
        [6, 3, 1]
    );
    assert_eq!(hit_test(&regions, 0, 6).unwrap().id.as_str(), "prompt");
}

#[test]
fn sized_layout_paint_and_hit_regions_share_rectangles() {
    let item = |id: &str, text: &str, size| UiElement::Sized {
        size,
        child: Box::new(UiElement::Region {
            id: UiComponentId::new(id).unwrap(),
            scroll_target: None,
            child: Box::new(UiElement::Text(vec![UiSpan {
                text: text.into(),
                tone: UiTone::Normal,
            }])),
        }),
    };
    let tree = UiElement::Rows(vec![
        item("body", "A", marlin_agent_ui::UiSize::Flex(1)),
        item(
            "prompt",
            "B",
            marlin_agent_ui::UiSize::Binding(UiComponentId::new("height").unwrap()),
        ),
    ]);
    let extent = |_: &UiComponentId| Extent::Cells(2);
    let theme = UiTheme::default();
    let inputs = UiRenderInputs {
        theme: &theme,
        offset: &|_| 0,
        facts: &|_| false,
        extent: &extent,
        bindings: &[],
    };
    let mut layout = Vec::new();
    let mut painted = Vec::new();
    let area = Rect::new(0, 0, 12, 6);
    layout_regions_with_facts(&tree, area, &extent, &mut layout, inputs.facts);
    let mut terminal = Terminal::new(TestBackend::new(12, 6)).unwrap();
    terminal
        .draw(|frame| render_with_facts(frame, area, &tree, &mut painted, &inputs))
        .unwrap();
    assert_eq!(
        layout.iter().map(|r| (&r.id, r.area)).collect::<Vec<_>>(),
        painted.iter().map(|r| (&r.id, r.area)).collect::<Vec<_>>()
    );
    assert_eq!(terminal.backend().buffer()[(0, 4)].symbol(), "B");
    assert_eq!(hit_test(&painted, 0, 4).unwrap().id.as_str(), "prompt");
}

#[test]
fn scrolling_sized_rows_keeps_paint_and_hit_testing_aligned() {
    let row = |id: &str, text: &str| UiElement::Sized {
        size: marlin_agent_ui::UiSize::Cells(2),
        child: Box::new(UiElement::Region {
            id: UiComponentId::new(id).unwrap(),
            scroll_target: None,
            child: Box::new(UiElement::Text(vec![UiSpan {
                text: text.into(),
                tone: UiTone::Normal,
            }])),
        }),
    };
    let tree = UiElement::Region {
        id: UiComponentId::new("viewport").unwrap(),
        scroll_target: Some(marlin_agent_ui::UiScrollTarget::new("transcript").unwrap()),
        child: Box::new(UiElement::Rows(vec![row("a", "A\nA"), row("b", "B\nB")])),
    };
    let mut terminal = Terminal::new(TestBackend::new(8, 3)).unwrap();
    let mut regions = Vec::new();
    terminal
        .draw(|frame| {
            super::ui_elements::render_with_scroll(
                frame,
                frame.area(),
                &tree,
                &UiTheme::default(),
                &mut regions,
                &|_| 1,
            )
        })
        .unwrap();
    assert_eq!(terminal.backend().buffer()[(0, 0)].symbol(), "A");
    assert_eq!(terminal.backend().buffer()[(0, 1)].symbol(), "B");
    assert_eq!(hit_test(&regions, 0, 1).unwrap().id.as_str(), "b");
}

#[test]
fn column_width_constraints_do_not_become_parent_row_heights() {
    let column = |id: &str| UiElement::Sized {
        size: marlin_agent_ui::UiSize::Cells(2),
        child: Box::new(UiElement::Region {
            id: UiComponentId::new(id).unwrap(),
            scroll_target: None,
            child: Box::new(UiElement::Text(vec![UiSpan {
                text: id.into(),
                tone: UiTone::Normal,
            }])),
        }),
    };
    let tree = UiElement::Rows(vec![
        UiElement::Columns(vec![column("a"), column("b")]),
        UiElement::Region {
            id: UiComponentId::new("next").unwrap(),
            scroll_target: None,
            child: Box::new(UiElement::Text(vec![])),
        },
    ]);
    let mut terminal = Terminal::new(TestBackend::new(8, 4)).unwrap();
    let mut painted = Vec::new();
    terminal
        .draw(|frame| {
            render_with_regions(
                frame,
                frame.area(),
                &tree,
                &UiTheme::default(),
                &mut painted,
            )
        })
        .unwrap();
    assert_eq!(
        painted
            .iter()
            .find(|region| region.id.as_str() == "next")
            .unwrap()
            .area
            .y,
        1
    );
    assert_eq!(
        painted
            .iter()
            .find(|region| region.id.as_str() == "b")
            .unwrap()
            .area
            .x,
        2
    );
}

proptest! {
    #[test]
    fn sized_layout_matches_painted_regions_across_terminal_sizes(
        width in 1u16..60, height in 1u16..35,
        sizes in proptest::collection::vec((0u16..12, proptest::bool::ANY), 1..16),
        vertical in proptest::bool::ANY,
    ) {
        let children = sizes.iter().enumerate().map(|(index, (size, flex))| UiElement::Sized {
            size: if *flex { marlin_agent_ui::UiSize::Flex(size.saturating_add(1)) } else { marlin_agent_ui::UiSize::Cells(*size) },
            child: Box::new(UiElement::Region {
                id: UiComponentId::new(&format!("sized-{index}")).unwrap(),
                scroll_target: None,
                child: Box::new(UiElement::Text(vec![])),
            }),
        }).collect();
        let tree = if vertical { UiElement::Rows(children) } else { UiElement::Columns(children) };
        let area = Rect::new(0, 0, width, height);
        let extent = |_: &UiComponentId| Extent::Cells(0);
        let theme = UiTheme::default();
        let inputs = UiRenderInputs { theme: &theme, offset: &|_| 0, facts: &|_| false, extent: &extent, bindings: &[] };
        let mut expected = Vec::new();
        let mut painted = Vec::new();
        layout_regions_with_facts(&tree, area, &extent, &mut expected, inputs.facts);
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|frame| render_with_facts(frame, area, &tree, &mut painted, &inputs)).unwrap();
        let snapshot = |regions: Vec<super::ui_elements::HitRegion>| regions.into_iter().map(|r| { let target = r.scroll_target().map(str::to_owned); (r.id, r.area, target) }).collect::<Vec<_>>();
        prop_assert_eq!(snapshot(painted), snapshot(expected));
    }

    #[test]
    fn nested_frame_groups_preserve_flat_rectangles(
        height in 0u16..80,
        width in 1u16..40,
        extensions in 0u16..16,
        editor in 0u16..20,
        candidates in 0u16..20,
        footer in 0u16..5,
    ) {
        let slots: Vec<_> = ["transcript", "extensions", "modeline", "editor", "candidates", "minibuffer"]
            .into_iter().map(|name| UiElement::Sized {
                size: if name == "transcript" { marlin_agent_ui::UiSize::Flex(1) }
                      else { marlin_agent_ui::UiSize::Binding(UiComponentId::new(name).unwrap()) },
                child: Box::new(UiElement::Region {
                    id: UiComponentId::new(name).unwrap(),
                    scroll_target: matches!(name, "transcript" | "candidates").then(|| marlin_agent_ui::UiScrollTarget::new(name).unwrap()),
                    child: Box::new(UiElement::Text(vec![])),
                }),
            }).collect();
        let flat = UiElement::Rows(slots.clone());
        let nested = UiElement::Rows(vec![UiElement::Rows(slots[..2].to_vec()), slots[2].clone(), UiElement::Region {
            id: UiComponentId::new("minibuffer-hover").unwrap(), scroll_target: None,
            child: Box::new(UiElement::Rows(slots[3..].to_vec())),
        }]);
        let extent = |id: &UiComponentId| match id.as_str() {
            "extensions" => Extent::Cells(extensions),
            "modeline" => Extent::Cells(1),
            "editor" => Extent::Cells(editor),
            "candidates" => Extent::Cells(candidates),
            _ => Extent::Cells(footer),
        };
        let area = Rect::new(0, 0, width, height);
        let mut expected = Vec::new();
        let mut actual = Vec::new();
        super::ui_elements::layout_regions(&flat, area, &extent, &mut expected);
        super::ui_elements::layout_regions(&nested, area, &extent, &mut actual);
        let snapshot = |regions: Vec<super::ui_elements::HitRegion>| regions.into_iter().filter(|region| region.id.as_str() != "minibuffer-hover")
            .map(|region| { let target = region.scroll_target().map(str::to_owned); (region.id, region.area, target) }).collect::<Vec<_>>();
        prop_assert_eq!(snapshot(actual), snapshot(expected));
    }

    #[test]
    fn prompt_keeps_a_bounded_suffix_and_cursor(width in 1u16..80, length in 0usize..300) {
        let tree = UiElement::Template {
            background: marlin_agent_ui::UiBackground::None,
            spans: vec![
                UiTemplateSpan { value: UiTemplateValue::Literal("> ".into()), tone: UiTone::Normal, bold: false },
                UiTemplateSpan { value: UiTemplateValue::Binding(UiComponentId::new("query").unwrap()), tone: UiTone::Normal, bold: false },
            ],
        };
        let theme = UiTheme::default();
        let inputs = UiRenderInputs { theme: &theme, offset: &|_| 0, facts: &|_| false, extent: &|_| Extent::Cells(0), bindings: &[] };
        let query = "x".repeat(length);
        let mut terminal = Terminal::new(TestBackend::new(width, 1)).unwrap();
        let mut cursor = None;
        terminal.draw(|frame| cursor = render_prompt(frame, frame.area(), &tree, &inputs, "query", &query)).unwrap();
        let available = usize::from(width).saturating_sub(2).saturating_sub(1);
        let visible = length.min(available);
        prop_assert_eq!(cursor, Some(((2 + visible).min(usize::from(width - 1)) as u16, 0)));
        for column in 2..(2 + visible) {
            prop_assert_eq!(terminal.backend().buffer()[(column as u16, 0)].symbol(), "x");
        }
    }

    #[test]
    fn choice_selects_first_true_fact(flags in proptest::collection::vec(proptest::bool::ANY, 0..32)) {
        let text = |value: String| UiElement::Text(vec![UiSpan { text: value, tone: UiTone::Normal }]);
        let tree = UiElement::Choice {
            cases: flags.iter().enumerate().map(|(index, _)| {
                (UiComponentId::new(&format!("fact-{index}")).unwrap(), text(index.to_string()))
            }).collect(),
            fallback: Box::new(text("fallback".into())),
        };
        let chosen = resolve_choice(&tree, &|fact| {
            fact.as_str().strip_prefix("fact-").and_then(|index| index.parse::<usize>().ok())
                .and_then(|index| flags.get(index)).copied().unwrap_or(false)
        });
        let expected = flags.iter().position(|flag| *flag).map_or_else(|| "fallback".into(), |index| index.to_string());
        prop_assert_eq!(chosen, &text(expected));
    }

    #[test]
    fn scrolling_and_hit_testing_share_visible_rows(count in 1usize..20, height in 1u16..10, offset in 0usize..25) {
        let rows = (0..count).map(|index| UiElement::Region {
            id: UiComponentId::new(&format!("row-{index}")).unwrap(),
            scroll_target: None,
            child: Box::new(UiElement::Text(vec![UiSpan { text: "X".into(), tone: UiTone::Normal }])),
        }).collect();
        let tree = UiElement::Region { id: UiComponentId::new("scroll").unwrap(), scroll_target: Some(marlin_agent_ui::UiScrollTarget::new("transcript").unwrap()),
            child: Box::new(UiElement::Rows(rows)) };
        let mut terminal = Terminal::new(TestBackend::new(8, height)).unwrap();
        let mut regions = Vec::new();
        terminal.draw(|frame| render_with_scroll(frame, frame.area(), &tree, &UiTheme::default(), &mut regions, &|_| offset)).unwrap();
        for y in 0..height {
            let index = offset + usize::from(y);
            let region = hit_test(&regions, 0, y).unwrap();
            if index < count {
                prop_assert_eq!(region.id.as_str(), format!("row-{index}"));
                prop_assert_eq!(terminal.backend().buffer()[(0, y)].symbol(), "X");
            } else {
                prop_assert_eq!(region.id.as_str(), "scroll");
            }
        }
    }
    #[test]
    fn hit_test_respects_terminal_bounds(width in 1u16..120, height in 1u16..50) {
        let tree = UiElement::Region {
            id: UiComponentId::new("viewport").unwrap(),
            scroll_target: Some(marlin_agent_ui::UiScrollTarget::new("transcript").unwrap()),
            child: Box::new(UiElement::Text(vec![])),
        };
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        let mut regions = Vec::new();
        terminal.draw(|frame| {
            render_with_regions(frame, frame.area(), &tree, &UiTheme::default(), &mut regions);
        }).unwrap();
        prop_assert!(hit_test_scroll(&regions, width - 1, height - 1).is_some());
        prop_assert!(hit_test_scroll(&regions, width, height - 1).is_none());
        prop_assert!(hit_test_scroll(&regions, width - 1, height).is_none());
    }

    #[test]
    fn columns_partition_the_visible_width(width in 1u16..100, count in 1usize..32) {
        let tree = UiElement::Columns((0..count).map(|index| UiElement::Region {
            id: UiComponentId::new(&format!("item-{index}")).unwrap(),
            scroll_target: Some(marlin_agent_ui::UiScrollTarget::new("transcript").unwrap()),
            child: Box::new(UiElement::Text(vec![])),
        }).collect());
        let mut terminal = Terminal::new(TestBackend::new(width, 1)).unwrap();
        let mut regions = Vec::new();
        terminal.draw(|frame| render_with_regions(frame, frame.area(), &tree, &UiTheme::default(), &mut regions)).unwrap();
        for x in 0..width {
            prop_assert!(hit_test_scroll(&regions, x, 0).is_some());
            prop_assert_eq!(regions.iter().filter(|region| x >= region.area.x && x < region.area.right()).count(), 1);
        }
        prop_assert!(hit_test(&regions, width, 0).is_none());
    }
}
