"""Keep a provider request unresolved and verify restart does not retire it."""

import fcntl
import json
import os
import pty
import re
import select
import socket
import struct
import subprocess
import tempfile
import termios
import threading
import time

binary = os.environ.get("MARLIN_TUI_BINARY", "target/debug/marlin-tui")
listener = socket.socket()
listener.bind(("127.0.0.1", 0))
listener.listen(4)
listener.settimeout(.2)
port = listener.getsockname()[1]
stop = threading.Event()
requests = []
errors = []


def handle(connection):
    try:
        with connection:
            connection.settimeout(10)
            data = b""
            while b"\r\n\r\n" not in data:
                part = connection.recv(4096)
                if not part:
                    return
                data += part
            headers, body = data.split(b"\r\n\r\n", 1)
            length = int(next(line.split(b":", 1)[1] for line in headers.split(b"\r\n") if line.lower().startswith(b"content-length:")))
            while len(body) < length:
                body += connection.recv(4096)
            requests.append(json.loads(body[:length]))
            stop.wait(20)
    except (ConnectionResetError, BrokenPipeError):
        pass
    except Exception as error:
        errors.append(error)


def serve():
    while not stop.is_set():
        try:
            connection, _ = listener.accept()
        except TimeoutError:
            continue
        except OSError:
            return
        threading.Thread(target=handle, args=(connection,), daemon=True).start()


thread = threading.Thread(target=serve, daemon=True)
thread.start()
with tempfile.TemporaryDirectory() as directory:
    config = os.path.join(directory, "config.toml")
    with open(config, "w") as file:
        file.write(f'provider="openai"\nmodel="gpt-fixture"\nendpoint="http://127.0.0.1:{port}/v1/responses"\nprotocol="open-ai-responses"\ncredential_env="MARLIN_TUI_FIXTURE_KEY"\ntimeout_seconds=1\n[session]\npath={json.dumps(os.path.join(directory, "session.db"))}\nproject="project:uncertain"\nsession="session:uncertain"\nagent="agent:uncertain"\n')

    def launch():
        master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 30, 120, 0, 0))
        process = subprocess.Popen([binary, config], stdin=slave, stdout=slave, stderr=slave, env=dict(os.environ, MARLIN_TUI_FIXTURE_KEY="fixture"))
        return process, master, slave

    def until(master, needle):
        output = bytearray()
        deadline = time.monotonic() + 20
        while time.monotonic() < deadline:
            visible = re.sub(rb"\x1b\[[0-?]*[ -/]*[@-~]", b"", output).replace(b" ", b"")
            if needle.replace(b" ", b"") in visible:
                return
            if select.select([master], [], [], .2)[0]:
                try:
                    output.extend(os.read(master, 65536))
                except OSError:
                    break
        raise AssertionError(f"missing {needle!r}: {bytes(output[-1200:])!r}")

    def close(process, master, slave):
        if process.poll() is None:
            process.kill()
            process.wait(timeout=5)
        os.close(master)
        os.close(slave)

    process, master, slave = launch()
    try:
        until(master, b"Marlin")
        os.write(master, b"first\r")
        until(master, b"remote outcome is unknown")
        assert len(requests) == 1, requests
        os.write(master, b"\x11")
        assert process.wait(timeout=10) == 0
    finally:
        close(process, master, slave)

    process, master, slave = launch()
    try:
        until(master, b"remote outcome and ownership are unknown")
        os.write(master, b"second\r")
        until(master, b"Session state is uncertain")
        time.sleep(.5)
        assert len(requests) == 1, requests
        assert [message["content"] for message in requests[0]["input"]] == ["first"]
        assert not errors, errors
        os.write(master, b"\x11")
        assert process.wait(timeout=10) == 0
        print("PTY open uncertainty passed: dispatched request remained unresolved after restart")
    finally:
        close(process, master, slave)
        stop.set()
        listener.close()
        thread.join(timeout=5)
