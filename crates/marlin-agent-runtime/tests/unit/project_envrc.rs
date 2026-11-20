// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

use marlin_agent_gitops::working_copy::WorkingCopyGitTopLevel;
use marlin_agent_protocol::{ModelToolCall, ModelToolName};
use tempfile::TempDir;

use crate::{
    ModelToolDeclaration, ModelToolOutcome, ShellToolDeclarationRequest, ShellToolHandler,
    TokioAgentRuntime, TokioAgentRuntimeBuildRequest, ToolRuntime,
};

use super::{EnvrcIntent, EnvrcPlan, EnvrcStatus, ProjectEnvrc, ProjectEnvrcDiscovery};

fn git(cwd: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(["-c", "core.hooksPath=/dev/null"])
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .unwrap()
        .trim_end()
        .to_owned()
}

fn worktree(path: &Path, state: &Path) -> (ProjectEnvrc, TokioAgentRuntime) {
    let root = PathBuf::from(git(path, &["rev-parse", "--show-toplevel"]));
    let mut env = ["HOME", "PATH", "SHELL"]
        .into_iter()
        .filter_map(|name| {
            std::env::var(name)
                .ok()
                .map(|value| (name.to_owned(), value))
        })
        .collect::<BTreeMap<_, _>>();
    env.insert(
        "XDG_DATA_HOME".to_owned(),
        state.to_str().unwrap().to_owned(),
    );
    env.insert(
        "XDG_CONFIG_HOME".to_owned(),
        state.join("config").to_str().unwrap().to_owned(),
    );
    let binding = ProjectEnvrc::new(WorkingCopyGitTopLevel::from_resolved_path(root), env);
    let (runtime, _events) = TokioAgentRuntime::from_build_request(
        TokioAgentRuntimeBuildRequest::runtime_root(8).with_project_envrc(binding.clone()),
    );
    (binding, runtime)
}

async fn execute(runtime: &TokioAgentRuntime, path: &Path) -> ModelToolOutcome {
    let name = ModelToolName::new("shell").unwrap();
    let declaration = ModelToolDeclaration::shell(ShellToolDeclarationRequest {
        name: name.clone(),
        description: "Read the worktree environment".to_owned(),
        argument: "command".to_owned(),
        timeout_seconds: 5,
        output_limit: 1024,
    })
    .unwrap();
    let tool = ShellToolHandler::new(path.to_path_buf(), declaration);
    let call = ModelToolCall {
        id: "envrc-test".to_owned().try_into().unwrap(),
        name,
        arguments: serde_json::Map::from_iter([(
            "command".to_owned(),
            serde_json::Value::String(r#"printf '%s' "$MARLIN_WORKTREE_ENVRC""#.to_owned()),
        )]),
    };
    tool.run_tool(call, runtime.context()).await
}

async fn wait_status(binding: &ProjectEnvrc, expected: EnvrcStatus) {
    let mut output = binding.0.output.subscribe();
    let observation = tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            let status = output.borrow_and_update().status();
            if status != EnvrcStatus::Loading {
                return status;
            }
            output.changed().await.unwrap();
        }
    })
    .await
    .unwrap();
    assert_eq!(observation, expected);
}

fn assert_output(outcome: ModelToolOutcome, expected: &str) {
    match outcome {
        ModelToolOutcome::Completed(output) => {
            assert_eq!(output, format!("exit code: 0\n{expected}"))
        }
        other => panic!("expected successful shell execution, got {other:?}"),
    }
}

#[tokio::test]
async fn two_git_worktrees_authorize_reload_cancel_and_deny_independently() {
    let fixture = TempDir::new().unwrap();
    let first = fixture.path().join("first");
    let second = fixture.path().join("second");
    fs::create_dir(&first).unwrap();
    git(&first, &["init", "-q"]);
    git(
        &first,
        &[
            "-c",
            "user.name=Marlin",
            "-c",
            "user.email=marlin@example.test",
            "commit",
            "--allow-empty",
            "-qm",
            "Initialize ENVRC fixture",
        ],
    );
    git(
        &first,
        &["worktree", "add", "-qb", "second", second.to_str().unwrap()],
    );
    fs::write(
        first.join(".envrc"),
        "if test -f hang; then /bin/sleep 30 & printf '%s' \"$!\" > hang.pid; wait; fi\nexport MARLIN_WORKTREE_ENVRC=first\n",
    )
    .unwrap();
    fs::write(
        second.join(".envrc"),
        "export MARLIN_WORKTREE_ENVRC=second\n",
    )
    .unwrap();
    let state = fixture.path().join("direnv");
    let (first_envrc, _) = worktree(&first, &state);
    let discovery = ProjectEnvrcDiscovery::pending();
    let (first_runtime, _events) = TokioAgentRuntime::from_build_request(
        TokioAgentRuntimeBuildRequest::runtime_root(8)
            .with_project_envrc_discovery(discovery.clone()),
    );
    let (second_envrc, second_runtime) = worktree(&second, &state);
    let startup_probe = tokio::spawn({
        let runtime = first_runtime.clone();
        let path = first_envrc.root().to_path_buf();
        async move { execute(&runtime, &path).await }
    });
    tokio::task::yield_now().await;
    assert!(!startup_probe.is_finished());
    assert_eq!(
        first_runtime.project_envrc_status(),
        Some(EnvrcStatus::Loading)
    );
    first_envrc.submit(&first_runtime, EnvrcIntent::Activate, EnvrcPlan::Export);
    discovery.complete(Some(first_envrc.clone()));
    second_envrc.submit(&second_runtime, EnvrcIntent::Activate, EnvrcPlan::Export);
    wait_status(&first_envrc, EnvrcStatus::Blocked).await;
    wait_status(&second_envrc, EnvrcStatus::Blocked).await;
    assert!(matches!(
        startup_probe.await.unwrap(),
        ModelToolOutcome::Failed(_)
    ));

    first_envrc.submit(
        &first_runtime,
        EnvrcIntent::Allow,
        EnvrcPlan::AllowAndExport,
    );
    wait_status(&first_envrc, EnvrcStatus::Ready).await;
    assert_eq!(second_envrc.status(), EnvrcStatus::Blocked);
    second_envrc.submit(
        &second_runtime,
        EnvrcIntent::Allow,
        EnvrcPlan::AllowAndExport,
    );
    wait_status(&second_envrc, EnvrcStatus::Ready).await;
    assert_output(execute(&first_runtime, first_envrc.root()).await, "first");
    assert_output(
        execute(&second_runtime, second_envrc.root()).await,
        "second",
    );
    assert_output(execute(&first_runtime, second_envrc.root()).await, "");

    let (reopened_envrc, reopened_runtime) = worktree(&second, &state);
    reopened_envrc.submit(&reopened_runtime, EnvrcIntent::Activate, EnvrcPlan::Export);
    wait_status(&reopened_envrc, EnvrcStatus::Ready).await;
    assert_output(
        execute(&reopened_runtime, reopened_envrc.root()).await,
        "second",
    );

    fs::write(first.join("hang"), "").unwrap();
    first_envrc.submit(
        &first_runtime,
        EnvrcIntent::Reload,
        EnvrcPlan::ReloadAndExport,
    );
    let pid = tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            match fs::read_to_string(first.join("hang.pid")) {
                Ok(pid) if !pid.is_empty() => break pid,
                Ok(_) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => panic!("{error}"),
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(first_envrc.status(), EnvrcStatus::Loading);
    assert_output(
        execute(&second_runtime, second_envrc.root()).await,
        "second",
    );

    fs::remove_file(first.join("hang")).unwrap();
    first_envrc.submit(
        &first_runtime,
        EnvrcIntent::Reload,
        EnvrcPlan::ReloadAndExport,
    );
    wait_status(&first_envrc, EnvrcStatus::Ready).await;
    assert_output(execute(&first_runtime, first_envrc.root()).await, "first");
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let output = Command::new("ps")
                .args(["-p", &pid, "-o", "stat="])
                .output()
                .unwrap();
            if !output.status.success()
                || output
                    .stdout
                    .iter()
                    .copied()
                    .find(|c| !c.is_ascii_whitespace())
                    == Some(b'Z')
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
    })
    .await
    .expect("superseded direnv child is still running");

    first_envrc.submit(&first_runtime, EnvrcIntent::Deny, EnvrcPlan::Deny);
    wait_status(&first_envrc, EnvrcStatus::Denied).await;
    assert!(matches!(
        execute(&first_runtime, first_envrc.root()).await,
        ModelToolOutcome::Failed(_)
    ));
    first_envrc.submit(
        &first_runtime,
        EnvrcIntent::Reload,
        EnvrcPlan::ReloadAndExport,
    );
    wait_status(&first_envrc, EnvrcStatus::Blocked).await;
    assert_output(
        execute(&second_runtime, second_envrc.root()).await,
        "second",
    );
}
