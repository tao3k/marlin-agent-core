use std::{collections::BTreeMap, path::PathBuf};

use marlin_agent_protocol::{
    RuntimeConfigLayer, RuntimeConfigLayerSource, RuntimeEnvironment,
    RuntimeEnvironmentActivationPolicy, RuntimeEnvironmentDelta, RuntimeHome, RuntimeHomeSource,
    RuntimeSandboxPolicy,
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
    assert!(environment.workspace_projects.is_empty());
    assert_eq!(environment.active_workspace_project, None);
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
