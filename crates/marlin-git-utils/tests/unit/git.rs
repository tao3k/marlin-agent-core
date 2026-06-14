use std::process::Command;

use marlin_git_utils::ProcessGitTooling;

#[tokio::test]
async fn process_git_tooling_resolves_repository_root_from_nested_path() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let nested = tempdir.path().join("a/b");
    std::fs::create_dir_all(&nested).expect("nested dirs");

    let status = Command::new("git")
        .arg("init")
        .current_dir(tempdir.path())
        .status()
        .expect("git init should run");
    assert!(status.success());

    let root = ProcessGitTooling::resolve_repository_root(&nested)
        .await
        .expect("repository root should resolve");

    let expected = std::fs::canonicalize(tempdir.path()).expect("canonical tempdir");
    assert_eq!(root.as_path(), expected.as_path());
}
