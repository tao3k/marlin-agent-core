#![cfg_attr(not(feature = "linked-native"), allow(dead_code))]

#[cfg(feature = "linked-native")]
use std::time::Duration;

#[cfg(feature = "linked-native")]
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
#[cfg(feature = "linked-native")]
use marlin_agent_protocol::{
    ModelCommandMatcher, ModelEndpoint, ModelRouteRequest, ModelRouteRule,
};
#[cfg(feature = "linked-native")]
use marlin_deck_runtime_native::{
    DeckRuntimeNativeRouteResolver, linked_deck_runtime_native_selector,
};
#[cfg(feature = "linked-native")]
use marlin_gerbil_scheme::{
    GerbilDeckRuntimeModelRoutePolicy, GerbilDeckRuntimeModelRoutePolicyRequest,
};

#[cfg(feature = "linked-native")]
fn bench_linked_native_selector(c: &mut Criterion) {
    let selector = linked_deck_runtime_native_selector();
    let mut group = c.benchmark_group("deck_runtime_linked_native_selector");

    for policy_count in [1_u64, 8, 64, 256] {
        let request = route_request(policy_count as usize);
        let warmup = selector
            .select_policy_index(&request)
            .expect("linked native selector should initialize and select before benchmarking");
        assert_eq!(warmup, Some(0));

        group.throughput(Throughput::Elements(policy_count));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{policy_count}_policy")),
            &request,
            |bencher, request| {
                bencher.iter(|| {
                    let selected = selector
                        .select_policy_index(std::hint::black_box(request))
                        .expect("linked native selector benchmark should select");
                    std::hint::black_box(selected);
                });
            },
        );
    }

    group.finish();
}

#[cfg(feature = "linked-native")]
fn bench_linked_native_resolver_compiled_candidate(c: &mut Criterion) {
    let selector = linked_deck_runtime_native_selector();
    let request = ModelRouteRequest::command(["cargo", "test", "-p", "demo"]);
    let mut group = c.benchmark_group("deck_runtime_linked_native_resolver_compiled_candidate");

    for policy_count in [1_u64, 8, 64, 256] {
        let resolver =
            DeckRuntimeNativeRouteResolver::new(selector, route_rules(policy_count as usize))
                .expect("linked native resolver should compile route rules");
        let warmup = resolver
            .resolve(&request)
            .expect("linked native resolver should resolve before benchmarking");
        assert!(warmup.is_some());

        group.throughput(Throughput::Elements(policy_count));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{policy_count}_policy")),
            &resolver,
            |bencher, resolver| {
                bencher.iter(|| {
                    let decision = resolver
                        .resolve(std::hint::black_box(&request))
                        .expect("linked native resolver benchmark should resolve");
                    std::hint::black_box(decision);
                });
            },
        );
    }

    group.finish();
}

#[cfg(feature = "linked-native")]
fn bench_high_level_request_build(c: &mut Criterion) {
    let mut group = c.benchmark_group("deck_runtime_high_level_policy_request_build");

    for policy_count in [1_u64, 8, 64, 256] {
        group.throughput(Throughput::Elements(policy_count));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{policy_count}_policy")),
            &(policy_count as usize),
            |bencher, policy_count| {
                bencher.iter(|| {
                    let request = route_request(std::hint::black_box(*policy_count));
                    std::hint::black_box(request);
                });
            },
        );
    }

    group.finish();
}

#[cfg(feature = "linked-native")]
fn route_request(policy_count: usize) -> GerbilDeckRuntimeModelRoutePolicyRequest {
    let mut request =
        GerbilDeckRuntimeModelRoutePolicyRequest::new("cargo test -p demo", "sub-agent");

    for index in 0..policy_count {
        request = request.with_policy(
            GerbilDeckRuntimeModelRoutePolicy::new(
                format!("linked-cargo-test-{index}"),
                "openai",
                "gpt-5.4-mini",
            )
            .with_command_prefix("cargo test")
            .with_agent_scope("sub-agent"),
        );
    }

    request
}

#[cfg(feature = "linked-native")]
fn route_rules(policy_count: usize) -> Vec<ModelRouteRule> {
    (0..policy_count)
        .map(|index| {
            ModelRouteRule::new(
                format!("linked-cargo-test-{index}"),
                1,
                ModelCommandMatcher::new().with_argv_glob("cargo test*"),
                ModelEndpoint::new("openai", "gpt-5.4-mini"),
            )
        })
        .collect()
}

#[cfg(feature = "linked-native")]
criterion_group! {
    name = linked_native_selector;
    config = Criterion::default()
        .sample_size(20)
        .warm_up_time(Duration::from_millis(100))
        .measurement_time(Duration::from_millis(500));
    targets =
        bench_linked_native_selector,
        bench_linked_native_resolver_compiled_candidate,
        bench_high_level_request_build
}

#[cfg(feature = "linked-native")]
criterion_main!(linked_native_selector);

#[cfg(not(feature = "linked-native"))]
fn main() {
    eprintln!("skipping linked selector benchmark; enable the linked-native feature");
}
