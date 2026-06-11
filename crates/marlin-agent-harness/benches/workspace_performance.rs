use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use marlin_agent_protocol::PERFORMANCE_EVIDENCE_KEYS;
use marlin_gerbil_scheme::{
    GerbilAotBackendRepairStatus, GerbilAotProbeConfig, GerbilAotProbeStatus,
};
use rust_lang_project_harness::{
    RustHarnessConfig, RustVerificationPhase, RustVerificationPolicy, RustVerificationProfileHint,
    RustVerificationTaskKind, RustVerificationTaskState, build_rust_verification_performance_index,
    plan_rust_project_verification_with_config,
};

const WORKSPACE_PERFORMANCE_BENCH_BUDGET: Duration = Duration::from_secs(10);
const GERBIL_AOT_PROBE_BENCH_BUDGET: Duration = Duration::from_secs(20);

fn main() {
    let crates = workspace_crates();
    let expected_crate_count = crates.len();

    assert!(
        expected_crate_count >= 20,
        "workspace performance bench expected at least 20 crates, got {}",
        expected_crate_count,
    );

    let started_at = Instant::now();
    let handles = crates
        .into_iter()
        .map(|crate_dir| thread::spawn(move || performance_index_for_crate(&crate_dir)));
    let records: Vec<_> = handles
        .map(|handle| {
            handle
                .join()
                .expect("workspace performance bench worker should finish")
        })
        .collect();

    let workspace_duration = started_at.elapsed();
    let gerbil_aot_probe = gerbil_aot_probe_bench();
    let report = PerformanceBenchReport::new(records, workspace_duration, gerbil_aot_probe);
    println!("[bench] {}", report.render_slowest(5));

    assert_eq!(report.crate_count(), expected_crate_count);
    assert!(
        report.workspace_duration <= WORKSPACE_PERFORMANCE_BENCH_BUDGET,
        "workspace performance bench exceeded budget: {}",
        report.render_slowest(5),
    );
    assert!(
        report.gerbil_aot_probe.duration <= GERBIL_AOT_PROBE_BENCH_BUDGET,
        "Gerbil AOT probe bench exceeded budget: {}",
        report.render_slowest(5),
    );
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PerformanceBenchRecord {
    crate_name: String,
    duration: Duration,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct GerbilAotProbeBenchRecord {
    status: GerbilAotProbeStatus,
    duration: Duration,
    backend_gsc: Option<PathBuf>,
    backend_repair: GerbilAotBackendRepairStatus,
    executable_ready: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PerformanceBenchReport {
    records: Vec<PerformanceBenchRecord>,
    workspace_duration: Duration,
    gerbil_aot_probe: GerbilAotProbeBenchRecord,
}

impl PerformanceBenchReport {
    fn new(
        records: Vec<PerformanceBenchRecord>,
        workspace_duration: Duration,
        gerbil_aot_probe: GerbilAotProbeBenchRecord,
    ) -> Self {
        Self {
            records,
            workspace_duration,
            gerbil_aot_probe,
        }
    }

    fn crate_count(&self) -> usize {
        self.records.len()
    }

    fn slowest_crates(&self, limit: usize) -> Vec<&PerformanceBenchRecord> {
        let mut records = self.records.iter().collect::<Vec<_>>();
        records.sort_by(|left, right| {
            right
                .duration
                .cmp(&left.duration)
                .then_with(|| left.crate_name.cmp(&right.crate_name))
        });
        records.truncate(limit);
        records
    }

    fn render_slowest(&self, limit: usize) -> String {
        let slowest = self
            .slowest_crates(limit)
            .into_iter()
            .map(|record| format!("{}={}ms", record.crate_name, record.duration.as_millis()))
            .collect::<Vec<_>>()
            .join(", ");

        format!(
            "workspace={}ms crate_count={} slowest=[{}] gerbil_aot_probe={}",
            self.workspace_duration.as_millis(),
            self.crate_count(),
            slowest,
            self.gerbil_aot_probe.render(),
        )
    }
}

impl GerbilAotProbeBenchRecord {
    fn render(&self) -> String {
        let backend_gsc = self
            .backend_gsc
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "-".to_owned());
        format!(
            "status={:?},duration={}ms,backend_gsc={},backend_repair={:?},executable_ready={}",
            self.status,
            self.duration.as_millis(),
            backend_gsc,
            self.backend_repair,
            self.executable_ready,
        )
    }
}

fn performance_index_for_crate(crate_dir: &Path) -> PerformanceBenchRecord {
    let started_at = Instant::now();
    let owner_path = PathBuf::from("src/lib.rs");
    let crate_name = crate_dir
        .file_name()
        .and_then(|name| name.to_str())
        .expect("workspace crate should have a utf-8 directory name");
    let profile_hint = RustVerificationProfileHint {
        owner_path: owner_path.clone(),
        responsibilities: BTreeSet::new(),
        task_kinds: Some(BTreeSet::from([RustVerificationTaskKind::Performance])),
        task_contract_overrides: BTreeMap::new(),
        rationale: Some(format!(
            "{crate_name} owns crate-level benchmark and performance evidence",
        )),
    };
    let config = RustHarnessConfig {
        verification_policy: RustVerificationPolicy::default().with_profile_hint(profile_hint),
        ..Default::default()
    };
    let plan = plan_rust_project_verification_with_config(crate_dir, &config)
        .unwrap_or_else(|error| panic!("{crate_name} performance verification plan: {error}"));
    let index = build_rust_verification_performance_index(&plan);
    let records = index.records_for_owner(&owner_path);

    assert!(
        !index.is_empty(),
        "{crate_name} should produce a performance verification index",
    );
    assert!(
        plan.tasks.iter().any(|task| {
            task.kind == RustVerificationTaskKind::Performance
                && task.state == RustVerificationTaskState::Pending
                && task.phase == RustVerificationPhase::AfterUnitTestsPass
        }),
        "{crate_name} should plan a pending performance task after unit tests pass",
    );
    assert!(
        plan.report_obligations
            .iter()
            .any(|obligation| obligation.key == "performance_index_json"),
        "{crate_name} should require a performance index report",
    );
    assert_eq!(
        records.len(),
        1,
        "{crate_name} should produce one crate-level performance record",
    );
    assert_eq!(records[0].state, RustVerificationTaskState::Pending);
    for key in PERFORMANCE_EVIDENCE_KEYS {
        assert!(
            records[0]
                .required_evidence_keys
                .iter()
                .any(|required_key| required_key == key),
            "{crate_name} performance index missing evidence key {key}",
        );
    }

    PerformanceBenchRecord {
        crate_name: crate_name.to_owned(),
        duration: started_at.elapsed(),
    }
}

fn gerbil_aot_probe_bench() -> GerbilAotProbeBenchRecord {
    let root = temp_bench_root("gerbil-aot-probe");
    let cache_path = workspace_root()
        .join("target")
        .join("marlin-gerbil-aot-probe-cache.json");
    let started_at = Instant::now();
    let receipt = GerbilAotProbeConfig::new(&root)
        .probe_with_toolchain_cache(&cache_path)
        .expect("Gerbil AOT probe cache should be readable and writable");
    let duration = started_at.elapsed();
    let executable_ready =
        receipt.status == GerbilAotProbeStatus::ExecutableReady && receipt.executable.is_file();
    let shim_root = workspace_root().join("target").join("gerbil-backend-shims");
    let backend_repair = receipt
        .plan_backend_gsc_repair(&shim_root)
        .expect("Gerbil AOT backend repair plan should be checkable")
        .status;

    assert!(
        duration <= GERBIL_AOT_PROBE_BENCH_BUDGET,
        "Gerbil AOT probe exceeded budget: {}ms",
        duration.as_millis(),
    );
    if receipt.status == GerbilAotProbeStatus::GscBackendUnavailable {
        assert!(
            receipt.backend_gsc.is_some(),
            "Gerbil AOT backend failure should report the backend gsc path",
        );
    }
    if matches!(
        receipt.status,
        GerbilAotProbeStatus::MissingGxc | GerbilAotProbeStatus::MissingGsc
    ) {
        assert!(
            receipt.module_compile.is_none(),
            "missing Gerbil toolchain probes should not attempt module compilation",
        );
    }
    if receipt.status == GerbilAotProbeStatus::ExecutableReady {
        assert!(
            executable_ready,
            "Gerbil AOT executable-ready probe should leave an executable artifact",
        );
    }

    let record = GerbilAotProbeBenchRecord {
        status: receipt.status,
        duration,
        backend_gsc: receipt.backend_gsc,
        backend_repair,
        executable_ready,
    };
    let _ = fs::remove_dir_all(root);
    record
}

fn temp_bench_root(name: &str) -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    std::env::temp_dir().join(format!(
        "marlin-agent-harness-{name}-{}-{suffix}",
        std::process::id()
    ))
}

fn workspace_crates() -> Vec<PathBuf> {
    let workspace_root = workspace_root();
    let crates_dir = workspace_root.join("crates");
    let mut crates = fs::read_dir(&crates_dir)
        .unwrap_or_else(|error| panic!("read workspace crates dir {crates_dir:?}: {error}"))
        .map(|entry| entry.expect("workspace crate entry").path())
        .filter(|path| path.join("Cargo.toml").is_file() && path.join("src/lib.rs").is_file())
        .collect::<Vec<_>>();

    crates.sort();
    crates
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("harness crate should live under workspace/crates")
        .to_path_buf()
}
