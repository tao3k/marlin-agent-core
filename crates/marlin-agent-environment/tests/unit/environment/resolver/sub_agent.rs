use std::path::PathBuf;

use marlin_agent_environment::{
    RuntimeEnvironmentError, RuntimeEnvironmentRequest, RuntimeEnvironmentResolver,
    SESSION_FLAGS_CONFIG_PRECEDENCE, SUB_AGENT_CONFIG_PRECEDENCE, SubAgentEnvironmentRequest,
    USER_CONFIG_PRECEDENCE,
};
use marlin_agent_protocol::{
    RuntimeConfigLayerSource, RuntimeEnvironment, RuntimeEnvironmentActivationPolicy,
    RuntimeHomeSource, RuntimeShellIsolationPolicy,
};

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
