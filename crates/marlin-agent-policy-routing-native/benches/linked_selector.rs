#[cfg(feature = "linked-native")]
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
#[cfg(feature = "linked-native")]
use marlin_agent_policy_routing_native::linked_agent_policy_routing_native_selector;
#[cfg(feature = "linked-native")]
use marlin_gerbil_scheme::{
    GERBIL_AGENT_POLICY_ROUTING_TYPE_ID, GerbilAgentPolicyRoutingEvidenceKind,
    GerbilAgentPolicyRoutingNativeEpochBacking, GerbilAgentPolicyRoutingNativeSelectEdgesRequest,
    GerbilSchemeTypeRegistry, gerbil_agent_policy_routing_native_request_conversion_profile,
    gerbil_agent_policy_routing_type_manifest, project_gerbil_agent_policy_routing_native_receipt,
};
#[cfg(feature = "linked-native")]
use std::time::Duration;

#[cfg(feature = "linked-native")]
fn bench_linked_native_selector(c: &mut Criterion) {
    let selector = linked_agent_policy_routing_native_selector();
    let mut marshalling_group =
        c.benchmark_group("agent_policy_routing_native_request_marshalling");

    for edge_count in [1_u64, 8, 32, 128, 1024] {
        let request = route_request(edge_count as usize);
        let profile = gerbil_agent_policy_routing_native_request_conversion_profile(&request);
        assert_eq!(profile.candidate_edge_count, edge_count as usize);

        marshalling_group.throughput(Throughput::Elements(edge_count));
        marshalling_group.bench_with_input(
            BenchmarkId::from_parameter(format!("{edge_count}_candidate_edge")),
            &request,
            |bencher, request| {
                bencher.iter(|| {
                    let profile = gerbil_agent_policy_routing_native_request_conversion_profile(
                        std::hint::black_box(request),
                    );
                    std::hint::black_box(profile);
                });
            },
        );
    }
    marshalling_group.finish();

    let mut epoch_marshalling_group =
        c.benchmark_group("agent_policy_routing_native_epoch_payload_marshalling");

    for edge_count in [1_u64, 8, 32, 128, 1024] {
        let request = route_request(edge_count as usize);
        let epoch_backing = GerbilAgentPolicyRoutingNativeEpochBacking::from_request(&request);
        let payload = request.payload();
        let profile = epoch_backing.native_conversion_profile_for_payload(&payload);
        assert_eq!(profile.candidate_edge_count, edge_count as usize);
        assert_eq!(profile.reused_epoch_scalar_string_count, 3);

        epoch_marshalling_group.throughput(Throughput::Elements(edge_count));
        epoch_marshalling_group.bench_with_input(
            BenchmarkId::from_parameter(format!("{edge_count}_candidate_edge")),
            &(epoch_backing, payload),
            |bencher, (epoch_backing, payload)| {
                bencher.iter(|| {
                    let profile = std::hint::black_box(epoch_backing)
                        .native_conversion_profile_for_payload(std::hint::black_box(payload));
                    std::hint::black_box(profile);
                });
            },
        );
    }
    epoch_marshalling_group.finish();

    let mut selector_typed_value_group =
        c.benchmark_group("agent_policy_routing_linked_native_selector_typed_value");

    for edge_count in [1_u64, 8, 32, 128, 1024] {
        let request = route_request(edge_count as usize);
        let warmup = selector
            .project_typed_value(&request)
            .expect("linked native selector should initialize and project typed value");
        assert_eq!(
            warmup.type_id().as_str(),
            GERBIL_AGENT_POLICY_ROUTING_TYPE_ID
        );

        selector_typed_value_group.throughput(Throughput::Elements(edge_count));
        selector_typed_value_group.bench_with_input(
            BenchmarkId::from_parameter(format!("{edge_count}_candidate_edge")),
            &request,
            |bencher, request| {
                bencher.iter(|| {
                    let typed_value = selector
                        .project_typed_value(std::hint::black_box(request))
                        .expect("linked native selector benchmark should project typed value");
                    std::hint::black_box(typed_value);
                });
            },
        );
    }

    selector_typed_value_group.finish();

    let mut epoch_selector_typed_value_group =
        c.benchmark_group("agent_policy_routing_linked_native_selector_epoch_typed_value");

    for edge_count in [1_u64, 8, 32, 128, 1024] {
        let request = route_request(edge_count as usize);
        let epoch_backing = GerbilAgentPolicyRoutingNativeEpochBacking::from_request(&request);
        let payload = request.payload();
        let warmup = selector
            .project_typed_value_with_epoch_backing(&epoch_backing, &payload)
            .expect("linked native selector should initialize and project typed value");
        assert_eq!(
            warmup.type_id().as_str(),
            GERBIL_AGENT_POLICY_ROUTING_TYPE_ID
        );

        epoch_selector_typed_value_group.throughput(Throughput::Elements(edge_count));
        epoch_selector_typed_value_group.bench_with_input(
            BenchmarkId::from_parameter(format!("{edge_count}_candidate_edge")),
            &(epoch_backing, payload),
            |bencher, (epoch_backing, payload)| {
                bencher.iter(|| {
                    let typed_value = selector
                        .project_typed_value_with_epoch_backing(
                            std::hint::black_box(epoch_backing),
                            std::hint::black_box(payload),
                        )
                        .expect(
                            "linked native epoch selector benchmark should project typed value",
                        );
                    std::hint::black_box(typed_value);
                });
            },
        );
    }

    epoch_selector_typed_value_group.finish();

    let receipt_registry =
        GerbilSchemeTypeRegistry::new(gerbil_agent_policy_routing_type_manifest())
            .expect("agent policy routing type registry should build");
    let mut receipt_projection_group =
        c.benchmark_group("agent_policy_routing_native_receipt_projection");

    for edge_count in [1_u64, 8, 32, 128, 1024] {
        let request = route_request(edge_count as usize);
        let typed_value = selector
            .project_typed_value(&request)
            .expect("linked native selector should produce receipt projection input");
        let (_, warmup) =
            project_gerbil_agent_policy_routing_native_receipt(&receipt_registry, &typed_value)
                .expect("receipt projection warmup should decode");
        assert_eq!(warmup.candidate_edges.len(), edge_count as usize);

        receipt_projection_group.throughput(Throughput::Elements(edge_count));
        receipt_projection_group.bench_with_input(
            BenchmarkId::from_parameter(format!("{edge_count}_candidate_edge")),
            &typed_value,
            |bencher, typed_value| {
                bencher.iter(|| {
                    let projection = project_gerbil_agent_policy_routing_native_receipt(
                        std::hint::black_box(&receipt_registry),
                        std::hint::black_box(typed_value),
                    )
                    .expect("receipt projection benchmark should decode");
                    std::hint::black_box(projection);
                });
            },
        );
    }

    receipt_projection_group.finish();

    let mut selector_group =
        c.benchmark_group("agent_policy_routing_linked_native_selector_end_to_end");

    for edge_count in [1_u64, 8, 32, 128, 1024] {
        let request = route_request(edge_count as usize);
        let (_, warmup) = selector
            .project_policy_receipt(&request)
            .expect("linked native selector should initialize and project before benchmarking");
        assert_eq!(warmup.candidate_edges.len(), edge_count as usize);

        selector_group.throughput(Throughput::Elements(edge_count));
        selector_group.bench_with_input(
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

    selector_group.finish();

    let mut epoch_selector_group =
        c.benchmark_group("agent_policy_routing_linked_native_selector_epoch_end_to_end");

    for edge_count in [1_u64, 8, 32, 128, 1024] {
        let request = route_request(edge_count as usize);
        let epoch_backing = GerbilAgentPolicyRoutingNativeEpochBacking::from_request(&request);
        let payload = request.payload();
        let (_, warmup) = selector
            .project_policy_receipt_with_epoch_backing(&epoch_backing, &payload)
            .expect("linked native selector should initialize and project before benchmarking");
        assert_eq!(warmup.candidate_edges.len(), edge_count as usize);

        epoch_selector_group.throughput(Throughput::Elements(edge_count));
        epoch_selector_group.bench_with_input(
            BenchmarkId::from_parameter(format!("{edge_count}_candidate_edge")),
            &(epoch_backing, payload),
            |bencher, (epoch_backing, payload)| {
                bencher.iter(|| {
                    let projection = selector
                        .project_policy_receipt_with_epoch_backing(
                            std::hint::black_box(epoch_backing),
                            std::hint::black_box(payload),
                        )
                        .expect("linked native epoch selector benchmark should project");
                    std::hint::black_box(projection);
                });
            },
        );
    }

    epoch_selector_group.finish();
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
