#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 tao3k team and Contributors
# SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

"""Prepare the PR 34 Git fixture and run the native provider and Lab suites."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import time

DESIGN_BLOBS = {
    "index.org": "c163d9cedb362802f911ec090551afcc0a9ed630",
    "topology.org": "795549e538c25088c0dbc1cf90e3985429a65ce3",
}
BRANCH_ORG = "branches/review/topology.org"
BINDING_FIELDS = (
    "GITOPS_PROJECT",
    "GITOPS_COMMON_DIR",
    "GITOPS_GIT_DIR",
    "GITOPS_WORKTREE",
    "GITOPS_REF",
    "GITOPS_SOURCE_CUT",
    "SOURCE_GENERATION",
)


def require(condition: bool, diagnostic: str) -> None:
    if not condition:
        raise RuntimeError(diagnostic)


def git(repo: Path, *args: str | Path) -> str:
    return subprocess.check_output(["git", "-C", repo, *args], text=True).rstrip("\n")


def observe(repo: Path) -> list[str]:
    return [
        git(repo, "remote", "get-url", "origin"),
        *git(repo, "rev-parse", "--path-format=absolute",
             "--git-common-dir", "--absolute-git-dir", "--show-toplevel").splitlines(),
        git(repo, "symbolic-ref", "HEAD"),
    ]


def observe_cut(repo: Path, cut: str) -> list[str]:
    return [*observe(repo), cut, git(repo, "rev-list", "--count", cut)]


def drawer(properties: dict[str, str]) -> str:
    return ":PROPERTIES:\n" + "".join(f":{key}: {value}\n" for key, value in properties.items()) + ":END:\n"


def project(repo: Path, path: str, design: str, contract: str, root_id: str, values: list[str]) -> None:
    binding = {"ID": "binding", "CUSTOM_ID": "binding", **dict(zip(BINDING_FIELDS, values, strict=True))}
    document = drawer({"ID": root_id, "CONTRACT_ID": contract, "CONTRACT_REVISION": "1"})
    (repo / path).write_text(
        f"{document}{design}\n* Observed Git binding\n{drawer(binding)}", encoding="utf-8"
    )


def capture(repo: Path, path: str, name: str, revision: str, observed: list[str], output: Path) -> str:
    # Producer-side snapshot for property controls. The protected reader uses
    # the object ID and never substitutes these working-file bytes for a read.
    raw = (repo / path).read_bytes()
    snapshot = output / f"{name}.org"
    snapshot.write_bytes(raw)
    blob = git(repo, "rev-parse", f"{revision}:{path}")
    require(git(repo, "hash-object", snapshot) == blob, f"producer snapshot differs from {blob}")
    values = [*observed, revision, blob, f"sha256:{hashlib.sha256(raw).hexdigest()}"]
    (output / f"{name}.git").write_text("\n".join(values) + "\n", encoding="utf-8")
    return blob


def run_logged(command: list[str | Path], log: Path, *, cwd: Path, env: dict[str, str]) -> str:
    with log.open("w", encoding="utf-8") as transcript:
        with subprocess.Popen(command, cwd=cwd, env=env, stdout=transcript, stderr=subprocess.STDOUT) as process:
            print(f"{log.stem}_pid={process.pid}", flush=True)
            status = process.wait()
    text = log.read_text(encoding="utf-8")
    print(text, end="", flush=True)
    if status:
        raise subprocess.CalledProcessError(status, command)
    return text


def create_worktrees(output: Path, provider: Path) -> tuple[Path, Path]:
    config = output / "worktrunk.toml"
    config.write_text('worktree-path = "{{ repo_path }}/../{{ repo }}-{{ branch }}"\n', encoding="utf-8")
    logs = run_logged(
        [provider, "--ignored", "--exact", "create_pr34_worktrees", "--nocapture"],
        output / "provider.log",
        cwd=output,
        env={**os.environ, "MARLIN_GITOPS_FIXTURE": str(output), "WORKTRUNK_CONFIG_PATH": str(config)},
    )
    require("test create_pr34_worktrees ... ok" in logs.splitlines(), "Rust provider test did not run")
    a = json.loads((output / "a-worktrunk.json").read_bytes())
    b = json.loads((output / "b-worktrunk.json").read_bytes())
    return Path(a["working_copy"]["path"]), Path(b["working_copy"]["path"])


def publish_fixture(a: Path, b: Path, design: dict[str, str], output: Path) -> None:
    for generation in (1, 2):
        for repo in (a, b):
            (repo / BRANCH_ORG).parent.mkdir(parents=True, exist_ok=True)
            observed = observe_cut(repo, git(repo, "rev-parse", "HEAD"))
            project(repo, "index.org", design["index.org"], "marlin.fixture.gitops.index.v1", "report", observed)
            project(repo, "topology.org", design["topology.org"],
                    "context.gitops.worktree.worktrunk.v1", "worktree-topology", observed)
            project(repo, BRANCH_ORG, design["topology.org"],
                    "marlin.fixture.gitops.branch.v1", "branch-topology", observed)

        git(b, "add", "index.org", "topology.org", BRANCH_ORG)
        git(b, "commit", "--quiet", "--no-gpg-sign", "-m", f"Publish B Context generation {generation}")
        target_revision = git(b, "rev-parse", "HEAD")
        target_observed = observe_cut(b, git(b, "rev-parse", f"{target_revision}^"))
        target_blob = capture(b, BRANCH_ORG, f"b{generation}-branch", target_revision, target_observed, output)
        relation = {
            "ID": "related",
            "ORIGIN_ROUTE": target_observed[0],
            "ORIGIN_REF": target_observed[4],
            "ORIGIN_PATH": BRANCH_ORG,
            "ORIGIN_ANCHOR": "binding",
            "ORIGIN_PUBLICATION": target_revision,
            "ORIGIN_BLOB": target_blob,
            "ORIGIN_GENERATION": target_observed[-1],
        }
        with (a / "index.org").open("a", encoding="utf-8") as index:
            index.write(f"\n* Related branch\n{drawer(relation)}")
        git(a, "add", "index.org", "topology.org", BRANCH_ORG)
        git(a, "commit", "--quiet", "--no-gpg-sign", "-m", f"Publish A Context generation {generation}")
        source_revision = git(a, "rev-parse", "HEAD")
        source_observed = observe_cut(a, git(a, "rev-parse", f"{source_revision}^"))
        capture(a, "index.org", f"a{generation}-index", source_revision, source_observed, output)
        if generation == 2:
            capture(a, BRANCH_ORG, "a2-branch", source_revision, source_observed, output)
            capture(a, "topology.org", "a2-worktree", source_revision, source_observed, output)

    capture(b, "index.org", "b2-index", target_revision, target_observed, output)
    capture(b, "topology.org", "b2-worktree", target_revision, target_observed, output)
    alias = output / "b-alias"
    git(output, "clone", "--quiet", "--branch", "review", b, alias)
    git(alias, "remote", "set-url", "origin", git(b, "remote", "get-url", "origin"))
    alias_observed = observe_cut(alias, git(alias, "rev-parse", f"{target_revision}^"))
    capture(alias, BRANCH_ORG, "b-alias-branch", target_revision, alias_observed, output)


def protected_blobs(output: Path) -> dict[str, tuple[Path, str]]:
    blobs = {}
    for side, entry in (("a", "a2-index"), ("b", "b2-branch")):
        _, common, _, _, _, _, _, _, oid, _ = (output / f"{entry}.git").read_text(encoding="utf-8").splitlines()
        directory = Path(common) / "objects"
        require(not (directory / oid[:2] / oid[2:]).exists(), f"{side} blob is still loose")
        blobs[side] = (directory, oid)
    return blobs


def verify_pack_reads(output: Path, blobs: dict[str, tuple[Path, str]], attempts: tuple[str, ...]) -> None:
    # Correlate the consumer trace with the exact object offset after the
    # acceptance cases. The Git inspection here belongs to the fixture producer.
    for side, (directory, oid) in blobs.items():
        indexes = list((directory / "pack").glob("*.idx"))
        require(len(indexes) == 1, f"expected one pack index for {side}")
        index = indexes[0]
        rows = git(output, "verify-pack", "-v", index).splitlines()
        offsets = [parts[4] for row in rows if (parts := row.split())[:2] == [oid, "blob"]]
        require(len(offsets) == 1, f"missing unique pack offset for {oid}")
        offset = offsets[0]
        for attempt in attempts:
            trace = output / f"preopen-{attempt}-{side}.pack"
            accesses = trace.read_text(encoding="utf-8").splitlines()
            require(any(line.endswith(f"{index.with_suffix('.pack')} {offset}") for line in accesses),
                    f"{attempt}: no observed pack read for {oid} at {offset}")
            print(f"GIT-PACK-OID-OK {attempt} {side} {oid} offset={offset}", flush=True)


def main(design_dir: Path, lab: Path, output: Path, provider: Path, suite_name: str,
         locator_check: Path | None) -> None:
    started = time.monotonic()
    design = {}
    for name, expected in DESIGN_BLOBS.items():
        require(git(design_dir, "hash-object", name) == expected, f"unexpected PR 34 blob for {name}")
        design[name] = (design_dir / name).read_text(encoding="utf-8")
    a, b = create_worktrees(output, provider)
    publish_fixture(a, b, design, output)
    for repo in (a, b):
        git(repo, "repack", "-ad")
    blobs = protected_blobs(output)

    env = dict(os.environ)
    env["MARLIN_GITOPS_FIXTURE"] = str(output)
    env["GERBIL_LOADPATH"] = f"{env['GERBIL_LOADPATH']}:{lab}"
    gambit_options = env.get("GAMBOPT")
    env["GAMBOPT"] = ",".join(filter(None, [gambit_options, "max-heap=2G,debug=q"]))
    suite = Path(__file__).resolve().parents[2] / "gitops-cross-repo-test.ss"
    log = run_logged(
        ["gerbil", "-:max-heap=2G,debug=q", "test", "--run", f"^{suite_name}$", suite],
        output / "scheme.log", cwd=lab, env=env,
    )
    require(not re.search(r"ERROR (?:CASE|CHECK|HARNESS)|Heap overflow|Stack overflow", log),
            "Lab reported a fixture or harness failure")
    if suite_name == "gitops-inflight-test":
        markers = ("GIT-INFLIGHT-AUTHORITY-OK",)
        attempts = (
            "inflight-revoked-source", "inflight-revoked-target",
            "inflight-revoked-source-fresh", "inflight-revoked-target-fresh",
        )
    else:
        markers = (
            "LAB-TEMPLATE-CONTRACTS-OK", "GIT-BINDINGS-OK",
            "ORG-LINEAGE-AND-GRANTS-OK", "GIT-PREOPEN-AUTHORITY-OK",
        )
        attempts = ("allowed",)
    for marker in markers:
        require(marker in log.splitlines(), f"missing fixture marker {marker}")
    for marker in ("MODULE-OK ", "HARNESS-OK "):
        require(any(line.startswith(marker) for line in log.splitlines()), f"missing Lab receipt {marker}")
    require(log.splitlines()[-1] == "OK", "missing final Lab success receipt")
    verify_pack_reads(output, blobs, attempts)
    if locator_check is not None:
        test = "index_replays_the_real_worktrunk_publications"
        locator_log = run_logged(
            [locator_check, "--ignored", "--exact", test, "--nocapture"],
            output / "locator.log", cwd=output, env=env,
        )
        require(f"test {test} ... ok" in locator_log.splitlines(),
                "the locator index did not verify the admitted publications")
    print(f"elapsed_seconds={time.monotonic() - started:.3f}", flush=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    for directory in ("design_dir", "lab", "output", "provider"):
        parser.add_argument(directory, type=Path)
    parser.add_argument("--suite", choices=("gitops-inflight-test", "gitops-cross-repo-test"),
                        default="gitops-inflight-test")
    parser.add_argument("--locator-check", type=Path)
    arguments = parser.parse_args()
    main(*(getattr(arguments, name).resolve(strict=True)
           for name in ("design_dir", "lab", "output", "provider")), arguments.suite,
         arguments.locator_check and arguments.locator_check.resolve(strict=True))
