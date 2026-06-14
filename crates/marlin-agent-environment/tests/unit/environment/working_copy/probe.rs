use marlin_agent_environment::{
    ProcessWorkingCopyCommandRunner, WorkingCopyProviderExecutableStatus,
};
use marlin_workspace_protocol::WorkingCopyCommandProgram;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[tokio::test]
async fn process_runner_can_probe_git_executable_without_mutating_workspace() {
    let probe = ProcessWorkingCopyCommandRunner::probe_program(WorkingCopyCommandProgram::Git)
        .await
        .expect("git probe should run");

    assert_eq!(probe.program, WorkingCopyCommandProgram::Git);
    assert_eq!(probe.executable, "git");
    assert_eq!(probe.status, WorkingCopyProviderExecutableStatus::Available);
    assert_eq!(probe.status_code, Some(0));
    assert!(
        probe
            .stdout
            .as_deref()
            .or(probe.stderr.as_deref())
            .is_some_and(|output| output.contains("git"))
    );
}

#[tokio::test]
#[ignore = "requires globally installed wt"]
async fn process_runner_can_probe_global_worktrunk_executable() {
    let probe =
        ProcessWorkingCopyCommandRunner::probe_program(WorkingCopyCommandProgram::Worktrunk)
            .await
            .expect("wt probe should run");

    assert_eq!(probe.program, WorkingCopyCommandProgram::Worktrunk);
    assert_eq!(probe.executable, "wt");
    assert_eq!(probe.status, WorkingCopyProviderExecutableStatus::Available);
    assert_eq!(probe.status_code, Some(0));
    assert!(
        probe
            .stdout
            .as_deref()
            .or(probe.stderr.as_deref())
            .is_some_and(|output| output.contains("wt"))
    );
}

#[test]
#[ignore = "requires globally installed wt and mutates a temporary git repository"]
fn process_runner_can_smoke_worktrunk_switch_list_remove_lifecycle() {
    let temp_dir = TempDirGuard::new("marlin-wt-lifecycle-smoke");
    let repo = temp_dir.path().join("repo");
    init_git_repo(&repo);

    let switch_stdout = run_in(
        &repo,
        "wt",
        [
            "switch",
            "--no-cd",
            "--no-hooks",
            "--format",
            "json",
            "--create",
            "--base",
            "HEAD",
            "feature/marlin-smoke",
        ],
    );
    assert!(switch_stdout.contains(r#""action":"created""#));
    assert!(switch_stdout.contains(r#""branch":"feature/marlin-smoke""#));

    let list_stdout = run_in(&repo, "wt", ["list", "--format", "json", "--branches"]);
    assert!(list_stdout.contains(r#""branch": "feature/marlin-smoke""#));

    let remove_stdout = run_in(
        &repo,
        "wt",
        [
            "remove",
            "--foreground",
            "--no-hooks",
            "--format",
            "json",
            "--force",
            "feature/marlin-smoke",
        ],
    );
    assert!(remove_stdout.contains(r#""branch": "feature/marlin-smoke""#));
    assert!(remove_stdout.contains(r#""branch_deleted": true"#));
}

struct TempDirGuard {
    path: PathBuf,
}

impl TempDirGuard {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()));
        fs::create_dir_all(&path).expect("create temp dir");
        Self { path }
    }

    fn path(&self) -> &Path {
        self.path.as_path()
    }
}

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn init_git_repo(repo: &Path) {
    fs::create_dir_all(repo).expect("create git repo dir");
    run_in(repo, "git", ["init", "-q"]);
    run_in(
        repo,
        "git",
        ["config", "user.email", "marlin@example.invalid"],
    );
    run_in(repo, "git", ["config", "user.name", "Marlin"]);
    fs::write(repo.join("README.md"), "seed\n").expect("write seed file");
    run_in(repo, "git", ["add", "README.md"]);
    run_in(repo, "git", ["commit", "-q", "-m", "init"]);
}

fn run_in<const N: usize>(cwd: &Path, program: &str, args: [&str; N]) -> String {
    let output = Command::new(program)
        .current_dir(cwd)
        .args(args)
        .output()
        .unwrap_or_else(|error| panic!("run {program}: {error}"));
    assert!(
        output.status.success(),
        "{program} failed: status={:?}\nstdout={}\nstderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("utf8 stdout")
}
