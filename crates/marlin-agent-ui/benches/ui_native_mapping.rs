// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Rooted native-value projection only; excludes VM startup and terminal drawing.
use criterion::{Criterion, criterion_group, criterion_main};
use gerbil_scheme::{CollectionKind, GerbilRuntime, IdentifierKind, RootedSchemeObject};
use marlin_agent_ui::decode_element;
use std::{hint::black_box, time::Duration};

fn vector<'a>(
    runtime: &'a GerbilRuntime,
    values: &[RootedSchemeObject<'a>],
) -> RootedSchemeObject<'a> {
    runtime
        .collection_from_objects(CollectionKind::Vector, &values.iter().collect::<Vec<_>>())
        .unwrap()
        .into_object()
}
fn symbol<'a>(runtime: &'a GerbilRuntime, value: &str) -> RootedSchemeObject<'a> {
    runtime
        .intern_identifier(IdentifierKind::Symbol, value)
        .unwrap()
}
fn projection(c: &mut Criterion) {
    let runtime = GerbilRuntime::initialize().unwrap();
    for count in [8, 64] {
        let rows: Vec<_> = (0..count)
            .map(|index| {
                let span = vector(
                    &runtime,
                    &[
                        symbol(&runtime, "info"),
                        runtime
                            .string_from_utf8("message 🐟")
                            .unwrap()
                            .into_object(),
                    ],
                );
                let text = vector(
                    &runtime,
                    &[symbol(&runtime, "text"), vector(&runtime, &[span])],
                );
                vector(
                    &runtime,
                    &[
                        symbol(&runtime, "region"),
                        vector(
                            &runtime,
                            &[
                                symbol(&runtime, &format!("item-{index}")),
                                runtime
                                    .scalar_object(gerbil_scheme::SchemeScalar::Boolean(true))
                                    .unwrap(),
                                text,
                            ],
                        ),
                    ],
                )
            })
            .collect();
        let root = vector(
            &runtime,
            &[symbol(&runtime, "rows"), vector(&runtime, &rows)],
        );
        let holder = runtime
            .collection_from_objects(CollectionKind::Vector, &[&root])
            .unwrap();
        c.bench_function(&format!("ui/projection/rows-{count}"), |b| {
            b.iter(|| black_box(decode_element(holder.element(0).unwrap()).unwrap()))
        });
    }
}
criterion_group! { name = benches; config = Criterion::default().sample_size(30).measurement_time(Duration::from_secs(3)); targets = projection }
criterion_main!(benches);
