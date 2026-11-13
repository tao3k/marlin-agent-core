#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 tao3k team and Contributors
# SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later
"""Bounded Linux cgroup-v2 experiment, not a production containment backend.

Run as root inside a disposable delegated systemd scope; only child cgroups
created by this process are modified. The negative control has a privileged
member. The positive control drops to uid/gid 65534 before executing its test.
"""

import ctypes
import json
import os
from pathlib import Path
import select
import signal
import sys
import time
import uuid

CGROUP_ROOT = Path("/sys/fs/cgroup")


def current_scope():
    if sys.platform != "linux" or os.geteuid() != 0:
        raise RuntimeError("requires Linux and root within a disposable systemd scope")
    line = Path("/proc/self/cgroup").read_text().strip()
    if not line.startswith("0::"):
        raise RuntimeError("requires a single cgroup-v2 hierarchy")
    path = CGROUP_ROOT / line[3:].lstrip("/")
    if not path.name.startswith("run-") or not path.name.endswith(".scope"):
        raise RuntimeError(f"refusing to alter non-disposable scope {path}")
    if not (path / "cgroup.kill").exists():
        raise RuntimeError("cgroup.kill is unavailable")
    return path


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def events(path):
    return dict(line.split() for line in (path / "cgroup.events").read_text().splitlines())


def wait_until(predicate, description):
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline:
        if predicate():
            return
        time.sleep(0.01)
    raise RuntimeError(f"timeout waiting for {description}")


def read_line(fd):
    result = bytearray()
    while not result.endswith(b"\n"):
        if not select.select([fd], [], [], 5)[0]:
            raise RuntimeError("child readiness timeout")
        chunk = os.read(fd, 1)
        if not chunk:
            raise RuntimeError("child exited without readiness receipt")
        result.extend(chunk)
    return json.loads(result)


def child_main(scope, group, gate_r, report_w, data_w, restricted):
    try:
        # Before parent admission no untrusted program is executed. An orphaned
        # child must exit if its supervisor vanishes before opening the gate.
        if os.read(gate_r, 1) != b"1":
            os._exit(70)
        if restricted:
            os.setgroups([])
            os.setgid(65534)
            os.setuid(65534)
            if ctypes.CDLL(None, use_errno=True).prctl(38, 1, 0, 0, 0) != 0:
                raise OSError(ctypes.get_errno(), "PR_SET_NO_NEW_PRIVS")
        os.setsid()
        denied = False
        try:
            (scope / "cgroup.procs").write_text(str(os.getpid()))
        except PermissionError:
            denied = True
        if not restricted:
            os.write(report_w, (json.dumps({"denied": denied, "pid": os.getpid(), "membership": Path("/proc/self/cgroup").read_text().strip()}) + "\n").encode())
            time.sleep(30)
            os._exit(0)
        ready_r, ready_w = os.pipe()
        descendant = os.fork()
        if descendant == 0:
            os.close(ready_r)
            os.setsid()
            os.write(ready_w, b"1")
            os.close(ready_w)
            time.sleep(30)
            os._exit(0)
        os.close(ready_w)
        if os.read(ready_r, 1) != b"1":
            raise RuntimeError("descendant did not confirm setsid")
        os.close(ready_r)
        status = Path("/proc/self/status").read_text()
        os.write(report_w, (json.dumps({"denied": denied, "pid": os.getpid(), "descendant": descendant, "membership": Path("/proc/self/cgroup").read_text().strip(), "caps_zero": "CapEff:\t0000000000000000" in status and "CapPrm:\t0000000000000000" in status, "no_new_privs": "NoNewPrivs:\t1" in status}) + "\n").encode())
        time.sleep(30)
    except BaseException as error:
        try:
            os.write(report_w, (json.dumps({"error": str(error)}) + "\n").encode())
        except OSError:
            pass
    os._exit(71)


def scenario(scope, restricted):
    group = scope / ("marlin-qualification-" + uuid.uuid4().hex)
    group.mkdir()
    gate_r, gate_w = os.pipe()
    report_r, report_w = os.pipe()
    data_r, data_w = os.pipe()
    # A writer deliberately retained outside the cgroup models a transferred fd.
    external_writer = os.dup(data_w) if restricted else None
    child = os.fork()
    if child == 0:
        os.close(gate_w)
        os.close(report_r)
        os.close(data_r)
        if external_writer is not None:
            os.close(external_writer)
        child_main(scope, group, gate_r, report_w, data_w, restricted)
    os.close(gate_r)
    os.close(report_w)
    os.close(data_w)
    try:
        (group / "cgroup.procs").write_text(str(child))
        os.write(gate_w, b"1")
        os.close(gate_w)
        gate_w = None
        receipt = read_line(report_r)
        if "error" in receipt:
            raise RuntimeError(receipt["error"])
        if restricted:
            require(receipt["denied"] and receipt["caps_zero"] and receipt["no_new_privs"], "restricted child retained authority")
            require(receipt["membership"].endswith("/" + group.name), "restricted child escaped cgroup")
            wait_until(lambda: str(receipt["descendant"]) in (group / "cgroup.procs").read_text().splitlines(), "descendant membership")
            require(events(group)["populated"] == "1", "cgroup is not populated")
            (group / "cgroup.kill").write_text("1")
            wait_until(lambda: events(group)["populated"] == "0", "empty cgroup")
            os.waitpid(child, 0)
            child = None
            require(not select.select([data_r], [], [], 0.1)[0], "external writer did not prevent EOF")
            os.close(external_writer)
            external_writer = None
            require(bool(select.select([data_r], [], [], 5)[0]) and os.read(data_r, 1) == b"", "expected EOF after external writer closed")
            return {"scenario": "restricted_uid", "setsid_descendant_in_cgroup": True, "migration_denied": True, "kill_empty": True, "external_writer_blocked_eof": True, "eof_after_writer_close": True}
        require(not receipt["denied"] and receipt["membership"] == "0::/" + str(scope.relative_to(CGROUP_ROOT)), "privileged escape was not observed")
        require(events(group)["populated"] == "0", "escaped child still registered in cgroup")
        os.kill(child, 0)
        return {"scenario": "privileged_negative", "escaped_cgroup": True, "group_empty_while_child_alive": True}
    finally:
        if gate_w is not None:
            os.close(gate_w)
        if external_writer is not None:
            os.close(external_writer)
        if child is not None:
            try:
                os.kill(child, signal.SIGKILL)
            except ProcessLookupError:
                pass
            try:
                os.waitpid(child, 0)
            except ChildProcessError:
                pass
        try:
            (group / "cgroup.kill").write_text("1")
        except OSError:
            pass
        os.close(report_r)
        os.close(data_r)
        wait_until(lambda: events(group)["populated"] == "0", "cleanup")
        group.rmdir()


def late_migration_negative(scope):
    """Moving the leader after untrusted execution misses an existing child."""
    group = scope / ("marlin-qualification-" + uuid.uuid4().hex)
    group.mkdir()
    report_r, report_w = os.pipe()
    leader = os.fork()
    if leader == 0:
        os.close(report_r)
        descendant = os.fork()
        if descendant == 0:
            time.sleep(30)
            os._exit(0)
        os.write(report_w, (json.dumps({"descendant": descendant}) + "\n").encode())
        time.sleep(30)
        os._exit(0)
    os.close(report_w)
    descendant = None
    try:
        descendant = read_line(report_r)["descendant"]
        # The descendant already exists in the parent scope at this point.
        (group / "cgroup.procs").write_text(str(leader))
        require(str(leader) in (group / "cgroup.procs").read_text().splitlines(), "leader was not migrated")
        require(str(descendant) not in (group / "cgroup.procs").read_text().splitlines(), "existing descendant unexpectedly migrated")
        (group / "cgroup.kill").write_text("1")
        wait_until(lambda: events(group)["populated"] == "0", "late migration group empty")
        os.waitpid(leader, 0)
        leader = None
        os.kill(descendant, 0)
        return {"scenario": "late_migration_negative", "earlier_descendant_escaped": True, "group_empty_while_descendant_alive": True}
    finally:
        for pid in (leader, descendant):
            if pid is not None:
                try:
                    os.kill(pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
        if leader is not None:
            try:
                os.waitpid(leader, 0)
            except ChildProcessError:
                pass
        try:
            (group / "cgroup.kill").write_text("1")
        except OSError:
            pass
        os.close(report_r)
        wait_until(lambda: events(group)["populated"] == "0", "late migration cleanup")
        group.rmdir()

def main():
    scope = current_scope()
    print(json.dumps(late_migration_negative(scope)), flush=True)
    print(json.dumps(scenario(scope, restricted=False)), flush=True)
    print(json.dumps(scenario(scope, restricted=True)), flush=True)


if __name__ == "__main__":
    main()
