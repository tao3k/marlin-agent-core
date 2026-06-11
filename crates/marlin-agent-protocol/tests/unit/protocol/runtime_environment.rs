use std::path::PathBuf;

use marlin_agent_protocol::{
    RuntimeConfigLayer, RuntimeConfigLayerSource, RuntimeEnvironment, RuntimeHome,
    RuntimeHomeSource, RuntimeSandboxPolicy,
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
