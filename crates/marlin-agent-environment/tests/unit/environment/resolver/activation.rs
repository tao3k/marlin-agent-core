use marlin_agent_environment::{RuntimeEnvironmentRequest, RuntimeEnvironmentResolver};
use marlin_agent_protocol::{
    RuntimeEnvironmentActivation, RuntimeEnvironmentActivationPolicy,
    RuntimeEnvironmentActivationStatus, RuntimeEnvrcPolicy, RuntimeShellIsolationPolicy,
};

#[test]
fn resolver_preserves_environment_activation_policy() {
    let activation = RuntimeEnvironmentActivationPolicy::direnv_project()
        .with_shell(RuntimeShellIsolationPolicy::isolated().with_allowed("PATH"));
    let environment = RuntimeEnvironmentResolver::new().resolve(
        RuntimeEnvironmentRequest::default()
            .with_cwd("/repo")
            .with_activation(activation.clone()),
    );

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
}

#[test]
fn resolver_returns_activation_receipt_for_direnv_policy() {
    let activation = RuntimeEnvironmentActivationPolicy::direnv_project();
    let resolution = RuntimeEnvironmentResolver::new().resolve_with_receipt(
        RuntimeEnvironmentRequest::default()
            .with_cwd("/repo")
            .with_activation(activation.clone()),
    );

    assert_eq!(resolution.environment.activation, activation);
    assert_eq!(
        resolution.activation_receipt.status,
        RuntimeEnvironmentActivationStatus::Planned
    );
    assert!(matches!(
        resolution.activation_receipt.activation,
        RuntimeEnvironmentActivation::Direnv {
            envrc: RuntimeEnvrcPolicy::Project,
            capture_delta: true,
        }
    ));
}

#[test]
fn resolver_returns_disabled_activation_receipt_by_default() {
    let resolution = RuntimeEnvironmentResolver::new()
        .resolve_with_receipt(RuntimeEnvironmentRequest::default());

    assert_eq!(
        resolution.activation_receipt.status,
        RuntimeEnvironmentActivationStatus::Disabled
    );
    assert_eq!(
        resolution.environment.activation,
        RuntimeEnvironmentActivationPolicy::disabled()
    );
}
