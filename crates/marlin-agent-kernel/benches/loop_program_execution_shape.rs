use std::{
    alloc::{GlobalAlloc, Layout, System},
    env, fs,
    hint::black_box,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Instant,
};

use marlin_agent_kernel::{
    LoopProgramEventMapper, LoopProgramExecutionDriver, LoopProgramExecutionRequest,
    LoopProgramExecutionStatus, LoopProgramRuntimeHandoffHandler, LoopProgramRuntimeHandoffRouter,
    LoopProgramRuntimeHandoffRouterHandlers, LoopProgramRuntimeOwner,
    ScriptedLoopProgramEventMapper, StaticLoopProgramRuntimeHandoffHandler,
};
use marlin_agent_protocol::{
    LoopMechanismPolicyId, LoopPolicyDigest, LoopPolicyEpoch, LoopProgram, LoopProgramActionKind,
    LoopProgramEventKind, LoopProgramId, LoopProgramInput, LoopProgramStateId,
    LoopProgramTransition, LoopProgramTransitionId,
};
use marlin_rust_project_harness_policy::{
    RustScenarioBenchmarkStatus, render_rust_scenario_benchmark_snapshot,
    validate_rust_scenario_benchmark,
};
use tempfile::TempDir;

#[global_allocator]
static GLOBAL_ALLOCATOR: CountingAllocator = CountingAllocator;

static ALLOCATION_CALLS: AtomicU64 = AtomicU64::new(0);
static DEALLOCATION_CALLS: AtomicU64 = AtomicU64::new(0);
static REALLOCATION_CALLS: AtomicU64 = AtomicU64::new(0);
static ALLOCATED_BYTES: AtomicU64 = AtomicU64::new(0);
static DEALLOCATED_BYTES: AtomicU64 = AtomicU64::new(0);

struct CountingAllocator;

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

#[derive(Clone, Copy, Debug)]
struct AllocationSnapshot {
    allocations: u64,
    reallocations: u64,
    allocated_bytes: u64,
}

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

#[derive(Clone, Copy, Debug)]
struct ExecutionShapeRow {
    run_count: usize,
    elapsed_ms: u64,
    total_steps: usize,
    allocations: AllocationSnapshot,
}

fn main() {
    let driver = loop_program_execution_driver();
    let program = sample_loop_program();

    let warmup = driver.run(LoopProgramExecutionRequest::new(
        program.clone(),
        vec![LoopProgramEventKind::Start],
    ));
    assert_eq!(warmup.status, LoopProgramExecutionStatus::Stopped);
    assert_eq!(warmup.steps.len(), EXPECTED_STEPS_PER_RUN);

    let started_at = Instant::now();
    let mut rows = Vec::new();
    for run_count in [1_usize, 8, 32, 128, 512] {
        rows.push(measure_execution_shape(&driver, &program, run_count));
    }

    let observed_total_ms = elapsed_ms(started_at);
    assert_execution_shape_gate(&rows);
    print_execution_shape_observations(&rows, observed_total_ms);

    let scenario_root = observed_scenario_benchmark_root(&rows, observed_total_ms);
    let receipt = validate_rust_scenario_benchmark(scenario_root.path())
        .expect("loop program execution shape scenario benchmark should validate");
    let snapshot = render_rust_scenario_benchmark_snapshot(&receipt);
    let snapshot_path = write_scenario_benchmark_snapshot(&snapshot);
    println!("{snapshot}");
    assert_eq!(
        receipt.status,
        RustScenarioBenchmarkStatus::Pass,
        "{receipt:#?}"
    );
    println!(
        "agent-kernel loop-program execution scenario benchmark passed snapshot={}",
        snapshot_path.display()
    );
}

fn loop_program_execution_driver() -> LoopProgramExecutionDriver {
    let handlers = LoopProgramRuntimeHandoffRouterHandlers {
        control_handler: handled_by("runtime.control"),
        model_handler: handled_by("runtime.model"),
        tool_handler: handled_by("runtime.tool"),
        graph_handler: handled_by("runtime.graph"),
        verification_handler: handled_by("runtime.verification"),
        ..LoopProgramRuntimeHandoffRouterHandlers::default()
    };

    LoopProgramExecutionDriver::new(LoopProgramRuntimeHandoffRouter::new(handlers))
        .with_event_mapper(loop_script())
        .with_max_steps(16)
}

fn measure_execution_shape(
    driver: &LoopProgramExecutionDriver,
    program: &LoopProgram,
    run_count: usize,
) -> ExecutionShapeRow {
    reset_allocation_counters();
    let started_at = Instant::now();
    let mut total_steps = 0;

    for _ in 0..run_count {
        let receipt = driver.run(LoopProgramExecutionRequest::new(
            black_box(program.clone()),
            vec![LoopProgramEventKind::Start],
        ));
        assert_eq!(receipt.status, LoopProgramExecutionStatus::Stopped);
        assert_eq!(receipt.steps.len(), EXPECTED_STEPS_PER_RUN);
        total_steps += receipt.steps.len();
        black_box(receipt);
    }

    ExecutionShapeRow {
        run_count,
        elapsed_ms: elapsed_ms(started_at),
        total_steps,
        allocations: AllocationSnapshot::current(),
    }
}

fn reset_allocation_counters() {
    ALLOCATION_CALLS.store(0, Ordering::Relaxed);
    DEALLOCATION_CALLS.store(0, Ordering::Relaxed);
    REALLOCATION_CALLS.store(0, Ordering::Relaxed);
    ALLOCATED_BYTES.store(0, Ordering::Relaxed);
    DEALLOCATED_BYTES.store(0, Ordering::Relaxed);
}

const EXPECTED_STEPS_PER_RUN: usize = 6;

fn assert_execution_shape_gate(rows: &[ExecutionShapeRow]) {
    let mut failures = Vec::new();

    if rows.len() < 5 {
        failures.push(format!(
            "expected at least 5 execution-shape rows, got {}",
            rows.len()
        ));
    }
    if !rows.iter().any(|row| row.run_count == 512) {
        failures.push("missing 512-run stress row".to_owned());
    }

    for row in rows {
        let expected_steps = row.run_count * EXPECTED_STEPS_PER_RUN;
        if row.total_steps != expected_steps {
            failures.push(format!(
                "runs={} total_steps={} expected_steps={expected_steps}",
                row.run_count, row.total_steps
            ));
        }
        if row.allocations.allocation_events() == 0 {
            failures.push(format!("runs={} recorded no allocations", row.run_count));
        }
        if row.elapsed_ms > 250 {
            failures.push(format!(
                "runs={} elapsed_ms={} exceeded hard row budget 250ms",
                row.run_count, row.elapsed_ms
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "loop program execution scenario gate failed:\n{}",
        failures.join("\n")
    );
}

fn print_execution_shape_observations(rows: &[ExecutionShapeRow], observed_total_ms: u64) {
    println!(
        "execution-shape-observed total_ms={} memory_bytes={}",
        observed_total_ms,
        observed_memory_bytes(rows)
    );
    for row in rows {
        println!(
            "execution-shape-row runs={} elapsed_ms={} steps={} allocation_events={} allocated_bytes={}",
            row.run_count,
            row.elapsed_ms,
            row.total_steps,
            row.allocations.allocation_events(),
            row.allocations.allocated_bytes
        );
    }
}

fn write_scenario_benchmark_snapshot(snapshot: &str) -> PathBuf {
    let path = target_scenario_snapshot_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("scenario benchmark snapshot dir should be created");
    }
    fs::write(&path, snapshot).expect("scenario benchmark snapshot should be written");
    path
}

fn target_scenario_snapshot_path() -> PathBuf {
    let target_dir = env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(default_workspace_target_dir);
    target_dir
        .join("marlin")
        .join("scenarios")
        .join("agent-kernel-loop-program-execution-shape.snap")
}

fn default_workspace_target_dir() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(Path::parent)
        .expect("crate manifest should live under the workspace crates directory");
    workspace_root.join("target")
}

fn elapsed_ms(started_at: Instant) -> u64 {
    u64::try_from(started_at.elapsed().as_millis())
        .unwrap_or(u64::MAX)
        .max(1)
}

fn observed_scenario_benchmark_root(rows: &[ExecutionShapeRow], observed_total_ms: u64) -> TempDir {
    let temp_dir = TempDir::new().expect("scenario benchmark temp dir should be created");
    let root = temp_dir.path();
    fs::create_dir_all(root.join("inputs")).expect("scenario inputs dir should be created");
    fs::create_dir_all(root.join("expected")).expect("scenario expected dir should be created");

    let fixture_root = scenario_fixture_root();
    let baseline = validate_rust_scenario_benchmark(&fixture_root)
        .expect("committed loop execution scenario benchmark fixture should validate");
    fs::copy(
        fixture_root.join("scenario.toml"),
        root.join("scenario.toml"),
    )
    .expect("scenario metadata should copy into observed scenario");
    fs::write(
        root.join("benchmark.toml"),
        observed_benchmark_toml(
            &baseline.benchmark.harness,
            baseline.benchmark.test.as_deref(),
            baseline.benchmark.bench.as_deref(),
            baseline.benchmark.case.as_deref(),
            baseline.benchmark.snapshot.as_deref(),
            &baseline.benchmark.target_total.to_string(),
            &baseline.benchmark.max_total.to_string(),
            observed_total_ms,
            &baseline.benchmark.regression_budget.to_string(),
            baseline.benchmark.memory_budget_bytes.as_u64(),
            observed_memory_bytes(rows),
            &baseline.benchmark.target_rationale,
            rows,
        ),
    )
    .expect("observed scenario benchmark contract should be written");

    temp_dir
}

fn scenario_fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("unit")
        .join("scenarios")
        .join("loop_program_execution_shape")
}

#[allow(clippy::too_many_arguments)]
fn observed_benchmark_toml(
    harness: &str,
    test: Option<&str>,
    bench: Option<&str>,
    case: Option<&str>,
    snapshot: Option<&str>,
    target_total: &str,
    max_total: &str,
    observed_total_ms: u64,
    regression_budget: &str,
    memory_budget_bytes: u64,
    observed_memory_bytes: u64,
    target_rationale: &str,
    rows: &[ExecutionShapeRow],
) -> String {
    let benchmark_entry = benchmark_entry_toml(harness, test, bench, case, snapshot);
    let mut observed_timings = format!("execution_shape_total_ms = \"{observed_total_ms}ms\"\n");
    for row in rows {
        observed_timings.push_str(&format!(
            "execution_shape_{}_runs_ms = \"{}ms\"\n",
            row.run_count, row.elapsed_ms
        ));
    }

    format!(
        r#"{benchmark_entry}target_total = "{target_total}"
max_total = "{max_total}"
observed_total = "{observed_total_ms}ms"
regression_budget = "{regression_budget}"
memory_budget_bytes = {memory_budget_bytes}
observed_memory_bytes = {observed_memory_bytes}
target_rationale = "{}"

[observed_timings]
{observed_timings}"#,
        toml_escape(target_rationale)
    )
}

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

fn push_optional_toml_string(output: &mut String, key: &str, value: Option<&str>) {
    if let Some(value) = value {
        output.push_str(&format!("{key} = \"{}\"\n", toml_escape(value)));
    }
}

fn toml_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn observed_memory_bytes(rows: &[ExecutionShapeRow]) -> u64 {
    rows.iter()
        .map(|row| row.allocations.allocated_bytes)
        .sum::<u64>()
        .max(1)
}

fn loop_script() -> impl LoopProgramEventMapper {
    ScriptedLoopProgramEventMapper::new(
        vec![
            (
                LoopProgramActionKind::InvokeModel,
                LoopProgramEventKind::ToolRequest,
            ),
            (
                LoopProgramActionKind::DispatchTools,
                LoopProgramEventKind::ToolReceipt,
            ),
            (
                LoopProgramActionKind::Continue,
                LoopProgramEventKind::ModelEvent,
            ),
            (
                LoopProgramActionKind::RewriteGraph,
                LoopProgramEventKind::RuntimeReceipt,
            ),
            (
                LoopProgramActionKind::Verify,
                LoopProgramEventKind::VerificationReceipt,
            ),
        ]
        .into_boxed_slice(),
    )
}

fn handled_by(owner: &'static str) -> Arc<dyn LoopProgramRuntimeHandoffHandler> {
    Arc::new(StaticLoopProgramRuntimeHandoffHandler::handled(
        LoopProgramRuntimeOwner::new(owner),
    ))
}

fn sample_loop_program() -> LoopProgram {
    LoopProgram::new(LoopProgramInput {
        program_id: LoopProgramId::new("execution-driver-loop-bench"),
        policy_epoch: LoopPolicyEpoch::new(8),
        policy_digest: LoopPolicyDigest::from_bytes([7_u8; 32]),
        mechanism_policies: vec![LoopMechanismPolicyId::new("reactive-tool-loop-base")]
            .into_boxed_slice(),
        initial_state: LoopProgramStateId::new("start"),
        transitions: vec![
            LoopProgramTransition {
                transition_id: LoopProgramTransitionId::new("start-model"),
                from: LoopProgramStateId::new("start"),
                event: LoopProgramEventKind::Start,
                action: LoopProgramActionKind::InvokeModel,
                to: LoopProgramStateId::new("await-model"),
            },
            LoopProgramTransition {
                transition_id: LoopProgramTransitionId::new("model-tools"),
                from: LoopProgramStateId::new("await-model"),
                event: LoopProgramEventKind::ToolRequest,
                action: LoopProgramActionKind::DispatchTools,
                to: LoopProgramStateId::new("await-tools"),
            },
            LoopProgramTransition {
                transition_id: LoopProgramTransitionId::new("tools-continue"),
                from: LoopProgramStateId::new("await-tools"),
                event: LoopProgramEventKind::ToolReceipt,
                action: LoopProgramActionKind::Continue,
                to: LoopProgramStateId::new("await-model"),
            },
            LoopProgramTransition {
                transition_id: LoopProgramTransitionId::new("dynamic-rewrite"),
                from: LoopProgramStateId::new("await-model"),
                event: LoopProgramEventKind::ModelEvent,
                action: LoopProgramActionKind::RewriteGraph,
                to: LoopProgramStateId::new("rewritten"),
            },
            LoopProgramTransition {
                transition_id: LoopProgramTransitionId::new("verify-rewrite"),
                from: LoopProgramStateId::new("rewritten"),
                event: LoopProgramEventKind::RuntimeReceipt,
                action: LoopProgramActionKind::Verify,
                to: LoopProgramStateId::new("verifying"),
            },
            LoopProgramTransition {
                transition_id: LoopProgramTransitionId::new("verification-stop"),
                from: LoopProgramStateId::new("verifying"),
                event: LoopProgramEventKind::VerificationReceipt,
                action: LoopProgramActionKind::Stop,
                to: LoopProgramStateId::new("stopped"),
            },
        ]
        .into_boxed_slice(),
    })
}
