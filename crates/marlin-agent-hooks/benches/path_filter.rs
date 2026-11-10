use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use marlin_agent_hooks::{HookPathFilter, HookPathPattern, HookTargetPath};

fn compiled_path_filter(c: &mut Criterion) {
    let mut group = c.benchmark_group("hook_path_filter_warm");
    for count in [8, 128] {
        let include: Vec<_> = (0..count)
            .map(|i| HookPathPattern::new(format!("crates/module-{i}/**/*.rs")))
            .collect();
        let filter = HookPathFilter::compile(&include, &[HookPathPattern::new("**/generated/**")])
            .expect("compile benchmark path filter once");
        let mut paths: Vec<_> = (0..31)
            .map(|i| {
                HookTargetPath::new(format!("crates/module-0/generated/{i}.rs"))
                    .expect("workspace-relative benchmark path")
            })
            .collect();
        paths.push(HookTargetPath::new("crates/module-0/src/lib.rs").expect("workspace path"));
        group.throughput(Throughput::Elements(paths.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(count), &paths, |b, paths| {
            b.iter(|| black_box(filter.matches(black_box(paths))));
        });
    }
    group.finish();
}

criterion_group!(benches, compiled_path_filter);
criterion_main!(benches);
