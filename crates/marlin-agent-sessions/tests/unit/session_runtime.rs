use marlin_agent_sessions::{
    RuntimeBlockingBridgeStrategy, RuntimeFanoutJoinPolicy, RuntimeTaskTrackerPolicy,
    SessionRuntimeSnapshot, TokioRuntimeDiagnosticsPolicy, TokioRuntimeFlavor, TokioRuntimePolicy,
    TokioRuntimePolicyReceipt,
};

#[test]
fn runtime_snapshot_defaults_to_stable_production_policy() {
    let snapshot = SessionRuntimeSnapshot::new("runtime/root", TokioRuntimePolicy::default());

    assert_eq!(snapshot.session_id().as_str(), "runtime/root");
    assert_eq!(
        snapshot.tokio_policy().flavor(),
        &TokioRuntimeFlavor::MultiThread
    );
    assert!(!snapshot.tokio_policy().diagnostics().enabled());
    assert_eq!(
        snapshot.blocking_bridge().strategy(),
        &RuntimeBlockingBridgeStrategy::MultiThreadBlockInPlace
    );
    assert!(snapshot.blocking_bridge().blocking_non_cancellable());
}

#[test]
fn current_thread_runtime_uses_spawn_blocking_bridge_policy() {
    let snapshot =
        SessionRuntimeSnapshot::new("runtime/current", TokioRuntimePolicy::current_thread());

    assert_eq!(
        snapshot.tokio_policy().flavor(),
        &TokioRuntimeFlavor::CurrentThread
    );
    assert_eq!(
        snapshot.blocking_bridge().strategy(),
        &RuntimeBlockingBridgeStrategy::SpawnBlocking
    );
}

#[test]
fn runtime_policy_receipt_records_unstable_diagnostics_requests() {
    let policy = TokioRuntimePolicy::production_default()
        .with_shutdown_timeout_ms(Some(1_500))
        .with_diagnostics(TokioRuntimeDiagnosticsPolicy::stable_metrics().with_taskdump());
    let snapshot = SessionRuntimeSnapshot::new("runtime/diagnostic", policy);

    let receipt = TokioRuntimePolicyReceipt::from_snapshot(&snapshot);

    assert_eq!(receipt.session_id().as_str(), "runtime/diagnostic");
    assert_eq!(receipt.shutdown_timeout_ms(), Some(1_500));
    assert!(receipt.diagnostics_enabled());
    assert!(receipt.unstable_diagnostics_requested());
    assert!(
        receipt
            .effective_policy()
            .diagnostics()
            .taskdump_requested()
    );
}

#[test]
fn runtime_snapshot_carries_task_tracking_and_fanout_policy() {
    let snapshot = SessionRuntimeSnapshot::new("runtime/fanout", TokioRuntimePolicy::default())
        .with_task_tracker(RuntimeTaskTrackerPolicy::cancel_on_shutdown(Some(250)))
        .with_fanout_join(
            RuntimeFanoutJoinPolicy::bounded(4)
                .with_cancel_on_first_error(false)
                .with_retain_completion_order(true)
                .with_shutdown_timeout_ms(Some(500)),
        );

    assert!(snapshot.task_tracker().cancel_on_shutdown_enabled());
    assert_eq!(snapshot.task_tracker().shutdown_timeout_ms(), Some(250));
    assert_eq!(snapshot.fanout_join().max_parallelism(), Some(4));
    assert!(!snapshot.fanout_join().cancel_on_first_error());
    assert!(snapshot.fanout_join().preserve_input_order());
    assert!(snapshot.fanout_join().retain_completion_order());
    assert_eq!(snapshot.fanout_join().shutdown_timeout_ms(), Some(500));
}
