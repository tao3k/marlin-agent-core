"""Check scroll anchoring during streamed output with a local Unix PTY."""

import fcntl
import json
import os
import pty
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
listener.listen(1)
port = listener.getsockname()[1]
release = threading.Event()
sent = threading.Event()
initial_sent = threading.Event()
errors = []


def serve():
    try:
        connection, _ = listener.accept()
        with connection:
            data = b""
            while b"\r\n\r\n" not in data:
                data += connection.recv(4096)
            headers, body = data.split(b"\r\n\r\n", 1)
            length = int(next(line.split(b":", 1)[1] for line in headers.split(b"\r\n") if line.lower().startswith(b"content-length:")))
            while len(body) < length:
                body += connection.recv(4096)
            connection.sendall(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n" + b'data: {"type":"response.output_text.delta","delta":"INITIAL"}\n\n')
            initial_sent.set()
            if not release.wait(15):
                raise TimeoutError("scroll fixture was not released")
            chunk = "\n" + "\n".join(f"FRESH_ROW_{i:02}" for i in range(30))
            events = [
                {"type": "response.output_text.delta", "delta": chunk},
                {"type": "response.completed", "response": {"id": "fixture", "model": "gpt-fixture", "status": "completed", "output": [{"type": "message", "content": [{"type": "output_text", "text": "INITIAL" + chunk}]}]}},
            ]
            connection.sendall(b"".join(b"data: " + json.dumps(event).encode() + b"\n\n" for event in events))
            sent.set()
    except Exception as error:
        errors.append(error)
        sent.set()


thread = threading.Thread(target=serve, daemon=True)
thread.start()
with tempfile.TemporaryDirectory() as directory:
    config = os.path.join(directory, "config.toml")
    with open(config, "w") as file:
        file.write(f'provider="openai"\nmodel="gpt-fixture"\nendpoint="http://127.0.0.1:{port}/v1/responses"\nprotocol="open-ai-responses"\ncredential_env="MARLIN_TUI_FIXTURE_KEY"\n')
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 20, 80, 0, 0))
    process = subprocess.Popen([binary, config], stdin=slave, stdout=slave, stderr=slave, env=dict(os.environ, MARLIN_TUI_FIXTURE_KEY="fixture"))
    output = bytearray()

    def pump(seconds):
        deadline = time.monotonic() + seconds
        while time.monotonic() < deadline:
            if select.select([master], [], [], min(.1, max(0, deadline - time.monotonic())))[0]:
                try:
                    output.extend(os.read(master, 65536))
                except OSError:
                    break

    def until(needle):
        deadline = time.monotonic() + 15
        while needle not in output and time.monotonic() < deadline:
            pump(.1)
        if needle not in output:
            raise AssertionError(f"missing {needle!r}: {bytes(output[-1200:])!r}")

    try:
        until(b"Marlin")
        prompt = "\n".join(f"HISTORY_ROW_{i:02}" for i in range(50)).encode()
        os.write(master, b"\x1b[200~" + prompt + b"\x1b[201~\r")
        assert initial_sent.wait(10), "fixture did not send the initial chunk"
        # Ratatui may split text into cell updates over pre-existing content.
        # The assistant role establishes that the client rendered the chunk;
        # searching the raw output for its contiguous text is not reliable.
        until(b"Assistant")
        for rows, cols in [(12, 32), (8, 12), (24, 100), (20, 80)]:
            fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))
            pump(.25)
            assert process.poll() is None, f"TUI exited during resize to {cols}x{rows}"
        output.clear()
        os.write(master, b"\x1b[5~")
        until(b"HISTORY_ROW_")
        pump(.3)
        output.clear()
        release.set()
        assert sent.wait(10) and not errors, errors
        pump(1)
        assert b"FRESH_ROW_" not in output, "new output replaced the scrolled history"
        output.clear()
        for _ in range(6):
            os.write(master, b"\x1b[6~")
            pump(.1)
        # Ratatui emits terminal-cell differences, so later rows can be updates
        # to only their numeric suffix rather than complete strings.
        until(b"FRESH_ROW_00")
        # Search is a modal preview. Its Enter accepts into the composer; it
        # must not submit a second request, and Escape restores the draft.
        os.write(master, b"draft before search")
        pump(.2)
        output.clear()
        os.write(master, b"\x12")  # Ctrl-R
        until(b"Search:")
        os.write(master, b"HISTORY_ROW_05")
        pump(.3)
        output.clear()
        os.write(master, b"\x1b")  # cancel search
        until(b"draft before search")
        output.clear()
        os.write(master, b"\x12HISTORY_ROW_05")
        until(b"Search:")
        os.write(master, b"\r")  # accept preview, do not submit
        pump(.3)
        assert process.poll() is None, "TUI exited while searching history"
        assert not select.select([listener], [], [], .2)[0], "accepting history sent a second request"
        os.write(master, b"\x11")
        assert process.wait(timeout=10) == 0
        print("PTY fixture passed: resize, streaming scroll, and history search input")
    finally:
        release.set()
        if process.poll() is None:
            process.kill()
            process.wait()
        os.close(master)
        os.close(slave)
        listener.close()
        thread.join(timeout=2)
