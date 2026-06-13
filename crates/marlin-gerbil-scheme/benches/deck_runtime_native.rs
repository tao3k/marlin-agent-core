use std::{
    env, fs,
    path::Path,
    process::Command,
    slice, str,
    time::{Duration, Instant},
};

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use marlin_gerbil_scheme::{
    GERBIL_DECK_RUNTIME_NATIVE_ABI_VERSION, GERBIL_LOADPATH_ENV, GerbilDeckRuntimeModelRoutePolicy,
    GerbilDeckRuntimeModelRoutePolicyRequest, GerbilDeckRuntimeModelRoutePolicyRuntimeBinding,
    GerbilDeckRuntimeNativeAotBuildStatus, GerbilDeckRuntimeNativeAotConfig,
    GerbilDeckRuntimeNativeModelRouteRequest, GerbilDeckRuntimeNativeModelRouteSelection,
    GerbilDeckRuntimeNativeModelRouteSelector, GerbilDeckRuntimeNativeStatus,
    GerbilDeckRuntimeNativeUtf8, decode_gerbil_deck_runtime_model_route_selection,
    default_gerbil_gsc_program, default_gerbil_gxc_program, default_gerbil_gxi_program,
    gerbil_runtime_loadpath, write_gerbil_runtime_assets,
};

const RECEIPT_JSON: &str = r#"{"schema_id":"marlin-deck-runtime.model-route-selection.v1","command":"cargo test","agent_scope":"sub-agent","matched":true,"policy":{"kind":"marlin-deck-runtime.model-route-policy.v1","name":"cheap-test-runner","provider":"openai","model":"gpt-5-mini","command_prefixes":["cargo test"],"agent_scopes":["sub-agent"],"context_mode":"forked-context","isolation_mode":"workspace-isolated"}}"#;
const REAL_PACKAGE_BENCH_ENV: &str = "MARLIN_GERBIL_REAL_PACKAGE_BENCH";
const REAL_STRATEGY_BENCH_ENV: &str = "MARLIN_GERBIL_REAL_STRATEGY_BENCH";
const REAL_COMPILED_POLICY_BENCH_ENV: &str = "MARLIN_GERBIL_REAL_COMPILED_POLICY_BENCH";
const REAL_NATIVE_AOT_BENCH_ENV: &str = "MARLIN_GERBIL_NATIVE_AOT_BENCH";
const COMPILED_POLICY_BATCH_ITERATIONS: usize = 10_000;

fn bench_native_selector(c: &mut Criterion) {
    let selector = GerbilDeckRuntimeNativeModelRouteSelector::new(fake_select_model_route_fast);
    let mut group = c.benchmark_group("deck_runtime_native_selector");

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

fn bench_receipt_decode(c: &mut Criterion) {
    c.bench_function("deck_runtime_native_receipt_decode", |bencher| {
        bencher.iter(|| {
            let receipt = decode_gerbil_deck_runtime_model_route_selection(std::hint::black_box(
                RECEIPT_JSON,
            ))
            .expect("bench receipt should decode");
            std::hint::black_box(receipt);
        });
    });
}

fn bench_real_scheme_package_selector(c: &mut Criterion) {
    if !real_package_bench_enabled() {
        eprintln!(
            "skipping real Scheme package benchmark; set {REAL_PACKAGE_BENCH_ENV}=1 to run it"
        );
        return;
    }

    let gxi = default_gerbil_gxi_program();
    assert!(
        executable_exists(&gxi),
        "real Scheme package benchmark requires gxi at {}; set MARLIN_GERBIL_GXI to override",
        gxi.display()
    );

    let loadpath_root = tempfile::Builder::new()
        .prefix("marlin-gerbil-real-package-bench-")
        .tempdir()
        .expect("create real Scheme package benchmark loadpath");
    let binding = GerbilDeckRuntimeModelRoutePolicyRuntimeBinding::new(&gxi, loadpath_root.path())
        .expect("write real Scheme package runtime assets");
    let evaluator = binding.evaluator().clone();
    let request = route_request(2);

    let warmup_receipt = evaluator
        .evaluate(request.clone())
        .expect("real Scheme package selector should evaluate before benchmarking");
    assert!(warmup_receipt.matched);

    let mut group = c.benchmark_group("deck_runtime_real_scheme_package");
    group.sample_size(10);
    group.warm_up_time(Duration::from_millis(100));
    group.measurement_time(Duration::from_secs(2));
    group.bench_function("model_route_policy_process_roundtrip", |bencher| {
        bencher.iter(|| {
            let receipt = evaluator
                .evaluate(std::hint::black_box(request.clone()))
                .expect("real Scheme package selector bench receipt should decode");
            std::hint::black_box(receipt);
        });
    });
    group.finish();
}

fn bench_real_scheme_strategy_selector(c: &mut Criterion) {
    if !real_strategy_bench_enabled() {
        eprintln!(
            "skipping real Scheme strategy benchmark; set {REAL_STRATEGY_BENCH_ENV}=1 to run it"
        );
        return;
    }

    let gxi = default_gerbil_gxi_program();
    assert!(
        executable_exists(&gxi),
        "real Scheme strategy benchmark requires gxi at {}; set MARLIN_GERBIL_GXI to override",
        gxi.display()
    );

    let loadpath_root = tempfile::Builder::new()
        .prefix("marlin-gerbil-real-strategy-bench-")
        .tempdir()
        .expect("create real Scheme strategy benchmark loadpath");
    write_gerbil_runtime_assets(loadpath_root.path())
        .expect("write real Scheme strategy benchmark assets");
    let script = loadpath_root.path().join("deck-runtime-strategy-bench.ss");
    write_strategy_bench_script(&script);

    let warmup = run_real_scheme_strategy_script(&gxi, loadpath_root.path(), &script);
    assert!(
        warmup.contains("marlin-deck-runtime.strategy-selection.v1"),
        "real Scheme strategy benchmark warmup returned unexpected output: {warmup}"
    );

    let mut group = c.benchmark_group("deck_runtime_real_scheme_strategy");
    group.sample_size(10);
    group.warm_up_time(Duration::from_millis(100));
    group.measurement_time(Duration::from_secs(2));
    group.bench_function("complex_strategy_process_roundtrip", |bencher| {
        bencher.iter(|| {
            let output = run_real_scheme_strategy_script(
                std::hint::black_box(&gxi),
                std::hint::black_box(loadpath_root.path()),
                std::hint::black_box(&script),
            );
            std::hint::black_box(output);
        });
    });
    group.finish();
}

fn bench_real_scheme_compiled_policy_selector(c: &mut Criterion) {
    if !real_compiled_policy_bench_enabled() {
        eprintln!(
            "skipping real Scheme compiled policy benchmark; set {REAL_COMPILED_POLICY_BENCH_ENV}=1 to run it"
        );
        return;
    }

    let gxi = default_gerbil_gxi_program();
    assert!(
        executable_exists(&gxi),
        "real Scheme compiled policy benchmark requires gxi at {}; set MARLIN_GERBIL_GXI to override",
        gxi.display()
    );

    let loadpath_root = tempfile::Builder::new()
        .prefix("marlin-gerbil-real-compiled-policy-bench-")
        .tempdir()
        .expect("create real Scheme compiled policy benchmark loadpath");
    write_gerbil_runtime_assets(loadpath_root.path())
        .expect("write real Scheme compiled policy benchmark assets");
    run_real_scheme_package_build_in_root(&gxi, loadpath_root.path());
    let script = loadpath_root
        .path()
        .join("deck-runtime-compiled-policy-bench.ss");
    let batch_script = loadpath_root
        .path()
        .join("deck-runtime-compiled-policy-batch-bench.ss");
    write_compiled_policy_bench_script(&script);
    write_compiled_policy_batch_bench_script(&batch_script, COMPILED_POLICY_BATCH_ITERATIONS);

    let warmup = run_real_scheme_strategy_script(&gxi, loadpath_root.path(), &script);
    assert!(
        warmup.contains("marlin-deck-runtime.compiled-policy.v1"),
        "real Scheme compiled policy benchmark warmup returned unexpected output: {warmup}"
    );
    let batch_warmup = run_real_scheme_strategy_script(&gxi, loadpath_root.path(), &batch_script);
    assert!(
        batch_warmup.contains("matches=10000"),
        "real Scheme compiled policy batch benchmark warmup returned unexpected output: {batch_warmup}"
    );
    let policy_batch_elapsed_us =
        parse_compiled_policy_batch_elapsed_us(&batch_warmup, "policy_elapsed_us=");
    let index_batch_elapsed_us =
        parse_compiled_policy_batch_elapsed_us(&batch_warmup, "index_elapsed_us=");
    eprintln!(
        "real Scheme compiled policy template module internal loop: iterations={COMPILED_POLICY_BATCH_ITERATIONS} policy_elapsed_us={policy_batch_elapsed_us} approx_policy_ns_per_call={}",
        (policy_batch_elapsed_us * 1000) / COMPILED_POLICY_BATCH_ITERATIONS as u64
    );
    eprintln!(
        "real Scheme compiled policy template index loop: iterations={COMPILED_POLICY_BATCH_ITERATIONS} index_elapsed_us={index_batch_elapsed_us} approx_index_ns_per_call={}",
        (index_batch_elapsed_us * 1000) / COMPILED_POLICY_BATCH_ITERATIONS as u64
    );

    let mut group = c.benchmark_group("deck_runtime_real_scheme_compiled_policy");
    group.sample_size(10);
    group.warm_up_time(Duration::from_millis(100));
    group.measurement_time(Duration::from_secs(2));
    group.bench_function("compiled_policy_process_roundtrip", |bencher| {
        bencher.iter(|| {
            let output = run_real_scheme_strategy_script(
                std::hint::black_box(&gxi),
                std::hint::black_box(loadpath_root.path()),
                std::hint::black_box(&script),
            );
            std::hint::black_box(output);
        });
    });
    group.bench_function(
        format!("compiled_policy_batch_{COMPILED_POLICY_BATCH_ITERATIONS}_process_roundtrip"),
        |bencher| {
            bencher.iter(|| {
                let output = run_real_scheme_strategy_script(
                    std::hint::black_box(&gxi),
                    std::hint::black_box(loadpath_root.path()),
                    std::hint::black_box(&batch_script),
                );
                std::hint::black_box(output);
            });
        },
    );
    group.finish();
}

fn bench_real_scheme_package_build(c: &mut Criterion) {
    if !real_package_bench_enabled() {
        eprintln!(
            "skipping real Scheme package build benchmark; set {REAL_PACKAGE_BENCH_ENV}=1 to run it"
        );
        return;
    }

    let gxi = default_gerbil_gxi_program();
    assert!(
        executable_exists(&gxi),
        "real Scheme package build benchmark requires gxi at {}; set MARLIN_GERBIL_GXI to override",
        gxi.display()
    );

    let mut group = c.benchmark_group("deck_runtime_real_scheme_package_build");
    group.sample_size(10);
    group.warm_up_time(Duration::from_millis(100));
    group.measurement_time(Duration::from_secs(2));
    group.bench_function("build_ss_compile_package_assets", |bencher| {
        bencher.iter_custom(|iterations| {
            let started = Instant::now();
            for _ in 0..iterations {
                run_real_scheme_package_build(&gxi);
            }
            started.elapsed()
        });
    });
    group.finish();
}

fn bench_real_native_aot_link_unit_build(c: &mut Criterion) {
    if !real_native_aot_bench_enabled() {
        eprintln!(
            "skipping real native AOT link-unit benchmark; set {REAL_NATIVE_AOT_BENCH_ENV}=1 to run it"
        );
        return;
    }

    let gxc = default_gerbil_gxc_program();
    let gsc = default_gerbil_gsc_program();
    assert!(
        executable_exists(&gxc),
        "real native AOT benchmark requires gxc at {}; set MARLIN_GERBIL_GXC to override",
        gxc.display()
    );
    assert!(
        executable_exists(&gsc),
        "real native AOT benchmark requires gsc at {}; set MARLIN_GERBIL_GSC to override",
        gsc.display()
    );

    let mut group = c.benchmark_group("deck_runtime_real_native_aot_link_unit_build");
    group.sample_size(10);
    group.warm_up_time(Duration::from_millis(100));
    group.measurement_time(Duration::from_secs(2));
    group.bench_function("build_link_unit", |bencher| {
        bencher.iter_custom(|iterations| {
            let started = Instant::now();
            for _ in 0..iterations {
                run_real_native_aot_link_unit_build();
            }
            started.elapsed()
        });
    });
    group.finish();
}

fn run_real_scheme_package_build(gxi: &Path) {
    let loadpath_root = tempfile::Builder::new()
        .prefix("marlin-gerbil-real-package-build-bench-")
        .tempdir()
        .expect("create real Scheme package build benchmark loadpath");
    write_gerbil_runtime_assets(loadpath_root.path())
        .expect("write real Scheme package build benchmark assets");

    run_real_scheme_package_build_in_root(gxi, loadpath_root.path());
}

fn run_real_scheme_package_build_in_root(gxi: &Path, loadpath_root: &Path) {
    let output = Command::new(gxi)
        .env(GERBIL_LOADPATH_ENV, gerbil_runtime_loadpath(loadpath_root))
        .current_dir(loadpath_root)
        .arg(loadpath_root.join("build.ss"))
        .arg("compile")
        .output()
        .expect("run real Scheme package build script");

    assert!(
        output.status.success(),
        "real Scheme package build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    std::hint::black_box(output);
}

fn run_real_scheme_strategy_script(gxi: &Path, loadpath_root: &Path, script: &Path) -> String {
    let output = Command::new(gxi)
        .env(GERBIL_LOADPATH_ENV, gerbil_runtime_loadpath(loadpath_root))
        .arg(script)
        .output()
        .expect("run real Scheme strategy benchmark script");

    assert!(
        output.status.success(),
        "real Scheme strategy benchmark failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout).expect("real Scheme strategy output should be UTF-8")
}

fn write_strategy_bench_script(script: &Path) {
    fs::write(
        script,
        r#"(import :clan/poo/object
        :marlin/deck-runtime
        :marlin/deck-runtime-strategy)

(def policies
  (list
   (make-marlin-deck-runtime-model-route-policy
    "cheap-test-runner"
    "openai"
    "gpt-5-mini"
    (list "cargo test")
    (list "sub-agent" "hook")
    "forked-context"
    "workspace-isolated")
   (make-marlin-deck-runtime-model-route-policy
    "deep-customer-reviewer"
    "anthropic"
    "claude-opus-4-8"
    (list "codex customer-review" "cargo test")
    (list "sub-agent")
    "shared-context"
    "isolated-session")))

(defmarlin-deck-runtime-strategy-rule
  customer-release-subagent
  "customer-release-subagent"
  "deep-customer-reviewer"
  "release-session"
  (list "root-agent" "customer-agent")
  (list "real-gxi-ready" "org-memory-indexed")
  (list "hook-roadmap" "runtime-positioning")
  "customer-agent"
  "defer"
  "")

(def context
  (make-marlin-deck-runtime-strategy-context
   "release-session"
   (list "root-agent" "customer-agent" "review-agent")
   (list "real-gxi-ready" "org-memory-indexed" "dirty-docs-ok")
   (list "hook-roadmap" "runtime-positioning" "native-aot-benchmark")
   "customer-agent"))

(def selection
  (marlin-deck-runtime-strategy-selection
   policies
   (list customer-release-subagent)
   context
   "codex customer-review --session release-session"
   "sub-agent"))

(display (.get selection kind))
(newline)
(display (if (.get selection matched) "matched" "miss"))
(newline)
(display (.get selection strategy-rule))
(newline)
"#,
    )
    .expect("write real Scheme strategy benchmark script");
}

fn write_compiled_policy_bench_script(script: &Path) {
    fs::write(
        script,
        r#"(import :marlin/deck-runtime-compiled-policy-sample)

(display-marlin-deck-runtime-sample-compiled-policy-batch-metrics
 1
 "cargo test -p marlin-gerbil-scheme"
 "sub-agent")

(display-marlin-deck-runtime-sample-compiled-policy-index-batch-metrics
 1
 "cargo test -p marlin-gerbil-scheme"
 "sub-agent")
"#,
    )
    .expect("write real Scheme compiled policy template benchmark script");
}

fn write_compiled_policy_batch_bench_script(script: &Path, iterations: usize) {
    fs::write(
        script,
        format!(
            r#"(import :marlin/deck-runtime-compiled-policy-sample)

(display-marlin-deck-runtime-sample-compiled-policy-batch-metrics
 {iterations}
 "cargo test -p marlin-gerbil-scheme"
 "sub-agent")

(display-marlin-deck-runtime-sample-compiled-policy-index-batch-metrics
 {iterations}
 "cargo test -p marlin-gerbil-scheme"
 "sub-agent")
"#
        ),
    )
    .expect("write real Scheme compiled policy template batch benchmark script");
}

fn parse_compiled_policy_batch_elapsed_us(output: &str, field_prefix: &str) -> u64 {
    output
        .split_whitespace()
        .find_map(|field| field.strip_prefix(field_prefix))
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or_else(|| {
            panic!("missing {field_prefix} field in compiled policy batch output: {output}")
        })
}

fn run_real_native_aot_link_unit_build() {
    let root = tempfile::Builder::new()
        .prefix("marlin-gerbil-native-aot-build-bench-")
        .tempdir()
        .expect("create real native AOT benchmark root");
    let mut config = GerbilDeckRuntimeNativeAotConfig::new(root.path());
    if cfg!(target_os = "macos") {
        config = config.with_c_compiler("clang");
    }
    let receipt = config.build_link_unit();

    assert_eq!(
        receipt.status,
        GerbilDeckRuntimeNativeAotBuildStatus::LinkUnitReady,
        "real native AOT link-unit build failed: {receipt:?}"
    );
    assert!(receipt.plan.object.is_file());
    assert!(receipt.plan.link_object.is_file());
    std::hint::black_box(receipt);
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

fn real_package_bench_enabled() -> bool {
    env::var_os(REAL_PACKAGE_BENCH_ENV).as_deref() == Some(std::ffi::OsStr::new("1"))
}

fn real_strategy_bench_enabled() -> bool {
    env::var_os(REAL_STRATEGY_BENCH_ENV).as_deref() == Some(std::ffi::OsStr::new("1"))
}

fn real_compiled_policy_bench_enabled() -> bool {
    env::var_os(REAL_COMPILED_POLICY_BENCH_ENV).as_deref() == Some(std::ffi::OsStr::new("1"))
}

fn real_native_aot_bench_enabled() -> bool {
    env::var_os(REAL_NATIVE_AOT_BENCH_ENV).as_deref() == Some(std::ffi::OsStr::new("1"))
}

fn executable_exists(path: &Path) -> bool {
    path.is_file()
}

unsafe extern "C" fn fake_select_model_route_fast(
    request: *const GerbilDeckRuntimeNativeModelRouteRequest,
    selection: *mut GerbilDeckRuntimeNativeModelRouteSelection,
) -> GerbilDeckRuntimeNativeStatus {
    if request.is_null() || selection.is_null() {
        return GerbilDeckRuntimeNativeStatus::new(2);
    }

    let request = unsafe { &*request };
    if request.abi_version != GERBIL_DECK_RUNTIME_NATIVE_ABI_VERSION {
        return GerbilDeckRuntimeNativeStatus::new(3);
    }
    let _command = native_utf8_to_string(request.command);
    let _agent_scope = native_utf8_to_string(request.agent_scope);

    unsafe {
        *selection = GerbilDeckRuntimeNativeModelRouteSelection::matched(0);
    }

    GerbilDeckRuntimeNativeStatus::OK
}

fn native_utf8_to_string(value: GerbilDeckRuntimeNativeUtf8) -> String {
    if value.ptr.is_null() {
        return String::new();
    }

    let bytes = unsafe { slice::from_raw_parts(value.ptr, value.len) };
    str::from_utf8(bytes)
        .expect("bench request should contain UTF-8")
        .to_owned()
}

criterion_group! {
    name = deck_runtime_native;
    config = Criterion::default()
        .sample_size(20)
        .warm_up_time(Duration::from_millis(100))
        .measurement_time(Duration::from_millis(500));
    targets = bench_native_selector, bench_receipt_decode, bench_real_scheme_package_selector, bench_real_scheme_strategy_selector, bench_real_scheme_compiled_policy_selector, bench_real_scheme_package_build, bench_real_native_aot_link_unit_build
}
criterion_main!(deck_runtime_native);
