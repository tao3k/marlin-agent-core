use std::path::PathBuf;

use marlin_workspace_protocol::{
    WorkspaceCtx, WorkspaceProjectGitHubOperation, WorkspaceProjectGitHubOps,
    WorkspaceProjectGitHubRepository, WorkspaceProjectPersistence, WorkspaceProjectPolicyPlane,
    WorkspaceProjectProfile, WorkspaceProjectRepository, WorkspaceProjectRoot,
    WorkspaceProjectVcsBackend, WorkspaceProjectVcsExtension, WorkspaceProjectVcsExtensionKind,
};

#[test]
fn project_profile_is_git_core_repository_first_with_org_metadata_file() {
    let repository =
        WorkspaceProjectRepository::trusted_git("/repo").with_vcs_metadata_dir("/repo/.git");
    let profile = WorkspaceProjectProfile::new("marlin-core", repository)
        .with_display_name("Marlin Agent Core")
        .with_org_metadata_file("/repo/.marlin/project.org")
        .with_additional_root(WorkspaceProjectRoot::trusted("/repo/packages"))
        .with_additional_root(WorkspaceProjectRoot::review_required("/repo/third-party"))
        .with_additional_root(WorkspaceProjectRoot::denied("/repo/secrets"))
        .with_persistence(WorkspaceProjectPersistence::repository_state_dir(
            "/repo/.marlin/state",
        ));

    assert_eq!(profile.id.as_str(), "marlin-core");
    assert_eq!(
        profile.repository.vcs_backend,
        WorkspaceProjectVcsBackend::Git
    );
    assert_eq!(profile.repository_root(), PathBuf::from("/repo").as_path());
    assert_eq!(
        profile.repository.vcs_metadata_path(),
        PathBuf::from("/repo/.git")
    );
    assert!(profile.repository.is_trusted());
    assert_eq!(
        profile.org_metadata_file.as_deref(),
        Some(PathBuf::from("/repo/.marlin/project.org").as_path())
    );
    assert!(profile.is_durable());
    assert_eq!(
        profile.persistence.state_dir(),
        Some(PathBuf::from("/repo/.marlin/state").as_path())
    );
    assert_eq!(profile.trusted_additional_roots().count(), 1);
}

#[test]
fn project_profile_supports_jj_scheme_extension_with_github_ops() {
    let github_ops =
        WorkspaceProjectGitHubOps::new(WorkspaceProjectGitHubRepository::new("jj-vcs", "jj"))
            .with_remote_name("origin")
            .with_operation(WorkspaceProjectGitHubOperation::PullRequests)
            .with_operation(WorkspaceProjectGitHubOperation::Issues)
            .with_operation(WorkspaceProjectGitHubOperation::Checks);
    let repository = WorkspaceProjectRepository::trusted_git("/src/jj").with_vcs_extension(
        WorkspaceProjectVcsExtension::jj_scheme_policy().with_metadata_dir("/src/jj/.jj"),
    );
    let profile = WorkspaceProjectProfile::new("jj", repository).with_github_ops(github_ops);

    assert_eq!(
        profile.repository.vcs_backend,
        WorkspaceProjectVcsBackend::Git
    );
    assert_eq!(
        profile.repository.vcs_metadata_path(),
        PathBuf::from("/src/jj/.git")
    );
    let jj_extension = profile
        .repository
        .vcs_extensions
        .first()
        .expect("jj extension");
    assert_eq!(jj_extension.kind, WorkspaceProjectVcsExtensionKind::Jj);
    assert_eq!(
        jj_extension.policy_plane,
        WorkspaceProjectPolicyPlane::SchemeExtension
    );
    assert_eq!(
        jj_extension.metadata_path(profile.repository_root()),
        PathBuf::from("/src/jj/.jj")
    );
    let github_ops = profile.github_ops.as_ref().expect("github ops");
    assert_eq!(github_ops.repository.slug(), "jj-vcs/jj");
    assert!(github_ops.supports(&WorkspaceProjectGitHubOperation::PullRequests));
    assert!(github_ops.supports(&WorkspaceProjectGitHubOperation::Issues));
    assert!(github_ops.supports(&WorkspaceProjectGitHubOperation::Checks));
    assert!(!github_ops.supports(&WorkspaceProjectGitHubOperation::Actions));
}

#[test]
fn workspace_context_can_carry_project_identity_without_runtime_activation() {
    let ctx = WorkspaceCtx::new("reviewer")
        .with_project_id("marlin-core")
        .with_run_id("run-1")
        .with_trace_id("trace-1");

    assert_eq!(ctx.actor, "reviewer");
    assert_eq!(
        ctx.project_id.as_ref().map(|id| id.as_str()),
        Some("marlin-core")
    );
    assert_eq!(ctx.run_id.as_deref(), Some("run-1"));
    assert_eq!(ctx.trace_id.as_deref(), Some("trace-1"));
}
