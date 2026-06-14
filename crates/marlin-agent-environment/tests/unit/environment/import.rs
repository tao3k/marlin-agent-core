use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use async_trait::async_trait;
use marlin_agent_environment::{
    DirenvCommandRunner, RuntimeEnvironmentActivationError, RuntimeEnvironmentActivationRequest,
    RuntimeEnvironmentActivator, RuntimeWorkspaceProjectImportRequest,
    RuntimeWorkspaceProjectImporter,
};
use marlin_agent_protocol::{
    RuntimeEnvironment, RuntimeEnvironmentActivationAction, RuntimeEnvironmentActivationPolicy,
    RuntimeEnvironmentActivationStatus, RuntimeShellIsolationPolicy, RuntimeWorkspaceProject,
    RuntimeWorkspaceProjectImportAction, RuntimeWorkspaceProjectImportActionStatus,
    RuntimeWorkspaceProjectImportStatus, RuntimeWorkspaceProjectTrust,
};

#[derive(Clone, Debug)]
struct ImportRecordingDirenvRunner {
    cwd: PathBuf,
    environment: BTreeMap<String, String>,
    actions: Arc<Mutex<Vec<&'static str>>>,
    fail_allow: bool,
}

#[async_trait]
impl DirenvCommandRunner for ImportRecordingDirenvRunner {
    async fn allow(
        &self,
        cwd: &Path,
        environment: &BTreeMap<String, String>,
    ) -> Result<(), RuntimeEnvironmentActivationError> {
        assert_eq!(cwd, self.cwd.as_path());
        assert_eq!(environment, &self.environment);
        self.actions.lock().expect("actions lock").push("allow");
        if self.fail_allow {
            return Err(RuntimeEnvironmentActivationError::CommandFailed { status: Some(2) });
        }
        Ok(())
    }

    async fn export_json(
        &self,
        _cwd: &Path,
        _environment: &BTreeMap<String, String>,
    ) -> Result<String, RuntimeEnvironmentActivationError> {
        panic!("workspace project import must not export direnv json")
    }
}

#[tokio::test]
async fn importer_allows_trusted_direnv_project_on_import() {
    let base_environment = BTreeMap::from([
        ("PATH".to_owned(), "/bin".to_owned()),
        ("SECRET".to_owned(), "hidden".to_owned()),
    ]);
    let command_environment = BTreeMap::from([("PATH".to_owned(), "/bin".to_owned())]);
    let actions = Arc::new(Mutex::new(Vec::new()));
    let activation = RuntimeEnvironmentActivationPolicy::direnv_project()
        .with_shell(RuntimeShellIsolationPolicy::isolated().with_allowed("PATH"));
    let project = RuntimeWorkspaceProject::trusted("main", "/repo").with_activation(activation);
    let importer = RuntimeWorkspaceProjectImporter::with_runner(ImportRecordingDirenvRunner {
        cwd: PathBuf::from("/repo"),
        environment: command_environment,
        actions: Arc::clone(&actions),
        fail_allow: false,
    });

    let result = importer
        .import_project(RuntimeWorkspaceProjectImportRequest::new(
            project,
            base_environment,
        ))
        .await;

    assert_eq!(actions.lock().expect("actions lock").as_slice(), ["allow"]);
    assert_eq!(
        result.receipt.status,
        RuntimeWorkspaceProjectImportStatus::Imported
    );
    assert_eq!(result.receipt.actions.len(), 1);
    assert_eq!(
        result.receipt.actions[0].action,
        RuntimeWorkspaceProjectImportAction::DirenvAllow
    );
    assert_eq!(
        result.receipt.actions[0].status,
        RuntimeWorkspaceProjectImportActionStatus::Applied
    );
}

#[tokio::test]
async fn importer_does_not_allow_review_required_project() {
    let actions = Arc::new(Mutex::new(Vec::new()));
    let project = RuntimeWorkspaceProject::new("main", "/repo")
        .with_activation(RuntimeEnvironmentActivationPolicy::direnv_project());
    let importer = RuntimeWorkspaceProjectImporter::with_runner(ImportRecordingDirenvRunner {
        cwd: PathBuf::from("/repo"),
        environment: BTreeMap::new(),
        actions: Arc::clone(&actions),
        fail_allow: false,
    });

    let result = importer
        .import_project(RuntimeWorkspaceProjectImportRequest::new(
            project,
            BTreeMap::new(),
        ))
        .await;

    assert!(actions.lock().expect("actions lock").is_empty());
    assert_eq!(
        result.receipt.status,
        RuntimeWorkspaceProjectImportStatus::Rejected
    );
    assert_eq!(
        result.receipt.reason.as_deref(),
        Some("workspace project requires trust review before import")
    );
}

#[tokio::test]
async fn importer_rejects_trusted_project_when_direnv_allow_fails() {
    let actions = Arc::new(Mutex::new(Vec::new()));
    let project = RuntimeWorkspaceProject::new("main", "/repo")
        .with_trust(RuntimeWorkspaceProjectTrust::Trusted)
        .with_activation(RuntimeEnvironmentActivationPolicy::direnv_project());
    let importer = RuntimeWorkspaceProjectImporter::with_runner(ImportRecordingDirenvRunner {
        cwd: PathBuf::from("/repo"),
        environment: BTreeMap::new(),
        actions: Arc::clone(&actions),
        fail_allow: true,
    });

    let result = importer
        .import_project(RuntimeWorkspaceProjectImportRequest::new(
            project,
            BTreeMap::new(),
        ))
        .await;

    assert_eq!(actions.lock().expect("actions lock").as_slice(), ["allow"]);
    assert_eq!(
        result.receipt.status,
        RuntimeWorkspaceProjectImportStatus::Rejected
    );
    assert_eq!(result.receipt.actions.len(), 1);
    assert_eq!(
        result.receipt.actions[0].status,
        RuntimeWorkspaceProjectImportActionStatus::Rejected
    );
    assert_eq!(
        result.receipt.reason.as_deref(),
        Some("direnv command failed with status Some(2)")
    );
}

#[tokio::test]
async fn importer_real_direnv_allow_enables_activation_reload_when_available() {
    if Command::new("direnv").arg("version").output().is_err() {
        return;
    }

    let temp_root = std::env::temp_dir().join(format!(
        "marlin-import-direnv-native-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_nanos()
    ));
    fs::create_dir_all(&temp_root).expect("temp project should be created");
    fs::write(
        temp_root.join(".envrc"),
        "export MARLIN_IMPORT_DIRENV_NATIVE_TEST=ok\n",
    )
    .expect("envrc should be written");

    let project = RuntimeWorkspaceProject::trusted("main", temp_root.clone())
        .with_activation(RuntimeEnvironmentActivationPolicy::direnv_project());
    let import_result = RuntimeWorkspaceProjectImporter::new()
        .import_project(RuntimeWorkspaceProjectImportRequest::new(
            project,
            std::env::vars().collect(),
        ))
        .await;
    let runtime_environment = RuntimeEnvironment::default()
        .with_cwd(temp_root.clone())
        .with_activation(RuntimeEnvironmentActivationPolicy::direnv_project().with_direnv_reload());
    let activation_result = RuntimeEnvironmentActivator::new()
        .activate(RuntimeEnvironmentActivationRequest::new(
            runtime_environment,
            std::env::vars().collect(),
        ))
        .await;

    let _ = fs::remove_dir_all(&temp_root);

    assert_eq!(
        import_result.receipt.status,
        RuntimeWorkspaceProjectImportStatus::Imported,
        "direnv import receipt: {:?}",
        import_result.receipt
    );
    assert_eq!(
        import_result.receipt.actions[0].action,
        RuntimeWorkspaceProjectImportAction::DirenvAllow
    );
    assert_eq!(
        import_result.receipt.actions[0].status,
        RuntimeWorkspaceProjectImportActionStatus::Applied
    );
    assert_eq!(
        activation_result.receipt.status,
        RuntimeEnvironmentActivationStatus::Applied,
        "direnv activation receipt: {:?}",
        activation_result.receipt
    );
    assert_eq!(
        activation_result
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
    assert_eq!(
        activation_result
            .environment
            .get("MARLIN_IMPORT_DIRENV_NATIVE_TEST")
            .map(String::as_str),
        Some("ok")
    );
}
