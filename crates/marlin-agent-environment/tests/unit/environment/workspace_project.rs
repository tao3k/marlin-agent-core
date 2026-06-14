use std::path::PathBuf;

use marlin_agent_environment::{
    PROJECT_CONFIG_PRECEDENCE, RuntimeEnvironmentRequest, RuntimeEnvironmentResolver,
    SESSION_FLAGS_CONFIG_PRECEDENCE,
};
use marlin_agent_protocol::{
    RuntimeConfigLayerSource, RuntimeEnvironmentActivationPolicy,
    RuntimeEnvironmentActivationStatus, RuntimeSandboxPolicy, RuntimeShellIsolationPolicy,
    RuntimeWorkspaceProject, RuntimeWorkspaceProjectImportStatus, RuntimeWorkspaceProjectTrust,
};

#[test]
fn resolver_imports_active_workspace_project_as_runtime_environment() {
    let sandbox = RuntimeSandboxPolicy {
        writable_roots: vec![PathBuf::from("/repo")],
        network_access: true,
        exclude_tmpdir_env_var: false,
        exclude_slash_tmp: true,
    };
    let activation = RuntimeEnvironmentActivationPolicy::direnv_project()
        .with_direnv_reload()
        .with_shell(RuntimeShellIsolationPolicy::isolated().with_allowed("PATH"));
    let project = RuntimeWorkspaceProject::trusted("main", "/repo")
        .with_project_config("/repo/.marlin")
        .with_activation(activation.clone())
        .with_sandbox(sandbox.clone());
    let resolution = RuntimeEnvironmentResolver::new().resolve_with_receipt(
        RuntimeEnvironmentRequest::default()
            .with_cwd("/fallback")
            .with_workspace_project(project)
            .with_active_workspace_project("main")
            .with_session_flags(),
    );

    assert_eq!(resolution.environment.cwd, Some(PathBuf::from("/repo")));
    assert_eq!(resolution.environment.sandbox, sandbox);
    assert_eq!(resolution.environment.activation, activation);
    assert_eq!(
        resolution
            .environment
            .active_workspace_project
            .as_ref()
            .map(|project_id| project_id.as_str()),
        Some("main")
    );
    assert_eq!(resolution.environment.workspace_projects.len(), 1);
    assert_eq!(
        resolution
            .environment
            .config_layers
            .iter()
            .map(|layer| layer.precedence)
            .collect::<Vec<_>>(),
        vec![PROJECT_CONFIG_PRECEDENCE, SESSION_FLAGS_CONFIG_PRECEDENCE]
    );
    assert!(matches!(
        resolution.environment.config_layers[0].source,
        RuntimeConfigLayerSource::Project { .. }
    ));
    assert_eq!(
        resolution.activation_receipt.status,
        RuntimeEnvironmentActivationStatus::Planned
    );
    assert_eq!(resolution.project_import_receipts.len(), 1);
    assert_eq!(
        resolution.project_import_receipts[0].status,
        RuntimeWorkspaceProjectImportStatus::Imported
    );
}

#[test]
fn resolver_reports_missing_active_workspace_project() {
    let resolution = RuntimeEnvironmentResolver::new().resolve_with_receipt(
        RuntimeEnvironmentRequest::default()
            .with_cwd("/fallback")
            .with_active_workspace_project("missing"),
    );

    assert_eq!(resolution.environment.cwd, Some(PathBuf::from("/fallback")));
    assert_eq!(resolution.environment.active_workspace_project, None);
    assert_eq!(resolution.project_import_receipts.len(), 1);
    assert_eq!(
        resolution.project_import_receipts[0].status,
        RuntimeWorkspaceProjectImportStatus::Rejected
    );
    assert_eq!(
        resolution.project_import_receipts[0].reason.as_deref(),
        Some("active workspace project was not imported")
    );
}

#[test]
fn resolver_records_all_imported_workspace_projects_before_selecting_active_project() {
    let first = RuntimeWorkspaceProject::trusted("first", "/repo/first");
    let second = RuntimeWorkspaceProject::trusted("second", "/repo/second");

    let resolution = RuntimeEnvironmentResolver::new().resolve_with_receipt(
        RuntimeEnvironmentRequest::default()
            .with_workspace_project(first)
            .with_workspace_project(second)
            .with_active_workspace_project("second"),
    );

    assert_eq!(resolution.project_import_receipts.len(), 2);
    assert_eq!(
        resolution
            .project_import_receipts
            .iter()
            .map(|receipt| receipt.project_id.as_str())
            .collect::<Vec<_>>(),
        vec!["first", "second"]
    );
    assert!(
        resolution
            .project_import_receipts
            .iter()
            .all(|receipt| receipt.status == RuntimeWorkspaceProjectImportStatus::Imported)
    );
    assert_eq!(
        resolution.environment.cwd,
        Some(PathBuf::from("/repo/second"))
    );
    assert_eq!(
        resolution
            .environment
            .active_workspace_project
            .as_ref()
            .map(|project_id| project_id.as_str()),
        Some("second")
    );
}

#[test]
fn resolver_rejects_review_required_active_workspace_project() {
    let project = RuntimeWorkspaceProject::new("main", "/repo")
        .with_trust(RuntimeWorkspaceProjectTrust::ReviewRequired);

    let resolution = RuntimeEnvironmentResolver::new().resolve_with_receipt(
        RuntimeEnvironmentRequest::default()
            .with_cwd("/fallback")
            .with_workspace_project(project)
            .with_active_workspace_project("main"),
    );

    assert_eq!(resolution.environment.cwd, Some(PathBuf::from("/fallback")));
    assert_eq!(resolution.environment.active_workspace_project, None);
    assert!(resolution.environment.workspace_projects.is_empty());
    assert_eq!(resolution.project_import_receipts.len(), 2);
    assert_eq!(
        resolution.project_import_receipts[0].status,
        RuntimeWorkspaceProjectImportStatus::Rejected
    );
    assert_eq!(
        resolution.project_import_receipts[0].reason.as_deref(),
        Some("workspace project requires trust review before import")
    );
    assert_eq!(
        resolution.project_import_receipts[1].reason.as_deref(),
        Some("active workspace project was not imported")
    );
}
