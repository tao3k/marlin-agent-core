// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use crate::ui_elements::{
    Extent, UiRenderInputs, hit_test, hit_test_scroll, layout_regions_with_facts,
    layout_regions_with_scroll, render_prompt, render_with_facts, render_with_regions,
    render_with_scroll, resolve_choice,
};
use marlin_agent_ui::{
    UiComponentId, UiElement, UiSpan, UiTemplateSpan, UiTemplateValue, UiTheme, UiTone,
};
use proptest::{prop_assert, prop_assert_eq, proptest};
use ratatui::{Terminal, backend::TestBackend, layout::Rect};

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
        let snapshot = |regions: Vec<crate::ui_elements::HitRegion>| regions.into_iter().map(|r| { let target = r.scroll_target().map(str::to_owned); (r.id, r.area, target) }).collect::<Vec<_>>();
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
                    scroll_target: matches!(name, "transcript" | "candidates").then(|| marlin_agent_ui::UiScrollTarget::new(name, 3).unwrap()),
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
        crate::ui_elements::layout_regions(&flat, area, &extent, &mut expected);
        crate::ui_elements::layout_regions(&nested, area, &extent, &mut actual);
        let snapshot = |regions: Vec<crate::ui_elements::HitRegion>| regions.into_iter().filter(|region| region.id.as_str() != "minibuffer-hover")
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
        let tree = UiElement::Region { id: UiComponentId::new("scroll").unwrap(), scroll_target: Some(marlin_agent_ui::UiScrollTarget::new("transcript", 3).unwrap()),
            child: Box::new(UiElement::Rows(rows)) };
        let mut terminal = Terminal::new(TestBackend::new(8, height)).unwrap();
        let mut regions = Vec::new();
        terminal.draw(|frame| render_with_scroll(frame, frame.area(), &tree, &UiTheme::default(), &mut regions, &|_| offset)).unwrap();
        let mut measured = Vec::new();
        layout_regions_with_scroll(&tree, Rect::new(0, 0, 8, height), &|_| Extent::Cells(0),
                                   &mut measured, &|_| false, &|_| offset);
        let snapshot = |items: &[crate::ui_elements::HitRegion]| items.iter()
            .map(|region| (region.id.as_str().to_owned(), region.area, region.scroll_target().map(str::to_owned)))
            .collect::<Vec<_>>();
        prop_assert_eq!(snapshot(&regions), snapshot(&measured));
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
            scroll_target: Some(marlin_agent_ui::UiScrollTarget::new("transcript", 3).unwrap()),
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
            scroll_target: Some(marlin_agent_ui::UiScrollTarget::new("transcript", 3).unwrap()),
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
