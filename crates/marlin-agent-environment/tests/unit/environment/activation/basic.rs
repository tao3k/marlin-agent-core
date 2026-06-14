use std::{collections::BTreeMap, path::PathBuf};

use marlin_agent_environment::{
    RuntimeEnvironmentActivationError, RuntimeEnvironmentActivationRequest,
    RuntimeEnvironmentActivator,
};
use marlin_agent_protocol::{
    RuntimeEnvironment, RuntimeEnvironmentActivationPolicy, RuntimeEnvironmentActivationStatus,
    RuntimeShellIsolationPolicy,
};

use super::FakeDirenvRunner;

#[tokio::test]
async fn activator_applies_direnv_json_and_records_name_only_delta() {
    let base_environment = BTreeMap::from([
        ("PATH".to_owned(), "/bin".to_owned()),
        ("REMOVE_ME".to_owned(), "old".to_owned()),
        ("SECRET_TOKEN".to_owned(), "old-secret".to_owned()),
    ]);
    let command_environment = BTreeMap::from([("PATH".to_owned(), "/bin".to_owned())]);
    let runtime_environment = RuntimeEnvironment::default()
        .with_cwd("/repo")
        .with_activation(
            RuntimeEnvironmentActivationPolicy::direnv_project()
                .with_shell(RuntimeShellIsolationPolicy::isolated().with_allowed("PATH")),
        );
    let activator = RuntimeEnvironmentActivator::with_runner(FakeDirenvRunner::Success {
        cwd: PathBuf::from("/repo"),
        environment: command_environment,
        json: r#"{"PATH":"/direnv/bin","NEW_VAR":"new-secret","REMOVE_ME":null}"#.to_owned(),
    });

    let result = activator
        .activate(RuntimeEnvironmentActivationRequest::new(
            runtime_environment,
            base_environment,
        ))
        .await;
    let serialized_receipt =
        serde_json::to_string(&result.receipt).expect("receipt should serialize");

    assert_eq!(
        result.receipt.status,
        RuntimeEnvironmentActivationStatus::Applied
    );
    assert_eq!(
        result.environment.get("PATH").map(String::as_str),
        Some("/direnv/bin")
    );
    assert_eq!(
        result.environment.get("NEW_VAR").map(String::as_str),
        Some("new-secret")
    );
    assert!(!result.environment.contains_key("REMOVE_ME"));
    assert_eq!(result.receipt.delta.added, vec!["NEW_VAR"]);
    assert_eq!(result.receipt.delta.changed, vec!["PATH"]);
    assert_eq!(result.receipt.delta.removed, vec!["REMOVE_ME"]);
    assert!(!serialized_receipt.contains("new-secret"));
    assert!(!serialized_receipt.contains("/direnv/bin"));
}

#[tokio::test]
async fn activator_rejects_direnv_without_cwd() {
    let base_environment = BTreeMap::from([("PATH".to_owned(), "/bin".to_owned())]);
    let runtime_environment = RuntimeEnvironment::default()
        .with_activation(RuntimeEnvironmentActivationPolicy::direnv_project());
    let activator = RuntimeEnvironmentActivator::with_runner(FakeDirenvRunner::Success {
        cwd: PathBuf::from("/repo"),
        environment: BTreeMap::new(),
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
    assert_eq!(
        result.receipt.reason.as_deref(),
        Some("runtime environment activation requires cwd")
    );
}

#[tokio::test]
async fn activator_rejects_direnv_command_failure() {
    let base_environment = BTreeMap::from([("PATH".to_owned(), "/bin".to_owned())]);
    let runtime_environment = RuntimeEnvironment::default()
        .with_cwd("/repo")
        .with_activation(RuntimeEnvironmentActivationPolicy::direnv_project());
    let activator = RuntimeEnvironmentActivator::with_runner(FakeDirenvRunner::Error(
        RuntimeEnvironmentActivationError::CommandFailed { status: Some(1) },
    ));

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
    assert_eq!(
        result.receipt.reason.as_deref(),
        Some("direnv command failed with status Some(1)")
    );
}
