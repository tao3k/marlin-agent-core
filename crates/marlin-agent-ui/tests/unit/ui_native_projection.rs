// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use gerbil_scheme::{
    CollectionKind, GerbilRuntime, IdentifierBuffer, IdentifierKind, RootedSchemeObject,
    SchemeScalar,
};
use marlin_agent_protocol::HumanDecision;
use marlin_agent_ui::{
    UiComponentId, UiContent, UiIntent, UiOpenFacts, UiRegion, UiScrollDirection, UiStatusItem,
    decode_intent, decode_review_menu, decode_workbench, prepare_open_request,
};

fn assert_review_menu_projection_requires_each_typed_decision_once(runtime: &GerbilRuntime) {
    fn menu<'a>(runtime: &'a GerbilRuntime, decisions: &[(&str, &str)]) -> RootedSchemeObject<'a> {
        let entries = decisions
            .iter()
            .map(|(decision, label)| {
                vector(
                    runtime,
                    &[
                        symbol(runtime, decision),
                        string(runtime, label),
                        string(runtime, "description"),
                    ],
                )
            })
            .collect::<Vec<_>>();
        vector(
            runtime,
            &[string(runtime, "Review: "), vector(runtime, &entries)],
        )
    }
    let valid = menu(
        runtime,
        &[
            ("patch-requested", "patch"),
            ("rejected", "deny"),
            ("approved", "approve"),
            ("needs-question", "question"),
        ],
    );
    let projected = decode_review_menu(valid).expect("typed menu");
    assert_eq!(projected.choices[0].decision, HumanDecision::PatchRequested);
    assert_eq!(projected.choices[2].decision, HumanDecision::Approved);
    assert!(
        decode_review_menu(menu(
            runtime,
            &[
                ("approved", "a"),
                ("approved", "b"),
                ("rejected", "r"),
                ("needs-question", "q"),
            ]
        ))
        .is_err()
    );
    assert!(decode_review_menu(menu(runtime, &[("approved", "a")])).is_err());
    assert!(
        decode_review_menu(menu(
            runtime,
            &[
                ("approved", "a"),
                ("rejected", "r"),
                ("needs-question", "q"),
                ("unknown", "u"),
            ]
        ))
        .is_err()
    );
}

fn vector<'a>(
    runtime: &'a GerbilRuntime,
    objects: &[RootedSchemeObject<'a>],
) -> RootedSchemeObject<'a> {
    runtime
        .collection_from_objects(CollectionKind::Vector, &objects.iter().collect::<Vec<_>>())
        .unwrap()
        .into_object()
}
fn symbol<'a>(runtime: &'a GerbilRuntime, value: &str) -> RootedSchemeObject<'a> {
    runtime
        .intern_identifier(IdentifierKind::Symbol, value)
        .unwrap()
}
fn string<'a>(runtime: &'a GerbilRuntime, value: &str) -> RootedSchemeObject<'a> {
    runtime.string_from_utf8(value).unwrap().into_object()
}
fn region<'a>(
    runtime: &'a GerbilRuntime,
    id: &str,
    kind: &str,
    data: RootedSchemeObject<'a>,
) -> RootedSchemeObject<'a> {
    vector(runtime, &[symbol(runtime, id), symbol(runtime, kind), data])
}

fn frame_state<'a>(runtime: &'a GerbilRuntime) -> RootedSchemeObject<'a> {
    vector(
        runtime,
        &[
            symbol(runtime, "main"),
            vector(
                runtime,
                &[vector(
                    runtime,
                    &[symbol(runtime, "main"), symbol(runtime, "conversation")],
                )],
            ),
            symbol(runtime, "main"),
            symbol(runtime, "minibuffer"),
            vector(
                runtime,
                &[symbol(runtime, "conversation"), symbol(runtime, "messages")],
            ),
        ],
    )
}

fn keymap_state<'a>(runtime: &'a GerbilRuntime) -> RootedSchemeObject<'a> {
    vector(
        runtime,
        &[vector(
            runtime,
            &[
                vector(runtime, &[symbol(runtime, "M-x")]),
                symbol(runtime, "open-commands"),
            ],
        )],
    )
}
fn frame<'a>(
    runtime: &'a GerbilRuntime,
    transcript: isize,
    editor: isize,
    footer: isize,
    extensions: bool,
    bordered: bool,
    prompt: &str,
) -> RootedSchemeObject<'a> {
    vector(
        runtime,
        &[
            runtime
                .scalar_object(SchemeScalar::Fixnum(transcript))
                .unwrap(),
            runtime.scalar_object(SchemeScalar::Fixnum(editor)).unwrap(),
            runtime.scalar_object(SchemeScalar::Fixnum(footer)).unwrap(),
            runtime
                .scalar_object(SchemeScalar::Boolean(extensions))
                .unwrap(),
            runtime
                .scalar_object(SchemeScalar::Boolean(bordered))
                .unwrap(),
            string(runtime, prompt),
        ],
    )
}
fn layout<'a>(runtime: &'a GerbilRuntime) -> RootedSchemeObject<'a> {
    // Deliberately distinct from any Scheme profile: this tests ABI decoding.
    vector(
        runtime,
        &[
            frame(runtime, 2, 0, 1, true, true, "input> "),
            frame(runtime, 0, 1, 0, false, false, "command> "),
            runtime.scalar_object(SchemeScalar::Fixnum(7)).unwrap(),
            runtime.scalar_object(SchemeScalar::Fixnum(4)).unwrap(),
            vector(
                runtime,
                &[
                    runtime.scalar_object(SchemeScalar::Boolean(true)).unwrap(),
                    string(runtime, "u: "),
                    string(runtime, ""),
                    string(runtime, "folded"),
                    string(runtime, "revealed"),
                ],
            ),
        ],
    )
}
fn palette<'a>(runtime: &'a GerbilRuntime) -> RootedSchemeObject<'a> {
    let colors = [
        [40, 44, 52],
        [187, 194, 207],
        [28, 31, 36],
        [101, 120, 221],
        [81, 175, 239],
        [152, 190, 101],
        [236, 190, 123],
        [115, 121, 126],
    ];
    vector(
        runtime,
        &colors.map(|channels| {
            vector(
                runtime,
                &channels.map(|value| runtime.scalar_object(SchemeScalar::Fixnum(value)).unwrap()),
            )
        }),
    )
}
#[test]
fn sdk_roots_project_to_typed_ui_workbench() {
    let runtime = GerbilRuntime::initialize().unwrap();
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
        ],
    );
    let workbench = decode_workbench(root).unwrap();
    assert_eq!(workbench.model_label, "model");
    assert_eq!(workbench.layout.max_chatbox_rows, 7);
    assert_eq!(workbench.layout.message_frame.prompt, "input> ");
    assert!(workbench.layout.message_frame.bordered);
    assert_eq!(workbench.layout.command_frame.fixed_editor_rows, 1);
    assert_eq!(workbench.layout.command_frame.footer_rows, 0);
    assert!(!workbench.layout.command_frame.bordered);
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
        workbench.key_command(&["M-x"]),
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
    assert!(decode_workbench(malformed).is_err());
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
        ],
    );
    assert!(decode_workbench(invalid_focus).is_err());
    let invalid_layout = vector(
        &runtime,
        &[
            frame(&runtime, 2, 0, 1, true, true, " Message "),
            frame(&runtime, 0, 1, 0, false, false, " M-x "),
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
