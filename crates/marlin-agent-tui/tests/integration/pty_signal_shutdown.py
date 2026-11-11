"""Exercise Unix signal shutdown and unfinished persisted turn recovery."""

import fcntl
import json
import os
import pty
import re
import select
import signal
import socket
import struct
import subprocess
import tempfile
import termios
import threading
import time

binary = os.environ.get("MARLIN_TUI_BINARY", "target/debug/marlin-tui")


def launch(config):
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 30, 120, 0, 0))
    before = termios.tcgetattr(slave)
    process = subprocess.Popen([binary, config], stdin=slave, stdout=slave, stderr=slave, env=dict(os.environ, MARLIN_TUI_FIXTURE_KEY="fixture"))
    return process, master, slave, before


def until(master, needle):
    output = bytearray()
    deadline = time.monotonic() + 20
    while time.monotonic() < deadline:
        plain = re.sub(rb"\x1b\[[0-?]*[ -/]*[@-~]", b"", output).replace(b" ", b"")
        if needle.replace(b" ", b"") in plain:
            return output
        if select.select([master], [], [], .2)[0]:
            output.extend(os.read(master, 65536))
    raise AssertionError(f"missing {needle!r}: {bytes(output[-1200:])!r}")


def drain(master, output):
    while select.select([master], [], [], 0)[0]:
        try:
            output.extend(os.read(master, 65536))
        except OSError:
            break


def close(process, master, slave):
    if process.poll() is None:
        process.kill()
        process.wait(timeout=5)
    os.close(master)
    os.close(slave)


def config_text(port):
    return f'provider="openai"\nmodel="gpt-fixture"\nendpoint="http://127.0.0.1:{port}/v1/responses"\nprotocol="open-ai-responses"\ncredential_env="MARLIN_TUI_FIXTURE_KEY"\n'


with tempfile.TemporaryDirectory() as directory:
    config = os.path.join(directory, "config.toml")
    with open(config, "w") as file:
        file.write(config_text(1))
    for kind in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
        process, master, slave, before = launch(config)
        try:
            output = until(master, b"Marlin")
            process.send_signal(kind)
            assert process.wait(timeout=10) == 128 + kind, process.returncode
            drain(master, output)
            assert termios.tcgetattr(slave) == before
            assert b"\x1b[?1049h" in output and b"\x1b[?1049l" in output
            assert b"\x1b[?2004h" in output and b"\x1b[?2004l" in output
        finally:
            close(process, master, slave)

    listener = socket.socket()
    listener.bind(("127.0.0.1", 0))
    listener.listen(1)
    listener.settimeout(10)
    stop = threading.Event()
    requests = []
    errors = []

    def serve():
        try:
            connection, _ = listener.accept()
            with connection:
                connection.settimeout(10)
                data = b""
                while b"\r\n\r\n" not in data:
                    data += connection.recv(4096)
                headers, body = data.split(b"\r\n\r\n", 1)
                length = int(next(line.split(b":", 1)[1] for line in headers.split(b"\r\n") if line.lower().startswith(b"content-length:")))
                while len(body) < length:
                    body += connection.recv(4096)
                requests.append(json.loads(body[:length]))
                connection.sendall(b'HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\ndata: {"type":"response.output_text.delta","delta":"signal-prefix"}\n\n')
                stop.wait(20)
        except Exception as error:
            errors.append(error)

    thread = threading.Thread(target=serve, daemon=True)
    thread.start()
    with open(config, "w") as file:
        file.write(config_text(listener.getsockname()[1]))
        file.write(f'[session]\npath={json.dumps(os.path.join(directory, "session.db"))}\nproject="project:signal"\nsession="session:signal"\nagent="agent:signal"\n')
    process, master, slave, before = launch(config)
    try:
        output = until(master, b"Marlin")
        os.write(master, b"first\r")
        output.extend(until(master, b"signal-prefix"))
        process.send_signal(signal.SIGTERM)
        assert process.wait(timeout=10) == 128 + signal.SIGTERM
        drain(master, output)
        assert termios.tcgetattr(slave) == before
        assert b"\x1b[?1049l" in output and b"\x1b[?2004l" in output
    finally:
        close(process, master, slave)
    process, master, slave, _ = launch(config)
    try:
        replayed = until(master, b"Unfinished prior turn")
        assert b"signal-prefix" in replayed
        os.write(master, b"second\r")
        until(master, b"Session state is uncertain")
        assert len(requests) == 1 and requests[0]["input"][0]["content"] == "first"
        os.write(master, b"\x11")
        assert process.wait(timeout=10) == 0
        assert not errors, errors
        print("PTY signals passed: restored modes and preserved unfinished persistent turn")
    finally:
        close(process, master, slave)
        stop.set()
        listener.close()
        thread.join(timeout=5)
