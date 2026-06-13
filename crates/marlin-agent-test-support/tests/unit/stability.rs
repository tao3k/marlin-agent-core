use std::time::Duration;

use marlin_agent_protocol::{LoopEvidenceKind, STABILITY_EVIDENCE_KEYS};
use marlin_agent_test_support::{RuntimeStabilityEvidenceInput, runtime_stability_budget_evidence};

#[test]
fn runtime_stability_budget_helper_projects_stability_evidence() {
    let evidence = runtime_stability_budget_evidence(RuntimeStabilityEvidenceInput {
        subject: "crates/marlin-agent-harness/src/runtime.rs".to_owned(),
        stability_command: "cargo test -p marlin-agent-harness --test unit_test".to_owned(),
        duration: Duration::from_millis(7),
        duration_budget: Duration::from_millis(250),
        event_count: 5,
        event_budget: 5,
        custom_event_count: Some(1),
        span_count: 4,
        span_budget: 32,
        diagnostic_count: 0,
        state_growth: "event_queue=drained,trace_spans=bounded".to_owned(),
        determinism: "scripted-eventful-executor,node_order=stable".to_owned(),
        stability_artifact: "target/agent-harness/stability/runtime-performance.json".to_owned(),
    });
    let detail = evidence.detail.as_deref().expect("stability detail");

    assert!(evidence.present);
    assert_eq!(evidence.kind, LoopEvidenceKind::Stability);
    assert_eq!(
        evidence.subject,
        "crates/marlin-agent-harness/src/runtime.rs"
    );
    for key in STABILITY_EVIDENCE_KEYS {
        assert!(detail.contains(key), "missing stability evidence key {key}");
    }
    for observation in [
        "duration_ms=7",
        "duration_budget_ms=250",
        "event_count=5",
        "event_budget=5",
        "custom_event_count=1",
        "span_count=4",
        "span_budget=32",
        "diagnostic_count=0",
    ] {
        assert!(
            detail.contains(observation),
            "missing stability observation {observation}"
        );
    }
}
