//! Reusable runtime environment fixtures for custom homes and sub-agents.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use async_trait::async_trait;
use marlin_agent_environment::{DirenvCommandRunner, RuntimeEnvironmentActivationError};
use marlin_agent_protocol::{
    RuntimeConfigLayer, RuntimeConfigLayerSource, RuntimeEnvironment, RuntimeHome,
    RuntimeHomeSource,
};

/// Fixture describing root, hook, and sub-agent runtime environments.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeEnvironmentFixture {
    root_environment: RuntimeEnvironment,
    hook_environment: RuntimeEnvironment,
    sub_agent_environment: RuntimeEnvironment,
}

impl RuntimeEnvironmentFixture {
    /// Environment visible to the root agent runtime.
    pub fn root_environment(&self) -> &RuntimeEnvironment {
        &self.root_environment
    }

    /// Environment expected for hooks that inherit root runtime state.
    pub fn hook_environment(&self) -> &RuntimeEnvironment {
        &self.hook_environment
    }

    /// Environment expected for a custom sub-agent home.
    pub fn sub_agent_environment(&self) -> &RuntimeEnvironment {
        &self.sub_agent_environment
    }
}

/// Deterministic direnv runner for runtime activation tests.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScriptedDirenvCommandRunner {
    cwd: PathBuf,
    environment: BTreeMap<String, String>,
    json: String,
}

impl ScriptedDirenvCommandRunner {
    pub fn success(
        cwd: impl Into<PathBuf>,
        environment: BTreeMap<String, String>,
        json: impl Into<String>,
    ) -> Self {
        Self {
            cwd: cwd.into(),
            environment,
            json: json.into(),
        }
    }
}

#[async_trait]
impl DirenvCommandRunner for ScriptedDirenvCommandRunner {
    async fn export_json(
        &self,
        cwd: &Path,
        environment: &BTreeMap<String, String>,
    ) -> Result<String, RuntimeEnvironmentActivationError> {
        assert_eq!(cwd, self.cwd.as_path());
        assert_eq!(environment, &self.environment);
        Ok(self.json.clone())
    }
}

/// Fixture for root custom home plus inherited custom sub-agent home.
pub fn custom_home_runtime_environment_fixture() -> RuntimeEnvironmentFixture {
    let root_home = RuntimeHome::custom("test-home/root").with_profile("main");
    let root_environment = RuntimeEnvironment::default()
        .with_home(root_home.clone())
        .with_cwd("test-workspace/root")
        .with_config_layer(RuntimeConfigLayer::new(
            RuntimeConfigLayerSource::User {
                file: PathBuf::from("test-home/root/config.toml"),
                profile: Some("main".to_owned()),
            },
            10,
        ))
        .with_config_layer(RuntimeConfigLayer::new(
            RuntimeConfigLayerSource::Project {
                dot_marlin_folder: PathBuf::from("test-workspace/root/.marlin"),
            },
            20,
        ));
    let hook_environment = root_environment.clone();
    let sub_agent_environment = RuntimeEnvironment::default()
        .with_home(RuntimeHome {
            path: PathBuf::from("test-home/root/sub/reviewer"),
            source: RuntimeHomeSource::InheritedSubAgent {
                parent_home: root_home.path,
            },
            profile: Some("reviewer".to_owned()),
        })
        .with_cwd("test-workspace/root/sub/reviewer")
        .with_config_layer(RuntimeConfigLayer::new(
            RuntimeConfigLayerSource::SubAgent {
                agent_reference: "reviewer".to_owned(),
            },
            30,
        ));

    RuntimeEnvironmentFixture {
        root_environment,
        hook_environment,
        sub_agent_environment,
    }
}

/// Assert that a hook observed the root custom-home environment.
pub fn assert_hook_environment_uses_root_home(
    fixture: &RuntimeEnvironmentFixture,
    observed: &RuntimeEnvironment,
) {
    assert_eq!(observed, fixture.hook_environment());
    let home = observed
        .home
        .as_ref()
        .expect("hook should observe root home");
    assert_eq!(home.source, RuntimeHomeSource::Custom);
    assert_eq!(home.profile.as_deref(), Some("main"));
    assert_eq!(observed.config_layers.len(), 2);
}

/// Assert that a sub-agent observed an inherited custom home.
pub fn assert_custom_sub_agent_environment(
    fixture: &RuntimeEnvironmentFixture,
    observed: &RuntimeEnvironment,
) {
    assert_eq!(observed, fixture.sub_agent_environment());
    let home = observed
        .home
        .as_ref()
        .expect("sub-agent should observe custom home");
    assert_eq!(home.profile.as_deref(), Some("reviewer"));
    assert!(matches!(
        &home.source,
        RuntimeHomeSource::InheritedSubAgent { parent_home }
            if parent_home == &PathBuf::from("test-home/root")
    ));
    assert_eq!(observed.config_layers.len(), 1);
    assert!(matches!(
        &observed.config_layers[0].source,
        RuntimeConfigLayerSource::SubAgent { agent_reference }
            if agent_reference == "reviewer"
    ));
}
