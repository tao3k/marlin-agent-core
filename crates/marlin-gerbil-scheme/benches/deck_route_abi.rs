use std::{slice, str, time::Duration};

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use marlin_gerbil_scheme::{
    GERBIL_DECK_ROUTE_ABI_VERSION, GerbilDeckRouteAbiModelRouteRequest,
    GerbilDeckRouteAbiModelRouteSelection, GerbilDeckRouteAbiModelRouteSelector,
    GerbilDeckRouteAbiStatus, GerbilDeckRouteAbiUtf8, GerbilDeckRuntimeModelRoutePolicy,
    GerbilDeckRuntimeModelRoutePolicyRequest,
};

fn bench_native_selector(c: &mut Criterion) {
    let selector = GerbilDeckRouteAbiModelRouteSelector::new(fake_select_model_route_fast);
    let mut group = c.benchmark_group("deck_route_abi_selector");

    for policy_count in [1_u64, 8] {
        let request = route_request(policy_count as usize);
        group.throughput(Throughput::Elements(policy_count));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{policy_count}_policy")),
            &request,
            |bencher, request| {
                bencher.iter(|| {
                    let receipt = selector
                        .evaluate(std::hint::black_box(request))
                        .expect("native selector bench should project typed selection");
                    std::hint::black_box(receipt);
                });
            },
        );
    }

    group.finish();
}

fn route_request(policy_count: usize) -> GerbilDeckRuntimeModelRoutePolicyRequest {
    let mut request = GerbilDeckRuntimeModelRoutePolicyRequest::new("cargo test", "sub-agent");

    for index in 0..policy_count {
        request = request.with_policy(
            GerbilDeckRuntimeModelRoutePolicy::new(
                format!("cheap-test-runner-{index}"),
                "openai",
                "gpt-5-mini",
            )
            .with_command_prefix("cargo test")
            .with_agent_scope("sub-agent"),
        );
    }

    request
}

unsafe extern "C" fn fake_select_model_route_fast(
    request: *const GerbilDeckRouteAbiModelRouteRequest,
    selection: *mut GerbilDeckRouteAbiModelRouteSelection,
) -> GerbilDeckRouteAbiStatus {
    if request.is_null() || selection.is_null() {
        return GerbilDeckRouteAbiStatus::new(2);
    }

    let request = unsafe { &*request };
    if request.abi_version != GERBIL_DECK_ROUTE_ABI_VERSION {
        return GerbilDeckRouteAbiStatus::new(3);
    }
    let _command = native_utf8_to_string(request.command);
    let _agent_scope = native_utf8_to_string(request.agent_scope);

    unsafe {
        *selection = GerbilDeckRouteAbiModelRouteSelection::matched(0);
    }

    GerbilDeckRouteAbiStatus::OK
}

fn native_utf8_to_string(value: GerbilDeckRouteAbiUtf8) -> String {
    if value.ptr.is_null() {
        return String::new();
    }

    let bytes = unsafe { slice::from_raw_parts(value.ptr, value.len) };
    str::from_utf8(bytes)
        .expect("bench request should contain UTF-8")
        .to_owned()
}

criterion_group! {
    name = deck_route_abi;
    config = Criterion::default()
        .sample_size(20)
        .warm_up_time(Duration::from_millis(100))
        .measurement_time(Duration::from_millis(500));
    targets = bench_native_selector
}
criterion_main!(deck_route_abi);
