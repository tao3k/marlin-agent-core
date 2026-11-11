"""Exercise Anthropic thinking continuation across a restart."""

import fcntl
import json
import os
import pty
import select
import socket
import sqlite3
import struct
import subprocess
import tempfile
import termios
import threading
import time
from contextlib import closing

binary = os.environ.get("MARLIN_TUI_BINARY", "target/debug/marlin-tui")
listener = socket.socket()
listener.bind(("127.0.0.1", 0))
listener.listen(2)
listener.settimeout(15)
port = listener.getsockname()[1]
requests = []
errors = []


def serve():
    try:
        for answer in ("answer-fixture", "answer-live", "answer-restart"):
            connection, _ = listener.accept()
            with connection:
                data = b""
                while b"\r\n\r\n" not in data:
                    data += connection.recv(4096)
                header, body = data.split(b"\r\n\r\n", 1)
                length = int(next(line.split(b":", 1)[1] for line in header.split(b"\r\n") if line.lower().startswith(b"content-length:")))
                while len(body) < length:
                    body += connection.recv(4096)
                requests.append(json.loads(body[:length]))
                events = [
                    {"type": "message_start", "message": {"id": "fixture", "model": "fixture", "usage": {"input_tokens": 1, "output_tokens": 0}}},
                    {"type": "content_block_start", "index": 0, "content_block": {"type": "thinking", "thinking": ""}},
                    {"type": "content_block_delta", "index": 0, "delta": {"type": "thinking_delta", "thinking": "reasoning-fixture"}},
                    {"type": "content_block_delta", "index": 0, "delta": {"type": "signature_delta", "signature": "signature-fixture"}},
                    {"type": "content_block_stop", "index": 0},
                    {"type": "content_block_start", "index": 1, "content_block": {"type": "text", "text": ""}},
                    {"type": "content_block_delta", "index": 1, "delta": {"type": "text_delta", "text": answer}},
                    {"type": "content_block_stop", "index": 1},
                    {"type": "message_delta", "delta": {"stop_reason": "end_turn"}, "usage": {"output_tokens": 1}},
                    {"type": "message_stop"},
                ]
                wire = b"".join(b"data: " + json.dumps(event).encode() + b"\n\n" for event in events)
                connection.sendall(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n" + wire)
    except Exception as error:
        errors.append(error)


thread = threading.Thread(target=serve, daemon=True)
thread.start()
with tempfile.TemporaryDirectory() as directory:
    config = os.path.join(directory, "config.toml")
    with open(config, "w") as file:
        file.write(f'provider="fixture"\nmodel="fixture"\nprotocol="anthropic"\ncredential_env="MARLIN_TUI_FIXTURE_KEY"\n[anthropic]\nthinking={{type="adaptive"}}\neffort="low"\nmax_tokens=8192\n[session]\npath={json.dumps(os.path.join(directory, "session.db"))}\nproject="project:pty"\nsession="session:pty"\nagent="agent:pty"\n')
    env = dict(os.environ, MARLIN_TUI_FIXTURE_KEY="fixture", ANTHROPIC_BASE_URL=f"http://127.0.0.1:{port}")

    def run(first, changed_credential=False):
        master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 30, 100, 0, 0))
        selected_env = dict(env, MARLIN_TUI_FIXTURE_KEY="changed") if changed_credential else env
        process = subprocess.Popen([binary, config], stdin=slave, stdout=slave, stderr=slave, env=selected_env)
        output = bytearray()

        def until(needle):
            deadline = time.monotonic() + 20
            while needle not in output and time.monotonic() < deadline:
                if select.select([master], [], [], .2)[0]:
                    try:
                        output.extend(os.read(master, 65536))
                    except OSError:
                        break
            if needle not in output:
                raise AssertionError(f"missing {needle!r}: {bytes(output[-1500:])!r}")

        try:
            until(b"Marlin")
            if first:
                os.write(master, b"hello\r")
            until(b"reasoning-fixture")
            until(b"answer-fixture")
            time.sleep(.5)
            output.clear()
            os.write(master, b"follow-up\r")
            # Incremental terminal redraw can reuse the previous answer prefix.
            if changed_credential:
                until(b"restored")
            else:
                until(b"answer-live" if first else b"restart")
            time.sleep(.5)
            os.write(master, b"\x11")
            assert process.wait(timeout=10) == 0
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()
            os.close(master)
            os.close(slave)

    try:
        run(True)
        with closing(sqlite3.connect(os.path.join(directory, "session.db"))) as database:
            bodies = [bytes(row[0]) for row in database.execute("SELECT body FROM session_events")]
        assert any(b"signature-fixture" in body and b"reasoning-fixture" in body for body in bodies), "signed provider continuation was not stored"
        run(False)
        thread.join(timeout=5)
        assert not thread.is_alive() and not errors, errors
        run(False, changed_credential=True)
        listener.settimeout(.5)
        try:
            extra, _ = listener.accept()
            extra.close()
            raise AssertionError("credential mismatch dispatched a request")
        except TimeoutError:
            pass
        with closing(sqlite3.connect(os.path.join(directory, "session.db"))) as database:
            finished = [json.loads(row[0]) for row in database.execute("SELECT body FROM session_events WHERE event_kind = 'marlin.conversation.turn.finished.v1'")]
        assert any("RejectedBeforeDispatch" in record for record in finished)
        assert len(requests) == 3
        for request in requests:
            assert request["thinking"] == {"type": "adaptive"}
            assert request["output_config"]["effort"] == "low"
            assert request["max_tokens"] == 8192
        for request in requests[1:]:
            blocks = request["messages"][1]["content"]
            assert blocks[0] == {"type": "thinking", "thinking": "reasoning-fixture", "signature": "signature-fixture"}
            assert blocks[1]["text"] == "answer-fixture"
        print("Anthropic continuation and rejected credential change passed")
    finally:
        listener.close()
