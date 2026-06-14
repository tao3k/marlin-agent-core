use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use marlin_agent_environment::{RuntimeEnvironmentActivationRequest, RuntimeEnvironmentActivator};
use marlin_agent_protocol::{
    RuntimeEnvironment, RuntimeEnvironmentActivationAction,
    RuntimeEnvironmentActivationActionStatus, RuntimeEnvironmentActivationPolicy,
    RuntimeEnvironmentActivationStatus,
};

use super::RecordingDirenvRunner;

#[tokio::test]
async fn activator_runs_direnv_reload_before_export() {
    let base_environment = BTreeMap::from([("PATH".to_owned(), "/bin".to_owned())]);
    let actions = Arc::new(Mutex::new(Vec::new()));
    let runtime_environment = RuntimeEnvironment::default()
        .with_cwd("/repo")
        .with_activation(RuntimeEnvironmentActivationPolicy::direnv_project().with_direnv_reload());
    let activator = RuntimeEnvironmentActivator::with_runner(RecordingDirenvRunner {
        cwd: PathBuf::from("/repo"),
        environment: base_environment.clone(),
        actions: Arc::clone(&actions),
        fail_reload: false,
        json: r#"{"PATH":"/direnv/bin"}"#.to_owned(),
    });

    let result = activator
        .activate(RuntimeEnvironmentActivationRequest::new(
            runtime_environment,
            base_environment,
        ))
        .await;

    assert_eq!(
        actions.lock().expect("actions lock").as_slice(),
        ["reload", "export_json"]
    );
    assert_eq!(
        result.receipt.status,
        RuntimeEnvironmentActivationStatus::Applied
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
async fn activator_records_rejected_direnv_reload_action() {
    let base_environment = BTreeMap::from([("PATH".to_owned(), "/bin".to_owned())]);
    let runtime_environment = RuntimeEnvironment::default()
        .with_cwd("/repo")
        .with_activation(RuntimeEnvironmentActivationPolicy::direnv_project().with_direnv_reload());
    let activator = RuntimeEnvironmentActivator::with_runner(RecordingDirenvRunner {
        cwd: PathBuf::from("/repo"),
        environment: base_environment.clone(),
        actions: Arc::new(Mutex::new(Vec::new())),
        fail_reload: true,
        json: "{}".to_owned(),
    });

    let result = activator
        .activate(RuntimeEnvironmentActivationRequest::new(
            runtime_environment,
            base_environment.clone(),
        ))
        .await;

    assert_eq!(result.environment, base_environment);
    assert_eq!(
        result.receipt.status,
        RuntimeEnvironmentActivationStatus::Rejected
    );
    assert_eq!(result.receipt.actions.len(), 1);
    assert_eq!(
        result.receipt.actions[0].action,
        RuntimeEnvironmentActivationAction::DirenvReload
    );
    assert_eq!(
        result.receipt.actions[0].status,
        RuntimeEnvironmentActivationActionStatus::Rejected
    );
    assert_eq!(
        result.receipt.reason.as_deref(),
        Some("direnv command failed with status Some(1)")
    );
}
