// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use criterion::{BenchmarkId, Criterion};
use marlin_agent_tui::ui_elements::{
    Extent, UiRenderInputs, UiSlotRenderer, render_input, render_with_slots,
};
use marlin_agent_ui::{
    UiBackground, UiComponentId, UiElement, UiInputAdornment, UiTemplateSpan, UiTemplateValue,
    UiTheme, UiTone,
};
use ratatui::{Terminal, backend::TestBackend};
use std::hint::black_box;

pub(super) fn mapping(c: &mut Criterion) {
    let id = |name| UiComponentId::new(name).unwrap();
    let input = UiElement::Input {
        bordered: false,
        background: UiBackground::Bar,
        prefix: None,
        title: None,
        active: id("active"),
        trailing: Some(UiInputAdornment {
            cells: 18,
            view: Box::new(UiElement::Template {
                background: UiBackground::None,
                spans: vec![UiTemplateSpan {
                    value: UiTemplateValue::Binding(id("feedback")),
                    tone: UiTone::Muted,
                    bold: false,
                }],
            }),
        }),
    };
    let theme = UiTheme::default();
    let mut group = c.benchmark_group("ui/mapping/input-adornment");
    for (case, width, feedback) in [
        ("empty", 80, ""),
        ("visible", 80, "Feedback ready"),
        ("clipped-wide", 24, "剪贴板 · feedback"),
    ] {
        let inputs = UiRenderInputs {
            theme: &theme,
            offset: &|_| 0,
            facts: &|_| true,
            extent: &|_| Extent::Cells(0),
            bindings: &[("feedback", feedback)],
        };
        let mut terminal = Terminal::new(TestBackend::new(width, 2)).unwrap();
        group.bench_function(BenchmarkId::new(case, width), |b| {
            b.iter(|| {
                terminal
                    .draw(|frame| {
                        render_input(frame, frame.area(), &input, &inputs, true, |_, _| {
                            Some((vec!["draft".into(), "next".into()], (4, 1)))
                        });
                    })
                    .unwrap();
            });
        });
    }
    group.finish();
}

pub(super) fn plain(c: &mut Criterion, theme: &UiTheme) {
    let input = UiElement::Input {
        bordered: false,
        background: marlin_agent_ui::UiBackground::Bar,
        prefix: Some(Box::new(UiElement::Template {
            background: marlin_agent_ui::UiBackground::None,
            spans: vec![marlin_agent_ui::UiTemplateSpan {
                value: marlin_agent_ui::UiTemplateValue::Binding(
                    UiComponentId::new("prompt").unwrap(),
                ),
                tone: UiTone::Accent,
                bold: false,
            }],
        })),
        title: None,
        active: UiComponentId::new("active").unwrap(),
        trailing: None,
    };
    let input_values = UiRenderInputs {
        theme,
        offset: &|_| 0,
        facts: &|_| false,
        extent: &|_| Extent::Cells(0),
        bindings: &[("prompt", "M-x ")],
    };
    let mut input_terminal = Terminal::new(TestBackend::new(80, 1)).unwrap();
    c.bench_function("ui/mapping/input", |b| {
        b.iter(|| {
            input_terminal
                .draw(|frame| {
                    render_input(frame, frame.area(), &input, &input_values, true, |_, _| {
                        Some((vec![black_box("command").to_owned()], (7, 0)))
                    });
                })
                .unwrap();
        })
    });
    struct Editor;
    impl UiSlotRenderer for Editor {
        fn render(
            &mut self,
            _: &mut ratatui::Frame<'_>,
            _: ratatui::layout::Rect,
            _: &marlin_agent_ui::UiSlotSource,
            _: Option<&UiElement>,
            _: &UiRenderInputs<'_>,
            _: usize,
        ) {
        }
        fn input(
            &mut self,
            frame: &mut ratatui::Frame<'_>,
            area: ratatui::layout::Rect,
            element: &UiElement,
            inputs: &UiRenderInputs<'_>,
        ) {
            render_input(frame, area, element, inputs, true, |_, _| {
                Some((vec![black_box("command").to_owned()], (7, 0)))
            });
        }
    }
    let input_tree = UiElement::Rows(vec![UiElement::Sized {
        size: marlin_agent_ui::UiSize::Flex(1),
        child: Box::new(input.clone()),
    }]);
    c.bench_function("ui/mapping/input-tree", |b| {
        b.iter(|| {
            input_terminal
                .draw(|frame| {
                    render_with_slots(
                        frame,
                        frame.area(),
                        &input_tree,
                        &mut Vec::new(),
                        &input_values,
                        &mut Editor,
                    )
                })
                .unwrap();
        })
    });
}
