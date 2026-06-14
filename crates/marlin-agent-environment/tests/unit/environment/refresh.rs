use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use async_trait::async_trait;
use marlin_agent_environment::{
    DirenvCommandRunner, RuntimeEnvironmentActivationError, RuntimeEnvironmentRefreshRequest,
    RuntimeEnvironmentRefresher,
};
use marlin_agent_protocol::{
    RuntimeEnvironment, RuntimeEnvironmentActivationAction, RuntimeEnvironmentActivationPolicy,
    RuntimeEnvironmentRefreshCachePolicy, RuntimeEnvironmentRefreshExecution,
    RuntimeEnvironmentRefreshStatus,
};

#[derive(Clone, Debug, Eq, PartialEq)]
enum RefreshDirenvRunner {
    Success {
        cwd: PathBuf,
        environment: BTreeMap<String, String>,
        json: String,
        allow_reload: bool,
    },
    Error(RuntimeEnvironmentActivationError),
}

#[async_trait]
impl DirenvCommandRunner for RefreshDirenvRunner {
    async fn reload(
        &self,
        cwd: &Path,
        environment: &BTreeMap<String, String>,
    ) -> Result<(), RuntimeEnvironmentActivationError> {
        match self {
            Self::Success {
                cwd: expected_cwd,
                environment: expected_environment,
                allow_reload,
                ..
            } => {
                assert!(
                    *allow_reload,
                    "direnv reload should only run when configured"
                );
                assert_eq!(cwd, expected_cwd.as_path());
                assert_eq!(environment, expected_environment);
                Ok(())
            }
            Self::Error(error) => Err(error.clone()),
        }
    }

    async fn export_json(
        &self,
        cwd: &Path,
        environment: &BTreeMap<String, String>,
    ) -> Result<String, RuntimeEnvironmentActivationError> {
        match self {
            Self::Success {
                cwd: expected_cwd,
                environment: expected_environment,
                json,
                ..
            } => {
                assert_eq!(cwd, expected_cwd.as_path());
                assert_eq!(environment, expected_environment);
                Ok(json.clone())
            }
            Self::Error(error) => Err(error.clone()),
        }
    }
}

#[tokio::test]
async fn refresher_records_background_external_tool_cache_policy() {
    let base_environment = BTreeMap::from([("PATH".to_owned(), "/bin".to_owned())]);
    let runtime_environment = RuntimeEnvironment::default()
        .with_cwd("/repo")
        .with_activation(RuntimeEnvironmentActivationPolicy::direnv_project().with_direnv_reload());
    let refresher = RuntimeEnvironmentRefresher::with_runner(RefreshDirenvRunner::Success {
        cwd: PathBuf::from("/repo"),
        environment: base_environment.clone(),
        json: r#"{"PATH":"/devenv/bin","MARLIN_REFRESH":"ok"}"#.to_owned(),
        allow_reload: true,
    });

    let result = refresher
        .refresh(
            RuntimeEnvironmentRefreshRequest::new(runtime_environment, base_environment)
                .background(),
        )
        .await;

    assert_eq!(
        result.receipt.execution,
        RuntimeEnvironmentRefreshExecution::Background
    );
    assert_eq!(
        result.receipt.cache_policy,
        RuntimeEnvironmentRefreshCachePolicy::ExternalToolOwned
    );
    assert_eq!(
        result.receipt.status,
        RuntimeEnvironmentRefreshStatus::Applied
    );
    assert_eq!(
        result.environment.get("MARLIN_REFRESH").map(String::as_str),
        Some("ok")
    );
    assert_eq!(
        result
            .receipt
            .activation_receipt
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
async fn refresher_records_foreground_external_tool_cache_policy_by_default() {
    let base_environment = BTreeMap::from([("PATH".to_owned(), "/bin".to_owned())]);
    let runtime_environment = RuntimeEnvironment::default()
        .with_cwd("/repo")
        .with_activation(RuntimeEnvironmentActivationPolicy::direnv_project());
    let refresher = RuntimeEnvironmentRefresher::with_runner(RefreshDirenvRunner::Success {
        cwd: PathBuf::from("/repo"),
        environment: base_environment.clone(),
        json: r#"{"PATH":"/direnv/bin","MARLIN_REFRESH":"foreground"}"#.to_owned(),
        allow_reload: false,
    });

    let result = refresher
        .refresh(RuntimeEnvironmentRefreshRequest::new(
            runtime_environment,
            base_environment,
        ))
        .await;

    assert_eq!(
        result.receipt.execution,
        RuntimeEnvironmentRefreshExecution::Foreground
    );
    assert_eq!(
        result.receipt.cache_policy,
        RuntimeEnvironmentRefreshCachePolicy::ExternalToolOwned
    );
    assert_eq!(
        result.receipt.status,
        RuntimeEnvironmentRefreshStatus::Applied
    );
    assert_eq!(
        result.environment.get("MARLIN_REFRESH").map(String::as_str),
        Some("foreground")
    );
    assert_eq!(
        result
            .receipt
            .activation_receipt
            .actions
            .iter()
            .map(|action| action.action.clone())
            .collect::<Vec<_>>(),
        vec![RuntimeEnvironmentActivationAction::DirenvExportJson]
    );
}

#[tokio::test]
async fn background_refresher_returns_existing_environment_when_refresh_fails() {
    let base_environment = BTreeMap::from([("PATH".to_owned(), "/bin".to_owned())]);
    let runtime_environment = RuntimeEnvironment::default()
        .with_cwd("/repo")
        .with_activation(RuntimeEnvironmentActivationPolicy::direnv_project().with_direnv_reload());
    let refresher = RuntimeEnvironmentRefresher::with_runner(RefreshDirenvRunner::Error(
        RuntimeEnvironmentActivationError::CommandFailed { status: Some(1) },
    ));

    let result = refresher
        .spawn_background_refresh(RuntimeEnvironmentRefreshRequest::new(
            runtime_environment,
            base_environment.clone(),
        ))
        .await
        .expect("background refresh task should finish");

    assert_eq!(result.environment, base_environment);
    assert_eq!(
        result.receipt.execution,
        RuntimeEnvironmentRefreshExecution::Background
    );
    assert_eq!(
        result.receipt.cache_policy,
        RuntimeEnvironmentRefreshCachePolicy::ExternalToolOwned
    );
    assert_eq!(
        result.receipt.status,
        RuntimeEnvironmentRefreshStatus::Rejected
    );
    assert_eq!(
        result.receipt.reason.as_deref(),
        Some("direnv command failed with status Some(1)")
    );
}
