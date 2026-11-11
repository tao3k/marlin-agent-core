"""Exercise keyboard cancellation while a local SSE producer is active."""

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


listener = socket.socket()
listener.bind(("127.0.0.1", 0))
listener.listen(1)
port = listener.getsockname()[1]
stop = threading.Event()
connected = threading.Event()


def serve():
    try:
        conn, _ = listener.accept()
        with conn:
            conn.settimeout(2)
            headers = b""
            while b"\r\n\r\n" not in headers:
                headers += conn.recv(4096)
            head, body = headers.split(b"\r\n\r\n", 1)
            length = int(next(line.split(b":", 1)[1] for line in head.split(b"\r\n") if line.lower().startswith(b"content-length:")))
            while len(body) < length:
                body += conn.recv(4096)
            conn.sendall(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n")
            connected.set()
            event = b'data: {"type":"response.output_text.delta","delta":"PIPE "}\n\n'
            while not stop.is_set():
                conn.sendall(event * 64)
    except (BrokenPipeError, ConnectionResetError, TimeoutError, OSError):
        pass


thread = threading.Thread(target=serve, daemon=True)
thread.start()
with tempfile.TemporaryDirectory() as temp:
    config = os.path.join(temp, "config.toml")
    with open(config, "w") as file:
        file.write(f'provider="openai"\nmodel="gpt-fixture"\nendpoint="http://127.0.0.1:{port}/v1/responses"\nprotocol="open-ai-responses"\ncredential_env="MARLIN_TUI_FIXTURE_KEY"\n')
        if os.environ.get("MARLIN_PTY_PERSIST") == "1":
            file.write(f'[session]\npath={json.dumps(os.path.join(temp, "session.db"))}\nproject="project:flood"\nsession="session:flood"\nagent="agent:flood"\n')
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 30, 100, 0, 0))
    before = termios.tcgetattr(slave)
    process = subprocess.Popen(["target/debug/marlin-tui", config], stdin=slave, stdout=slave, stderr=slave, env=dict(os.environ, MARLIN_TUI_FIXTURE_KEY="fixture"))
    output = bytearray()

    def wait_for(text, timeout):
        deadline = time.monotonic() + timeout
        while text not in output and time.monotonic() < deadline:
            if select.select([master], [], [], 0.1)[0]:
                output.extend(os.read(master, 65536))
        assert text in output, f"missing {text!r}, output tail={bytes(output[-1000:])!r}"

    def wait_for_cancel(timeout):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            plain = re.sub(rb"\x1b\[[0-?]*[ -/]*[@-~]", b"", output)
            if b"Requestcancelledlocally." in plain.replace(b" ", b""):
                return
            if select.select([master], [], [], 0.1)[0]:
                output.extend(os.read(master, 65536))
        raise AssertionError(f"missing cancellation, output tail={bytes(output[-1000:])!r}")

    try:
        wait_for(b"Marlin", 15)
        os.write(master, b"flood test\r")
        assert connected.wait(5)
        wait_for(b"PIPE", 10)
        started = time.monotonic()
        os.write(master, b"\x03")
        wait_for_cancel(10)
        elapsed = time.monotonic() - started
        stop.set()
        os.write(master, b"\x11")
        assert process.wait(timeout=5) == 0
        assert termios.tcgetattr(slave) == before
        print(f"PTY flood cancellation passed (persistent={os.environ.get('MARLIN_PTY_PERSIST') == '1'}): observed locally after {elapsed:.3f}s")
    finally:
        stop.set()
        if process.poll() is None:
            process.kill()
            process.wait()
        os.close(master)
        os.close(slave)
        listener.close()
