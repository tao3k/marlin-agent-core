use std::path::PathBuf;

use marlin_agent_protocol::{
    RuntimeEnvironment, RuntimeEnvironmentActivationPolicy, RuntimeSandboxPolicy,
    RuntimeWorkspaceProject, RuntimeWorkspaceProjectImportAction,
    RuntimeWorkspaceProjectImportActionReceipt, RuntimeWorkspaceProjectImportActionStatus,
    RuntimeWorkspaceProjectImportReceipt, RuntimeWorkspaceProjectImportStatus,
    RuntimeWorkspaceProjectTrust,
};

#[test]
fn runtime_environment_records_imported_workspace_projects() {
    let sandbox = RuntimeSandboxPolicy {
        writable_roots: vec![PathBuf::from("/repo")],
        network_access: true,
        exclude_tmpdir_env_var: false,
        exclude_slash_tmp: true,
    };
    let project = RuntimeWorkspaceProject::trusted("main", "/repo")
        .with_project_config("/repo/.marlin")
        .with_activation(RuntimeEnvironmentActivationPolicy::direnv_project())
        .with_sandbox(sandbox.clone());
    let environment = RuntimeEnvironment::default()
        .with_workspace_project(project.clone())
        .with_active_workspace_project("main");
    let receipt = RuntimeWorkspaceProjectImportReceipt::imported(&project);

    assert_eq!(environment.workspace_projects, vec![project.clone()]);
    assert_eq!(
        environment
            .active_workspace_project
            .as_ref()
            .map(|id| id.as_str()),
        Some("main")
    );
    assert_eq!(receipt.project_id.as_str(), "main");
    assert_eq!(receipt.root, Some(PathBuf::from("/repo")));
    assert_eq!(project.trust, RuntimeWorkspaceProjectTrust::Trusted);
    assert_eq!(
        receipt.status,
        RuntimeWorkspaceProjectImportStatus::Imported
    );
    assert_eq!(receipt.reason, None);
    assert!(receipt.actions.is_empty());
}

#[test]
fn runtime_environment_records_trusted_direnv_import_allow_receipt() {
    let project = RuntimeWorkspaceProject::trusted("main", "/repo")
        .with_activation(RuntimeEnvironmentActivationPolicy::direnv_project());
    let receipt = RuntimeWorkspaceProjectImportReceipt::imported_with_actions(
        &project,
        vec![RuntimeWorkspaceProjectImportActionReceipt::applied(
            RuntimeWorkspaceProjectImportAction::DirenvAllow,
        )],
    );

    assert_eq!(receipt.project_id.as_str(), "main");
    assert_eq!(
        receipt.status,
        RuntimeWorkspaceProjectImportStatus::Imported
    );
    assert_eq!(receipt.actions.len(), 1);
    assert_eq!(
        receipt.actions[0].action,
        RuntimeWorkspaceProjectImportAction::DirenvAllow
    );
    assert_eq!(
        receipt.actions[0].status,
        RuntimeWorkspaceProjectImportActionStatus::Applied
    );
}
