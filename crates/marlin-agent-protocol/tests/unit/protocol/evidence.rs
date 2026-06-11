use marlin_agent_protocol::{
    LoopEvidence, LoopEvidenceKind, LoopPerformanceEvidence, PERFORMANCE_EVIDENCE_KEYS,
};

#[test]
fn evidence_distinguishes_present_and_missing_facts() {
    let present =
        LoopEvidence::present(LoopEvidenceKind::Safety, "safety-doc").with_detail("parser-owned");
    let missing = LoopEvidence::missing(LoopEvidenceKind::Budget, "loop-budget");

    assert!(present.present);
    assert_eq!(present.detail.as_deref(), Some("parser-owned"));
    assert!(!missing.present);
}

#[test]
fn performance_evidence_carries_benchmark_contract_keys() {
    let evidence: LoopEvidence = LoopPerformanceEvidence {
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

    assert_eq!(evidence.kind, LoopEvidenceKind::Performance);
    assert_eq!(evidence.subject, "src/runtime.rs");
    assert!(evidence.present);
    for key in PERFORMANCE_EVIDENCE_KEYS {
        assert!(
            detail.contains(key),
            "missing performance evidence key {key}"
        );
    }
}
