//! Reusable runtime environment fixtures for custom homes and sub-agents.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use async_trait::async_trait;
use marlin_agent_environment::{
    DirenvCommandRunner, RuntimeEnvironmentActivationError, RuntimeEnvironmentActivationResult,
};
use marlin_agent_protocol::{
    RuntimeConfigLayer, RuntimeConfigLayerSource, RuntimeEnvironment,
    RuntimeEnvironmentActivationAction, RuntimeEnvironmentActivationActionReceipt,
    RuntimeEnvironmentActivationPolicy, RuntimeEnvironmentActivationStatus, RuntimeHome,
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
    expect_reload: bool,
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
            expect_reload: false,
        }
    }

    pub fn with_expected_reload(mut self) -> Self {
        self.expect_reload = true;
        self
    }
}

#[async_trait]
impl DirenvCommandRunner for ScriptedDirenvCommandRunner {
    async fn reload(
        &self,
        cwd: &Path,
        environment: &BTreeMap<String, String>,
    ) -> Result<(), RuntimeEnvironmentActivationError> {
        assert!(self.expect_reload, "unexpected direnv reload preflight");
        assert_eq!(cwd, self.cwd.as_path());
        assert_eq!(environment, &self.environment);
        Ok(())
    }

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

/// Fixture for one `.envrc`-backed direnv activation scenario.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DirenvActivationFixture {
    name: &'static str,
    environment: RuntimeEnvironment,
    base_environment: BTreeMap<String, String>,
    command_environment: BTreeMap<String, String>,
    expected_environment: BTreeMap<String, String>,
    command_cwd: PathBuf,
    envrc_file: PathBuf,
    envrc_contents: &'static str,
    export_json: &'static str,
    expect_reload: bool,
}

impl DirenvActivationFixture {
    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn environment(&self) -> &RuntimeEnvironment {
        &self.environment
    }

    pub fn base_environment(&self) -> &BTreeMap<String, String> {
        &self.base_environment
    }

    pub fn expected_environment(&self) -> &BTreeMap<String, String> {
        &self.expected_environment
    }

    pub fn envrc_file(&self) -> &Path {
        &self.envrc_file
    }

    pub fn envrc_contents(&self) -> &'static str {
        self.envrc_contents
    }

    pub fn runner(&self) -> ScriptedDirenvCommandRunner {
        let runner = ScriptedDirenvCommandRunner::success(
            self.command_cwd.clone(),
            self.command_environment.clone(),
            self.export_json,
        );

        if self.expect_reload {
            runner.with_expected_reload()
        } else {
            runner
        }
    }
}

/// Native direnv, nix-direnv, and direnv-instant fixtures learned from the
/// environment activation design. They are intentionally scripted and do not
/// require direnv, nix, or direnv-instant binaries during tests.
pub fn direnv_activation_fixtures() -> Vec<DirenvActivationFixture> {
    vec![
        native_direnv_activation_fixture(),
        nix_direnv_activation_fixture(),
        direnv_instant_activation_fixture(),
    ]
}

/// Assert that a scripted `.envrc` activation produced the fixture receipt.
pub fn assert_direnv_activation_fixture(
    fixture: &DirenvActivationFixture,
    result: &RuntimeEnvironmentActivationResult,
) {
    assert_eq!(result.environment, *fixture.expected_environment());
    assert_eq!(
        result.receipt.activation,
        fixture.environment().activation.activation
    );
    assert_eq!(result.receipt.shell, fixture.environment().activation.shell);
    assert_eq!(
        result.receipt.status,
        RuntimeEnvironmentActivationStatus::Applied
    );
    assert_eq!(result.receipt.reason, None);
    assert_eq!(
        result.receipt.delta,
        marlin_agent_protocol::RuntimeEnvironmentDelta::from_snapshots(
            fixture.base_environment(),
            fixture.expected_environment()
        )
    );

    let mut expected_actions = Vec::new();
    if fixture.expect_reload {
        expected_actions.push(RuntimeEnvironmentActivationActionReceipt::applied(
            RuntimeEnvironmentActivationAction::DirenvReload,
        ));
    }
    expected_actions.push(RuntimeEnvironmentActivationActionReceipt::applied(
        RuntimeEnvironmentActivationAction::DirenvExportJson,
    ));
    assert_eq!(result.receipt.actions, expected_actions);
}

fn native_direnv_activation_fixture() -> DirenvActivationFixture {
    let root = PathBuf::from("fixtures/runtime-environments/native-direnv");
    let envrc_file = root.join(".envrc");
    let base_environment = environment_map(&[
        ("MARLIN_ENV_KIND", "host"),
        ("PATH", "/usr/bin"),
        ("SECRET_TOKEN", "host-secret"),
    ]);
    let expected_environment = environment_map(&[
        ("MARLIN_ENV_KIND", "native-direnv"),
        ("NATIVE_DIRENV_READY", "1"),
        ("PATH", "/workspace/native-direnv/bin:/usr/bin"),
    ]);

    DirenvActivationFixture {
        name: "native-direnv-project",
        environment: RuntimeEnvironment::default()
            .with_cwd(root.clone())
            .with_activation(RuntimeEnvironmentActivationPolicy::direnv_project()),
        base_environment: base_environment.clone(),
        command_environment: base_environment,
        expected_environment,
        command_cwd: root,
        envrc_file,
        envrc_contents: "export MARLIN_ENV_KIND=native-direnv\nexport PATH=/workspace/native-direnv/bin:$PATH\n",
        export_json: r#"{"MARLIN_ENV_KIND":"native-direnv","NATIVE_DIRENV_READY":"1","PATH":"/workspace/native-direnv/bin:/usr/bin","SECRET_TOKEN":null}"#,
        expect_reload: false,
    }
}

fn nix_direnv_activation_fixture() -> DirenvActivationFixture {
    let root = PathBuf::from("fixtures/runtime-environments/nix-direnv");
    let envrc_file = root.join(".envrc");
    let base_environment = environment_map(&[("PATH", "/usr/bin")]);
    let expected_environment = environment_map(&[
        ("IN_NIX_SHELL", "pure"),
        ("MARLIN_NIX_PROFILE", "nix-direnv"),
        ("PATH", "/nix/store/marlin/bin:/usr/bin"),
    ]);

    DirenvActivationFixture {
        name: "nix-direnv-explicit-envrc",
        environment: RuntimeEnvironment::default()
            .with_cwd(root.clone())
            .with_activation(
                RuntimeEnvironmentActivationPolicy::direnv_file(&envrc_file).with_direnv_reload(),
            ),
        base_environment: base_environment.clone(),
        command_environment: base_environment,
        expected_environment,
        command_cwd: root,
        envrc_file,
        envrc_contents: "use flake\nexport MARLIN_NIX_PROFILE=nix-direnv\n",
        export_json: r#"{"IN_NIX_SHELL":"pure","MARLIN_NIX_PROFILE":"nix-direnv","PATH":"/nix/store/marlin/bin:/usr/bin"}"#,
        expect_reload: true,
    }
}

fn direnv_instant_activation_fixture() -> DirenvActivationFixture {
    let root = PathBuf::from("fixtures/runtime-environments/direnv-instant");
    let envrc_file = root.join(".envrc");
    let base_environment =
        environment_map(&[("DIRENV_INSTANT_SHELL_PID", "42"), ("PATH", "/usr/bin")]);
    let expected_environment = environment_map(&[
        (
            "__DIRENV_INSTANT_CURRENT_DIR",
            "fixtures/runtime-environments/direnv-instant",
        ),
        ("DIRENV_INSTANT_READY", "1"),
        ("DIRENV_INSTANT_SHELL_PID", "42"),
        ("PATH", "/usr/bin"),
    ]);

    DirenvActivationFixture {
        name: "direnv-instant-project",
        environment: RuntimeEnvironment::default()
            .with_cwd(root.clone())
            .with_activation(RuntimeEnvironmentActivationPolicy::direnv_project()),
        base_environment: base_environment.clone(),
        command_environment: base_environment,
        expected_environment,
        command_cwd: root,
        envrc_file,
        envrc_contents: "# loaded through direnv-instant\nexport DIRENV_INSTANT_READY=1\n",
        export_json: r#"{"__DIRENV_INSTANT_CURRENT_DIR":"fixtures/runtime-environments/direnv-instant","DIRENV_INSTANT_READY":"1"}"#,
        expect_reload: false,
    }
}

fn environment_map(entries: &[(&str, &str)]) -> BTreeMap<String, String> {
    entries
        .iter()
        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
        .collect()
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
