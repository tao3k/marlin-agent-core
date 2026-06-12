use std::{collections::BTreeSet, sync::mpsc, time::Duration};

use marlin_agent_runtime::{SubAgentSpawnProfile, observability};

use super::RecordingSubscriber;

#[test]
fn runtime_process_registry_sweeps_stale_and_live_cleanup_candidates() {
    let subscriber = RecordingSubscriber::new();
    let _guard = tracing::subscriber::set_default(subscriber.clone());
    let mut registry = observability::RuntimeProcessRegistry::new();
    registry.track(
        observability::RuntimeProcessObservation::new(
            100,
            observability::RuntimeProcessKind::SubAgent,
            "sub-agent:stale",
        )
        .with_command(
            observability::RuntimeCommandObservation::new("sub-agent-command")
                .with_argv(["marlin-subagent", "--role", "research"])
                .with_sub_agent_source("kernel-node")
                .with_sub_agent_profile(
                    SubAgentSpawnProfile::new("researcher", "asp_explorer", "research")
                        .with_nickname("Galileo"),
                ),
        )
        .with_started_at_ms(1),
    );
    registry.track(
        observability::RuntimeProcessObservation::new(
            101,
            observability::RuntimeProcessKind::Tool,
            "tool:live",
        )
        .with_command(
            observability::RuntimeCommandObservation::new("tool-command")
                .with_argv(["sleep", "10"]),
        )
        .with_started_at_ms(2),
    );
    registry.mark_orphaned(100, 30);
    registry.request_cleanup(101, 31);
    let mut controller = FakeCleanupController::with_alive([101]);

    let sweep = registry.sweep_cleanup_candidates(&mut controller, 50);

    assert_eq!(sweep.observed_at_ms, 50);
    assert_eq!(sweep.removed_stale.len(), 1);
    assert_eq!(sweep.removed_stale[0].pid, 100);
    assert_eq!(sweep.removed_stale[0].last_observed_at_ms(), Some(50));
    assert_eq!(sweep.termination_requested.len(), 1);
    assert_eq!(sweep.termination_requested[0].pid, 101);
    assert!(sweep.termination_failed.is_empty());
    assert_eq!(controller.terminated, vec![101]);
    assert!(registry.get(100).is_none());
    assert_eq!(
        registry.get(101).map(|process| process.status.clone()),
        Some(observability::RuntimeProcessStatus::CleanupRequested)
    );
    assert_eq!(
        registry
            .get(101)
            .and_then(|process| process.last_observed_at_ms()),
        Some(50)
    );
    let receipt = sweep.command_receipt();
    assert_eq!(receipt.observed_at_ms, 50);
    assert_eq!(receipt.entries.len(), 2);
    let stale = receipt
        .entries
        .iter()
        .find(|entry| entry.pid == 100)
        .expect("stale command receipt entry");
    assert_eq!(
        stale.outcome,
        observability::RuntimeCommandCleanupOutcome::RemovedStale
    );
    assert_eq!(stale.cleanup_attempts, 0);
    assert_eq!(stale.last_cleanup_attempt_at_ms, None);
    assert!(!stale.retained_in_registry);
    assert!(!stale.requires_follow_up);
    assert!(!stale.force_cleanup_recommended);
    assert_eq!(stale.kind, observability::RuntimeProcessKind::SubAgent);
    assert_eq!(
        stale
            .command
            .as_ref()
            .and_then(|command| command.sub_agent_source.as_deref()),
        Some("kernel-node")
    );
    assert_eq!(
        stale
            .command
            .as_ref()
            .and_then(|command| command.sub_agent_role.as_deref()),
        Some("research")
    );
    let stale_profile = stale
        .command
        .as_ref()
        .and_then(|command| command.sub_agent_profile.as_ref())
        .expect("sub-agent cleanup receipt should preserve profile");
    assert_eq!(stale_profile.profile_id, "researcher");
    assert_eq!(stale_profile.agent_type.as_str(), "asp_explorer");
    assert_eq!(stale_profile.role, "research");
    assert_eq!(stale_profile.nickname.as_deref(), Some("Galileo"));
    let live = receipt
        .entries
        .iter()
        .find(|entry| entry.pid == 101)
        .expect("live command receipt entry");
    assert_eq!(
        live.outcome,
        observability::RuntimeCommandCleanupOutcome::TerminationRequested
    );
    assert_eq!(live.cleanup_attempts, 1);
    assert_eq!(live.last_cleanup_attempt_at_ms, Some(50));
    assert!(live.retained_in_registry);
    assert!(live.requires_follow_up);
    assert!(!live.force_cleanup_recommended);
    assert_eq!(
        live.command.as_ref().map(|command| command.argv.clone()),
        Some(vec!["sleep".to_string(), "10".to_string()])
    );
    assert_eq!(live.error, None);

    let events = subscriber.events();
    let removed_stale = events
        .iter()
        .find(|event| {
            event.target == observability::TARGET_RUNTIME_PROCESS
                && event.has_value(
                    observability::FIELD_PROCESS_EVENT,
                    observability::PROCESS_EVENT_REMOVED_STALE,
                )
        })
        .expect("removed-stale cleanup event should be recorded");
    assert!(removed_stale.has_field(observability::FIELD_CLEANUP_OUTCOME));
    assert!(removed_stale.has_field(observability::FIELD_CLEANUP_ATTEMPTS));
    assert!(removed_stale.has_field(observability::FIELD_RETAINED_IN_REGISTRY));
    assert!(removed_stale.has_field(observability::FIELD_REQUIRES_FOLLOW_UP));
    assert!(removed_stale.has_field(observability::FIELD_FORCE_CLEANUP_RECOMMENDED));

    assert!(events.iter().any(|event| {
        event.target == observability::TARGET_RUNTIME_PROCESS
            && event.has_value(
                observability::FIELD_PROCESS_EVENT,
                observability::PROCESS_EVENT_TERMINATION_REQUESTED,
            )
            && event.has_value(observability::FIELD_RETAINED_IN_REGISTRY, "true")
            && event.has_value(observability::FIELD_REQUIRES_FOLLOW_UP, "true")
            && event.has_value(observability::FIELD_FORCE_CLEANUP_RECOMMENDED, "false")
            && event.has_value(observability::FIELD_CLEANUP_ATTEMPTS, "1")
    }));

    let sweep_summary = events
        .iter()
        .find(|event| {
            event.target == observability::TARGET_RUNTIME_PROCESS
                && event.has_value(
                    observability::FIELD_PROCESS_EVENT,
                    observability::PROCESS_EVENT_CLEANUP_SWEEP,
                )
        })
        .expect("cleanup sweep summary event should be recorded");
    assert!(sweep_summary.has_value(observability::FIELD_OBSERVED_AT_MS, "50"));
    assert!(sweep_summary.has_value(observability::FIELD_CLEANUP_CANDIDATE_COUNT, "2"));
    assert!(sweep_summary.has_value(observability::FIELD_REMOVED_STALE_COUNT, "1"));
    assert!(sweep_summary.has_value(observability::FIELD_TERMINATION_REQUESTED_COUNT, "1"));
    assert!(sweep_summary.has_value(observability::FIELD_TERMINATION_FAILED_COUNT, "0"));
}

#[test]
fn runtime_process_registry_retains_cleanup_candidate_when_termination_fails() {
    let subscriber = RecordingSubscriber::new();
    let _guard = tracing::subscriber::set_default(subscriber.clone());
    let mut registry = observability::RuntimeProcessRegistry::new();
    registry.track(
        observability::RuntimeProcessObservation::new(
            200,
            observability::RuntimeProcessKind::Provider,
            "provider:stuck",
        )
        .with_command(
            observability::RuntimeCommandObservation::new("provider-command")
                .with_argv(["provider", "serve"]),
        )
        .with_started_at_ms(1),
    );
    registry.request_cleanup(200, 40);
    let mut controller = FakeCleanupController::with_alive([200]).with_failure(200);

    let sweep = registry.sweep_cleanup_candidates(&mut controller, 60);

    assert!(sweep.removed_stale.is_empty());
    assert!(sweep.termination_requested.is_empty());
    assert_eq!(sweep.termination_failed.len(), 1);
    assert_eq!(sweep.termination_failed[0].process.pid, 200);
    assert_eq!(
        sweep.termination_failed[0].error,
        "pid 200 refused termination"
    );
    assert_eq!(
        registry.get(200).map(|process| process.status.clone()),
        Some(observability::RuntimeProcessStatus::CleanupRequested)
    );
    assert_eq!(
        registry
            .get(200)
            .and_then(|process| process.last_observed_at_ms()),
        Some(60)
    );
    assert_eq!(registry.cleanup_candidates().len(), 1);
    let receipt = sweep.command_receipt();
    assert_eq!(receipt.entries.len(), 1);
    assert_eq!(
        receipt.entries[0].outcome,
        observability::RuntimeCommandCleanupOutcome::TerminationFailed
    );
    assert_eq!(receipt.entries[0].cleanup_attempts, 1);
    assert_eq!(receipt.entries[0].last_cleanup_attempt_at_ms, Some(60));
    assert!(receipt.entries[0].retained_in_registry);
    assert!(receipt.entries[0].requires_follow_up);
    assert!(receipt.entries[0].force_cleanup_recommended);
    assert_eq!(
        receipt.entries[0].error.as_deref(),
        Some("pid 200 refused termination")
    );
    assert_eq!(
        receipt.entries[0]
            .command
            .as_ref()
            .map(|command| command.command_kind.as_str()),
        Some("provider-command")
    );

    let termination_failed = subscriber
        .events()
        .into_iter()
        .find(|event| {
            event.target == observability::TARGET_RUNTIME_PROCESS
                && event.has_value(
                    observability::FIELD_PROCESS_EVENT,
                    observability::PROCESS_EVENT_TERMINATION_FAILED,
                )
        })
        .expect("termination failure should emit a cleanup event");
    assert!(termination_failed.has_field("error"));
    assert!(termination_failed.has_value(observability::FIELD_RETAINED_IN_REGISTRY, "true"));
    assert!(termination_failed.has_value(observability::FIELD_REQUIRES_FOLLOW_UP, "true"));
    assert!(termination_failed.has_value(observability::FIELD_FORCE_CLEANUP_RECOMMENDED, "true"));
    assert!(termination_failed.has_value(observability::FIELD_CLEANUP_ATTEMPTS, "1"));
}

#[test]
fn runtime_process_cleanup_policy_gates_force_cleanup_by_attempt_count() {
    let mut registry = observability::RuntimeProcessRegistry::new();
    registry.track(
        observability::RuntimeProcessObservation::new(
            250,
            observability::RuntimeProcessKind::Provider,
            "provider:retry-policy",
        )
        .with_started_at_ms(1),
    );
    registry.request_cleanup(250, 40);
    let policy = observability::RuntimeProcessCleanupPolicy::new(2);
    let mut controller = FakeCleanupController::with_alive([250]).with_failure(250);

    let first_sweep = registry.sweep_cleanup_candidates(&mut controller, 60);
    let first_receipt = first_sweep.command_receipt_with_policy(&policy);

    assert_eq!(first_receipt.entries.len(), 1);
    assert_eq!(first_receipt.entries[0].cleanup_attempts, 1);
    assert!(!first_receipt.entries[0].force_cleanup_recommended);
    assert_eq!(
        registry.get(250).map(|process| process.cleanup_attempts),
        Some(1)
    );

    let second_sweep = registry.sweep_cleanup_candidates(&mut controller, 70);
    let second_receipt = second_sweep.command_receipt_with_policy(&policy);

    assert_eq!(second_receipt.entries.len(), 1);
    assert_eq!(second_receipt.entries[0].cleanup_attempts, 2);
    assert_eq!(
        second_receipt.entries[0].last_cleanup_attempt_at_ms,
        Some(70)
    );
    assert!(second_receipt.entries[0].force_cleanup_recommended);
    assert_eq!(
        registry.get(250).map(|process| process.cleanup_attempts),
        Some(2)
    );
}

#[test]
fn runtime_process_registry_handle_releases_lock_before_cleanup_controller_calls() {
    let registry = observability::RuntimeProcessRegistryHandle::new();
    registry.track(
        observability::RuntimeProcessObservation::new(
            260,
            observability::RuntimeProcessKind::Tool,
            "tool:lock-granularity",
        )
        .with_started_at_ms(1),
    );
    registry
        .request_cleanup(260, 40)
        .expect("cleanup process is tracked");
    let sweep_registry = registry.clone();
    let controller_registry = registry.clone();
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let mut controller = SnapshottingCleanupController::new(controller_registry, [260]);
        let sweep = sweep_registry.sweep_cleanup_candidates(&mut controller, 80);
        tx.send((sweep, controller.snapshot_count))
            .expect("lock granularity test receiver should be alive");
    });

    let (sweep, snapshot_count) = rx
        .recv_timeout(Duration::from_secs(2))
        .expect("cleanup controller should be able to observe registry without deadlocking");
    assert!(sweep.removed_stale.is_empty());
    assert_eq!(sweep.termination_requested.len(), 1);
    assert_eq!(snapshot_count, 1);
    assert_eq!(
        registry.get(260).map(|process| process.cleanup_attempts),
        Some(1)
    );
}

#[test]
fn runtime_process_registry_handle_exposes_command_cleanup_receipt() {
    let registry = observability::RuntimeProcessRegistryHandle::new();
    registry.track(
        observability::RuntimeProcessObservation::new(
            300,
            observability::RuntimeProcessKind::SubAgent,
            "sub-agent:stale",
        )
        .with_command(
            observability::RuntimeCommandObservation::new("sub-agent-command")
                .with_argv(["marlin-subagent", "--role", "audit"])
                .with_sub_agent_profile(
                    SubAgentSpawnProfile::new("audit-profile", "asp_explorer", "audit")
                        .with_nickname("Selector"),
                ),
        )
        .with_started_at_ms(1),
    );
    registry.track(
        observability::RuntimeProcessObservation::new(
            301,
            observability::RuntimeProcessKind::Tool,
            "tool:live",
        )
        .with_command(
            observability::RuntimeCommandObservation::new("tool-command")
                .with_argv(["sleep", "30"]),
        )
        .with_started_at_ms(2),
    );
    registry
        .request_cleanup(300, 10)
        .expect("stale process is tracked");
    registry
        .request_cleanup(301, 11)
        .expect("live process is tracked");
    let mut controller = FakeCleanupController::with_alive([301]);

    let receipt = registry.sweep_cleanup_command_receipt(&mut controller, 70);

    assert_eq!(receipt.observed_at_ms, 70);
    assert_eq!(receipt.entries.len(), 2);
    assert_eq!(controller.terminated, vec![301]);
    assert!(registry.get(300).is_none());
    assert_eq!(
        registry.get(301).map(|process| process.status),
        Some(observability::RuntimeProcessStatus::CleanupRequested)
    );

    let stale = receipt
        .entries
        .iter()
        .find(|entry| entry.pid == 300)
        .expect("stale command cleanup receipt");
    assert_eq!(
        stale.outcome,
        observability::RuntimeCommandCleanupOutcome::RemovedStale
    );
    assert_eq!(stale.cleanup_attempts, 0);
    assert_eq!(stale.last_cleanup_attempt_at_ms, None);
    assert!(!stale.retained_in_registry);
    assert!(!stale.requires_follow_up);
    assert!(!stale.force_cleanup_recommended);
    assert_eq!(
        stale
            .command
            .as_ref()
            .and_then(|command| command.sub_agent_profile.as_ref())
            .map(|profile| profile.profile_id.as_str()),
        Some("audit-profile")
    );

    let live = receipt
        .entries
        .iter()
        .find(|entry| entry.pid == 301)
        .expect("live command cleanup receipt");
    assert_eq!(
        live.outcome,
        observability::RuntimeCommandCleanupOutcome::TerminationRequested
    );
    assert_eq!(live.cleanup_attempts, 1);
    assert_eq!(live.last_cleanup_attempt_at_ms, Some(70));
    assert!(live.retained_in_registry);
    assert!(live.requires_follow_up);
    assert!(!live.force_cleanup_recommended);
}

struct FakeCleanupController {
    alive: BTreeSet<u32>,
    fail_termination: BTreeSet<u32>,
    terminated: Vec<u32>,
}

impl FakeCleanupController {
    fn with_alive(pids: impl IntoIterator<Item = u32>) -> Self {
        Self {
            alive: pids.into_iter().collect(),
            fail_termination: BTreeSet::new(),
            terminated: Vec::new(),
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
        self.terminated.push(pid);
        Ok(self.alive.contains(&pid))
    }
}

struct SnapshottingCleanupController {
    registry: observability::RuntimeProcessRegistryHandle,
    alive: BTreeSet<u32>,
    snapshot_count: usize,
}

impl SnapshottingCleanupController {
    fn new(
        registry: observability::RuntimeProcessRegistryHandle,
        alive: impl IntoIterator<Item = u32>,
    ) -> Self {
        Self {
            registry,
            alive: alive.into_iter().collect(),
            snapshot_count: 0,
        }
    }
}

impl observability::RuntimeProcessLiveness for SnapshottingCleanupController {
    fn is_process_alive(&mut self, pid: u32) -> bool {
        self.alive.contains(&pid)
    }
}

impl observability::RuntimeProcessTerminator for SnapshottingCleanupController {
    fn request_termination(&mut self, pid: u32) -> Result<bool, String> {
        let snapshot = self.registry.snapshot(79);
        assert_eq!(snapshot.cleanup_candidate_count, 1);
        self.snapshot_count += 1;
        Ok(self.alive.contains(&pid))
    }
}
