// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use gerbil_scheme::{
    CollectionKind, GerbilRuntime, IdentifierBuffer, IdentifierKind, RootedSchemeObject,
    SchemeScalar,
};
use marlin_agent_ui::{
    UiComponentId, UiContent, UiIntent, UiOpenFacts, UiRegion, UiScrollDirection, UiStatusItem,
    decode_intent, decode_workbench, prepare_open_request,
};

#[path = "ui_native_projection/review.rs"]
mod review;
use review::{
    assert_review_menu_projection_requires_each_typed_decision_once,
    assert_tool_review_projection_rejects_missing_or_duplicate_choices,
};
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
                symbol(runtime, "global"),
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
            string(runtime, prompt),
        ],
    )
}
fn layout<'a>(runtime: &'a GerbilRuntime) -> RootedSchemeObject<'a> {
    // Deliberately distinct from any Scheme profile: this tests ABI decoding.
    let children: Vec<_> = [
        "transcript",
        "extensions",
        "modeline",
        "editor",
        "candidates",
        "minibuffer",
    ]
    .into_iter()
    .map(|name| {
        let region = vector(
            runtime,
            &[
                symbol(runtime, "region"),
                vector(
                    runtime,
                    &[
                        symbol(runtime, name),
                        if matches!(name, "transcript" | "candidates") {
                            vector(
                                runtime,
                                &[
                                    symbol(runtime, name),
                                    runtime
                                        .scalar_object(SchemeScalar::Fixnum(
                                            if name == "candidates" { 1 } else { 3 },
                                        ))
                                        .unwrap(),
                                ],
                            )
                        } else {
                            runtime.scalar_object(SchemeScalar::Boolean(false)).unwrap()
                        },
                        if name == "transcript" {
                            vector(
                                runtime,
                                &[
                                    symbol(runtime, "panel"),
                                    vector(
                                        runtime,
                                        &[
                                            symbol(runtime, "info"),
                                            vector(
                                                runtime,
                                                &[
                                                    symbol(runtime, "template"),
                                                    vector(
                                                        runtime,
                                                        &[
                                                            symbol(runtime, "none"),
                                                            vector(runtime, &[]),
                                                        ],
                                                    ),
                                                ],
                                            ),
                                            runtime
                                                .scalar_object(SchemeScalar::Boolean(false))
                                                .unwrap(),
                                            vector(
                                                runtime,
                                                &[
                                                    symbol(runtime, "slot"),
                                                    vector(runtime, &[symbol(runtime, "static")]),
                                                ],
                                            ),
                                        ],
                                    ),
                                ],
                            )
                        } else if name == "candidates" {
                            let template = |highlight| {
                                vector(
                                    runtime,
                                    &[
                                        symbol(runtime, "template"),
                                        vector(
                                            runtime,
                                            &[
                                                symbol(
                                                    runtime,
                                                    if highlight { "bar" } else { "none" },
                                                ),
                                                vector(runtime, &[]),
                                            ],
                                        ),
                                    ],
                                )
                            };
                            vector(
                                runtime,
                                &[
                                    symbol(runtime, "rows"),
                                    vector(runtime, &[template(true), template(false)]),
                                ],
                            )
                        } else if name == "editor" {
                            vector(
                                runtime,
                                &[
                                    symbol(runtime, "input"),
                                    vector(
                                        runtime,
                                        &[
                                            runtime
                                                .scalar_object(SchemeScalar::Boolean(true))
                                                .unwrap(),
                                            symbol(runtime, "none"),
                                            runtime
                                                .scalar_object(SchemeScalar::Boolean(false))
                                                .unwrap(),
                                            symbol(runtime, "title"),
                                            symbol(runtime, "focused"),
                                        ],
                                    ),
                                ],
                            )
                        } else {
                            vector(runtime, &[symbol(runtime, "text"), vector(runtime, &[])])
                        },
                    ],
                ),
            ],
        );
        let (kind, size) = if name == "transcript" {
            (
                "flex",
                runtime.scalar_object(SchemeScalar::Fixnum(1)).unwrap(),
            )
        } else if name == "modeline" {
            (
                "cells",
                runtime.scalar_object(SchemeScalar::Fixnum(1)).unwrap(),
            )
        } else {
            ("binding", symbol(runtime, name))
        };
        vector(
            runtime,
            &[
                symbol(runtime, "sized"),
                vector(runtime, &[symbol(runtime, kind), size, region]),
            ],
        )
    })
    .collect();
    vector(
        runtime,
        &[
            frame(runtime, 2, 0, 1, true, "input> "),
            frame(runtime, 0, 1, 0, false, "command> "),
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
                    symbol(runtime, "accent"),
                    runtime.scalar_object(SchemeScalar::Boolean(false)).unwrap(),
                ],
            ),
            vector(
                runtime,
                &[
                    symbol(runtime, "rows"),
                    vector(
                        runtime,
                        &[
                            vector(
                                runtime,
                                &[symbol(runtime, "rows"), vector(runtime, &children[..2])],
                            ),
                            vector(
                                runtime,
                                &[symbol(runtime, "rows"), vector(runtime, &children[2..])],
                            ),
                        ],
                    ),
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

#[path = "ui_native_projection/workbench.rs"]
mod workbench;
