"""Kill a TUI after provider dispatch and check that restart does not retry."""

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
listener.listen(4)
listener.settimeout(.2)
port = listener.getsockname()[1]
requests = []
errors = []
stop = threading.Event()
first_open = threading.Event()


def handle(connection):
    try:
        connection.settimeout(10)
        with connection:
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
            connection.sendall(b'HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\ndata: {"type":"response.output_text.delta","delta":"held-output"}\n\ndata: {"type":"response.output_text.delta","delta":"-timed"}\n\n')
            first_open.set()
            stop.wait(30)
    except (BrokenPipeError, ConnectionResetError):
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


server = threading.Thread(target=serve, daemon=True)
server.start()
with tempfile.TemporaryDirectory() as directory:
    config = os.path.join(directory, "config.toml")
    with open(config, "w") as file:
        file.write(f'provider="openai"\nmodel="gpt-fixture"\nendpoint="http://127.0.0.1:{port}/v1/responses"\nprotocol="open-ai-responses"\ncredential_env="MARLIN_TUI_FIXTURE_KEY"\n[session]\npath={json.dumps(os.path.join(directory, "session.db"))}\nproject="project:crash"\nsession="session:crash"\nagent="agent:crash"\n')

    def launch(session=None):
        master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 30, 120, 0, 0))
        args = ["target/debug/marlin-tui", config]
        if session is not None:
            args.extend(["--session", session])
        process = subprocess.Popen(args, stdin=slave, stdout=slave, stderr=slave, env=dict(os.environ, MARLIN_TUI_FIXTURE_KEY="fixture"))
        return process, master, slave

    def until(master, needle):
        output = bytearray()
        deadline = time.monotonic() + 20
        def visible():
            return re.sub(rb"\x1b\[[0-?]*[ -/]*[@-~]", b"", output).replace(b" ", b"")

        while needle.replace(b" ", b"") not in visible() and time.monotonic() < deadline:
            if select.select([master], [], [], .2)[0]:
                try:
                    output.extend(os.read(master, 65536))
                except OSError:
                    break
        if needle.replace(b" ", b"") not in visible():
            raise AssertionError(f"missing {needle!r}: {bytes(output[-1200:])!r}")
        return visible()

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
        until(master, b"-timed")
        assert first_open.wait(5)
        time.sleep(1.5)
        process.kill()
        process.wait(timeout=5)
    finally:
        close(process, master, slave)

    process, master, slave = launch()
    try:
        reopened = until(master, b"Unfinished prior turn")
        assert b"held-output-timed" in reopened, "timed partial-output checkpoint was not replayed"
        os.write(master, b"second\r")
        until(master, b"Session state is uncertain")
        time.sleep(1)
        assert len(requests) == 1, requests
        assert requests[0]["input"][0]["content"] == "first"
        assert not errors, errors
        # The blocked draft is retained; clear it before entering the command.
        os.write(master, b'\x7f' * len(b"second") + b'/session "session:separate"\r')
        # Ratatui emits a screen diff, which need not contain the entire new ID.
        time.sleep(.5)
        os.write(master, b"new conversation\r")
        until(master, b"-timed")
        assert len(requests) == 2, requests
        assert [item["content"] for item in requests[1]["input"]] == ["new conversation"]
        os.write(master, b"\x11")
        assert process.wait(timeout=5) == 0
        print("PTY crash recovery passed: unfinished request remained blocked while switching to a separate session")
    finally:
        close(process, master, slave)
        stop.set()
        listener.close()
        server.join(timeout=5)
