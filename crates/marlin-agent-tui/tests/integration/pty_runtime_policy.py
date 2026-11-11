"""Inspect the runtime-owned stream-open receipt through the terminal."""

import fcntl
import os
import pty
import re
import select
import struct
import subprocess
import tempfile
import termios
import time

with tempfile.TemporaryDirectory() as directory:
    config = os.path.join(directory, "config.toml")
    with open(config, "w") as file:
        file.write('provider="openai"\nmodel="gpt-fixture"\nendpoint="http://127.0.0.1:9/v1/responses"\nprotocol="open-ai-responses"\ncredential_env="MARLIN_TUI_FIXTURE_KEY"\ntimeout_seconds=7\n')
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 30, 120, 0, 0))
    process = subprocess.Popen(["target/debug/marlin-tui", config], stdin=slave, stdout=slave, stderr=slave, env=dict(os.environ, MARLIN_TUI_FIXTURE_KEY="fixture"))
    output = bytearray()

    def until(needle):
        deadline = time.monotonic() + 15
        while time.monotonic() < deadline:
            plain = re.sub(rb"\x1b\[[0-?]*[ -/]*[@-~]", b"", output).replace(b" ", b"")
            if needle.replace(b" ", b"") in plain:
                return
            if select.select([master], [], [], .2)[0]:
                try:
                    output.extend(os.read(master, 65536))
                except OSError:
                    break
        raise AssertionError(f"missing {needle!r}: {bytes(output[-1200:])!r}")

    try:
        until(b"Marlin")
        os.write(master, b"/runtime\r")
        until(b"Stream-open timeout: 7000 ms")
        until(b"does not cover stream duration")
        os.write(master, b"\x11")
        assert process.wait(timeout=5) == 0
        print("PTY runtime policy passed: typed stream-open timeout displayed with its limits")
    finally:
        if process.poll() is None:
            process.kill()
            process.wait()
        os.close(master)
        os.close(slave)

    master, slave = pty.openpty()
    process = subprocess.Popen(["target/debug/marlin-tui", config, "--session", "session:missing"], stdin=slave, stdout=slave, stderr=slave, env=dict(os.environ, MARLIN_TUI_FIXTURE_KEY="fixture"))
    try:
        assert process.wait(timeout=5) == 1
        error = os.read(master, 65536)
        assert b"--session requires a [session] storage configuration" in error, error
        print("PTY session selection passed: missing storage configuration was rejected")
    finally:
        if process.poll() is None:
            process.kill()
            process.wait()
        os.close(master)
        os.close(slave)
