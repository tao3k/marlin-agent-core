// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Native editor cost after UI startup; excludes VM, terminal I/O and gateway.

// The benchmark includes the full private production module but times only
// the editing paths under study.
#[allow(dead_code)]
#[path = "../src/composer.rs"]
mod composer;

use composer::Composer;
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::{hint::black_box, time::Duration};

fn draft(text: &str) -> Composer {
    let mut composer = Composer::default();
    composer.insert_text(text);
    assert_eq!(composer.text(), text);
    composer
}

fn composer_hot_path(criterion: &mut Criterion) {
    let ascii_4k = "a".repeat(4 * 1024);
    let ascii_64k = "a".repeat(64 * 1024);
    let ascii_1m = "a".repeat(1024 * 1024);
    let unicode = "中e\u{301}🐟 ".repeat(4096);
    let workloads = [
        ("ascii-4k", draft(&ascii_4k)),
        ("ascii-64k", draft(&ascii_64k)),
        ("ascii-1m", draft(&ascii_1m)),
        ("unicode", draft(&unicode)),
    ];

    let mut view = criterion.benchmark_group("composer/view-80x8");
    for (name, composer) in &workloads {
        assert!(composer.view(80, 8).is_some());
        view.bench_with_input(
            BenchmarkId::from_parameter(name),
            composer,
            |bench, input| {
                bench.iter(|| black_box(black_box(input).view(80, 8)));
            },
        );
    }
    view.finish();

    let mut movement = criterion.benchmark_group("composer/left-right");
    for (name, mut composer) in workloads
        .into_iter()
        .filter(|(name, _)| *name != "ascii-4k")
    {
        composer.left();
        assert!(!composer.cursor_at_end());
        composer.right();
        assert!(composer.cursor_at_end());
        movement.bench_function(name, |bench| {
            bench.iter(|| {
                composer.left();
                composer.right();
                black_box(composer.cursor_at_end());
            });
        });
    }
    movement.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(30)
        .measurement_time(Duration::from_secs(5));
    targets = composer_hot_path
}
criterion_main!(benches);
