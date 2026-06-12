use std::collections::BTreeSet;

use marlin_agent_runtime::{TokioAgentRuntime, observability};

#[test]
fn runtime_context_carries_process_cleanup_policy_to_children() {
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let runtime =
        runtime.with_process_cleanup_policy(observability::RuntimeProcessCleanupPolicy::new(3));

    assert_eq!(
        runtime
            .process_cleanup_policy()
            .graceful_termination_attempts_before_force,
        3
    );
    assert_eq!(
        runtime
            .context()
            .process_cleanup_policy()
            .graceful_termination_attempts_before_force,
        3
    );
    assert_eq!(
        runtime
            .child_runtime()
            .process_cleanup_policy()
            .graceful_termination_attempts_before_force,
        3
    );
    assert_eq!(
        runtime
            .context()
            .child_context()
            .process_cleanup_policy()
            .graceful_termination_attempts_before_force,
        3
    );
}

#[test]
fn runtime_process_cleanup_driver_uses_runtime_policy() {
    let (runtime, _events) = TokioAgentRuntime::new(4);
    let runtime =
        runtime.with_process_cleanup_policy(observability::RuntimeProcessCleanupPolicy::new(2));
    let context = runtime.context();
    let registry = context.process_registry();
    registry.track(
        observability::RuntimeProcessObservation::new(
            2200,
            observability::RuntimeProcessKind::Tool,
            "tool:cleanup-policy",
        )
        .with_started_at_ms(1),
    );
    registry
        .request_cleanup(2200, 10)
        .expect("cleanup candidate should be tracked");
    let mut controller = FakeCleanupController::with_alive([2200]).with_failure(2200);

    let first_receipt = runtime.sweep_process_cleanup(&mut controller, 20);

    assert_eq!(first_receipt.entries.len(), 1);
    assert_eq!(first_receipt.entries[0].cleanup_attempts, 1);
    assert!(!first_receipt.entries[0].force_cleanup_recommended);

    let second_receipt = context.sweep_process_cleanup(&mut controller, 30);

    assert_eq!(second_receipt.entries.len(), 1);
    assert_eq!(second_receipt.entries[0].cleanup_attempts, 2);
    assert_eq!(
        second_receipt.entries[0].last_cleanup_attempt_at_ms,
        Some(30)
    );
    assert!(second_receipt.entries[0].force_cleanup_recommended);
}

#[test]
fn runtime_process_cleanup_sysinfo_driver_reports_empty_receipt_without_candidates() {
    let (runtime, _events) = TokioAgentRuntime::new(4);

    let receipt = runtime.sweep_process_cleanup_with_sysinfo(40);

    assert_eq!(receipt.observed_at_ms, 40);
    assert!(receipt.is_empty());
}

struct FakeCleanupController {
    alive: BTreeSet<u32>,
    fail_termination: BTreeSet<u32>,
}

impl FakeCleanupController {
    fn with_alive(pids: impl IntoIterator<Item = u32>) -> Self {
        Self {
            alive: pids.into_iter().collect(),
            fail_termination: BTreeSet::new(),
        }
    }

    fn with_failure(mut self, pid: u32) -> Self {
        self.fail_termination.insert(pid);
        self
    }
}

impl observability::RuntimeProcessLiveness for FakeCleanupController {
    fn is_process_alive(&mut self, pid: u32) -> bool {
        self.alive.contains(&pid)
    }
}

impl observability::RuntimeProcessTerminator for FakeCleanupController {
    fn request_termination(&mut self, pid: u32) -> Result<bool, String> {
        if self.fail_termination.contains(&pid) {
            return Err(format!("pid {pid} refused termination"));
        }
        Ok(self.alive.contains(&pid))
    }
}
