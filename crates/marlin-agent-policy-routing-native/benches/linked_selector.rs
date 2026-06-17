use std::time::Duration;

#[cfg(feature = "linked-native")]
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
#[cfg(feature = "linked-native")]
use marlin_agent_policy_routing_native::linked_agent_policy_routing_native_selector;
#[cfg(feature = "linked-native")]
use marlin_gerbil_scheme::{
    GerbilAgentPolicyRoutingEvidenceKind, GerbilAgentPolicyRoutingNativeSelectEdgesRequest,
};

#[cfg(feature = "linked-native")]
fn bench_linked_native_selector(c: &mut Criterion) {
    let selector = linked_agent_policy_routing_native_selector();
    let mut group = c.benchmark_group("agent_policy_routing_linked_native_selector");

    for edge_count in [1_u64, 8, 64, 256] {
        let request = route_request(edge_count as usize);
        let (_, warmup) = selector
            .project_policy_receipt(&request)
            .expect("linked native selector should initialize and project before benchmarking");
        assert_eq!(warmup.candidate_edges.len(), edge_count as usize);

        group.throughput(Throughput::Elements(edge_count));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{edge_count}_candidate_edge")),
            &request,
            |bencher, request| {
                bencher.iter(|| {
                    let projection = selector
                        .project_policy_receipt(std::hint::black_box(request))
                        .expect("linked native selector benchmark should project");
                    std::hint::black_box(projection);
                });
            },
        );
    }

    group.finish();
}

#[cfg(feature = "linked-native")]
fn route_request(edge_count: usize) -> GerbilAgentPolicyRoutingNativeSelectEdgesRequest {
    let mut request = GerbilAgentPolicyRoutingNativeSelectEdgesRequest::new(
        "agent-graph.bench",
        "gerbil.scope.agent-topology",
        "planner",
    )
    .with_evidence(
        GerbilAgentPolicyRoutingEvidenceKind::GerbilPolicyReceipt,
        "gerbil.policy.receipt.bench",
    );

    for index in 0..edge_count {
        request = request.with_candidate_edge(format!("planner-to-agent-{index}"));
    }

    request
}

#[cfg(feature = "linked-native")]
criterion_group! {
    name = linked_native_selector;
    config = Criterion::default()
        .sample_size(20)
        .warm_up_time(Duration::from_millis(100))
        .measurement_time(Duration::from_millis(500));
    targets = bench_linked_native_selector
}

#[cfg(feature = "linked-native")]
criterion_main!(linked_native_selector);

#[cfg(not(feature = "linked-native"))]
fn main() {
    eprintln!("skipping linked selector benchmark; enable the linked-native feature");
}
