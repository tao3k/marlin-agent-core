#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 tao3k team and Contributors
# SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later
"""Require both the Gerbil runner exit code and its module-success receipt."""

from pathlib import Path
import subprocess
import sys


def module_succeeded(output: str, module: str = "t/all-test.ss") -> bool:
    lines = output.splitlines()
    return f"MODULE-OK {module}" in lines and not any(
        line.startswith("ERROR ") for line in lines
    )


def main() -> None:
    root = Path(__file__).resolve().parents[2]
    module = sys.argv[1] if len(sys.argv) > 1 else "t/all-test.ss"
    if module not in ("t/all-test.ss", "t/ui-native-test.ss"):
        raise SystemExit("unsupported Gerbil test module")
    command = ["direnv", "exec", ".", "bash", "-ec", 'cd scheme && gxpkg env gerbil test "$1"', "--", module]
    result = subprocess.run(command, cwd=root, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    sys.stdout.write(result.stdout)
    if result.returncode != 0:
        raise SystemExit(result.returncode)
    if not module_succeeded(result.stdout, module):
        print("Gerbil property gate: missing module-success receipt or module error", file=sys.stderr)
        raise SystemExit(1)


if __name__ == "__main__":
    main()
