"""Check raw-mode Ctrl-C confirmation and terminal restoration without network I/O."""

import fcntl
import os
import pty
import select
import struct
import subprocess
import tempfile
import termios
import time

binary = os.environ.get("MARLIN_TUI_BINARY", "target/debug/marlin-tui")
with tempfile.TemporaryDirectory() as directory:
    config = os.path.join(directory, "config.toml")
    with open(config, "w") as file:
        file.write('provider="openai"\nmodel="gpt-fixture"\nendpoint="http://127.0.0.1:1/v1/responses"\nprotocol="open-ai-responses"\ncredential_env="MARLIN_TUI_FIXTURE_KEY"\n')
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 30, 100, 0, 0))
    before = termios.tcgetattr(slave)
    process = subprocess.Popen([binary, config], stdin=slave, stdout=slave, stderr=slave, env=dict(os.environ, MARLIN_TUI_FIXTURE_KEY="fixture"))
    output = bytearray()

    def until(needle):
        deadline = time.monotonic() + 15
        while needle not in output and time.monotonic() < deadline:
            if select.select([master], [], [], .2)[0]:
                output.extend(os.read(master, 65536))
        assert needle in output, f"missing {needle!r}: {bytes(output[-1000:])!r}"

    try:
        until(b"Marlin")
        os.write(master, b"unsent-draft\x03")
        until(b"again")
        assert process.poll() is None, "first Ctrl-C exited"
        os.write(master, b"\x03")
        assert process.wait(timeout=10) == 0
        while select.select([master], [], [], 0)[0]:
            try:
                output.extend(os.read(master, 65536))
            except OSError:
                break
        assert termios.tcgetattr(slave) == before
        assert b"\x1b[?1049h" in output and b"\x1b[?1049l" in output
        assert b"\x1b[?2004h" in output and b"\x1b[?2004l" in output
        print("PTY Ctrl-C passed: idle confirmation and terminal restoration")
    finally:
        if process.poll() is None:
            process.kill()
            process.wait()
        os.close(master)
        os.close(slave)
