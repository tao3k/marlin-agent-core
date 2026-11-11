"""Read a temporary worktree through the real terminal command."""

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

binary = os.path.abspath(os.environ.get("MARLIN_TUI_BINARY", "target/debug/marlin-tui"))
listener = socket.socket()
listener.bind(("127.0.0.1", 0))
listener.listen(1)
port = listener.getsockname()[1]
requests = []


def serve():
    connection, _ = listener.accept()
    with connection:
        data = b""
        while b"\r\n\r\n" not in data:
            data += connection.recv(4096)
        headers, body = data.split(b"\r\n\r\n", 1)
        length = int(next(line.split(b":", 1)[1] for line in headers.split(b"\r\n") if line.lower().startswith(b"content-length:")))
        while len(body) < length:
            body += connection.recv(4096)
        requests.append(json.loads(body[:length]))
        events = [
            {"type": "response.output_text.delta", "delta": "workspace-answer"},
            {"type": "response.completed", "response": {"id": "fixture", "model": "gpt-fixture", "status": "completed", "output": [{"type": "message", "content": [{"type": "output_text", "text": "workspace-answer"}]}]}},
        ]
        wire = b"".join(b"data: " + json.dumps(event).encode() + b"\n\n" for event in events)
        connection.sendall(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n" + wire)


thread = threading.Thread(target=serve, daemon=True)
thread.start()
with tempfile.TemporaryDirectory() as directory:
    subprocess.run(["git", "init", "-q", directory], check=True)
    index = os.path.join(directory, ".git", "index")
    assert not os.path.exists(index)
    tracked = os.path.join(directory, "tracked-fixture.txt")
    with open(tracked, "w") as file:
        file.write("staged-diff-content\n")
    subprocess.run(["git", "-C", directory, "add", "--", "tracked-fixture.txt"], check=True)
    with open(index, "rb") as file:
        index_before = file.read()
    with open(tracked, "w") as file:
        file.write("worktree-diff-content\n")
    with open(os.path.join(directory, "workspace-fixture.txt"), "w") as file:
        file.write("untracked")
    config = os.path.join(directory, "config.toml")
    with open(config, "w") as file:
        file.write(f'provider="openai"\nmodel="gpt-fixture"\nendpoint="http://127.0.0.1:{port}/v1/responses"\nprotocol="open-ai-responses"\ncredential_env="MARLIN_TUI_FIXTURE_KEY"\n')
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 100, 0, 0))
    before = termios.tcgetattr(slave)
    process = subprocess.Popen([binary, config], cwd=directory, stdin=slave, stdout=slave, stderr=slave, env=dict(os.environ, MARLIN_TUI_FIXTURE_KEY="fixture"))
    output = bytearray()

    def until(needle):
        deadline = time.monotonic() + 15
        while needle not in output and time.monotonic() < deadline:
            if select.select([master], [], [], .2)[0]:
                try:
                    output.extend(os.read(master, 65536))
                except OSError:
                    break
        if needle not in output:
            raise AssertionError(f"missing {needle!r}: {bytes(output[-1000:])!r}")

    try:
        until(b"Marlin")
        os.write(master, b"/status\r")
        until(b"workspace-fixture.txt")
        output.clear()
        os.write(master, b'/diff "tracked-fixture.txt"\r')
        until(b"worktree-diff-content")
        output.clear()
        os.write(master, b"/diff staged\r")
        until(b"staged-diff-content")
        os.write(master, b"hello\x03")
        time.sleep(.3)
        os.write(master, b"\r")
        until(b"workspace-answer")
        assert [item["content"] for item in requests[0]["input"]] == ["hello"]
        os.write(master, b"\x11")
        assert process.wait(timeout=10) == 0
        assert termios.tcgetattr(slave) == before
        assert os.path.exists(os.path.join(directory, "workspace-fixture.txt"))
        with open(index, "rb") as file:
            assert file.read() == index_before, "workspace reads changed the Git index"
        print("PTY workspace fixture passed: status and both diff scopes stayed out of model context")
    finally:
        if process.poll() is None:
            process.kill()
            process.wait()
        os.close(master)
        os.close(slave)
        listener.close()
        thread.join(timeout=2)
