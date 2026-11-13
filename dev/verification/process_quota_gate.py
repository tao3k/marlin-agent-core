#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 tao3k team and Contributors
# SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later
"""MAR-PROC-R01/R02: generate ITF, replay it, then test an isolated Rust mutant."""

import os
from pathlib import Path
import platform
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
MODEL = "dev/quint/process-quota.qnt"
TEST = "generated_quint_traces_match_os_processes"
CARGO = ["cargo", "test", "--locked", "-p", "marlin-agent-runtime", "--test", "unit_test", TEST, "--", "--ignored", "--nocapture"]


def run(command, *, cwd, env=None):
    subprocess.run(command, cwd=cwd, env=env, check=True)


def main():
    with tempfile.TemporaryDirectory(prefix="marlin-process-quota-") as temporary:
        root = Path(temporary)
        traces = root / "traces"
        traces.mkdir()
        run(["quint", "test", MODEL, "--main=processQuota", f"--out-itf={traces}/{{test}}-{{seq}}.json", "--max-samples=100"], cwd=ROOT)
        run(["quint", "run", MODEL, "--main=processQuota", "--seed=42", "--n-traces=20", "--max-samples=100", "--max-steps=8", f"--out-itf={traces}/random-{{seq}}.json", "--invariant=safe", "--verbosity=1"], cwd=ROOT)
        if len(list(traces.glob("*.json"))) != 23:
            raise RuntimeError("incomplete generated trace corpus")
        tree_traces = root / "tree-traces"
        tree_traces.mkdir()
        run(["quint", "test", "dev/quint/process-tree.qnt", "--main=processTree", f"--out-itf={tree_traces}/{{test}}-{{seq}}.json"], cwd=ROOT)
        if len(list(tree_traces.glob("*.json"))) != 2:
            raise RuntimeError("incomplete process-tree trace corpus")
        lease_traces = root / "lease-traces"
        lease_traces.mkdir()
        run(["quint", "test", "dev/quint/process-lease.qnt", "--main=processLease", f"--out-itf={lease_traces}/{{test}}-{{seq}}.json"], cwd=ROOT)
        if len(list(lease_traces.glob("*.json"))) != 2:
            raise RuntimeError("incomplete process-lease trace corpus")
        env = os.environ.copy()
        env["MARLIN_QUOTA_TRACE_DIR"] = str(traces)
        run(CARGO, cwd=ROOT, env=env)
        env["MARLIN_TREE_TRACE_DIR"] = str(tree_traces)
        tree_cargo = ["cargo", "test", "--locked", "-p", "marlin-agent-runtime", "--test", "unit_test", "generated_tree_traces_match_os_and_pipe_eof", "--", "--ignored", "--nocapture"]
        run(tree_cargo, cwd=ROOT, env=env)
        env["MARLIN_LEASE_TRACE_DIR"] = str(lease_traces)
        lease_cargo = ["cargo", "test", "--locked", "-p", "marlin-agent-runtime", "--test", "unit_test", "generated_lease_traces_match_process_and_pipe_ownership", "--", "--ignored", "--nocapture"]
        run(lease_cargo, cwd=ROOT, env=env)

        if platform.system() == "Darwin":
            owner_traces = root / "owner-traces"
            owner_traces.mkdir()
            owner_model = "dev/quint/runtime-process-owner.qnt"
            run(["quint", "test", owner_model, "--main=runtimeProcessOwner",
                 f"--out-itf={owner_traces}/{{test}}-{{seq}}.json"], cwd=ROOT)
            run(["quint", "run", owner_model, "--main=runtimeProcessOwner", "--seed=42",
                 "--n-traces=16", "--max-samples=100", "--max-steps=12", "--invariant=safe",
                 f"--out-itf={owner_traces}/random-{{seq}}.json", "--verbosity=1"], cwd=ROOT)
            env["MARLIN_OWNER_TRACE_DIR"] = str(owner_traces)
            owner_cargo = ["cargo", "test", "--locked", "-p", "marlin-agent-runtime", "--test", "unit_test",
                           "generated_owner_traces_match_macos_runtime", "--", "--ignored", "--nocapture"]
            run(owner_cargo, cwd=ROOT, env=env)

        # Copy the live source (including uncommitted changes) without touching it.
        mutant = root / "mutant"
        mutant.mkdir()
        for name in ("Cargo.toml", "Cargo.lock"):
            shutil.copy2(ROOT / name, mutant / name)
        for name in ("crates", "build-support", "bazel"):
            shutil.copytree(ROOT / name, mutant / name, symlinks=True,
                            ignore=shutil.ignore_patterns("target", ".git", ".DS_Store"))
        source = mutant / "crates/marlin-agent-runtime/src/process/managed.rs"
        text = source.read_text()
        text_source = text
        before = '    pub fn request_cleanup(&self) {\n        self.request_cleanup_observed_at(current_time_millis());\n    }'
        after = '    pub fn request_cleanup(&mut self) {\n        self.permit.release(); // mutation: release before child exit\n        self.request_cleanup_observed_at(current_time_millis());\n    }'
        if text.count(before) != 1:
            raise RuntimeError("mutation target changed; review the negative control")
        source.write_text(text.replace(before, after, 1))
        test = mutant / "crates/marlin-agent-runtime/tests/unit/runtime/process/quota.rs"
        text = test.read_text()
        before = 'let child_a = AsyncManagedChildProcess::spawn_with_spec('
        if text.count(before) < 2:
            raise RuntimeError("mutation test adaptation changed")
        test.write_text(text.replace(before, 'let mut child_a = AsyncManagedChildProcess::spawn_with_spec('))
        trace_test = mutant / "crates/marlin-agent-runtime/tests/unit/runtime/process/quota_trace.rs"
        text = trace_test.read_text()
        before = '                    task_a = Some(tokio::spawn(async move {\n'
        if text.count(before) != 1:
            raise RuntimeError("mutation trace adaptation changed")
        trace_test.write_text(text.replace(before, before + '                        let mut child = child;\n', 1))
        tree_test = mutant / "crates/marlin-agent-runtime/tests/unit/runtime/process/quota_tree_trace.rs"
        tree_text = tree_test.read_text()
        before = "                    let process = child.take().unwrap();"
        if tree_text.count(before) != 1:
            raise RuntimeError("mutation tree trace adaptation changed")
        tree_test.write_text(tree_text.replace(before, "                    let mut process = child.take().unwrap();", 1))
        lease_test = mutant / "crates/marlin-agent-runtime/tests/unit/runtime/process/lease_trace.rs"
        lease_text = lease_test.read_text()
        before = "                    let process = a.take().unwrap();"
        if lease_text.count(before) != 1:
            raise RuntimeError("mutation lease trace adaptation changed")
        lease_test.write_text(lease_text.replace(before, "                    let mut process = a.take().unwrap();", 1))
        # Keep mutant artifacts separate: Cargo test binaries embed source paths.
        env["CARGO_TARGET_DIR"] = str(mutant / "target")
        result = subprocess.run(CARGO, cwd=mutant, env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        expected_failure = ("B admitted while A is alive", "panicked at", "test result: FAILED")
        if result.returncode == 0 or not all(item in result.stdout for item in expected_failure):
            print(result.stdout)
            raise RuntimeError("mutant was not rejected by the intended OS admission assertion")
        print("PASS: isolated premature-release mutant rejected by generated-trace OS oracle", flush=True)
        # A separate mutation skips the post-leader process-group confirmation.
        before = "        wait_for_process_group_exit(self.pid).await?;"
        if text_source.count(before) != 1:
            raise RuntimeError("group mutation target changed")
        source.write_text(text_source.replace(before, "        // mutation: release before the process group exits", 1))
        group_cargo = ["cargo", "test", "--locked", "-p", "marlin-agent-runtime", "--test", "unit_test", "quota_survives_leader_exit_while_descendant_holds_output_pipe", "--", "--nocapture"]
        result = subprocess.run(group_cargo, cwd=mutant, env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        expected_failure = ("quota was returned while the descendant was alive", "panicked at", "test result: FAILED")
        if result.returncode == 0 or not all(item in result.stdout for item in expected_failure):
            print(result.stdout)
            raise RuntimeError("group mutant was not rejected by the intended OS assertion")
        print("PASS: isolated post-leader release mutant rejected by descendant OS oracle", flush=True)
        before = "                _lease: self.permit.0.clone(),"
        if text_source.count(before) != 1:
            raise RuntimeError("external pipe mutation target changed")
        source.write_text(text_source.replace(before, "                _lease: None, // mutation: forget caller-held pipe", 1))
        result = subprocess.run(lease_cargo, cwd=mutant, env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        expected_failure = ("B admitted while process or caller pipe is held", "panicked at", "test result: FAILED")
        if result.returncode == 0 or not all(item in result.stdout for item in expected_failure):
            print(result.stdout)
            raise RuntimeError("caller-held pipe mutant was not rejected by the intended admission assertion")
        print("PASS: isolated missing-pipe-lease mutant rejected by generated-trace oracle", flush=True)

        if platform.system() == "Darwin":
            before = "if self.resource_id() != observation.resource_id() {"
            after = "if self.resource_id().session() != observation.resource_id().session() || self.resource_id().operation() != observation.resource_id().operation() {"
            if text_source.count(before) != 1:
                raise RuntimeError("watchdog mutation target changed")
            source.write_text(text_source.replace(before, after, 1))
            result = subprocess.run(owner_cargo, cwd=mutant, env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
            expected_failure = ("stale watchdog report reaped a live replacement", "panicked at", "test result: FAILED")
            if result.returncode == 0 or not all(item in result.stdout for item in expected_failure):
                print(result.stdout)
                raise RuntimeError("watchdog mutant was not rejected by the OS reaping assertion")
            print("PASS: isolated generation-blind watchdog mutant reaped a live replacement", flush=True)

            before = "WatchdogCleanup::Advisory(self)"
            after = "{ drop(self); WatchdogCleanup::Started }"
            if text_source.count(before) != 1:
                raise RuntimeError("health mutation target changed")
            source.write_text(text_source.replace(before, after, 1))
            result = subprocess.run(owner_cargo, cwd=mutant, env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
            expected_failure = ("advisory watchdog report reaped a healthy process", "panicked at", "test result: FAILED")
            if result.returncode == 0 or not all(item in result.stdout for item in expected_failure):
                print(result.stdout)
                raise RuntimeError("health mutant was not rejected by the OS reaping assertion")
            print("PASS: isolated advisory-as-cleanup mutant reaped a healthy process", flush=True)


if __name__ == "__main__":
    main()
