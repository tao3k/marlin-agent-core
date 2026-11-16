// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use gerbil_scheme::{
    CollectionKind, GerbilRuntime, IdentifierKind, RootedSchemeObject, SchemeScalar,
};
use marlin_agent_tooling::decode_tooling;

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

fn declaration<'a>(
    runtime: &'a GerbilRuntime,
    name: &str,
    governance: &str,
) -> RootedSchemeObject<'a> {
    vector(
        runtime,
        &[
            symbol(runtime, name),
            runtime
                .string_from_utf8("bounded command")
                .unwrap()
                .into_object(),
            symbol(runtime, "shell"),
            symbol(runtime, governance),
            symbol(runtime, "command"),
            runtime.scalar_object(SchemeScalar::Fixnum(5)).unwrap(),
            runtime.scalar_object(SchemeScalar::Fixnum(1024)).unwrap(),
        ],
    )
}

#[test]
fn native_projection_admits_only_unique_ask_declarations() {
    let runtime = GerbilRuntime::initialize().unwrap();
    assert_eq!(
        decode_tooling(vector(
            &runtime,
            &[declaration(&runtime, "shell-one", "ask")]
        ))
        .unwrap()
        .len(),
        1
    );
    assert!(
        decode_tooling(vector(
            &runtime,
            &[declaration(&runtime, "shell-one", "allow")]
        ))
        .is_err()
    );
    assert!(
        decode_tooling(vector(
            &runtime,
            &[
                declaration(&runtime, "shell-one", "ask"),
                declaration(&runtime, "shell-one", "ask"),
            ],
        ))
        .is_err()
    );
}
