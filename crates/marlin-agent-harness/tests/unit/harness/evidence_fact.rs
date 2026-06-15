use marlin_agent_harness::{
    HARNESS_PERFORMANCE_EVIDENCE_KEYS, HARNESS_STABILITY_EVIDENCE_KEYS, HarnessEvidence,
    HarnessEvidenceKind, HarnessPerformanceEvidence, HarnessStabilityEvidence,
};

#[test]
fn harness_evidence_distinguishes_present_and_missing_facts() {
    let present = HarnessEvidence::present(HarnessEvidenceKind::Safety, "safety-doc")
        .with_detail("parser-owned");
    let missing = HarnessEvidence::missing(HarnessEvidenceKind::Budget, "loop-budget");

    assert!(present.present);
    assert_eq!(present.detail.as_deref(), Some("parser-owned"));
    assert!(!missing.present);
}

#[test]
fn harness_performance_evidence_carries_benchmark_contract_keys() {
    let evidence: HarnessEvidence = HarnessPerformanceEvidence {
        subject: "src/runtime.rs".to_owned(),
        benchmark_command: "cargo bench -p marlin-agent-harness".to_owned(),
        baseline: "p95=10ms".to_owned(),
        regression_threshold: "5%".to_owned(),
        latency_or_throughput: "throughput=1000/s".to_owned(),
        allocation_profile: "allocations=steady".to_owned(),
        profile_artifact: "target/criterion/report/index.html".to_owned(),
    }
    .into();

    let detail = evidence.detail.as_deref().expect("performance detail");

    assert_eq!(evidence.kind, HarnessEvidenceKind::Performance);
    assert_eq!(evidence.subject, "src/runtime.rs");
    assert!(evidence.present);
    for key in HARNESS_PERFORMANCE_EVIDENCE_KEYS {
        assert!(
            detail.contains(key),
            "missing performance evidence key {key}"
        );
    }
}

#[test]
fn harness_stability_evidence_carries_long_run_contract_keys() {
    let evidence: HarnessEvidence = HarnessStabilityEvidence {
        subject: "src/runtime.rs".to_owned(),
        stability_command: "cargo test -p marlin-agent-runtime stability".to_owned(),
        iteration_window: "iterations=1000 duration=60s".to_owned(),
        latency_distribution: "p95=12ms p99=20ms".to_owned(),
        resource_delta: "rss_delta=0 fd_delta=0".to_owned(),
        state_growth: "queue_depth_delta=0 cache_growth=bounded".to_owned(),
        determinism: "replay=stable".to_owned(),
        stability_artifact: "target/marlin/stability/runtime.json".to_owned(),
    }
    .into();

    let detail = evidence.detail.as_deref().expect("stability detail");

    assert_eq!(evidence.kind, HarnessEvidenceKind::Stability);
    assert_eq!(evidence.subject, "src/runtime.rs");
    assert!(evidence.present);
    for key in HARNESS_STABILITY_EVIDENCE_KEYS {
        assert!(detail.contains(key), "missing stability evidence key {key}");
    }
}
