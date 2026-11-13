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
fn choice_uses_declared_order_and_fallback() {
    let text = |value: &str| {
        UiElement::Text(vec![UiSpan {
            text: value.into(),
            tone: UiTone::Normal,
        }])
    };
    let tree = UiElement::Choice {
        cases: vec![
            (UiComponentId::new("notice").unwrap(), text("notice")),
            (UiComponentId::new("messages").unwrap(), text("messages")),
        ],
        fallback: Box::new(text("default")),
    };
    assert_eq!(resolve_choice(&tree, &|_| true), &text("notice"));
    assert_eq!(
        resolve_choice(&tree, &|id| id.as_str() == "messages"),
        &text("messages")
    );
    assert_eq!(resolve_choice(&tree, &|_| false), &text("default"));
    let nested = UiElement::Rows(vec![UiElement::Choice {
        cases: vec![(UiComponentId::new("tall").unwrap(), text("one\ntwo"))],
        fallback: Box::new(text("one")),
    }]);
    assert_eq!(nested.intrinsic_rows_with(&|_| true), 2);
    assert_eq!(nested.intrinsic_rows_with(&|_| false), 1);
}

#[test]
fn choice_uses_same_facts_for_layout_and_paint() {
    let region = |id: &str| UiElement::Region {
        id: UiComponentId::new(id).unwrap(),
        scrollable: true,
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
        scrollable: false,
        child: Box::new(UiElement::Text(vec![UiSpan {
            text: text.into(),
            tone: UiTone::Normal,
        }])),
    };
    let tree = UiElement::Region {
        id: UiComponentId::new("messages").unwrap(),
        scrollable: true,
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
        scrollable: false,
        child: Box::new(UiElement::Columns(vec![
            UiElement::Text(vec![]),
            UiElement::Region {
                id: UiComponentId::new("messages").unwrap(),
                scrollable: true,
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

proptest! {
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
            scrollable: false,
            child: Box::new(UiElement::Text(vec![UiSpan { text: "X".into(), tone: UiTone::Normal }])),
        }).collect();
        let tree = UiElement::Region { id: UiComponentId::new("scroll").unwrap(), scrollable: true,
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
            scrollable: true,
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
            scrollable: true,
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
