// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! MAR-PROC-R01/R02. Replay Quint ITF against the OS and check isolated mutations.

#![cfg(unix)]

use std::{
    fs,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    process::Command,
};

const RUNTIME: &str = "crates/marlin-agent-runtime";
const MANAGED: &str = "src/process/managed.rs";
const TEST_PROCESS: &str = "tests/unit/runtime/process";
const QUOTA_MODEL: &str = "dev/quint/process-quota.qnt";
const OWNER_MODEL: &str = "dev/quint/runtime-process-owner.qnt";

fn run(command: &mut Command) {
    let status = command.status().expect("start qualification command");
    assert!(status.success(), "{command:?} exited with {status}");
}

fn named_traces(workspace: &Path, dir: &Path, model: &str, module: &str) {
    fs::create_dir(dir).expect("create trace directory");
    run(Command::new("quint")
        .current_dir(workspace)
        .args([
            "test",
            model,
            &format!("--main={module}"),
            "--max-samples=100",
        ])
        .arg(format!("--out-itf={}/{{test}}-{{seq}}.json", dir.display())));
}

fn seeded_traces(workspace: &Path, dir: &Path, model: &str, module: &str, count: u8, steps: u8) {
    run(Command::new("quint")
        .current_dir(workspace)
        .args([
            "run",
            model,
            &format!("--main={module}"),
            "--seed=42",
            &format!("--n-traces={count}"),
            "--max-samples=100",
            &format!("--max-steps={steps}"),
            "--invariant=safe",
            "--verbosity=1",
        ])
        .arg(format!("--out-itf={}/random-{{seq}}.json", dir.display())));
}

fn require_trace_count(dir: &Path, expected: usize) {
    let actual = dir
        .read_dir()
        .expect("read generated trace directory")
        .map(|entry| entry.expect("read generated trace").path())
        .filter(|path| path.extension().is_some_and(|suffix| suffix == "json"))
        .count();
    assert_eq!(
        actual,
        expected,
        "incomplete ITF corpus at {}",
        dir.display()
    );
}

struct Replay<'a> {
    test: &'static str,
    traces: Option<(&'static str, &'a Path)>,
}

impl Replay<'_> {
    fn command(&self, workspace: &Path) -> Command {
        let mut command = Command::new(env!("CARGO"));
        command.current_dir(workspace).args([
            "test",
            "--locked",
            "-p",
            "marlin-agent-runtime",
            "--test",
            "unit_test",
            self.test,
            "--",
            "--nocapture",
        ]);
        if let Some((variable, directory)) = self.traces {
            command.env(variable, directory).arg("--ignored");
        }
        command
    }

    fn require_rejection(&self, mutant: &Path, oracle: &str) {
        let output = self
            .command(mutant)
            // Keep mutant artifacts separate. Test binaries embed source paths.
            .env("CARGO_TARGET_DIR", mutant.join("target"))
            .output()
            .expect("run isolated mutation");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !output.status.success()
                && stdout.contains("test result: FAILED")
                && (stdout.contains("panicked at") || stderr.contains("panicked at"))
                && (stdout.contains(oracle) || stderr.contains(oracle)),
            "{} did not reach the intended OS assertion {oracle:?}\n{stdout}\n{stderr}",
            self.test
        );
        println!("PASS: isolated mutation rejected by {oracle}");
    }
}

fn copy_source(source: &Path, destination: &Path) -> std::io::Result<()> {
    let kind = source.symlink_metadata()?.file_type();
    if kind.is_symlink() {
        symlink(source.read_link()?, destination)?;
    } else if kind.is_dir() {
        fs::create_dir(destination)?;
        for entry in source.read_dir()? {
            let entry = entry?;
            let name = entry.file_name();
            if matches!(name.to_str(), Some("target" | ".git" | ".DS_Store")) {
                continue;
            }
            copy_source(&entry.path(), &destination.join(name))?;
        }
    } else {
        fs::copy(source, destination)?;
    }
    Ok(())
}

fn snapshot_sources(workspace: &Path, mutant: &Path) {
    fs::create_dir(mutant).expect("create isolated mutation workspace");
    for name in [
        "Cargo.toml",
        "Cargo.lock",
        "crates",
        "build-support",
        "bazel",
    ] {
        copy_source(&workspace.join(name), &mutant.join(name)).expect("copy isolated sources");
    }
    // Cargo compiles all of unit_test, including the included plugin manifest.
    let fixture = Path::new("scheme/t/fixtures/brief-to-draft/plugin.toml");
    fs::create_dir_all(mutant.join(fixture).parent().expect("fixture parent"))
        .expect("create fixture directory");
    fs::copy(workspace.join(fixture), mutant.join(fixture)).expect("copy plugin manifest fixture");
}

fn replace_once(text: &str, before: &str, after: &str) -> String {
    assert_eq!(
        text.matches(before).count(),
        1,
        "mutation target changed: {before}"
    );
    text.replacen(before, after, 1)
}

fn premature_release(mutant: &Path, source: &Path, original: &str, quota: &Replay<'_>) {
    let before = "    pub fn request_cleanup(&self) {\n        self.request_cleanup_observed_at(current_time_millis());\n    }";
    let after = "    pub fn request_cleanup(&mut self) {\n        self.permit.release(); // mutation: release before child exit\n        self.request_cleanup_observed_at(current_time_millis());\n    }";
    fs::write(source, replace_once(original, before, after)).expect("inject premature release");
    // Adapt only the callers of the deliberately mutable mutant signature.
    let directory = mutant.join(RUNTIME).join(TEST_PROCESS);
    let quota_test = directory.join("quota.rs");
    let quota_original = fs::read_to_string(&quota_test).expect("read quota test");
    let declaration = "let child_a = AsyncManagedChildProcess::spawn_with_spec(";
    assert_eq!(
        quota_original.matches(declaration).count(),
        3,
        "quota adaptation changed"
    );
    fs::write(
        &quota_test,
        quota_original.replacen(
            declaration,
            "let mut child_a = AsyncManagedChildProcess::spawn_with_spec(",
            2,
        ),
    )
    .expect("adapt quota calls");
    let edits = [
        (
            "quota_trace.rs",
            "                    task_a = Some(tokio::spawn(async move {\n",
            "                    task_a = Some(tokio::spawn(async move {\n                        let mut child = child;\n",
        ),
        (
            "quota_tree_trace.rs",
            "                    let process = child.take().unwrap();",
            "                    let mut process = child.take().unwrap();",
        ),
        (
            "lease_trace.rs",
            "                    let process = a.take().unwrap();",
            "                    let mut process = a.take().unwrap();",
        ),
    ];
    let backups = edits.map(|(file, before, after)| {
        let path = directory.join(file);
        let original = fs::read_to_string(&path).expect("read replay caller");
        fs::write(&path, replace_once(&original, before, after)).expect("adapt replay caller");
        (path, original)
    });
    quota.require_rejection(mutant, "B admitted while A is alive");
    for (path, original) in backups.into_iter().chain([(quota_test, quota_original)]) {
        fs::write(path, original).expect("restore unmodified replay caller");
    }
}

#[test]
#[ignore = "qualification runs Quint and compiles isolated OS mutation controls"]
fn generated_process_traces_and_mutations() {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("canonical workspace");
    let temporary = tempfile::Builder::new()
        .prefix("marlin-process-quota-")
        .tempdir()
        .expect("create qualification workspace");
    let root = temporary.path();
    let quota_traces = root.join("quota-traces");
    let tree_traces = root.join("tree-traces");
    let lease_traces = root.join("lease-traces");
    let owner_traces = root.join("owner-traces");
    named_traces(&workspace, &quota_traces, QUOTA_MODEL, "processQuota");
    seeded_traces(
        &workspace,
        &quota_traces,
        QUOTA_MODEL,
        "processQuota",
        20,
        8,
    );
    require_trace_count(&quota_traces, 23);
    named_traces(
        &workspace,
        &tree_traces,
        "dev/quint/process-tree.qnt",
        "processTree",
    );
    require_trace_count(&tree_traces, 2);
    named_traces(
        &workspace,
        &lease_traces,
        "dev/quint/process-lease.qnt",
        "processLease",
    );
    require_trace_count(&lease_traces, 2);
    let quota = Replay {
        test: "generated_quint_traces_match_os_processes",
        traces: Some(("MARLIN_QUOTA_TRACE_DIR", &quota_traces)),
    };
    let tree = Replay {
        test: "generated_tree_traces_match_os_and_pipe_eof",
        traces: Some(("MARLIN_TREE_TRACE_DIR", &tree_traces)),
    };
    let lease = Replay {
        test: "generated_lease_traces_match_process_and_pipe_ownership",
        traces: Some(("MARLIN_LEASE_TRACE_DIR", &lease_traces)),
    };
    for replay in [&quota, &tree, &lease] {
        run(&mut replay.command(&workspace));
    }

    let owner = Replay {
        test: "generated_owner_traces_match_macos_runtime",
        traces: Some(("MARLIN_OWNER_TRACE_DIR", &owner_traces)),
    };
    if cfg!(target_os = "macos") {
        named_traces(
            &workspace,
            &owner_traces,
            OWNER_MODEL,
            "runtimeProcessOwner",
        );
        seeded_traces(
            &workspace,
            &owner_traces,
            OWNER_MODEL,
            "runtimeProcessOwner",
            16,
            12,
        );
        run(&mut owner.command(&workspace));
    }

    let mutant = root.join("mutant");
    snapshot_sources(&workspace, &mutant);
    let source = mutant.join(RUNTIME).join(MANAGED);
    let original = fs::read_to_string(&source).expect("read original process owner");
    premature_release(&mutant, &source, &original, &quota);
    let group = Replay {
        test: "quota_survives_leader_exit_while_descendant_holds_output_pipe",
        traces: None,
    };
    let lease_controls = [
        (
            &group,
            "        wait_for_process_group_exit(self.pid).await?;",
            "        // mutation: release before the process group exits",
            "quota was returned while the descendant was alive",
        ),
        (
            &lease,
            "                _lease: self.permit.0.clone(),",
            "                _lease: None, // mutation: forget caller-held pipe",
            "B admitted while process or caller pipe is held",
        ),
    ];
    let owner_controls = [
        (
            &owner,
            "if self.resource_id() != observation.resource_id() {",
            "if self.resource_id().session() != observation.resource_id().session() || self.resource_id().operation() != observation.resource_id().operation() {",
            "stale watchdog report reaped a live replacement",
        ),
        (
            &owner,
            "WatchdogCleanup::Advisory(self)",
            "{ drop(self); WatchdogCleanup::Started }",
            "advisory watchdog report reaped a healthy process",
        ),
    ];
    for (replay, before, after, oracle) in lease_controls.into_iter().chain(
        owner_controls
            .into_iter()
            .filter(|_| cfg!(target_os = "macos")),
    ) {
        fs::write(&source, replace_once(&original, before, after))
            .expect("inject isolated mutation");
        replay.require_rejection(&mutant, oracle);
    }
}
