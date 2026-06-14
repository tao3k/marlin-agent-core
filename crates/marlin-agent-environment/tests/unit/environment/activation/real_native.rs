use std::{
    fs,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use marlin_agent_environment::{RuntimeEnvironmentActivationRequest, RuntimeEnvironmentActivator};
use marlin_agent_protocol::{
    RuntimeEnvironment, RuntimeEnvironmentActivationAction, RuntimeEnvironmentActivationPolicy,
    RuntimeEnvironmentActivationStatus,
};

use super::{command_stdout, shell_single_quote};

#[tokio::test]
async fn activator_real_direnv_reload_applies_allowed_temp_project_when_available() {
    if Command::new("direnv").arg("version").output().is_err() {
        return;
    }

    let temp_root = std::env::temp_dir().join(format!(
        "marlin-direnv-native-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_nanos()
    ));
    fs::create_dir_all(&temp_root).expect("temp project should be created");
    fs::write(
        temp_root.join(".envrc"),
        "export MARLIN_DIRENV_NATIVE_TEST=ok\n",
    )
    .expect("envrc should be written");
    let allow_status = Command::new("direnv")
        .arg("allow")
        .current_dir(&temp_root)
        .status()
        .expect("direnv allow should run");
    assert!(allow_status.success(), "direnv allow should succeed");

    let runtime_environment = RuntimeEnvironment::default()
        .with_cwd(temp_root.clone())
        .with_activation(RuntimeEnvironmentActivationPolicy::direnv_project().with_direnv_reload());
    let activator = RuntimeEnvironmentActivator::new();
    let result = activator
        .activate(RuntimeEnvironmentActivationRequest::new(
            runtime_environment,
            std::env::vars().collect(),
        ))
        .await;

    let _ = fs::remove_dir_all(&temp_root);

    assert_eq!(
        result.receipt.status,
        RuntimeEnvironmentActivationStatus::Applied,
        "direnv activation receipt: {:?}",
        result.receipt
    );
    assert_eq!(
        result
            .environment
            .get("MARLIN_DIRENV_NATIVE_TEST")
            .map(String::as_str),
        Some("ok")
    );
    assert_eq!(
        result
            .receipt
            .actions
            .iter()
            .map(|action| action.action.clone())
            .collect::<Vec<_>>(),
        vec![
            RuntimeEnvironmentActivationAction::DirenvReload,
            RuntimeEnvironmentActivationAction::DirenvExportJson,
        ]
    );
}

#[tokio::test]
async fn activator_real_devenv_direnv_reload_applies_allowed_temp_project_when_enabled() {
    if std::env::var_os("MARLIN_RUN_DEVENV_NATIVE_TESTS").is_none() {
        return;
    }

    let devenv_bin = match command_stdout("sh", &["-lc", "command -v devenv"]) {
        Some(path) => path,
        None => return,
    };

    let temp_root = std::env::temp_dir().join(format!(
        "marlin-devenv-native-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_nanos()
    ));
    fs::create_dir_all(&temp_root).expect("temp project should be created");
    fs::write(
        temp_root.join("devenv.nix"),
        "{ pkgs, ... }: {\n  env.MARLIN_DEVENV_NATIVE_TEST = \"ok\";\n}\n",
    )
    .expect("devenv.nix should be written");
    fs::write(
        temp_root.join(".envrc"),
        format!(
            "export DEVENV_BIN={}\nsource <(\"$DEVENV_BIN\" direnvrc)\nuse devenv\n",
            shell_single_quote(&devenv_bin)
        ),
    )
    .expect("envrc should be written");
    let allow_status = Command::new("direnv")
        .arg("allow")
        .current_dir(&temp_root)
        .status()
        .expect("direnv allow should run");
    assert!(allow_status.success(), "direnv allow should succeed");

    let runtime_environment = RuntimeEnvironment::default()
        .with_cwd(temp_root.clone())
        .with_activation(RuntimeEnvironmentActivationPolicy::direnv_project().with_direnv_reload());
    let activator = RuntimeEnvironmentActivator::new();
    let result = activator
        .activate(RuntimeEnvironmentActivationRequest::new(
            runtime_environment,
            std::env::vars().collect(),
        ))
        .await;

    let _ = fs::remove_dir_all(&temp_root);

    assert_eq!(
        result.receipt.status,
        RuntimeEnvironmentActivationStatus::Applied,
        "devenv direnv activation receipt: {:?}",
        result.receipt
    );
    assert_eq!(
        result
            .environment
            .get("MARLIN_DEVENV_NATIVE_TEST")
            .map(String::as_str),
        Some("ok")
    );
    assert_eq!(
        result
            .receipt
            .actions
            .iter()
            .map(|action| action.action.clone())
            .collect::<Vec<_>>(),
        vec![
            RuntimeEnvironmentActivationAction::DirenvReload,
            RuntimeEnvironmentActivationAction::DirenvExportJson,
        ]
    );
}
