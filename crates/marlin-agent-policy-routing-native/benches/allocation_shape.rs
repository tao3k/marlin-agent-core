#[cfg(feature = "linked-native")]
use std::{
    alloc::{GlobalAlloc, Layout, System},
    env, fs,
    hint::black_box,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
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
use marlin_rust_project_harness_policy::{
    RustScenarioBenchmarkStatus, render_rust_scenario_benchmark_snapshot,
    validate_rust_scenario_benchmark,
};
#[cfg(feature = "linked-native")]
use tempfile::TempDir;

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

    fn allocation_events(self) -> u64 {
        self.allocations + self.reallocations
    }
}

#[cfg(feature = "linked-native")]
#[derive(Clone, Copy, Debug)]
struct AllocationRow {
    edge_count: usize,
    full_marshal: AllocationSnapshot,
    epoch_marshal: AllocationSnapshot,
    selector_typed_value: AllocationSnapshot,
    epoch_selector_typed_value: AllocationSnapshot,
    receipt_projection: AllocationSnapshot,
    full_e2e: AllocationSnapshot,
    epoch_e2e: AllocationSnapshot,
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

    let started_at = Instant::now();
    let mut rows = Vec::new();
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

        rows.push(AllocationRow {
            edge_count,
            full_marshal,
            epoch_marshal,
            selector_typed_value,
            epoch_selector_typed_value,
            receipt_projection,
            full_e2e,
            epoch_e2e,
        });
    }

    let observed_total_ms = elapsed_ms(started_at);
    assert_allocation_performance_gate(&rows);

    let scenario_root = observed_scenario_benchmark_root(&rows, observed_total_ms);
    let receipt = validate_rust_scenario_benchmark(scenario_root.path())
        .expect("linked native allocation shape scenario benchmark should validate");
    let snapshot = render_rust_scenario_benchmark_snapshot(&receipt);
    let snapshot_path = write_scenario_benchmark_snapshot(&snapshot);
    println!("{snapshot}");
    assert_eq!(
        receipt.status,
        RustScenarioBenchmarkStatus::Pass,
        "{receipt:#?}"
    );
    println!(
        "agent-policy-routing-native scenario allocation benchmark passed snapshot={}",
        snapshot_path.display()
    );
}

#[cfg(feature = "linked-native")]
fn write_scenario_benchmark_snapshot(snapshot: &str) -> PathBuf {
    let path = target_scenario_snapshot_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("scenario benchmark snapshot dir should be created");
    }
    fs::write(&path, snapshot).expect("scenario benchmark snapshot should be written");
    path
}

#[cfg(feature = "linked-native")]
fn target_scenario_snapshot_path() -> PathBuf {
    let target_dir = env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(default_workspace_target_dir);
    target_dir
        .join("marlin")
        .join("scenarios")
        .join("agent-policy-routing-native-allocation-shape.snap")
}

#[cfg(feature = "linked-native")]
fn default_workspace_target_dir() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(Path::parent)
        .expect("crate manifest should live under the workspace crates directory");
    workspace_root.join("target")
}

#[cfg(feature = "linked-native")]
fn elapsed_ms(started_at: Instant) -> u64 {
    u64::try_from(started_at.elapsed().as_millis())
        .unwrap_or(u64::MAX)
        .max(1)
}

#[cfg(feature = "linked-native")]
fn observed_scenario_benchmark_root(rows: &[AllocationRow], observed_total_ms: u64) -> TempDir {
    let temp_dir = TempDir::new().expect("scenario benchmark temp dir should be created");
    let root = temp_dir.path();
    fs::create_dir_all(root.join("inputs")).expect("scenario inputs dir should be created");
    fs::create_dir_all(root.join("expected")).expect("scenario expected dir should be created");

    let fixture_root = scenario_fixture_root();
    let baseline = validate_rust_scenario_benchmark(&fixture_root)
        .expect("committed scenario benchmark fixture should validate");
    fs::copy(
        fixture_root.join("scenario.toml"),
        root.join("scenario.toml"),
    )
    .expect("scenario metadata should copy into observed scenario");
    fs::write(
        root.join("benchmark.toml"),
        observed_benchmark_toml(ObservedBenchmarkToml {
            harness: &baseline.benchmark.harness,
            test: baseline.benchmark.test.as_deref(),
            bench: baseline.benchmark.bench.as_deref(),
            case: baseline.benchmark.case.as_deref(),
            snapshot: baseline.benchmark.snapshot.as_deref(),
            target_total: &baseline.benchmark.target_total.to_string(),
            max_total: &baseline.benchmark.max_total.to_string(),
            observed_total_ms,
            regression_budget: &baseline.benchmark.regression_budget.to_string(),
            memory_budget_bytes: baseline.benchmark.memory_budget_bytes.as_u64(),
            observed_memory_bytes: observed_memory_bytes(rows),
            target_rationale: &baseline.benchmark.target_rationale,
        }),
    )
    .expect("observed scenario benchmark contract should be written");

    temp_dir
}

#[cfg(feature = "linked-native")]
fn scenario_fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("unit")
        .join("scenarios")
        .join("linked_native_allocation_shape")
}

#[cfg(feature = "linked-native")]
struct ObservedBenchmarkToml<'a> {
    harness: &'a str,
    test: Option<&'a str>,
    bench: Option<&'a str>,
    case: Option<&'a str>,
    snapshot: Option<&'a str>,
    target_total: &'a str,
    max_total: &'a str,
    observed_total_ms: u64,
    regression_budget: &'a str,
    memory_budget_bytes: u64,
    observed_memory_bytes: u64,
    target_rationale: &'a str,
}

#[cfg(feature = "linked-native")]
fn observed_benchmark_toml(config: ObservedBenchmarkToml<'_>) -> String {
    let benchmark_entry = benchmark_entry_toml(
        config.harness,
        config.test,
        config.bench,
        config.case,
        config.snapshot,
    );
    let observed_total_ms = config.observed_total_ms;
    format!(
        r#"{benchmark_entry}target_total = "{}"
max_total = "{}"
observed_total = "{observed_total_ms}ms"
regression_budget = "{}"
memory_budget_bytes = {}
observed_memory_bytes = {}
target_rationale = "{}"

[observed_timings]
allocation_shape_ms = "{observed_total_ms}ms"
"#,
        config.target_total,
        config.max_total,
        config.regression_budget,
        config.memory_budget_bytes,
        config.observed_memory_bytes,
        toml_escape(config.target_rationale)
    )
}

#[cfg(feature = "linked-native")]
fn benchmark_entry_toml(
    harness: &str,
    test: Option<&str>,
    bench: Option<&str>,
    case: Option<&str>,
    snapshot: Option<&str>,
) -> String {
    let mut entry = format!("harness = \"{}\"\n", toml_escape(harness));
    push_optional_toml_string(&mut entry, "test", test);
    push_optional_toml_string(&mut entry, "bench", bench);
    push_optional_toml_string(&mut entry, "case", case);
    push_optional_toml_string(&mut entry, "snapshot", snapshot);
    entry
}

#[cfg(feature = "linked-native")]
fn push_optional_toml_string(output: &mut String, key: &str, value: Option<&str>) {
    if let Some(value) = value {
        output.push_str(&format!("{key} = \"{}\"\n", toml_escape(value)));
    }
}

#[cfg(feature = "linked-native")]
fn toml_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(feature = "linked-native")]
fn observed_memory_bytes(rows: &[AllocationRow]) -> u64 {
    rows.iter()
        .map(|row| {
            row.full_marshal.allocated_bytes
                + row.epoch_marshal.allocated_bytes
                + row.selector_typed_value.allocated_bytes
                + row.epoch_selector_typed_value.allocated_bytes
                + row.receipt_projection.allocated_bytes
                + row.full_e2e.allocated_bytes
                + row.epoch_e2e.allocated_bytes
        })
        .sum::<u64>()
        .max(1)
}

#[cfg(feature = "linked-native")]
fn assert_allocation_performance_gate(rows: &[AllocationRow]) {
    let mut failures = Vec::new();

    if rows.len() < 5 {
        failures.push(format!(
            "expected at least 5 allocation-shape rows, got {}",
            rows.len()
        ));
    }
    if !rows.iter().any(|row| row.edge_count == 1024) {
        failures.push("missing 1024-candidate stress row".to_owned());
    }

    for row in rows {
        assert_not_greater(
            &mut failures,
            row,
            "epoch_marshal_allocs",
            row.epoch_marshal.allocation_events(),
            "full_marshal_allocs",
            row.full_marshal.allocation_events(),
        );
        assert_not_greater(
            &mut failures,
            row,
            "epoch_marshal_bytes",
            row.epoch_marshal.allocated_bytes,
            "full_marshal_bytes",
            row.full_marshal.allocated_bytes,
        );
        assert_not_greater(
            &mut failures,
            row,
            "epoch_selector_allocs",
            row.epoch_selector_typed_value.allocation_events(),
            "selector_allocs",
            row.selector_typed_value.allocation_events(),
        );
        assert_not_greater(
            &mut failures,
            row,
            "epoch_selector_bytes",
            row.epoch_selector_typed_value.allocated_bytes,
            "selector_bytes",
            row.selector_typed_value.allocated_bytes,
        );
        assert_not_greater(
            &mut failures,
            row,
            "epoch_e2e_allocs",
            row.epoch_e2e.allocation_events(),
            "full_e2e_allocs",
            row.full_e2e.allocation_events(),
        );
        assert_not_greater(
            &mut failures,
            row,
            "epoch_e2e_bytes",
            row.epoch_e2e.allocated_bytes,
            "full_e2e_bytes",
            row.full_e2e.allocated_bytes,
        );
    }

    assert!(
        failures.is_empty(),
        "allocation performance gate failed:\n{}",
        failures.join("\n")
    );
}

#[cfg(feature = "linked-native")]
fn assert_not_greater(
    failures: &mut Vec<String>,
    row: &AllocationRow,
    left_name: &str,
    left: u64,
    right_name: &str,
    right: u64,
) {
    if left > right {
        failures.push(format!(
            "candidates={} {left_name}={left} exceeded {right_name}={right}",
            row.edge_count
        ));
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
