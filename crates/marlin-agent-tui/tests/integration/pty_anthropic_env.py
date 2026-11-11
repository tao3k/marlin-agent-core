import fcntl
import json
import os
import pty
import select
import socket
import struct
import subprocess
import termios
import threading
import time


binary = os.environ.get("MARLIN_TUI_BINARY", "target/debug/marlin-tui")
config = "crates/marlin-agent-tui/example.toml"
listener = socket.socket()
listener.bind(("127.0.0.1", 0))
listener.listen(1)
listener.settimeout(15)
port = listener.getsockname()[1]
observed = []
errors = []


def serve():
    try:
        for answer in ("fixture-connected", "fixture-second"):
            connection, _ = listener.accept()
            with connection:
                data = b""
                while b"\r\n\r\n" not in data:
                    data += connection.recv(4096)
                header, body = data.split(b"\r\n\r\n", 1)
                length = int(next(line.split(b":", 1)[1] for line in header.split(b"\r\n") if line.lower().startswith(b"content-length:")))
                while len(body) < length:
                    body += connection.recv(4096)
                observed.append((header, json.loads(body[:length])))
                events = [
                {"type": "message_start", "message": {"id": "fixture", "model": "deepseek-v4-pro[1m]", "usage": {"input_tokens": 1, "output_tokens": 0}}},
                {"type": "content_block_start", "index": 0, "content_block": {"type": "text", "text": ""}},
                {"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": answer}},
                {"type": "content_block_stop", "index": 0},
                {"type": "message_delta", "delta": {"stop_reason": "end_turn"}, "usage": {"output_tokens": 1}},
                {"type": "message_stop"},
                ]
                wire = b"".join(b"data: " + json.dumps(event).encode() + b"\n\n" for event in events)
                connection.sendall(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n" + wire)
    except Exception as error:
        errors.append(error)


thread = threading.Thread(target=serve, daemon=True)
thread.start()
master, slave = pty.openpty()
fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 30, 100, 0, 0))
env = dict(os.environ, ANTHROPIC_BASE_URL=f"http://127.0.0.1:{port}/anthropic", ANTHROPIC_API_KEY="fixture-key")
process = subprocess.Popen([binary, config], stdin=slave, stdout=slave, stderr=slave, env=env)
output = bytearray()


def until(needle):
    deadline = time.monotonic() + 20
    while needle not in output and time.monotonic() < deadline:
        if select.select([master], [], [], 0.2)[0]:
            try:
                output.extend(os.read(master, 65536))
            except OSError:
                break
    if needle not in output:
        raise AssertionError(f"missing {needle!r}: {bytes(output[-1200:])!r}")


try:
    until(b"Marlin")
    os.write(master, b"hello\r")
    until(b"fixture-connected")
    time.sleep(0.5)
    os.write(master, b"follow-up\r")
    until(b"fixture-second")
    os.write(master, b"\x11")
    assert process.wait(timeout=10) == 0
    thread.join(timeout=5)
    assert not thread.is_alive() and not errors, errors
    header, body = observed[0]
    assert header.startswith(b"POST /anthropic/v1/messages "), header.split(b"\r\n", 1)[0]
    assert b"x-api-key: fixture-key" in header.lower()
    assert body["model"] == "deepseek-v4-pro[1m]"
    assert body["messages"][0]["content"] == "hello"
    follow_up = observed[1][1]["messages"]
    assert follow_up[0]["content"] == "hello"
    assert follow_up[1]["content"][0]["text"] == "fixture-connected"
    assert follow_up[2]["content"] == "follow-up"
    print("Anthropic base URL PTY fixture passed")
finally:
    if process.poll() is None:
        process.kill()
        process.wait()
    os.close(master)
    os.close(slave)
    listener.close()
