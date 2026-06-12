use std::{collections::BTreeMap, path::PathBuf};

use marlin_agent_protocol::{
    RuntimeConfigLayer, RuntimeConfigLayerSource, RuntimeEnvironment, RuntimeEnvironmentActivation,
    RuntimeEnvironmentActivationPolicy, RuntimeEnvironmentActivationReceipt,
    RuntimeEnvironmentActivationStatus, RuntimeEnvironmentDelta, RuntimeEnvrcPolicy, RuntimeHome,
    RuntimeHomeSource, RuntimeSandboxPolicy, RuntimeShellIsolationPolicy,
};

#[test]
fn runtime_environment_records_custom_home_layers_and_sandbox() {
    let home = RuntimeHome::custom("/tmp/marlin-home").with_profile("fast");
    let sandbox = RuntimeSandboxPolicy {
        writable_roots: vec![PathBuf::from("/tmp/work")],
        network_access: true,
        exclude_tmpdir_env_var: true,
        exclude_slash_tmp: false,
    };

    let environment = RuntimeEnvironment::default()
        .with_home(home.clone())
        .with_cwd("/tmp/workspace")
        .with_sandbox(sandbox.clone())
        .with_config_layer(RuntimeConfigLayer::new(
            RuntimeConfigLayerSource::Project {
                dot_marlin_folder: PathBuf::from("/tmp/workspace/.marlin"),
            },
            40,
        ))
        .with_config_layer(RuntimeConfigLayer::new(
            RuntimeConfigLayerSource::SessionFlags,
            100,
        ));

    assert_eq!(environment.home, Some(home));
    assert_eq!(environment.cwd, Some(PathBuf::from("/tmp/workspace")));
    assert_eq!(environment.sandbox, sandbox);
    assert_eq!(environment.config_layers.len(), 2);
    assert_eq!(environment.config_layers[1].precedence, 100);
}

#[test]
fn runtime_home_can_record_sub_agent_inheritance() {
    let home = RuntimeHome {
        path: PathBuf::from("/tmp/marlin-home/sub/reviewer"),
        source: RuntimeHomeSource::InheritedSubAgent {
            parent_home: PathBuf::from("/tmp/marlin-home"),
        },
        profile: Some("review".to_owned()),
    };

    assert!(matches!(
        home.source,
        RuntimeHomeSource::InheritedSubAgent { .. }
    ));
    assert_eq!(home.profile.as_deref(), Some("review"));
}

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
fn runtime_environment_defaults_missing_activation_when_deserializing() {
    let environment: RuntimeEnvironment = serde_json::from_str(
        r#"{
          "home": null,
          "cwd": "/repo",
          "sandbox": {
            "writable_roots": [],
            "network_access": false,
            "exclude_tmpdir_env_var": false,
            "exclude_slash_tmp": false
          },
          "config_layers": []
        }"#,
    )
    .expect("legacy environment without activation should deserialize");

    assert_eq!(environment.cwd, Some(PathBuf::from("/repo")));
    assert_eq!(
        environment.activation,
        RuntimeEnvironmentActivationPolicy::disabled()
    );
}

#[test]
fn runtime_environment_delta_records_names_without_values() {
    let before = BTreeMap::from([
        ("KEEP".to_owned(), "same".to_owned()),
        ("PATH".to_owned(), "/bin".to_owned()),
        ("SECRET_TOKEN".to_owned(), "old-secret".to_owned()),
    ]);
    let after = BTreeMap::from([
        ("KEEP".to_owned(), "same".to_owned()),
        ("NEW_VAR".to_owned(), "new-value".to_owned()),
        ("PATH".to_owned(), "/usr/bin".to_owned()),
    ]);

    let delta = RuntimeEnvironmentDelta::from_snapshots(&before, &after);
    let serialized = serde_json::to_string(&delta).expect("delta should serialize");

    assert_eq!(delta.added, vec!["NEW_VAR"]);
    assert_eq!(delta.changed, vec!["PATH"]);
    assert_eq!(delta.removed, vec!["SECRET_TOKEN"]);
    assert!(!serialized.contains("old-secret"));
    assert!(!serialized.contains("new-value"));
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
    assert_eq!(receipt.reason, None);
}
