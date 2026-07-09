use std::path::PathBuf;

use marlin_agent_environment::{
    HOST_HOME_ENV_VAR, PROJECT_CONFIG_PRECEDENCE, RuntimeEnvironmentRequest,
    RuntimeEnvironmentResolver, SESSION_FLAGS_CONFIG_PRECEDENCE, SYSTEM_CONFIG_PRECEDENCE,
    USER_CONFIG_PRECEDENCE,
};
use marlin_agent_protocol::{
    MARLIN_HOME_ENV_VAR, MARLIN_SESSION_ID_ENV_VAR, RuntimeConfigLayerSource, RuntimeHomeSource,
    RuntimeSandboxPolicy, RuntimeSessionIdSource, RuntimeStateDirectoryKind,
    RuntimeStateStorageStatus,
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
    assert_eq!(
        environment
            .state_layout
            .as_ref()
            .expect("state layout")
            .path_for(RuntimeStateDirectoryKind::Cache),
        Some(&PathBuf::from("/home/custom/cache"))
    );
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
fn resolver_prefers_marlin_home_env_over_home_default() {
    let resolution = RuntimeEnvironmentResolver::new().resolve_with_receipt(
        RuntimeEnvironmentRequest::default()
            .with_home_from_host_env([
                (HOST_HOME_ENV_VAR, "/home/user"),
                (MARLIN_HOME_ENV_VAR, "/state/marlin"),
            ])
            .with_profile("env"),
    );

    let home = resolution.environment.home.as_ref().expect("env home");
    let layout = resolution
        .environment
        .state_layout
        .as_ref()
        .expect("state layout");

    assert_eq!(home.path, PathBuf::from("/state/marlin"));
    assert_eq!(home.source, RuntimeHomeSource::Custom);
    assert_eq!(home.profile.as_deref(), Some("env"));
    assert_eq!(
        layout.path_for(RuntimeStateDirectoryKind::Receipts),
        Some(&PathBuf::from("/state/marlin/receipts"))
    );
    assert!(resolution.state_storage_receipt.is_some());
}

#[test]
fn resolver_uses_marlin_session_id_from_host_env() {
    let environment = RuntimeEnvironmentResolver::new().resolve(
        RuntimeEnvironmentRequest::default()
            .with_home_from_host_env([(MARLIN_SESSION_ID_ENV_VAR, "marlin-session-1")]),
    );

    let session = environment.session.expect("runtime session");
    assert_eq!(session.id.as_str(), "marlin-session-1");
    assert_eq!(session.source, RuntimeSessionIdSource::MarlinSessionEnv);
}

#[test]
fn resolver_uses_home_env_for_default_marlin_home() {
    let environment = RuntimeEnvironmentResolver::new().resolve(
        RuntimeEnvironmentRequest::default()
            .with_home_from_host_env([(HOST_HOME_ENV_VAR, "/home/user")]),
    );

    let home = environment.home.as_ref().expect("default env home");

    assert_eq!(home.path, PathBuf::from("/home/user/.marlin"));
    assert_eq!(home.source, RuntimeHomeSource::Default);
    assert_eq!(
        environment
            .state_layout
            .as_ref()
            .expect("state layout")
            .path_for(RuntimeStateDirectoryKind::Sessions),
        Some(&PathBuf::from("/home/user/.marlin/sessions"))
    );
}

#[test]
fn resolver_builds_default_marlin_home_state_layout_and_receipt() {
    let resolution = RuntimeEnvironmentResolver::new().resolve_with_receipt(
        RuntimeEnvironmentRequest::default()
            .with_default_marlin_home("/home/user")
            .with_profile("default"),
    );

    let home = resolution
        .environment
        .home
        .as_ref()
        .expect("default marlin home should be resolved");
    let layout = resolution
        .environment
        .state_layout
        .as_ref()
        .expect("state layout should be derived from the default home");
    let receipt = resolution
        .state_storage_receipt
        .as_ref()
        .expect("state storage receipt should be planned");

    assert_eq!(home.path, PathBuf::from("/home/user/.marlin"));
    assert_eq!(home.source, RuntimeHomeSource::Default);
    assert_eq!(home.profile.as_deref(), Some("default"));
    assert_eq!(
        layout.path_for(RuntimeStateDirectoryKind::Sessions),
        Some(&PathBuf::from("/home/user/.marlin/sessions"))
    );
    assert_eq!(receipt.status, RuntimeStateStorageStatus::Planned);
    assert_eq!(
        receipt.directories[0].path,
        PathBuf::from("/home/user/.marlin/config")
    );
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
    assert_eq!(
        environment
            .state_layout
            .as_ref()
            .expect("state layout")
            .path_for(RuntimeStateDirectoryKind::Memory),
        Some(&PathBuf::from("/home/default/memory"))
    );
}
