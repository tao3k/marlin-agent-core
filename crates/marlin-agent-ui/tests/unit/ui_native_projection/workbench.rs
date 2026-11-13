// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use super::{
    CollectionKind, GerbilRuntime, IdentifierBuffer, IdentifierKind, SchemeScalar, UiComponentId,
    UiContent, UiIntent, UiOpenFacts, UiRegion, UiScrollDirection, UiStatusItem,
    assert_review_menu_projection_requires_each_typed_decision_once,
    assert_tool_review_projection_rejects_missing_or_duplicate_choices, decode_intent,
    decode_workbench, frame, frame_state, keymap_state, layout, palette, prepare_open_request,
    region, string, symbol, vector,
};

#[test]
fn sdk_roots_project_to_typed_ui_workbench() {
    let runtime = GerbilRuntime::initialize().unwrap();
    assert_tool_review_projection_rejects_missing_or_duplicate_choices(&runtime);
    let element = |tone: &str, value: &str| {
        vector(
            &runtime,
            &[
                symbol(&runtime, "text"),
                vector(
                    &runtime,
                    &[vector(
                        &runtime,
                        &[symbol(&runtime, tone), string(&runtime, value)],
                    )],
                ),
            ],
        )
    };
    assert!(marlin_agent_ui::decode_element(element("info", "hello 🐟")).is_ok());
    assert!(marlin_agent_ui::decode_element(element("unknown", "hello")).is_err());
    assert!(marlin_agent_ui::decode_element(element("normal", "bad\u{1b}")).is_err());
    let prompt = |field: &str, tail: &str| {
        vector(
            &runtime,
            &[
                symbol(&runtime, "prompt"),
                vector(
                    &runtime,
                    &[
                        symbol(&runtime, field),
                        vector(
                            &runtime,
                            &[
                                symbol(&runtime, "template"),
                                vector(
                                    &runtime,
                                    &[
                                        symbol(&runtime, "none"),
                                        vector(
                                            &runtime,
                                            &[vector(
                                                &runtime,
                                                &[
                                                    symbol(&runtime, "binding"),
                                                    symbol(&runtime, "normal"),
                                                    runtime
                                                        .scalar_object(SchemeScalar::Boolean(false))
                                                        .unwrap(),
                                                    symbol(&runtime, tail),
                                                ],
                                            )],
                                        ),
                                    ],
                                ),
                            ],
                        ),
                    ],
                ),
            ],
        )
    };
    assert!(marlin_agent_ui::decode_element(prompt("query", "query")).is_ok());
    assert!(marlin_agent_ui::decode_element(prompt("query", "other")).is_err());
    let choice_case = |fact: &str| {
        vector(
            &runtime,
            &[symbol(&runtime, fact), element("normal", "chosen")],
        )
    };
    let choice = |cases: Vec<_>| {
        vector(
            &runtime,
            &[
                symbol(&runtime, "choice"),
                vector(
                    &runtime,
                    &[vector(&runtime, &cases), element("normal", "fallback")],
                ),
            ],
        )
    };
    assert!(marlin_agent_ui::decode_element(choice(vec![choice_case("ready")])).is_ok());
    assert!(
        marlin_agent_ui::decode_element(choice(vec![choice_case("ready"), choice_case("ready")]))
            .is_err()
    );
    let interactive_region = |step| {
        vector(
            &runtime,
            &[
                symbol(&runtime, "region"),
                vector(
                    &runtime,
                    &[
                        symbol(&runtime, "messages"),
                        vector(
                            &runtime,
                            &[
                                symbol(&runtime, "transcript"),
                                runtime.scalar_object(SchemeScalar::Fixnum(step)).unwrap(),
                            ],
                        ),
                        element("info", "message"),
                    ],
                ),
            ],
        )
    };
    assert!(marlin_agent_ui::decode_element(interactive_region(3)).is_ok());
    assert!(marlin_agent_ui::decode_element(interactive_region(0)).is_err());
    assert!(marlin_agent_ui::decode_element(interactive_region(33)).is_err());
    let size_case = || {
        vector(
            &runtime,
            &[
                symbol(&runtime, "command"),
                vector(
                    &runtime,
                    &[
                        symbol(&runtime, "bounded"),
                        vector(
                            &runtime,
                            &[symbol(&runtime, "requested"), symbol(&runtime, "available")],
                        ),
                    ],
                ),
            ],
        )
    };
    let size_choice = |cases: Vec<_>| {
        vector(
            &runtime,
            &[
                symbol(&runtime, "sized"),
                vector(
                    &runtime,
                    &[
                        symbol(&runtime, "choice"),
                        vector(
                            &runtime,
                            &[
                                vector(&runtime, &cases),
                                vector(
                                    &runtime,
                                    &[
                                        symbol(&runtime, "cells"),
                                        runtime.scalar_object(SchemeScalar::Fixnum(0)).unwrap(),
                                    ],
                                ),
                            ],
                        ),
                        element("normal", "candidate"),
                    ],
                ),
            ],
        )
    };
    assert!(marlin_agent_ui::decode_element(size_choice(vec![size_case()])).is_ok());
    assert!(marlin_agent_ui::decode_element(size_choice(vec![size_case(), size_case()])).is_err());
    let cell = || {
        vector(
            &runtime,
            &[
                symbol(&runtime, "cells"),
                runtime.scalar_object(SchemeScalar::Fixnum(1)).unwrap(),
            ],
        )
    };
    let list = |offset| {
        vector(
            &runtime,
            &[
                symbol(&runtime, "list"),
                vector(
                    &runtime,
                    &[
                        symbol(&runtime, "commands"),
                        offset,
                        element("normal", "row"),
                    ],
                ),
            ],
        )
    };
    assert!(matches!(
        marlin_agent_ui::decode_element(list(cell())),
        Ok(marlin_agent_ui::UiElement::List { .. })
    ));
    let invalid_offset = vector(
        &runtime,
        &[
            symbol(&runtime, "cells"),
            runtime.scalar_object(SchemeScalar::Fixnum(-1)).unwrap(),
        ],
    );
    assert!(marlin_agent_ui::decode_element(list(invalid_offset)).is_err());
    let mut nested = cell();
    for _ in 0..13 {
        nested = vector(
            &runtime,
            &[symbol(&runtime, "add"), vector(&runtime, &[nested, cell()])],
        );
    }
    let too_deep = vector(
        &runtime,
        &[
            symbol(&runtime, "sized"),
            vector(
                &runtime,
                &[
                    symbol(&runtime, "add"),
                    vector(&runtime, &[nested, cell()]),
                    element("normal", "deep"),
                ],
            ),
        ],
    );
    assert!(marlin_agent_ui::decode_element(too_deep).is_err());
    let duplicate_regions = vector(
        &runtime,
        &[
            symbol(&runtime, "rows"),
            vector(&runtime, &[interactive_region(3), interactive_region(3)]),
        ],
    );
    assert!(marlin_agent_ui::decode_element(duplicate_regions).is_err());
    let part = vector(
        &runtime,
        &[
            symbol(&runtime, "binding"),
            symbol(&runtime, "info"),
            runtime.scalar_object(SchemeScalar::Boolean(true)).unwrap(),
            symbol(&runtime, "name"),
        ],
    );
    let column = vector(
        &runtime,
        &[
            symbol(&runtime, "column"),
            symbol(&runtime, "info"),
            runtime.scalar_object(SchemeScalar::Boolean(false)).unwrap(),
            symbol(&runtime, "name"),
            vector(
                &runtime,
                &[
                    symbol(&runtime, "cells"),
                    runtime.scalar_object(SchemeScalar::Fixnum(5)).unwrap(),
                ],
            ),
        ],
    );
    let column_template = vector(
        &runtime,
        &[
            symbol(&runtime, "template"),
            vector(
                &runtime,
                &[symbol(&runtime, "none"), vector(&runtime, &[column])],
            ),
        ],
    );
    assert!(
        matches!(marlin_agent_ui::decode_element(column_template).unwrap(),
        marlin_agent_ui::UiElement::Template { spans, .. }
            if matches!(&spans[0].value, marlin_agent_ui::UiTemplateValue::Column { .. }))
    );
    let template = vector(
        &runtime,
        &[
            symbol(&runtime, "template"),
            vector(
                &runtime,
                &[symbol(&runtime, "bar"), vector(&runtime, &[part])],
            ),
        ],
    );
    let input = |prefix| {
        vector(
            &runtime,
            &[
                symbol(&runtime, "input"),
                vector(
                    &runtime,
                    &[
                        runtime.scalar_object(SchemeScalar::Boolean(false)).unwrap(),
                        symbol(&runtime, "bar"),
                        prefix,
                        runtime.scalar_object(SchemeScalar::Boolean(false)).unwrap(),
                        symbol(&runtime, "focused"),
                    ],
                ),
            ],
        )
    };
    assert!(marlin_agent_ui::decode_element(input(element("normal", "not a template"))).is_err());
    let invalid_panel = vector(
        &runtime,
        &[
            symbol(&runtime, "panel"),
            vector(
                &runtime,
                &[
                    symbol(&runtime, "info"),
                    vector(
                        &runtime,
                        &[
                            symbol(&runtime, "template"),
                            vector(&runtime, &[symbol(&runtime, "none"), vector(&runtime, &[])]),
                        ],
                    ),
                    element("normal", "not a title template"),
                    element("normal", "content"),
                ],
            ),
        ],
    );
    assert!(marlin_agent_ui::decode_element(invalid_panel).is_err());
    assert!(matches!(
        marlin_agent_ui::decode_element(input(template)).unwrap(),
        marlin_agent_ui::UiElement::Input { prefix: Some(prefix), .. }
            if matches!(*prefix, marlin_agent_ui::UiElement::Template {
                background: marlin_agent_ui::UiBackground::Bar, ..
            })
    ));
    assert_review_menu_projection_requires_each_typed_decision_once(&runtime);
    let scope = runtime.void_object().unwrap();
    let status = [
        UiStatusItem::host(UiComponentId::new("extension/status").unwrap(), "Ready 🐟"),
        UiStatusItem::host(
            UiComponentId::new("runtime/process-budget").unwrap(),
            "Managed process budget enabled",
        ),
    ];
    let facts = UiOpenFacts {
        model_label: "model",
        session_label: "session",
        status: &status,
        extensions: &[],
        method_bundles: &[],
    };
    let request = prepare_open_request(&runtime, &scope, &facts)
        .unwrap()
        .into_collection(CollectionKind::Vector)
        .unwrap();
    assert_eq!(request.len(), 3);
    assert!(
        request
            .element(0)
            .unwrap()
            .as_ref()
            .is_identical(scope.as_ref())
    );
    let projected_facts = request
        .element(1)
        .unwrap()
        .into_collection(CollectionKind::Vector)
        .unwrap();
    assert_eq!(
        projected_facts
            .element(0)
            .unwrap()
            .into_string()
            .unwrap()
            .to_string()
            .into_result()
            .unwrap(),
        "model"
    );
    let rows = projected_facts
        .element(2)
        .unwrap()
        .into_collection(CollectionKind::Vector)
        .unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(
        rows.element(0)
            .unwrap()
            .into_collection(CollectionKind::Vector)
            .unwrap()
            .element(1)
            .unwrap()
            .into_string()
            .unwrap()
            .to_string()
            .into_result()
            .unwrap(),
        "Ready 🐟"
    );
    let row = rows
        .element(1)
        .unwrap()
        .into_collection(CollectionKind::Vector)
        .unwrap();
    let mut names = IdentifierBuffer::default();
    row.element(0)
        .unwrap()
        .as_ref()
        .copy_identifier_names(&mut names)
        .unwrap();
    assert_eq!(
        names.get(0),
        Some((IdentifierKind::Symbol, "runtime/process-budget"))
    );
    assert_eq!(
        row.element(1)
            .unwrap()
            .into_string()
            .unwrap()
            .to_string()
            .into_result()
            .unwrap(),
        "Managed process budget enabled"
    );
    assert!(UiComponentId::new("").is_err());
    let duplicate_status = [status[0].clone(), status[0].clone()];
    let duplicate_facts = UiOpenFacts {
        status: &duplicate_status,
        ..facts
    };
    assert!(prepare_open_request(&runtime, &scope, &duplicate_facts).is_err());
    let extensions = [runtime.scalar_object(SchemeScalar::Boolean(true)).unwrap()];
    let bundles = [runtime.nil_object().unwrap()];
    let with_contributions = UiOpenFacts {
        model_label: "model",
        session_label: "session",
        status: &status,
        extensions: &extensions,
        method_bundles: &bundles,
    };
    let request = prepare_open_request(&runtime, &scope, &with_contributions)
        .unwrap()
        .into_collection(CollectionKind::Vector)
        .unwrap();
    let projected_facts = request
        .element(1)
        .unwrap()
        .into_collection(CollectionKind::Vector)
        .unwrap();
    let projected_extensions = projected_facts
        .element(3)
        .unwrap()
        .into_collection(CollectionKind::Vector)
        .unwrap();
    let projected_bundles = request
        .element(2)
        .unwrap()
        .into_collection(CollectionKind::Vector)
        .unwrap();
    assert!(
        projected_extensions
            .element(0)
            .unwrap()
            .as_ref()
            .is_identical(extensions[0].as_ref())
    );
    assert!(
        projected_bundles
            .element(0)
            .unwrap()
            .as_ref()
            .is_identical(bundles[0].as_ref())
    );
    let empty = || vector(&runtime, &[]);
    let status_item = vector(
        &runtime,
        &[
            symbol(&runtime, "example/status"),
            symbol(&runtime, "status-item"),
            string(&runtime, "Ready 🐟"),
            string(&runtime, "Ready 🐟"),
            symbol(&runtime, "muted"),
            symbol(&runtime, "muted"),
            runtime.scalar_object(SchemeScalar::Fixnum(0)).unwrap(),
            runtime.scalar_object(SchemeScalar::Fixnum(0)).unwrap(),
            runtime.scalar_object(SchemeScalar::Fixnum(0)).unwrap(),
        ],
    );
    let focus = runtime.scalar_object(SchemeScalar::Boolean(true)).unwrap();
    let content = vector(
        &runtime,
        &[
            vector(
                &runtime,
                &[symbol(&runtime, "text"), string(&runtime, "Preview")],
            ),
            vector(
                &runtime,
                &[
                    symbol(&runtime, "field"),
                    string(&runtime, "Mode"),
                    string(&runtime, "Review"),
                ],
            ),
        ],
    );
    let extension = vector(&runtime, &[string(&runtime, "Workspace"), focus, content]);
    let regions = vector(
        &runtime,
        &[
            region(&runtime, "transcript", "transcript", empty()),
            region(&runtime, "composer", "composer", empty()),
            region(
                &runtime,
                "commands",
                "commands",
                vector(
                    &runtime,
                    &[vector(
                        &runtime,
                        &[
                            string(&runtime, "help"),
                            string(&runtime, "Help"),
                            symbol(&runtime, "none"),
                            symbol(&runtime, "clear"),
                            runtime.scalar_object(SchemeScalar::Boolean(false)).unwrap(),
                        ],
                    )],
                ),
            ),
            region(&runtime, "extension/example", "extension", extension),
            region(
                &runtime,
                "status",
                "status",
                vector(&runtime, &[status_item]),
            ),
        ],
    );
    let root = vector(
        &runtime,
        &[
            string(&runtime, "model"),
            string(&runtime, "session"),
            regions,
            symbol(&runtime, "composer"),
            layout(&runtime),
            palette(&runtime),
            frame_state(&runtime),
            keymap_state(&runtime),
            vector(&runtime, &[]),
        ],
    );
    let workbench = decode_workbench(root).unwrap();
    assert!(
        matches!(workbench.layout.frame_tree.region_child("transcript"), Some(marlin_agent_ui::UiElement::Panel { border_tone: marlin_agent_ui::UiTone::Info, child, .. }) if matches!(child.as_ref(), marlin_agent_ui::UiElement::Slot(id) if id.as_str() == "static"))
    );
    assert_eq!(
        workbench.layout.transcript_policy.role_tone,
        marlin_agent_ui::UiTone::Accent
    );
    assert!(!workbench.layout.transcript_policy.role_bold);
    assert!(matches!(
        workbench.layout.frame_tree.region_child("editor"),
        Some(marlin_agent_ui::UiElement::Input {
            bordered: true,
            title: Some(_),
            prefix: None,
            ..
        })
    ));
    let marlin_agent_ui::UiElement::Rows(groups) = &workbench.layout.frame_tree else {
        panic!("frame groups")
    };
    let marlin_agent_ui::UiElement::Rows(slots) = &groups[0] else {
        panic!("frame slots")
    };
    assert!(matches!(
        &slots[0],
        marlin_agent_ui::UiElement::Sized {
            size: marlin_agent_ui::UiSize::Flex(1),
            ..
        }
    ));
    assert!(
        matches!(&slots[1], marlin_agent_ui::UiElement::Sized { size: marlin_agent_ui::UiSize::Binding(name), .. } if name.as_str() == "extensions")
    );
    let marlin_agent_ui::UiElement::Rows(slots) = &groups[1] else {
        panic!("frame slots")
    };
    assert!(matches!(
        &slots[0],
        marlin_agent_ui::UiElement::Sized {
            size: marlin_agent_ui::UiSize::Cells(1),
            ..
        }
    ));
    assert_eq!(workbench.model_label, "model");
    assert_eq!(workbench.layout.max_chatbox_rows, 7);
    assert_eq!(workbench.layout.message_frame.prompt, "input> ");
    assert_eq!(workbench.layout.command_frame.fixed_editor_rows, 1);
    assert_eq!(workbench.layout.command_frame.footer_rows, 0);
    assert_eq!(
        (
            workbench
                .layout
                .transcript_policy
                .thinking_collapsed_role
                .as_str(),
            workbench
                .layout
                .transcript_policy
                .thinking_expanded_role
                .as_str(),
        ),
        ("folded", "revealed")
    );
    assert_eq!(workbench.theme.accent.red, 101);
    assert_eq!(workbench.focus.as_str(), "composer");
    assert_eq!(
        workbench.frame.selected_buffer().unwrap().as_str(),
        "conversation"
    );
    assert_eq!(
        workbench.key_command(&["M-x"], None),
        Some(marlin_agent_ui::UiKeyCommand::OpenCommands)
    );
    assert_eq!(workbench.commands()[0].name.as_str(), "help");
    assert!(!workbench.commands()[0].accepts_arguments());
    assert!(
        matches!(&workbench.regions[3], UiRegion::Extension { id, focusable: true, .. } if id.as_str() == "extension/example")
    );
    assert!(matches!(
        &workbench.regions[3],
        UiRegion::Extension { content, .. }
            if content == &vec![
                UiContent::Text("Preview".into()),
                UiContent::Field { label: "Mode".into(), value: "Review".into() }
            ]
    ));
    assert!(
        matches!(&workbench.regions[4], UiRegion::Status(items) if items[0].label == "Ready 🐟")
    );
    let malformed = vector(&runtime, &[string(&runtime, "bad")]);
    let error = decode_workbench(malformed).unwrap_err();
    assert!(error.to_string().contains("UI projection workbench:"));
    let invalid_focus: [_; 9] = std::array::from_fn(|_| string(&runtime, "bad"));
    let invalid_focus = vector(&runtime, &invalid_focus);
    let error = decode_workbench(invalid_focus).unwrap_err();
    assert!(error.to_string().contains("UI projection focus:"));
    let invalid_tone = vector(
        &runtime,
        &[
            symbol(&runtime, "extension/status"),
            symbol(&runtime, "status-item"),
            string(&runtime, "Ready"),
            string(&runtime, "Ready"),
            symbol(&runtime, "effect"),
            symbol(&runtime, "muted"),
            runtime.scalar_object(SchemeScalar::Fixnum(0)).unwrap(),
            runtime.scalar_object(SchemeScalar::Fixnum(0)).unwrap(),
            runtime.scalar_object(SchemeScalar::Fixnum(0)).unwrap(),
        ],
    );
    let invalid_tone_regions = vector(
        &runtime,
        &[
            region(&runtime, "transcript", "transcript", empty()),
            region(&runtime, "composer", "composer", empty()),
            region(&runtime, "commands", "commands", empty()),
            region(
                &runtime,
                "status",
                "status",
                vector(&runtime, &[invalid_tone]),
            ),
        ],
    );
    let invalid_tone_root = vector(
        &runtime,
        &[
            string(&runtime, "model"),
            string(&runtime, "session"),
            invalid_tone_regions,
            symbol(&runtime, "composer"),
            layout(&runtime),
            palette(&runtime),
            frame_state(&runtime),
            keymap_state(&runtime),
            vector(&runtime, &[]),
        ],
    );
    assert!(decode_workbench(invalid_tone_root).is_err());
    for (minimum, compact, priority) in [
        (-1_isize, 0, 0),
        (257, 0, 0),
        (1, 0, 0),
        (0, 1, 0),
        (0, 0, 4),
        (0, 0, 1),
    ] {
        let invalid_width = vector(
            &runtime,
            &[
                symbol(&runtime, "example/status"),
                symbol(&runtime, "status-item"),
                string(&runtime, "Ready"),
                string(&runtime, "Ready"),
                symbol(&runtime, "muted"),
                symbol(&runtime, "muted"),
                runtime
                    .scalar_object(SchemeScalar::Fixnum(minimum))
                    .unwrap(),
                runtime
                    .scalar_object(SchemeScalar::Fixnum(compact))
                    .unwrap(),
                runtime
                    .scalar_object(SchemeScalar::Fixnum(priority))
                    .unwrap(),
            ],
        );
        let regions = vector(
            &runtime,
            &[
                region(&runtime, "transcript", "transcript", empty()),
                region(&runtime, "composer", "composer", empty()),
                region(&runtime, "commands", "commands", empty()),
                region(
                    &runtime,
                    "status",
                    "status",
                    vector(&runtime, &[invalid_width]),
                ),
            ],
        );
        let workbench = vector(
            &runtime,
            &[
                string(&runtime, "model"),
                string(&runtime, "session"),
                regions,
                symbol(&runtime, "composer"),
                layout(&runtime),
                palette(&runtime),
                frame_state(&runtime),
                keymap_state(&runtime),
                vector(&runtime, &[]),
            ],
        );
        assert!(
            decode_workbench(workbench).is_err(),
            "presentation {minimum}/{compact}/{priority}"
        );
    }
    let duplicate = vector(
        &runtime,
        &[
            region(&runtime, "transcript", "transcript", empty()),
            region(&runtime, "composer", "composer", empty()),
            region(&runtime, "commands", "commands", empty()),
            region(
                &runtime,
                "composer",
                "extension",
                vector(
                    &runtime,
                    &[
                        string(&runtime, "duplicate"),
                        runtime.scalar_object(SchemeScalar::Boolean(true)).unwrap(),
                        empty(),
                    ],
                ),
            ),
            region(&runtime, "status", "status", empty()),
        ],
    );
    let duplicate_root = vector(
        &runtime,
        &[
            string(&runtime, "model"),
            string(&runtime, "session"),
            duplicate,
            symbol(&runtime, "composer"),
            layout(&runtime),
            palette(&runtime),
            frame_state(&runtime),
            keymap_state(&runtime),
            vector(&runtime, &[]),
        ],
    );
    assert!(decode_workbench(duplicate_root).is_err());
    let forged_content = vector(
        &runtime,
        &[vector(
            &runtime,
            &[symbol(&runtime, "effect"), string(&runtime, "run")],
        )],
    );
    let forged_regions = vector(
        &runtime,
        &[
            region(&runtime, "transcript", "transcript", empty()),
            region(&runtime, "composer", "composer", empty()),
            region(&runtime, "commands", "commands", empty()),
            region(
                &runtime,
                "extension/forged",
                "extension",
                vector(
                    &runtime,
                    &[
                        string(&runtime, "Forged"),
                        runtime.scalar_object(SchemeScalar::Boolean(true)).unwrap(),
                        forged_content,
                    ],
                ),
            ),
            region(&runtime, "status", "status", empty()),
        ],
    );
    let forged_root = vector(
        &runtime,
        &[
            string(&runtime, "model"),
            string(&runtime, "session"),
            forged_regions,
            symbol(&runtime, "composer"),
            layout(&runtime),
            palette(&runtime),
            frame_state(&runtime),
            keymap_state(&runtime),
            vector(&runtime, &[]),
        ],
    );
    assert!(decode_workbench(forged_root).is_err());
    let regions = vector(
        &runtime,
        &[
            region(&runtime, "transcript", "transcript", empty()),
            region(&runtime, "composer", "composer", empty()),
            region(&runtime, "commands", "commands", empty()),
            region(&runtime, "status", "status", empty()),
        ],
    );
    let invalid_focus = vector(
        &runtime,
        &[
            string(&runtime, "model"),
            string(&runtime, "session"),
            regions,
            symbol(&runtime, "status"),
            layout(&runtime),
            palette(&runtime),
            frame_state(&runtime),
            keymap_state(&runtime),
            vector(&runtime, &[]),
        ],
    );
    assert!(decode_workbench(invalid_focus).is_err());
    let invalid_layout = vector(
        &runtime,
        &[
            frame(&runtime, 2, 0, 1, true, " Message "),
            frame(&runtime, 0, 1, 0, false, " M-x "),
            runtime.scalar_object(SchemeScalar::Fixnum(1000)).unwrap(),
            runtime.scalar_object(SchemeScalar::Fixnum(4)).unwrap(),
        ],
    );
    let invalid_policy_root = vector(
        &runtime,
        &[
            string(&runtime, "model"),
            string(&runtime, "session"),
            vector(
                &runtime,
                &[
                    region(&runtime, "transcript", "transcript", empty()),
                    region(&runtime, "composer", "composer", empty()),
                    region(&runtime, "commands", "commands", empty()),
                    region(&runtime, "status", "status", empty()),
                ],
            ),
            symbol(&runtime, "composer"),
            invalid_layout,
            palette(&runtime),
            frame_state(&runtime),
            keymap_state(&runtime),
            vector(&runtime, &[]),
        ],
    );
    assert!(decode_workbench(invalid_policy_root).is_err());
    let scroll = vector(
        &runtime,
        &[symbol(&runtime, "scroll"), symbol(&runtime, "down")],
    );
    assert_eq!(
        decode_intent(scroll).unwrap(),
        UiIntent::Scroll(UiScrollDirection::Down)
    );
    let unknown = vector(&runtime, &[symbol(&runtime, "unknown")]);
    assert!(decode_intent(unknown).is_err());
    println!("MARLIN-UI-SDK-PROJECTION-OK");
}
