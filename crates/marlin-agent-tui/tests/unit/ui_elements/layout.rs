// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use crate::ui_elements::{
    Extent, UiRenderInputs, hit_test, hit_test_scroll, layout_regions_with_facts,
    layout_regions_with_scroll, render_element, render_with_facts, render_with_regions,
    render_with_scroll,
};
use marlin_agent_ui::{UiComponentId, UiElement, UiSpan, UiTheme, UiTone};
use ratatui::{Terminal, backend::TestBackend, layout::Rect, style::Color};

#[test]
fn nested_scrolling_hits_the_visible_inner_region() {
    let id = |name| UiComponentId::new(name).unwrap();
    let row = |name| UiElement::Region {
        id: id(name),
        scroll_target: None,
        child: Box::new(UiElement::Text(vec![UiSpan {
            text: name.into(),
            tone: UiTone::Normal,
        }])),
    };
    let tree = UiElement::Region {
        id: id("outer"),
        scroll_target: Some(marlin_agent_ui::UiScrollTarget::new("outer", 1).unwrap()),
        child: Box::new(UiElement::Rows(vec![
            row("header"),
            UiElement::Region {
                id: id("inner"),
                scroll_target: Some(marlin_agent_ui::UiScrollTarget::new("inner", 1).unwrap()),
                child: Box::new(UiElement::Rows(vec![
                    row("first"),
                    row("second"),
                    row("third"),
                ])),
            },
        ])),
    };
    let offset = |id: &UiComponentId| usize::from(id.as_str() == "inner");
    let mut terminal = Terminal::new(TestBackend::new(8, 2)).unwrap();
    let mut painted = Vec::new();
    terminal
        .draw(|frame| {
            render_with_scroll(
                frame,
                frame.area(),
                &tree,
                &UiTheme::default(),
                &mut painted,
                &offset,
            )
        })
        .unwrap();
    let mut measured = Vec::new();
    layout_regions_with_scroll(
        &tree,
        Rect::new(0, 0, 8, 2),
        &|_| Extent::Cells(0),
        &mut measured,
        &|_| false,
        &offset,
    );
    assert_eq!(terminal.backend().buffer()[(0, 1)].symbol(), "s");
    assert_eq!(hit_test(&painted, 0, 1).unwrap().id.as_str(), "second");
    assert_eq!(hit_test(&measured, 0, 1).unwrap().id.as_str(), "second");
    assert_eq!(
        hit_test_scroll(&measured, 0, 1).unwrap().scroll_target(),
        Some("inner")
    );
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
        crate::ui_elements::resolve_size(&size, &metrics, &|_| true),
        Extent::Cells(7)
    );
    assert_eq!(
        crate::ui_elements::resolve_size(&size, &metrics, &|_| false),
        Extent::Cells(0)
    );
    assert_eq!(
        crate::ui_elements::resolve_size(&size, &|_| Extent::Flex(1), &|_| true),
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
            crate::ui_elements::resolve_size(&size, &|_| Extent::Cells(0), &|_| false),
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
    let resolver = crate::ui_elements::RegionExtents::new(&tree, &raw, &facts);
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
        scroll_target: Some(marlin_agent_ui::UiScrollTarget::new("transcript", 3).unwrap()),
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
    impl crate::ui_elements::UiSlotRenderer for Labels {
        fn render(
            &mut self,
            frame: &mut ratatui::Frame<'_>,
            area: Rect,
            id: &UiComponentId,
            _: Option<&UiElement>,
            _: &UiRenderInputs<'_>,
            _: usize,
        ) {
            frame.render_widget(ratatui::widgets::Paragraph::new(id.as_str()), area);
        }
    }
    let region = |name: &str| UiElement::Region {
        id: UiComponentId::new(name).unwrap(),
        scroll_target: Some(marlin_agent_ui::UiScrollTarget::new("transcript", 3).unwrap()),
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
            crate::ui_elements::render_with_slots(
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
    let target = |name, step| Some(marlin_agent_ui::UiScrollTarget::new(name, step).unwrap());
    let region = |id, scroll_target| UiElement::Region {
        id: UiComponentId::new(id).unwrap(),
        scroll_target,
        child: Box::new(UiElement::Text(vec![])),
    };
    let tree = UiElement::Columns(vec![
        region("transcript", target("transcript", 3)),
        region("nested", target("candidates", 1)),
    ]);
    let mut regions = Vec::new();
    crate::ui_elements::layout_regions(
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
    assert_eq!(
        hit_test_scroll(&regions, 0, 0).unwrap().scroll_step(),
        Some(3)
    );
    assert_eq!(
        hit_test_scroll(&regions, 10, 0).unwrap().scroll_step(),
        Some(1)
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
        scroll_target: Some(marlin_agent_ui::UiScrollTarget::new("transcript", 3).unwrap()),
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
                scroll_target: Some(marlin_agent_ui::UiScrollTarget::new("transcript", 3).unwrap()),
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
        field_template: Box::new(UiElement::Template {
            background: marlin_agent_ui::UiBackground::None,
            spans: vec![],
        }),
        title: None,
        child: Box::new(UiElement::Rows(vec![UiElement::Choice {
            cases: vec![(UiComponentId::new("compact").unwrap(), input(false))],
            fallback: Box::new(input(true)),
        }])),
    };
    assert_eq!(crate::ui_elements::input_border(&tree, &|_| false), Some(4));
    assert_eq!(crate::ui_elements::input_border(&tree, &|_| true), Some(2));
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
    crate::ui_elements::layout_regions(
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
        scroll_target: Some(marlin_agent_ui::UiScrollTarget::new("transcript", 3).unwrap()),
        child: Box::new(UiElement::Rows(vec![row("a", "A\nA"), row("b", "B\nB")])),
    };
    let mut terminal = Terminal::new(TestBackend::new(8, 3)).unwrap();
    let mut regions = Vec::new();
    terminal
        .draw(|frame| {
            crate::ui_elements::render_with_scroll(
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
