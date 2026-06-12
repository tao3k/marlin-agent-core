use std::{
    fs,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

use marlin_agent_protocol::STABILITY_EVIDENCE_KEYS;
use rust_lang_project_harness::{
    RustHarnessConfig, RustOwnerResponsibility, RustVerificationPhase, RustVerificationPolicy,
    RustVerificationProfileHint, RustVerificationStabilityPictureConfig, RustVerificationTaskKind,
    RustVerificationTaskState, build_rust_verification_stability_index,
    build_rust_verification_stability_picture_from_index,
    plan_rust_project_verification_with_config, render_rust_verification_stability_index,
};

const WORKSPACE_STABILITY_COVERAGE_BUDGET: Duration = Duration::from_secs(10);

#[test]
fn rust_project_harness_stability_verification_covers_workspace_crates() {
    let crates = workspace_crates();
    let expected_crate_count = crates.len();

    assert!(
        expected_crate_count >= 20,
        "workspace stability coverage expected at least 20 crates, got {}",
        expected_crate_count,
    );

    let started_at = Instant::now();
    let handles = crates
        .into_iter()
        .map(|crate_dir| thread::spawn(move || assert_stability_index_for_crate(&crate_dir)));
    let records: Vec<_> = handles
        .map(|handle| {
            handle
                .join()
                .expect("workspace stability coverage worker should finish")
        })
        .collect();

    let report = StabilityCoverageReport::new(records, started_at.elapsed());

    assert!(
        report.total_duration <= WORKSPACE_STABILITY_COVERAGE_BUDGET,
        "workspace stability coverage exceeded budget: {}",
        report.render_slowest(5),
    );
    assert_eq!(report.crate_count(), expected_crate_count);
    assert!(
        report.slowest_crates(3).len() == 3,
        "workspace stability coverage should retain slowest crate evidence",
    );
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StabilityCoverageRecord {
    crate_name: String,
    duration: Duration,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StabilityCoverageReport {
    records: Vec<StabilityCoverageRecord>,
    total_duration: Duration,
}

impl StabilityCoverageReport {
    fn new(records: Vec<StabilityCoverageRecord>, total_duration: Duration) -> Self {
        Self {
            records,
            total_duration,
        }
    }

    fn crate_count(&self) -> usize {
        self.records.len()
    }

    fn slowest_crates(&self, limit: usize) -> Vec<&StabilityCoverageRecord> {
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

fn assert_stability_index_for_crate(crate_dir: &Path) -> StabilityCoverageRecord {
    let started_at = Instant::now();
    let owner_path = PathBuf::from("src/lib.rs");
    let crate_name = crate_dir
        .file_name()
        .and_then(|name| name.to_str())
        .expect("workspace crate should have a utf-8 directory name");
    let profile_hint = RustVerificationProfileHint::new(
        owner_path.clone(),
        [RustOwnerResponsibility::AvailabilityCritical],
    )
    .with_task_kinds([RustVerificationTaskKind::Stability])
    .with_rationale(format!(
        "{crate_name} owns crate-level long-run stability evidence",
    ));
    let stability_picture_config = RustVerificationStabilityPictureConfig::default();
    let config = RustHarnessConfig {
        verification_policy: RustVerificationPolicy::default()
            .with_profile_hint(profile_hint)
            .with_stability_picture(stability_picture_config.clone()),
        ..Default::default()
    };
    let plan = plan_rust_project_verification_with_config(crate_dir, &config)
        .unwrap_or_else(|error| panic!("{crate_name} stability verification plan: {error}"));
    let index = build_rust_verification_stability_index(&plan);
    let picture =
        build_rust_verification_stability_picture_from_index(&index, &stability_picture_config);
    let rendered = render_rust_verification_stability_index(&index);
    let records = index.records_for_owner(&owner_path);

    assert!(
        !index.is_empty(),
        "{crate_name} should produce a stability verification index",
    );
    assert!(
        plan.tasks.iter().any(|task| {
            task.kind == RustVerificationTaskKind::Stability
                && task.state == RustVerificationTaskState::Pending
                && task.phase == RustVerificationPhase::ScheduledRegression
        }),
        "{crate_name} should plan a pending scheduled stability task",
    );
    assert!(
        plan.report_obligations
            .iter()
            .any(|obligation| obligation.key == "stability_index_json"),
        "{crate_name} should require a stability index report",
    );
    assert_eq!(
        records.len(),
        1,
        "{crate_name} should produce one crate-level stability record",
    );
    assert_eq!(records[0].state, RustVerificationTaskState::Pending);
    for key in STABILITY_EVIDENCE_KEYS {
        assert!(
            records[0]
                .required_evidence_keys
                .iter()
                .any(|required_key| required_key == key),
            "{crate_name} stability index missing evidence key {key}",
        );
    }
    assert_eq!(picture.records.len(), 1);
    assert!(
        picture
            .actionable_records()
            .iter()
            .any(|record| record.owner_path == records[0].owner_path),
        "{crate_name} stability picture should expose an actionable owner",
    );
    assert!(
        rendered.contains("[stability-state]"),
        "{crate_name} rendered stability index should expose state",
    );

    StabilityCoverageRecord {
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
