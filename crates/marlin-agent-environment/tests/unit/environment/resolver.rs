use std::path::PathBuf;

use marlin_agent_environment::{
    PROJECT_CONFIG_PRECEDENCE, RuntimeEnvironmentError, RuntimeEnvironmentRequest,
    RuntimeEnvironmentResolver, SESSION_FLAGS_CONFIG_PRECEDENCE, SUB_AGENT_CONFIG_PRECEDENCE,
    SYSTEM_CONFIG_PRECEDENCE, SubAgentEnvironmentRequest, USER_CONFIG_PRECEDENCE,
};
use marlin_agent_protocol::{
    RuntimeConfigLayerSource, RuntimeEnvironment, RuntimeEnvironmentActivation,
    RuntimeEnvironmentActivationPolicy, RuntimeEnvironmentActivationStatus, RuntimeEnvrcPolicy,
    RuntimeHomeSource, RuntimeSandboxPolicy, RuntimeShellIsolationPolicy,
};

#[test]
fn resolver_prefers_custom_home_and_orders_config_layers() {
    let sandbox = RuntimeSandboxPolicy {
        writable_roots: vec![PathBuf::from("/repo")],
        network_access: true,
        exclude_tmpdir_env_var: true,
        exclude_slash_tmp: false,
    };
    let request = RuntimeEnvironmentRequest::default()
        .with_default_home("/home/default")
        .with_custom_home("/home/custom")
        .with_profile("work")
        .with_cwd("/repo")
        .with_sandbox(sandbox.clone())
        .with_system_config("/etc/marlin/config.toml")
        .with_user_config("/home/user/.marlin/config.toml")
        .with_project_config("/repo/.marlin")
        .with_session_flags();

    let environment = RuntimeEnvironmentResolver::new().resolve(request);

    let home = environment.home.expect("custom home should be resolved");
    assert_eq!(home.path, PathBuf::from("/home/custom"));
    assert_eq!(home.source, RuntimeHomeSource::Custom);
    assert_eq!(home.profile.as_deref(), Some("work"));
    assert_eq!(environment.cwd, Some(PathBuf::from("/repo")));
    assert_eq!(environment.sandbox, sandbox);
    assert_eq!(
        environment
            .config_layers
            .iter()
            .map(|layer| layer.precedence)
            .collect::<Vec<_>>(),
        vec![
            SYSTEM_CONFIG_PRECEDENCE,
            USER_CONFIG_PRECEDENCE,
            PROJECT_CONFIG_PRECEDENCE,
            SESSION_FLAGS_CONFIG_PRECEDENCE,
        ]
    );
    assert!(matches!(
        environment.config_layers[1].source,
        RuntimeConfigLayerSource::User { .. }
    ));
}

#[test]
fn resolver_records_default_home_when_custom_home_is_absent() {
    let environment = RuntimeEnvironmentResolver::new().resolve(
        RuntimeEnvironmentRequest::default()
            .with_default_home("/home/default")
            .with_profile("default"),
    );

    let home = environment.home.expect("default home should be resolved");
    assert_eq!(home.path, PathBuf::from("/home/default"));
    assert_eq!(home.source, RuntimeHomeSource::Default);
    assert_eq!(home.profile.as_deref(), Some("default"));
}

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

#[test]
fn sub_agent_environment_inherits_parent_home_and_layers() {
    let parent = RuntimeEnvironmentResolver::new().resolve(
        RuntimeEnvironmentRequest::default()
            .with_custom_home("/home/marlin")
            .with_profile("parent")
            .with_cwd("/repo")
            .with_user_config("/home/user/.marlin/config.toml")
            .with_session_flags(),
    );
    let child = RuntimeEnvironmentResolver::new()
        .resolve_sub_agent(
            &parent,
            SubAgentEnvironmentRequest::new("review/agent").with_cwd("/repo/sub"),
        )
        .expect("parent home should allow sub-agent resolution");

    assert_eq!(
        parent.home.as_ref().expect("parent home").path,
        PathBuf::from("/home/marlin")
    );
    let child_home = child.home.expect("child home should be resolved");
    assert_eq!(
        child_home.path,
        PathBuf::from("/home/marlin/sub-agents/review-agent")
    );
    assert_eq!(child_home.profile.as_deref(), Some("parent"));
    assert_eq!(
        child_home.source,
        RuntimeHomeSource::InheritedSubAgent {
            parent_home: PathBuf::from("/home/marlin"),
        }
    );
    assert_eq!(child.cwd, Some(PathBuf::from("/repo/sub")));
    assert_eq!(
        child
            .config_layers
            .iter()
            .map(|layer| layer.precedence)
            .collect::<Vec<_>>(),
        vec![
            USER_CONFIG_PRECEDENCE,
            SUB_AGENT_CONFIG_PRECEDENCE,
            SESSION_FLAGS_CONFIG_PRECEDENCE,
        ]
    );
    assert!(matches!(
        child.config_layers[1].source,
        RuntimeConfigLayerSource::SubAgent { .. }
    ));
}

#[test]
fn sub_agent_environment_inherits_parent_activation_policy() {
    let activation = RuntimeEnvironmentActivationPolicy::direnv_project()
        .with_shell(RuntimeShellIsolationPolicy::isolated().with_allowed("PATH"));
    let parent = RuntimeEnvironmentResolver::new().resolve(
        RuntimeEnvironmentRequest::default()
            .with_custom_home("/home/marlin")
            .with_cwd("/repo")
            .with_activation(activation.clone()),
    );
    let child = RuntimeEnvironmentResolver::new()
        .resolve_sub_agent(&parent, SubAgentEnvironmentRequest::new("review/agent"))
        .expect("parent home should allow sub-agent resolution");

    assert_eq!(child.activation, activation);
}

#[test]
fn sub_agent_environment_requires_parent_home() {
    let error = RuntimeEnvironmentResolver::new()
        .resolve_sub_agent(
            &RuntimeEnvironment::default(),
            SubAgentEnvironmentRequest::new("reviewer"),
        )
        .expect_err("missing parent home should be rejected");

    assert_eq!(error, RuntimeEnvironmentError::MissingParentHome);
}
