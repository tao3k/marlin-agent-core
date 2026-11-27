// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! SM34 (ADV-L4) hostile runtime exploit suite for `marlin-agent-runtime/environment`.
//!
//! Proves LIVE RED -> LIVE GREEN across 2 real runtime environment security boundaries:
//! - SM34-V1: `apply_direnv_json` admitting `LD_PRELOAD`/`DYLD_INSERT_LIBRARIES`/`BASH_ENV`,
//!   re-injecting shell-denylisted vars, and admitting malformed/NUL env keys/values.
//! - SM34-V2: `direnv_cwd` admitting escaping/symlinked explicit `.envrc` paths,
//!   `resolve_sub_agent` admitting escaping `home_path` overrides, and
//!   `RuntimeStateStorageInitializer::ensure_layout` following symlinks outside state home.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use async_trait::async_trait;
use marlin_agent_protocol::{
    RuntimeEnvironment, RuntimeEnvironmentActivationPolicy, RuntimeEnvironmentActivationStatus,
    RuntimeHome, RuntimeShellIsolationPolicy, RuntimeStateStorageStatus,
};
use marlin_agent_runtime::environment::{
    DirenvCommandRunner, RuntimeEnvironmentActivationError, RuntimeEnvironmentActivationRequest,
    RuntimeEnvironmentActivator, RuntimeEnvironmentRequest, RuntimeEnvironmentResolver,
    RuntimeStateStorageInitializer, SubAgentEnvironmentRequest,
};

struct StaticJsonDirenvRunner(String);

#[async_trait]
impl DirenvCommandRunner for StaticJsonDirenvRunner {
    async fn export_json(
        &self,
        _cwd: &Path,
        _environment: &BTreeMap<String, String>,
    ) -> Result<String, RuntimeEnvironmentActivationError> {
        Ok(self.0.clone())
    }
}

fn unique_temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "marlin-sm34-{label}-{}-{nanos}",
        std::process::id()
    ))
}

#[tokio::test]
async fn sm34_v1_direnv_export_refuses_loader_hijacks_denylisted_and_malformed_vars() {
    let base_environment = BTreeMap::from([("PATH".to_owned(), "/bin".to_owned())]);
    let runtime_environment = RuntimeEnvironment::default()
        .with_cwd("/repo")
        .with_activation(
            RuntimeEnvironmentActivationPolicy::direnv_project().with_shell(
                RuntimeShellIsolationPolicy::isolated()
                    .with_allowed("PATH")
                    .with_denied("OPENAI_API_KEY"),
            ),
        );

    for (case_name, hostile_json) in [
        ("LD_PRELOAD hijack", r#"{"LD_PRELOAD":"/tmp/evil.so"}"#),
        (
            "DYLD_INSERT_LIBRARIES hijack",
            r#"{"DYLD_INSERT_LIBRARIES":"/tmp/evil.dylib"}"#,
        ),
        ("BASH_ENV shell hook", r#"{"BASH_ENV":"/tmp/pwn.sh"}"#),
        (
            "denylisted secret reinjection",
            r#"{"OPENAI_API_KEY":"sk-hijacked"}"#,
        ),
        ("malformed key with equals", r#"{"BAD=KEY":"1"}"#),
        ("NUL byte in env value", r#"{"SAFE_VAR":"evil\u0000tail"}"#),
    ] {
        let activator =
            RuntimeEnvironmentActivator::with_runner(StaticJsonDirenvRunner(hostile_json.to_owned()));
        let result = activator
            .activate(RuntimeEnvironmentActivationRequest::new(
                runtime_environment.clone(),
                base_environment.clone(),
            ))
            .await;

        assert_eq!(
            result.receipt.status,
            RuntimeEnvironmentActivationStatus::Rejected,
            "SM34-V1 ({case_name}): hostile direnv export must be rejected fail-closed, got env={:?} receipt={:?}",
            result.environment,
            result.receipt
        );
        assert_eq!(
            result.environment, base_environment,
            "SM34-V1 ({case_name}): rejected direnv export must leave base_environment untouched"
        );
    }
}

#[tokio::test]
async fn sm34_v2_explicit_envrc_subagent_home_and_state_layout_refuse_escape_and_symlinks() {
    let root = unique_temp_dir("env-escape");
    let project_dir = root.join("project");
    let outside_dir = root.join("outside");
    fs::create_dir_all(&project_dir).expect("create project dir");
    fs::create_dir_all(&outside_dir).expect("create outside dir");
    let outside_envrc = outside_dir.join(".envrc");
    fs::write(&outside_envrc, "export PWNED=1\n").expect("write outside envrc");

    let base_env = BTreeMap::from([("PATH".to_owned(), "/bin".to_owned())]);
    let activator =
        RuntimeEnvironmentActivator::with_runner(StaticJsonDirenvRunner(r#"{"PWNED":"1"}"#.to_owned()));

    // 1. Explicit .envrc outside project cwd must be rejected.
    let escape_env = RuntimeEnvironment::default()
        .with_cwd(&project_dir)
        .with_activation(RuntimeEnvironmentActivationPolicy::direnv_file(&outside_envrc));
    let escape_res = activator
        .activate(RuntimeEnvironmentActivationRequest::new(
            escape_env,
            base_env.clone(),
        ))
        .await;
    assert_eq!(
        escape_res.receipt.status,
        RuntimeEnvironmentActivationStatus::Rejected,
        "SM34-V2: explicit .envrc outside project cwd must be rejected, got {:?}",
        escape_res.receipt
    );

    // 2. Traversal explicit .envrc (`project/../outside/.envrc`) must be rejected.
    let traversal_env = RuntimeEnvironment::default()
        .with_cwd(&project_dir)
        .with_activation(RuntimeEnvironmentActivationPolicy::direnv_file(
            project_dir.join("../outside/.envrc"),
        ));
    let traversal_res = activator
        .activate(RuntimeEnvironmentActivationRequest::new(
            traversal_env,
            base_env.clone(),
        ))
        .await;
    assert_eq!(
        traversal_res.receipt.status,
        RuntimeEnvironmentActivationStatus::Rejected,
        "SM34-V2: traversal explicit .envrc must be rejected"
    );

    // 3. Symlinked .envrc inside project dir pointing outside must be rejected.
    #[cfg(unix)]
    {
        let symlink_envrc = project_dir.join(".envrc");
        std::os::unix::fs::symlink(&outside_envrc, &symlink_envrc).expect("create envrc symlink");
        let symlink_env = RuntimeEnvironment::default()
            .with_cwd(&project_dir)
            .with_activation(RuntimeEnvironmentActivationPolicy::direnv_file(&symlink_envrc));
        let symlink_res = activator
            .activate(RuntimeEnvironmentActivationRequest::new(
                symlink_env,
                base_env.clone(),
            ))
            .await;
        assert_eq!(
            symlink_res.receipt.status,
            RuntimeEnvironmentActivationStatus::Rejected,
            "SM34-V2: symlinked .envrc pointing outside project must be rejected"
        );
    }

    // 4. Sub-agent explicit `home_path` escaping parent home must be rejected.
    let parent = RuntimeEnvironmentResolver::new().resolve(
        RuntimeEnvironmentRequest::default()
            .with_custom_home(root.join("marlin-home"))
            .with_cwd(&project_dir),
    );
    for hostile_home in [
        outside_dir.join("stolen-home"),
        root.join("marlin-home").join("../../outside"),
    ] {
        let child_res = RuntimeEnvironmentResolver::new().resolve_sub_agent(
            &parent,
            SubAgentEnvironmentRequest::new("rogue-subagent").with_home_path(hostile_home.clone()),
        );
        assert!(
            child_res.is_err(),
            "SM34-V2: escaping sub-agent home_path `{}` must be rejected, got {child_res:?}",
            hostile_home.display()
        );
    }

    // 5. State storage initializer must refuse symlinked state-home directory pointing outside.
    #[cfg(unix)]
    {
        let link_home = root.join("symlink-home");
        std::os::unix::fs::symlink(&outside_dir, &link_home).expect("create symlink home");
        let layout = RuntimeHome::custom(&link_home).state_layout();
        let receipt = RuntimeStateStorageInitializer::new().ensure_layout(&layout);
        assert_eq!(
            receipt.status,
            RuntimeStateStorageStatus::Failed,
            "SM34-V2: RuntimeStateStorageInitializer must refuse symlinked state home, got {receipt:?}"
        );
    }

    let _ = fs::remove_dir_all(&root);
}
