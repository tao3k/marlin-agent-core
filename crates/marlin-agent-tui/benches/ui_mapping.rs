// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Terminal mapping and hit testing; excludes VM initialization and projection.

// Reuse the production queue without exporting an internal terminal API.
#[path = "../src/input/wheel.rs"]
mod benchmark_input;

use criterion::{Criterion, criterion_group, criterion_main};
use marlin_agent_tui::ui_elements::{
    Extent, RegionExtents, UiContentSlot, UiListRow, UiListState, UiRenderInputs, UiSlotRenderer,
    hit_test, hit_test_scroll, layout_regions, layout_regions_with_facts,
    layout_regions_with_scroll, render_input, render_list, render_prompt, render_template,
    render_with_facts, render_with_regions, render_with_scroll, render_with_slots, resolve_choice,
    resolve_size,
};
use marlin_agent_ui::{
    UiComponentId, UiElement, UiSpan, UiTemplateSpan, UiTemplateValue, UiTheme, UiTone,
};
use ratatui::{Terminal, backend::TestBackend, layout::Rect, widgets::Paragraph};
use std::{hint::black_box, time::Duration};

fn clear_scroll_targets(element: &mut UiElement) {
    match element {
        UiElement::Region {
            scroll_target,
            child,
            ..
        } => {
            *scroll_target = None;
            clear_scroll_targets(child);
        }
        UiElement::Rows(children) | UiElement::Columns(children) => {
            children.iter_mut().for_each(clear_scroll_targets);
        }
        UiElement::Choice { cases, fallback } => {
            cases
                .iter_mut()
                .for_each(|(_, child)| clear_scroll_targets(child));
            clear_scroll_targets(fallback);
        }
        UiElement::Sized { child, .. } | UiElement::Panel { child, .. } => {
            clear_scroll_targets(child)
        }
        _ => {}
    }
}

#[derive(Default)]
struct AskFrameSlots {
    list: UiListState,
}

impl UiSlotRenderer for AskFrameSlots {
    fn render(
        &mut self,
        frame: &mut ratatui::Frame<'_>,
        area: Rect,
        _: &UiComponentId,
        _: Option<&UiElement>,
        _: &UiRenderInputs<'_>,
        _: usize,
    ) {
        frame.render_widget(Paragraph::new("conversation"), area);
    }

    fn input(
        &mut self,
        frame: &mut ratatui::Frame<'_>,
        area: Rect,
        _: &UiElement,
        _: &UiRenderInputs<'_>,
    ) {
        frame.render_widget(Paragraph::new("Tool request:"), area);
    }

    fn list(
        &mut self,
        frame: &mut ratatui::Frame<'_>,
        area: Rect,
        element: &UiElement,
        inputs: &UiRenderInputs<'_>,
    ) {
        render_list(
            frame,
            area,
            element,
            inputs.theme,
            marlin_agent_tui::ui_elements::UiListData {
                count: 2,
                selected: Some(0),
                extent: inputs.extent,
                facts: inputs.facts,
                bindings: inputs.bindings,
            },
            &mut self.list,
            |index| {
                let (name, description) = if index == 0 {
                    ("approve-once", "Run the requested tool once")
                } else {
                    ("reject", "Do not run the requested tool")
                };
                Some(UiListRow {
                    key: Some(UiComponentId::new(name).unwrap()),
                    bindings: vec![("name", name.into()), ("description", description.into())],
                })
            },
        );
    }
}

fn mapping(c: &mut Criterion) {
    let wheel = benchmark_input::WheelAccumulator::default();
    c.bench_function("ui/input/wheel-coalesced-16", |b| {
        b.iter(|| {
            for _ in 0..16 {
                black_box(wheel.push(true, 4, 3));
            }
            black_box(wheel.take_steps());
        })
    });
    c.bench_function("ui/input/wheel-coalesced-101", |b| {
        b.iter(|| {
            for _ in 0..101 {
                black_box(wheel.push(true, 4, 3));
            }
            while wheel.has_pending() {
                black_box(wheel.take_steps());
            }
        })
    });
    c.bench_function("ui/input/wheel-two-regions", |b| {
        b.iter(|| {
            black_box(wheel.push(true, 4, 3));
            black_box(wheel.push(false, 4, 18));
            black_box(wheel.take_steps());
            black_box(wheel.take_steps());
            black_box(wheel.has_pending());
        })
    });
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
    let size_choice = marlin_agent_ui::UiSize::Choice {
        cases: vec![
            (
                UiComponentId::new("review").unwrap(),
                marlin_agent_ui::UiSize::Bounded {
                    value: UiComponentId::new("review-count").unwrap(),
                    limit: UiComponentId::new("available").unwrap(),
                },
            ),
            (
                UiComponentId::new("command").unwrap(),
                marlin_agent_ui::UiSize::Bounded {
                    value: UiComponentId::new("limit").unwrap(),
                    limit: UiComponentId::new("available").unwrap(),
                },
            ),
        ],
        fallback: Box::new(marlin_agent_ui::UiSize::Cells(0)),
    };
    c.bench_function("ui/mapping/bounded-size-choice", |b| {
        b.iter(|| {
            black_box(resolve_size(
                black_box(&size_choice),
                &|id| Extent::Cells(if id.as_str() == "limit" { 10 } else { 7 }),
                &|id| id.as_str() == "command",
            ))
        })
    });
    // The expanded editor budget is a nested expression projected by Scheme.
    use marlin_agent_ui::UiSize;
    let bind = |name| UiSize::Binding(UiComponentId::new(name).unwrap());
    let binary = |f: fn(Box<UiSize>, Box<UiSize>) -> UiSize, a, b| f(Box::new(a), Box::new(b));
    let transcript = binary(
        UiSize::Min,
        bind("transcript-min"),
        binary(UiSize::Subtract, bind("terminal-height"), UiSize::Cells(2)),
    );
    let footer = binary(UiSize::Max, bind("footer-min"), bind("hint-rows"));
    let reserved = binary(
        UiSize::Add,
        binary(UiSize::Add, footer, UiSize::Cells(1)),
        transcript,
    );
    let editor = binary(
        UiSize::Min,
        bind("measured-editor"),
        binary(
            UiSize::Min,
            bind("chatbox-limit"),
            binary(UiSize::Subtract, bind("terminal-height"), reserved),
        ),
    );
    c.bench_function("ui/mapping/editor-budget", |b| {
        b.iter(|| {
            black_box(resolve_size(
                black_box(&editor),
                &|id| {
                    Extent::Cells(if id.as_str() == "terminal-height" {
                        40
                    } else {
                        7
                    })
                },
                &|_| false,
            ))
        })
    });
    let region = |name, size| UiElement::Sized {
        size,
        child: Box::new(UiElement::Region {
            id: UiComponentId::new(name).unwrap(),
            scroll_target: None,
            child: Box::new(UiElement::Text(Vec::new())),
        }),
    };
    let dependent_frame = UiElement::Rows(vec![
        region("editor", editor.clone()),
        region(
            "footer",
            binary(UiSize::Min, bind("region/editor"), UiSize::Cells(2)),
        ),
        region(
            "modeline",
            binary(UiSize::Add, bind("region/footer"), UiSize::Cells(1)),
        ),
    ]);
    let raw = |id: &UiComponentId| {
        Extent::Cells(if id.as_str() == "terminal-height" {
            40
        } else {
            7
        })
    };
    let facts = |_: &UiComponentId| false;
    c.bench_function("ui/mapping/dependent-frame-layout", |b| {
        b.iter(|| {
            let extents = RegionExtents::new(&dependent_frame, &raw, &facts);
            let mut regions = Vec::new();
            layout_regions_with_facts(
                black_box(&dependent_frame),
                ratatui::layout::Rect::new(0, 0, 80, 40),
                &|id| extents.extent(id),
                &mut regions,
                &facts,
            );
            black_box(regions)
        })
    });
    let mut column_template = template.clone();
    if let UiElement::Template { spans, .. } = &mut column_template {
        spans[1].value = UiTemplateValue::Column {
            id: UiComponentId::new("name").unwrap(),
            width: UiSize::Binding(UiComponentId::new("candidate-name-width").unwrap()),
        };
    }
    for (benchmark, row, keyed) in [
        ("ui/mapping/list-256-visible-10", template.clone(), false),
        (
            "ui/mapping/list-column-256-visible-10",
            column_template.clone(),
            false,
        ),
        ("ui/mapping/list-hit-256-visible-10", column_template, true),
    ] {
        let list_template = UiElement::List {
            id: UiComponentId::new("benchmark-list").unwrap(),
            offset: UiSize::Cells(246),
            child: Box::new(UiElement::Choice {
                cases: vec![(UiComponentId::new("selected").unwrap(), row.clone())],
                fallback: Box::new(row),
            }),
        };
        let mut list_terminal = Terminal::new(TestBackend::new(80, 10)).unwrap();
        let mut list_state = marlin_agent_tui::ui_elements::UiListState::default();
        c.bench_function(benchmark, |b| {
            b.iter(|| {
                list_state.begin_frame();
                list_terminal
                    .draw(|frame| {
                        render_list(
                            frame,
                            frame.area(),
                            &list_template,
                            &theme,
                            marlin_agent_tui::ui_elements::UiListData {
                                count: 256,
                                selected: Some(255),
                                extent: &|_| Extent::Cells(16),
                                facts: &|_| false,
                                bindings: &[],
                            },
                            &mut list_state,
                            |index| {
                                Some(UiListRow {
                                    key: keyed.then(|| {
                                        UiComponentId::new(&format!("command-{index}")).unwrap()
                                    }),
                                    bindings: vec![("name", format!("command-{index}").into())],
                                })
                            },
                        )
                    })
                    .unwrap();
                if keyed {
                    black_box(list_state.hit(1, 9));
                }
            })
        });
    }
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
    let prompt_tree = UiElement::Prompt {
        field: UiComponentId::new("name").unwrap(),
        template: Box::new(template.clone()),
    };
    let prompt_inputs = UiRenderInputs {
        bindings: &[("name", query.as_str())],
        ..inputs
    };
    c.bench_function("ui/mapping/prompt-tree-256", |b| {
        b.iter(|| {
            prompt_terminal
                .draw(|frame| {
                    render_with_facts(
                        frame,
                        frame.area(),
                        black_box(&prompt_tree),
                        &mut Vec::new(),
                        &prompt_inputs,
                    );
                })
                .unwrap();
        })
    });
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
    struct Editor;
    impl UiSlotRenderer for Editor {
        fn render(
            &mut self,
            _: &mut ratatui::Frame<'_>,
            _: ratatui::layout::Rect,
            _: &UiComponentId,
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
    let panel = UiElement::Panel {
        border_tone: UiTone::Info,
        field_template: Box::new(UiElement::Template {
            background: marlin_agent_ui::UiBackground::None,
            spans: vec![
                UiTemplateSpan {
                    value: UiTemplateValue::Binding(UiComponentId::new("label").unwrap()),
                    tone: UiTone::Accent,
                    bold: false,
                },
                UiTemplateSpan {
                    value: UiTemplateValue::Literal(": ".into()),
                    tone: UiTone::Accent,
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
            spans: vec![UiTemplateSpan {
                value: UiTemplateValue::Literal(" Panel ".into()),
                tone: UiTone::Info,
                bold: false,
            }],
        })),
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
        content: &fields,
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
    let template_inputs = UiRenderInputs {
        theme: &theme,
        offset: &|_| 0,
        facts: &|_| false,
        extent: &|_| Extent::Cells(0),
        bindings: &[("name", "model-selection")],
    };
    c.bench_function("ui/mapping/templates-64", |b| {
        b.iter(|| {
            template_terminal
                .draw(|frame| {
                    for y in 0..64 {
                        render_template(
                            frame,
                            ratatui::layout::Rect::new(0, y, 120, 1),
                            black_box(&template),
                            &template_inputs,
                        );
                    }
                })
                .unwrap();
        })
    });
    let status = UiElement::Columns(vec![
        UiElement::Sized {
            size: UiSize::Binding(UiComponentId::new("prefix-width").unwrap()),
            child: Box::new(UiElement::Choice {
                cases: vec![(UiComponentId::new("active").unwrap(), template.clone())],
                fallback: Box::new(template.clone()),
            }),
        },
        UiElement::Sized {
            size: UiSize::Flex(1),
            child: Box::new(template.clone()),
        },
    ]);
    let status_inputs = UiRenderInputs {
        theme: &theme,
        offset: &|_| 0,
        facts: &|id| id.as_str() == "active",
        extent: &|_| Extent::Cells(16),
        bindings: &[("name", "status")],
    };
    let mut status_terminal = Terminal::new(TestBackend::new(80, 1)).unwrap();
    c.bench_function("ui/mapping/status-columns", |b| {
        b.iter(|| {
            status_terminal
                .draw(|frame| {
                    render_with_facts(
                        frame,
                        frame.area(),
                        black_box(&status),
                        &mut Vec::new(),
                        &status_inputs,
                    );
                })
                .unwrap();
        })
    });
    let bar = UiElement::Bar {
        background: marlin_agent_ui::UiBackground::Bar,
        right_divisor: 3,
        segments: (0..10)
            .map(|index| marlin_agent_ui::UiBarSegment {
                side: if index < 6 {
                    marlin_agent_ui::UiBarSide::Left
                } else {
                    marlin_agent_ui::UiBarSide::Right
                },
                priority: if index == 0 { 0 } else { 2 },
                min_width: 0,
                condition: None,
                child: Box::new(template.clone()),
            })
            .collect(),
    };
    c.bench_function("ui/mapping/bar-10-segments", |b| {
        b.iter(|| {
            status_terminal
                .draw(|frame| {
                    render_with_facts(
                        frame,
                        frame.area(),
                        black_box(&bar),
                        &mut Vec::new(),
                        &status_inputs,
                    );
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
                scroll_target: matches!(id, "transcript" | "candidates")
                    .then(|| marlin_agent_ui::UiScrollTarget::new(id, 3).unwrap()),
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
            scroll_target: None,
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
    let mut unbound_frame = selected_frame.clone();
    clear_scroll_targets(&mut unbound_frame);
    c.bench_function("ui/mapping/selected-frame-no-scroll", |b| {
        b.iter(|| {
            mapped.clear();
            layout_regions_with_facts(
                black_box(&unbound_frame),
                ratatui::layout::Rect::new(0, 0, 120, 40),
                &|_| Extent::Cells(1),
                &mut mapped,
                &|id| id.as_str() == "active",
            );
            black_box(hit_test_scroll(&mapped, 20, 38));
        })
    });
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
            black_box(
                hit_test_scroll(&mapped, 20, 38)
                    .map(|hit| (hit.scroll_target(), hit.scroll_step())),
            );
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
    let sized = |size, child| UiElement::Sized {
        size,
        child: Box::new(child),
    };
    let ask_tree = UiElement::Choice {
        cases: vec![(
            UiComponentId::new("ask-active").unwrap(),
            UiElement::Rows(vec![
                sized(
                    marlin_agent_ui::UiSize::Flex(1),
                    UiElement::Region {
                        id: UiComponentId::new("conversation").unwrap(),
                        scroll_target: None,
                        child: Box::new(UiElement::Slot(UiComponentId::new("transcript").unwrap())),
                    },
                ),
                sized(
                    marlin_agent_ui::UiSize::Cells(1),
                    UiElement::Template {
                        background: marlin_agent_ui::UiBackground::Bar,
                        spans: vec![UiTemplateSpan {
                            value: UiTemplateValue::Literal("🐟 MARLIN".into()),
                            tone: UiTone::Accent,
                            bold: true,
                        }],
                    },
                ),
                sized(
                    marlin_agent_ui::UiSize::Cells(1),
                    UiElement::Input {
                        bordered: false,
                        background: marlin_agent_ui::UiBackground::Canvas,
                        prefix: None,
                        title: None,
                        active: UiComponentId::new("ask").unwrap(),
                    },
                ),
                sized(
                    marlin_agent_ui::UiSize::Cells(2),
                    UiElement::List {
                        id: UiComponentId::new("ask-choices").unwrap(),
                        offset: marlin_agent_ui::UiSize::Cells(0),
                        child: Box::new(template.clone()),
                    },
                ),
            ]),
        )],
        fallback: Box::new(UiElement::Text(Vec::new())),
    };
    let mut ask_terminal = Terminal::new(TestBackend::new(120, 40)).unwrap();
    let mut ask_slots = AskFrameSlots::default();
    let mut ask_regions = Vec::with_capacity(3);
    let ask_inputs = UiRenderInputs {
        theme: &theme,
        offset: &|_| 0,
        facts: &|id| id.as_str() == "ask-active",
        extent: &|_| Extent::Cells(0),
        bindings: &[],
    };
    // Criterion may skip the benchmark body when a different filter is used.
    // Paint once independently so the fixture check is valid in either case.
    ask_slots.list.begin_frame();
    ask_terminal
        .draw(|frame| {
            render_with_slots(
                frame,
                frame.area(),
                &ask_tree,
                &mut ask_regions,
                &ask_inputs,
                &mut ask_slots,
            );
        })
        .unwrap();
    assert!(
        ask_slots.list.hit(2, 39).is_some(),
        "the fixed Ask frame must paint its second choice"
    );
    c.bench_function("ui/mapping/selected-ask-frame-with-slots", |b| {
        b.iter(|| {
            ask_regions.clear();
            ask_slots.list.begin_frame();
            ask_terminal
                .draw(|frame| {
                    render_with_slots(
                        frame,
                        frame.area(),
                        black_box(&ask_tree),
                        &mut ask_regions,
                        &ask_inputs,
                        &mut ask_slots,
                    );
                })
                .unwrap();
            black_box(ask_slots.list.hit(2, 39));
        })
    });
    for count in [8, 64] {
        let tree = UiElement::Rows(
            (0..count)
                .map(|n| UiElement::Region {
                    id: UiComponentId::new(&format!("item-{n}")).unwrap(),
                    scroll_target: Some(
                        marlin_agent_ui::UiScrollTarget::new("transcript", 3).unwrap(),
                    ),
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
        scroll_target: Some(marlin_agent_ui::UiScrollTarget::new("transcript", 3).unwrap()),
        child: Box::new(UiElement::Rows(
            (0..64)
                .map(|n| UiElement::Region {
                    id: UiComponentId::new(&format!("item-{n}")).unwrap(),
                    scroll_target: None,
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
    c.bench_function("ui/mapping/scroll-hit-layout-64", |b| {
        b.iter(|| {
            regions.clear();
            layout_regions_with_scroll(
                black_box(&viewport),
                ratatui::layout::Rect::new(0, 0, 120, 30),
                &|_| Extent::Cells(0),
                &mut regions,
                &|_| false,
                &|_| 16,
            );
            black_box(hit_test_scroll(&regions, 1, 29));
        })
    });
}

criterion_group! { name = benches; config = Criterion::default().sample_size(30).measurement_time(Duration::from_secs(3)); targets = mapping }
criterion_main!(benches);
