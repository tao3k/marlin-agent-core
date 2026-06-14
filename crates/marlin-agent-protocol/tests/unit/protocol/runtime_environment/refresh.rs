use marlin_agent_protocol::{
    RuntimeEnvironmentActivationAction, RuntimeEnvironmentActivationActionReceipt,
    RuntimeEnvironmentActivationPolicy, RuntimeEnvironmentActivationReceipt,
    RuntimeEnvironmentDelta, RuntimeEnvironmentRefreshCachePolicy,
    RuntimeEnvironmentRefreshExecution, RuntimeEnvironmentRefreshReceipt,
    RuntimeEnvironmentRefreshStatus, RuntimeEnvironmentRefreshTimeout,
};

#[test]
fn runtime_environment_refresh_receipt_records_external_cache_ownership() {
    let policy = RuntimeEnvironmentActivationPolicy::direnv_project().with_direnv_reload();
    let activation_receipt = RuntimeEnvironmentActivationReceipt::applied_with_actions(
        &policy,
        RuntimeEnvironmentDelta::default(),
        vec![RuntimeEnvironmentActivationActionReceipt::applied(
            RuntimeEnvironmentActivationAction::DirenvReload,
        )],
    );
    let refresh_receipt = RuntimeEnvironmentRefreshReceipt::from_activation(
        RuntimeEnvironmentRefreshExecution::Background,
        RuntimeEnvironmentRefreshCachePolicy::ExternalToolOwned,
        activation_receipt,
    );

    assert_eq!(
        refresh_receipt.execution,
        RuntimeEnvironmentRefreshExecution::Background
    );
    assert_eq!(
        refresh_receipt.cache_policy,
        RuntimeEnvironmentRefreshCachePolicy::ExternalToolOwned
    );
    assert_eq!(
        refresh_receipt.status,
        RuntimeEnvironmentRefreshStatus::Applied
    );
    assert_eq!(
        refresh_receipt.activation_receipt.actions[0].action,
        RuntimeEnvironmentActivationAction::DirenvReload
    );
    assert_eq!(refresh_receipt.reason, None);
}

#[test]
fn runtime_environment_refresh_receipt_records_typed_timeout_request() {
    let policy = RuntimeEnvironmentActivationPolicy::direnv_project();
    let activation_receipt =
        RuntimeEnvironmentActivationReceipt::rejected(&policy, "direnv timed out");
    let refresh_receipt = RuntimeEnvironmentRefreshReceipt::timed_out(
        RuntimeEnvironmentRefreshTimeout::new(activation_receipt, 1_500, 1_000)
            .with_execution(RuntimeEnvironmentRefreshExecution::Background)
            .with_cache_policy(RuntimeEnvironmentRefreshCachePolicy::ExternalToolOwned),
    );

    assert_eq!(
        refresh_receipt.execution,
        RuntimeEnvironmentRefreshExecution::Background
    );
    assert_eq!(
        refresh_receipt.cache_policy,
        RuntimeEnvironmentRefreshCachePolicy::ExternalToolOwned
    );
    assert_eq!(
        refresh_receipt.status,
        RuntimeEnvironmentRefreshStatus::TimedOut
    );
    assert_eq!(refresh_receipt.elapsed_ms, Some(1_500));
    assert_eq!(refresh_receipt.timeout_ms, Some(1_000));
    assert_eq!(refresh_receipt.reason.as_deref(), Some("direnv timed out"));
}
