use marlin_agent_runtime::observability;

use super::RecordingSubscriber;

#[test]
fn runtime_process_registry_drops_finished_processes_from_active_tracking() {
    let mut registry = observability::RuntimeProcessRegistry::new();
    registry.track(
        observability::RuntimeProcessObservation::new(
            42,
            observability::RuntimeProcessKind::Tool,
            "tool:apply",
        )
        .with_started_at_ms(1),
    );

    let finished = registry.finish(42, 10).expect("process is tracked");

    assert_eq!(finished.pid, 42);
    assert_eq!(
        finished.status,
        observability::RuntimeProcessStatus::Finished
    );
    assert_eq!(finished.last_observed_at_ms(), Some(10));
    assert!(registry.get(42).is_none());
    assert!(registry.active_processes().is_empty());
}

#[test]
fn runtime_process_registry_rejects_duplicate_active_handle_and_allows_reuse_after_exit() {
    let mut registry = observability::RuntimeProcessRegistry::new();
    let handle = observability::RuntimeProcessHandle::new("tool:apply");
    registry.track(
        observability::RuntimeProcessObservation::new(
            42,
            observability::RuntimeProcessKind::Tool,
            "tool:apply:first",
        )
        .with_handle(handle.clone())
        .with_started_at_ms(1),
    );

    let duplicate = registry
        .try_track(
            observability::RuntimeProcessObservation::new(
                43,
                observability::RuntimeProcessKind::Tool,
                "tool:apply:second",
            )
            .with_handle(handle.clone())
            .with_started_at_ms(2),
        )
        .expect_err("duplicate active handles should be rejected");

    assert_eq!(
        duplicate,
        observability::RuntimeProcessRegistrationError::DuplicateActiveHandle {
            handle: handle.clone(),
            existing_pid: 42,
            new_pid: 43,
        }
    );
    assert_eq!(
        registry.get_by_handle(&handle).map(|process| process.pid),
        Some(42)
    );

    registry.finish(42, 10).expect("process is tracked");
    registry
        .try_track(
            observability::RuntimeProcessObservation::new(
                44,
                observability::RuntimeProcessKind::Tool,
                "tool:apply:third",
            )
            .with_handle(handle.clone())
            .with_started_at_ms(11),
        )
        .expect("finished handles should be reusable");

    assert_eq!(
        registry.get_by_handle(&handle).map(|process| process.pid),
        Some(44)
    );
}

#[test]
fn runtime_process_registry_reports_orphan_cleanup_candidates() {
    let mut registry = observability::RuntimeProcessRegistry::new();
    registry.track(
        observability::RuntimeProcessObservation::new(
            100,
            observability::RuntimeProcessKind::SubAgent,
            "sub-agent:review",
        )
        .with_started_at_ms(1),
    );
    registry.track(
        observability::RuntimeProcessObservation::new(
            101,
            observability::RuntimeProcessKind::Tool,
            "tool:cache-writer",
        )
        .with_started_at_ms(2),
    );
    registry
        .mark_orphaned(100, 30)
        .expect("sub-agent process is tracked");
    registry
        .request_cleanup(101, 31)
        .expect("tool process is tracked");

    let candidates = registry.cleanup_candidates();

    assert_eq!(candidates.len(), 2);
    assert_eq!(
        registry.get(100).map(|process| &process.status),
        Some(&observability::RuntimeProcessStatus::Orphaned)
    );
    assert_eq!(
        registry.get(101).map(|process| &process.status),
        Some(&observability::RuntimeProcessStatus::CleanupRequested)
    );
}

#[test]
fn runtime_process_registry_reports_count_only_snapshot() {
    let registry = observability::RuntimeProcessRegistryHandle::new();
    registry.track(
        observability::RuntimeProcessObservation::new(
            200,
            observability::RuntimeProcessKind::Tool,
            "tool:running",
        )
        .with_started_at_ms(1),
    );
    registry.track(
        observability::RuntimeProcessObservation::new(
            201,
            observability::RuntimeProcessKind::SubAgent,
            "sub-agent:orphaned",
        )
        .with_started_at_ms(2),
    );
    registry.track(
        observability::RuntimeProcessObservation::new(
            202,
            observability::RuntimeProcessKind::Provider,
            "provider:cleanup",
        )
        .with_started_at_ms(3),
    );
    registry.track(
        observability::RuntimeProcessObservation::new(
            203,
            observability::RuntimeProcessKind::Hook,
            "hook:finished",
        )
        .with_started_at_ms(4),
    );
    registry
        .mark_orphaned(201, 20)
        .expect("sub-agent process is tracked");
    registry
        .request_cleanup(202, 21)
        .expect("provider process is tracked");
    registry.finish(203, 22).expect("hook process is tracked");

    let snapshot = registry.snapshot(30);

    assert_eq!(snapshot.observed_at_ms, 30);
    assert_eq!(snapshot.active_count, 3);
    assert_eq!(snapshot.cleanup_candidate_count, 2);
    assert_eq!(snapshot.status_counts.running, 1);
    assert_eq!(snapshot.status_counts.finished, 0);
    assert_eq!(snapshot.status_counts.failed, 0);
    assert_eq!(snapshot.status_counts.orphaned, 1);
    assert_eq!(snapshot.status_counts.cleanup_requested, 1);
    assert_eq!(snapshot.kind_counts.tool, 1);
    assert_eq!(snapshot.kind_counts.sub_agent, 1);
    assert_eq!(snapshot.kind_counts.provider, 1);
    assert_eq!(snapshot.kind_counts.hook, 0);
    assert_eq!(snapshot.kind_counts.other, 0);
}

#[test]
fn runtime_process_registry_emits_structured_lifecycle_events() {
    let subscriber = RecordingSubscriber::new();
    let _guard = tracing::subscriber::set_default(subscriber.clone());
    let mut registry = observability::RuntimeProcessRegistry::new();
    registry.track(
        observability::RuntimeProcessObservation::new(
            500,
            observability::RuntimeProcessKind::Tool,
            "tool:apply",
        )
        .with_handle("tool:apply:500")
        .with_started_at_ms(1),
    );
    registry.mark_orphaned(500, 10).expect("process is tracked");
    registry
        .request_cleanup(500, 11)
        .expect("process is tracked");
    registry.finish(500, 12).expect("process is tracked");

    let events = subscriber.events();
    let tracked = events
        .iter()
        .find(|event| {
            event.target == observability::TARGET_RUNTIME_PROCESS
                && event.has_value(
                    observability::FIELD_PROCESS_EVENT,
                    observability::PROCESS_EVENT_TRACKED,
                )
        })
        .expect("process tracking event should be recorded");
    assert!(tracked.has_field(observability::FIELD_PROCESS_ID));
    assert!(tracked.has_field(observability::FIELD_PROCESS_STATUS));
    assert!(tracked.has_field(observability::FIELD_PROCESS_KIND));
    assert!(tracked.has_field(observability::FIELD_PROCESS_HANDLE));
    assert!(tracked.has_field(observability::FIELD_OWNER_REFERENCE));
    assert!(tracked.has_field(observability::FIELD_AGENT_REFERENCE));
    assert!(tracked.has_field(observability::FIELD_SUB_AGENT_SOURCE));
    assert!(tracked.has_value(
        observability::FIELD_RUNTIME_KIND,
        observability::RUNTIME_KIND_TOOL
    ));

    assert!(events.iter().any(|event| {
        event.target == observability::TARGET_RUNTIME_PROCESS
            && event.has_value(
                observability::FIELD_PROCESS_EVENT,
                observability::PROCESS_EVENT_ORPHANED,
            )
    }));
    assert!(events.iter().any(|event| {
        event.target == observability::TARGET_RUNTIME_PROCESS
            && event.has_value(
                observability::FIELD_PROCESS_EVENT,
                observability::PROCESS_EVENT_CLEANUP_REQUESTED,
            )
    }));
    assert!(events.iter().any(|event| {
        event.target == observability::TARGET_RUNTIME_PROCESS
            && event.has_value(
                observability::FIELD_PROCESS_EVENT,
                observability::PROCESS_EVENT_FINISHED,
            )
    }));
}
