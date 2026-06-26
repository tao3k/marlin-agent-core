#[cfg(feature = "linked-native")]
use std::{
    alloc::{GlobalAlloc, Layout, System},
    hint::black_box,
    sync::atomic::{AtomicU64, Ordering},
};

#[cfg(feature = "linked-native")]
use marlin_agent_policy_routing_native::linked_agent_policy_routing_native_selector;
#[cfg(feature = "linked-native")]
use marlin_gerbil_scheme::{
    GerbilAgentPolicyRoutingEvidenceKind, GerbilAgentPolicyRoutingNativeEpochBacking,
    GerbilAgentPolicyRoutingNativeSelectEdgesRequest, GerbilSchemeTypeRegistry,
    gerbil_agent_policy_routing_native_request_conversion_profile,
    gerbil_agent_policy_routing_type_manifest, project_gerbil_agent_policy_routing_native_receipt,
};

#[cfg(feature = "linked-native")]
#[global_allocator]
static GLOBAL_ALLOCATOR: CountingAllocator = CountingAllocator;

#[cfg(feature = "linked-native")]
static ALLOCATION_CALLS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "linked-native")]
static DEALLOCATION_CALLS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "linked-native")]
static REALLOCATION_CALLS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "linked-native")]
static ALLOCATED_BYTES: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "linked-native")]
static DEALLOCATED_BYTES: AtomicU64 = AtomicU64::new(0);

#[cfg(feature = "linked-native")]
struct CountingAllocator;

#[cfg(feature = "linked-native")]
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            ALLOCATION_CALLS.fetch_add(1, Ordering::Relaxed);
            ALLOCATED_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        }
        ptr
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc_zeroed(layout) };
        if !ptr.is_null() {
            ALLOCATION_CALLS.fetch_add(1, Ordering::Relaxed);
            ALLOCATED_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        DEALLOCATION_CALLS.fetch_add(1, Ordering::Relaxed);
        DEALLOCATED_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        unsafe { System.dealloc(ptr, layout) };
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let next = unsafe { System.realloc(ptr, layout, new_size) };
        if !next.is_null() {
            REALLOCATION_CALLS.fetch_add(1, Ordering::Relaxed);
            ALLOCATED_BYTES.fetch_add(new_size as u64, Ordering::Relaxed);
            DEALLOCATED_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        }
        next
    }
}

#[cfg(feature = "linked-native")]
#[derive(Clone, Copy, Debug)]
struct AllocationSnapshot {
    allocations: u64,
    reallocations: u64,
    allocated_bytes: u64,
}

#[cfg(feature = "linked-native")]
impl AllocationSnapshot {
    fn current() -> Self {
        Self {
            allocations: ALLOCATION_CALLS.load(Ordering::Relaxed),
            reallocations: REALLOCATION_CALLS.load(Ordering::Relaxed),
            allocated_bytes: ALLOCATED_BYTES.load(Ordering::Relaxed),
        }
    }
}

#[cfg(feature = "linked-native")]
fn reset_allocation_counters() {
    ALLOCATION_CALLS.store(0, Ordering::Relaxed);
    DEALLOCATION_CALLS.store(0, Ordering::Relaxed);
    REALLOCATION_CALLS.store(0, Ordering::Relaxed);
    ALLOCATED_BYTES.store(0, Ordering::Relaxed);
    DEALLOCATED_BYTES.store(0, Ordering::Relaxed);
}

#[cfg(feature = "linked-native")]
fn measure_allocations<T>(operation: impl FnOnce() -> T) -> (AllocationSnapshot, T) {
    reset_allocation_counters();
    let output = operation();
    let snapshot = AllocationSnapshot::current();
    (snapshot, output)
}

#[cfg(feature = "linked-native")]
fn main() {
    let selector = linked_agent_policy_routing_native_selector();
    let receipt_registry =
        GerbilSchemeTypeRegistry::new(gerbil_agent_policy_routing_type_manifest())
            .expect("agent policy routing type registry should build");

    println!("[agent-policy-routing-native-allocation-shape]");
    println!(
        "candidates\tfull_marshal_allocs\tfull_marshal_bytes\tepoch_marshal_allocs\tepoch_marshal_bytes\tselector_allocs\tselector_bytes\tepoch_selector_allocs\tepoch_selector_bytes\treceipt_allocs\treceipt_bytes\tfull_e2e_allocs\tfull_e2e_bytes\tepoch_e2e_allocs\tepoch_e2e_bytes"
    );

    for edge_count in [1_usize, 8, 32, 128, 1024] {
        let request = route_request(edge_count);
        let epoch_backing = GerbilAgentPolicyRoutingNativeEpochBacking::from_request(&request);
        let payload = request.payload();

        let typed_value = selector
            .project_typed_value(&request)
            .expect("linked native selector should initialize and project typed value");
        let _ = selector
            .project_typed_value_with_epoch_backing(&epoch_backing, &payload)
            .expect("linked native selector should project epoch typed value");
        let _ = project_gerbil_agent_policy_routing_native_receipt(&receipt_registry, &typed_value)
            .expect("receipt projection warmup should decode");

        let (full_marshal, _) = measure_allocations(|| {
            gerbil_agent_policy_routing_native_request_conversion_profile(black_box(&request))
        });
        let (epoch_marshal, _) = measure_allocations(|| {
            epoch_backing.native_conversion_profile_for_payload(black_box(&payload))
        });
        let (selector_typed_value, _) = measure_allocations(|| {
            selector
                .project_typed_value(black_box(&request))
                .expect("linked native selector allocation receipt should project typed value")
        });
        let (epoch_selector_typed_value, _) = measure_allocations(|| {
            selector
                .project_typed_value_with_epoch_backing(
                    black_box(&epoch_backing),
                    black_box(&payload),
                )
                .expect(
                    "linked native epoch selector allocation receipt should project typed value",
                )
        });
        let (receipt_projection, _) = measure_allocations(|| {
            project_gerbil_agent_policy_routing_native_receipt(
                black_box(&receipt_registry),
                black_box(&typed_value),
            )
            .expect("receipt projection allocation receipt should decode")
        });
        let (full_e2e, _) = measure_allocations(|| {
            selector
                .project_policy_receipt(black_box(&request))
                .expect("linked native selector allocation receipt should project receipt")
        });
        let (epoch_e2e, _) = measure_allocations(|| {
            selector
                .project_policy_receipt_with_epoch_backing(
                    black_box(&epoch_backing),
                    black_box(&payload),
                )
                .expect("linked native epoch selector allocation receipt should project receipt")
        });

        println!(
            "{edge_count}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            full_marshal.allocations + full_marshal.reallocations,
            full_marshal.allocated_bytes,
            epoch_marshal.allocations + epoch_marshal.reallocations,
            epoch_marshal.allocated_bytes,
            selector_typed_value.allocations + selector_typed_value.reallocations,
            selector_typed_value.allocated_bytes,
            epoch_selector_typed_value.allocations + epoch_selector_typed_value.reallocations,
            epoch_selector_typed_value.allocated_bytes,
            receipt_projection.allocations + receipt_projection.reallocations,
            receipt_projection.allocated_bytes,
            full_e2e.allocations + full_e2e.reallocations,
            full_e2e.allocated_bytes,
            epoch_e2e.allocations + epoch_e2e.reallocations,
            epoch_e2e.allocated_bytes,
        );
    }
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

#[cfg(not(feature = "linked-native"))]
fn main() {
    eprintln!("allocation_shape bench requires --features linked-native");
}
