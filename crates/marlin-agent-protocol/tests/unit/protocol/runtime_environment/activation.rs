use std::path::PathBuf;

use marlin_agent_protocol::{
    RuntimeEnvironment, RuntimeEnvironmentActivation, RuntimeEnvironmentActivationAction,
    RuntimeEnvironmentActivationActionReceipt, RuntimeEnvironmentActivationActionStatus,
    RuntimeEnvironmentActivationPolicy, RuntimeEnvironmentActivationReceipt,
    RuntimeEnvironmentActivationStatus, RuntimeEnvironmentDelta, RuntimeEnvrcPolicy,
    RuntimeShellIsolationPolicy,
};

#[test]
fn runtime_environment_records_direnv_activation_and_shell_isolation() {
    let activation = RuntimeEnvironmentActivationPolicy::direnv_project().with_shell(
        RuntimeShellIsolationPolicy::isolated()
            .with_allowed("PATH")
            .with_denied("AWS_SECRET_ACCESS_KEY"),
    );

    let environment = RuntimeEnvironment::default().with_activation(activation.clone());

    assert_eq!(environment.activation, activation);
    assert!(matches!(
        environment.activation.activation,
        RuntimeEnvironmentActivation::Direnv {
            envrc: RuntimeEnvrcPolicy::Project,
            capture_delta: true,
        }
    ));
    assert!(environment.activation.shell.isolate_host_environment);
    assert_eq!(environment.activation.shell.allowlist, vec!["PATH"]);
    assert_eq!(
        environment.activation.shell.denylist,
        vec!["AWS_SECRET_ACCESS_KEY"]
    );
}

#[test]
fn runtime_environment_records_direnv_reload_preflight_action() {
    let activation = RuntimeEnvironmentActivationPolicy::direnv_project().with_direnv_reload();

    assert_eq!(
        activation.preflight_actions,
        vec![RuntimeEnvironmentActivationAction::DirenvReload]
    );
}

#[test]
fn runtime_environment_can_name_explicit_envrc_file() {
    let activation = RuntimeEnvironmentActivationPolicy::direnv_file("/repo/.envrc");

    assert!(matches!(
        activation.activation,
        RuntimeEnvironmentActivation::Direnv {
            envrc: RuntimeEnvrcPolicy::Explicit { ref file },
            capture_delta: true,
        } if file == &PathBuf::from("/repo/.envrc")
    ));
}

#[test]
fn runtime_environment_activation_receipt_records_status_and_policy() {
    let policy = RuntimeEnvironmentActivationPolicy::direnv_project()
        .with_shell(RuntimeShellIsolationPolicy::isolated().with_allowed("PATH"));
    let receipt = RuntimeEnvironmentActivationReceipt::planned(&policy);

    assert_eq!(receipt.status, RuntimeEnvironmentActivationStatus::Planned);
    assert_eq!(receipt.activation, policy.activation);
    assert_eq!(receipt.shell, policy.shell);
    assert!(receipt.delta.is_empty());
    assert!(receipt.actions.is_empty());
    assert_eq!(receipt.reason, None);
}

#[test]
fn runtime_environment_activation_receipt_records_direnv_actions() {
    let policy = RuntimeEnvironmentActivationPolicy::direnv_project().with_direnv_reload();
    let receipt = RuntimeEnvironmentActivationReceipt::applied_with_actions(
        &policy,
        RuntimeEnvironmentDelta::default(),
        vec![RuntimeEnvironmentActivationActionReceipt::rejected(
            RuntimeEnvironmentActivationAction::DirenvReload,
            "direnv reload failed",
        )],
    );

    assert_eq!(receipt.status, RuntimeEnvironmentActivationStatus::Applied);
    assert_eq!(receipt.actions.len(), 1);
    assert_eq!(
        receipt.actions[0].status,
        RuntimeEnvironmentActivationActionStatus::Rejected
    );
    assert_eq!(
        receipt.actions[0].reason.as_deref(),
        Some("direnv reload failed")
    );
}
