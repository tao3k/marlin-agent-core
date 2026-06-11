use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use marlin_agent_protocol::PERFORMANCE_EVIDENCE_KEYS;
use rust_lang_project_harness::{
    RustHarnessConfig, RustVerificationPhase, RustVerificationPolicy, RustVerificationProfileHint,
    RustVerificationTaskKind, RustVerificationTaskState, build_rust_verification_performance_index,
    plan_rust_project_verification_with_config, render_rust_verification_performance_index,
};

const WORKSPACE_PERFORMANCE_COVERAGE_BUDGET: Duration = Duration::from_secs(10);

#[test]
fn rust_project_harness_performance_verification_covers_workspace_crates() {
    let crates = workspace_crates();
    let expected_crate_count = crates.len();

    assert!(
        expected_crate_count >= 20,
        "workspace performance coverage expected at least 20 crates, got {}",
        expected_crate_count,
    );

    let started_at = Instant::now();
    let mut records = Vec::new();

    for crate_dir in crates {
        records.push(assert_performance_index_for_crate(&crate_dir));
    }

    let report = PerformanceCoverageReport::new(records, started_at.elapsed());

    assert!(
        report.total_duration <= WORKSPACE_PERFORMANCE_COVERAGE_BUDGET,
        "workspace performance coverage exceeded budget: {}",
        report.render_slowest(5),
    );
    assert_eq!(report.crate_count(), expected_crate_count);
    assert!(
        report.slowest_crates(3).len() == 3,
        "workspace performance coverage should retain slowest crate evidence",
    );
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PerformanceCoverageRecord {
    crate_name: String,
    duration: Duration,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PerformanceCoverageReport {
    records: Vec<PerformanceCoverageRecord>,
    total_duration: Duration,
}

impl PerformanceCoverageReport {
    fn new(records: Vec<PerformanceCoverageRecord>, total_duration: Duration) -> Self {
        Self {
            records,
            total_duration,
        }
    }

    fn crate_count(&self) -> usize {
        self.records.len()
    }

    fn slowest_crates(&self, limit: usize) -> Vec<&PerformanceCoverageRecord> {
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
            "total={}ms crate_count={} slowest=[{}]",
            self.total_duration.as_millis(),
            self.crate_count(),
            slowest,
        )
    }
}

fn assert_performance_index_for_crate(crate_dir: &Path) -> PerformanceCoverageRecord {
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
    let rendered = render_rust_verification_performance_index(&index);
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
    assert!(
        rendered.contains("[perf-state]"),
        "{crate_name} rendered performance index should expose state",
    );

    PerformanceCoverageRecord {
        crate_name: crate_name.to_owned(),
        duration: started_at.elapsed(),
    }
}

fn workspace_crates() -> Vec<PathBuf> {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("harness crate should live under workspace/crates");
    let crates_dir = workspace_root.join("crates");
    let mut crates = fs::read_dir(&crates_dir)
        .unwrap_or_else(|error| panic!("read workspace crates dir {crates_dir:?}: {error}"))
        .map(|entry| entry.expect("workspace crate entry").path())
        .filter(|path| path.join("Cargo.toml").is_file() && path.join("src/lib.rs").is_file())
        .collect::<Vec<_>>();

    crates.sort();
    crates
}
