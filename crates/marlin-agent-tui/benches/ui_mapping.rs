// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Terminal mapping and hit testing; excludes VM initialization and projection.

use criterion::{Criterion, criterion_group, criterion_main};
use marlin_agent_tui::ui_elements::{
    Extent, UiContentSlot, UiListRow, UiRenderInputs, hit_test, hit_test_scroll, layout_regions,
    layout_regions_with_facts, render_input, render_list, render_prompt, render_template,
    render_with_regions, render_with_scroll, render_with_slots, resolve_choice,
};
use marlin_agent_ui::{
    UiComponentId, UiElement, UiSpan, UiTemplateSpan, UiTemplateValue, UiTheme, UiTone,
};
use ratatui::{Terminal, backend::TestBackend};
use std::{hint::black_box, time::Duration};

fn mapping(c: &mut Criterion) {
    let template = UiElement::Template {
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
    let list_template = UiElement::Choice {
        cases: vec![(UiComponentId::new("selected").unwrap(), template.clone())],
        fallback: Box::new(template.clone()),
    };
    let mut list_terminal = Terminal::new(TestBackend::new(80, 10)).unwrap();
    c.bench_function("ui/mapping/list-256-visible-10", |b| {
        b.iter(|| {
            list_terminal
                .draw(|frame| {
                    render_list(
                        frame,
                        frame.area(),
                        &list_template,
                        &theme,
                        256,
                        Some(255),
                        |index| {
                            Some(UiListRow {
                                bindings: vec![("name", format!("command-{index}").into())],
                            })
                        },
                    )
                })
                .unwrap();
        })
    });
    let choice = UiElement::Choice {
        cases: (0..10)
            .map(|index| {
                (
                    UiComponentId::new(&format!("fact-{index}")).unwrap(),
                    template.clone(),
                )
            })
            .collect(),
        fallback: Box::new(template.clone()),
    };
    c.bench_function("ui/mapping/choice-last-of-10", |b| {
        b.iter(|| {
            black_box(resolve_choice(black_box(&choice), &|id| {
                id.as_str() == "fact-9"
            }))
        })
    });
    let mut prompt_terminal = Terminal::new(TestBackend::new(80, 1)).unwrap();
    let inputs = UiRenderInputs {
        theme: &theme,
        offset: &|_| 0,
        facts: &|_| false,
        extent: &|_| Extent::Cells(0),
        bindings: &[],
    };
    let query = "x".repeat(256);
    c.bench_function("ui/mapping/prompt-256", |b| {
        b.iter(|| {
            prompt_terminal
                .draw(|frame| {
                    black_box(render_prompt(
                        frame,
                        frame.area(),
                        &template,
                        &inputs,
                        "name",
                        black_box(&query),
                    ));
                })
                .unwrap();
        })
    });
    let input = UiElement::Input {
        bordered: false,
        background: marlin_agent_ui::UiBackground::Bar,
        prefix: Some(UiComponentId::new("prompt").unwrap()),
        title: None,
        active: UiComponentId::new("active").unwrap(),
    };
    let input_values = UiRenderInputs {
        theme: &theme,
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
    let panel = UiElement::Panel {
        border_tone: UiTone::Info,
        field_tone: UiTone::Accent,
        child: Box::new(UiElement::Slot(UiComponentId::new("static").unwrap())),
    };
    let fields: Vec<_> = (0..16)
        .map(|n| marlin_agent_ui::UiContent::Field {
            label: format!("field-{n}"),
            value: format!("value-{n}"),
        })
        .collect();
    let mut panel_terminal = Terminal::new(TestBackend::new(80, 20)).unwrap();
    let mut panel_slot = UiContentSlot {
        title: "Panel",
        content: &fields,
        theme: &theme,
        scroll: 0,
    };
    c.bench_function("ui/mapping/panel-16-fields", |b| {
        b.iter(|| {
            panel_terminal
                .draw(|frame| {
                    render_with_slots(
                        frame,
                        frame.area(),
                        black_box(&panel),
                        &mut Vec::new(),
                        &input_values,
                        black_box(&mut panel_slot),
                    );
                })
                .unwrap();
        })
    });
    let mut template_terminal = Terminal::new(TestBackend::new(120, 80)).unwrap();
    c.bench_function("ui/mapping/templates-64", |b| {
        b.iter(|| {
            template_terminal
                .draw(|frame| {
                    for y in 0..64 {
                        render_template(
                            frame,
                            ratatui::layout::Rect::new(0, y, 120, 1),
                            black_box(&template),
                            &theme,
                            &[("name", "model-selection")],
                        );
                    }
                })
                .unwrap();
        })
    });
    let frame_tree = UiElement::Rows(
        [
            "transcript",
            "extensions",
            "modeline",
            "editor",
            "candidates",
            "minibuffer",
        ]
        .into_iter()
        .map(|id| UiElement::Sized {
            size: if id == "transcript" {
                marlin_agent_ui::UiSize::Flex(1)
            } else {
                marlin_agent_ui::UiSize::Binding(UiComponentId::new(id).unwrap())
            },
            child: Box::new(UiElement::Region {
                id: UiComponentId::new(id).unwrap(),
                scrollable: matches!(id, "transcript" | "candidates"),
                child: Box::new(UiElement::Text(vec![])),
            }),
        })
        .collect(),
    );
    let mut mapped = Vec::with_capacity(7);
    c.bench_function("ui/mapping/frame-hit", |b| {
        b.iter(|| {
            mapped.clear();
            layout_regions(
                black_box(&frame_tree),
                ratatui::layout::Rect::new(0, 0, 120, 40),
                &|_| Extent::Cells(1),
                &mut mapped,
            );
            black_box(hit_test(&mapped, 20, 38));
        })
    });
    let UiElement::Rows(slots) = &frame_tree else {
        unreachable!()
    };
    let grouped_frame = UiElement::Rows(vec![
        UiElement::Rows(slots[..2].to_vec()),
        slots[2].clone(),
        UiElement::Region {
            id: UiComponentId::new("minibuffer-hover").unwrap(),
            scrollable: false,
            child: Box::new(UiElement::Rows(slots[3..].to_vec())),
        },
    ]);
    c.bench_function("ui/mapping/grouped-frame-hit", |b| {
        b.iter(|| {
            mapped.clear();
            layout_regions(
                black_box(&grouped_frame),
                ratatui::layout::Rect::new(0, 0, 120, 40),
                &|_| Extent::Cells(1),
                &mut mapped,
            );
            black_box(hit_test(&mapped, 20, 38));
        })
    });
    let selected_frame = UiElement::Choice {
        cases: vec![(UiComponentId::new("active").unwrap(), grouped_frame.clone())],
        fallback: Box::new(frame_tree),
    };
    c.bench_function("ui/mapping/selected-frame-hit", |b| {
        b.iter(|| {
            mapped.clear();
            layout_regions_with_facts(
                black_box(&selected_frame),
                ratatui::layout::Rect::new(0, 0, 120, 40),
                &|_| Extent::Cells(1),
                &mut mapped,
                &|id| id.as_str() == "active",
            );
            black_box(hit_test_scroll(&mapped, 20, 38));
        })
    });
    let mut empty = Terminal::new(TestBackend::new(120, 80)).unwrap();
    c.bench_function("ui/mapping/empty-frame", |b| {
        b.iter(|| {
            empty
                .draw(|frame| {
                    black_box(frame.area());
                })
                .unwrap();
        });
    });
    for count in [8, 64] {
        let tree = UiElement::Rows(
            (0..count)
                .map(|n| UiElement::Region {
                    id: UiComponentId::new(&format!("item-{n}")).unwrap(),
                    scrollable: true,
                    child: Box::new(UiElement::Text(vec![UiSpan {
                        text: "message 🐟".into(),
                        tone: UiTone::Info,
                    }])),
                })
                .collect(),
        );
        let theme = UiTheme::default();
        let mut terminal = Terminal::new(TestBackend::new(120, 80)).unwrap();
        let mut regions = Vec::with_capacity(count);
        c.bench_function(&format!("ui/mapping/rows-{count}"), |b| {
            b.iter(|| {
                regions.clear();
                terminal
                    .draw(|frame| {
                        render_with_regions(
                            frame,
                            frame.area(),
                            black_box(&tree),
                            &theme,
                            &mut regions,
                        )
                    })
                    .unwrap();
                black_box(hit_test_scroll(&regions, 1, (count - 1) as u16));
            })
        });
    }
    let viewport = UiElement::Region {
        id: UiComponentId::new("viewport").unwrap(),
        scrollable: true,
        child: Box::new(UiElement::Rows(
            (0..64)
                .map(|n| UiElement::Region {
                    id: UiComponentId::new(&format!("item-{n}")).unwrap(),
                    scrollable: false,
                    child: Box::new(UiElement::Text(vec![UiSpan {
                        text: "message 🐟".into(),
                        tone: UiTone::Info,
                    }])),
                })
                .collect(),
        )),
    };
    let theme = UiTheme::default();
    let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
    let mut regions = Vec::with_capacity(65);
    c.bench_function("ui/mapping/scroll-64", |b| {
        b.iter(|| {
            regions.clear();
            terminal
                .draw(|frame| {
                    render_with_scroll(
                        frame,
                        frame.area(),
                        black_box(&viewport),
                        &theme,
                        &mut regions,
                        &|_| 16,
                    )
                })
                .unwrap();
            black_box(hit_test_scroll(&regions, 1, 29));
        })
    });
}

criterion_group! { name = benches; config = Criterion::default().sample_size(30).measurement_time(Duration::from_secs(3)); targets = mapping }
criterion_main!(benches);
