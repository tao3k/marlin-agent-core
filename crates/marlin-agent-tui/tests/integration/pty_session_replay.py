"""Exercise local session persistence through the real terminal and SSE path."""

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
listener.listen(5)
port = listener.getsockname()[1]
requests = []
errors = []


def serve():
    try:
        for answer in ("answer-one", "answer-foreign", "answer-two", "answer-probe", "answer-three", "answer-four", "answer-five"):
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
                events = (
                    {"type": "response.output_text.delta", "delta": answer},
                    {"type": "response.completed", "response": {"id": "fixture", "model": "gpt-fixture", "status": "completed", "output": [{"type": "message", "role": "assistant", "content": [{"type": "output_text", "text": answer}]}]}},
                )
                wire = b"".join(b"data: " + json.dumps(event).encode() + b"\n\n" for event in events)
                connection.sendall(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n" + wire)
    except Exception as error:
        errors.append(error)


thread = threading.Thread(target=serve, daemon=True)
thread.start()
with tempfile.TemporaryDirectory() as directory:
    config = os.path.join(directory, "config.toml")
    database = os.path.join(directory, "session.db")
    with open(config, "w") as file:
        file.write(f'provider="openai"\nmodel="gpt-fixture"\nendpoint="http://127.0.0.1:{port}/v1/responses"\nprotocol="open-ai-responses"\ncredential_env="MARLIN_TUI_FIXTURE_KEY"\n[session]\npath={json.dumps(database)}\nproject="project:pty"\nsession="session:pty"\nagent="agent:pty"\n[models.alt]\nprovider="openai"\nmodel="gpt-fixture-alt"\nendpoint="http://127.0.0.1:{port}/v1/responses"\nprotocol="open-ai-responses"\ncredential_env="MARLIN_TUI_FIXTURE_KEY"\n')
    foreign_config = os.path.join(directory, "foreign.toml")
    with open(config) as file:
        foreign_contents = file.read().replace('agent="agent:pty"', 'agent="agent:foreign"')
    with open(foreign_config, "w") as file:
        file.write(foreign_contents)

    def run(prompt, session, answer, switch_to=None, selected_config=config):
        master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 30, 100, 0, 0))
        process = subprocess.Popen([binary, selected_config, "--session", session], stdin=slave, stdout=slave, stderr=slave, env=dict(os.environ, MARLIN_TUI_FIXTURE_KEY="fixture"))
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
                raise AssertionError(f"missing {needle!r}: {bytes(output[-1200:])!r}")

        try:
            until(b"Marlin")
            until(session.encode())
            os.write(master, prompt + b"\r")
            until(answer)
            time.sleep(.5)
            if switch_to:
                os.write(master, b'/session "session:foreign"\r')
                until(b"retained:")
                os.write(master, b"probe\r")
                until(b"answer-probe")
                time.sleep(.5)
                os.write(master, b"/sessions\r")
                until(switch_to.encode())
                os.write(master, b'/session "' + switch_to.encode() + b'"\r')
                until(b"answer-one")
                os.write(master, b"third\x0fcontext\r")
                until(b"accounting:")
                assert len(requests) == 4, "context command dispatched a model request"
                os.write(master, b"\x1bp\x1bn\r")
                until(b"answer-three")
                time.sleep(.5)
                os.write(master, b'blocked\x0finspect "session:A"\r')
                until(b"Read-only")
                os.write(master, b"\r")
                until(b"retained.")
                time.sleep(.5)
                assert len(requests) == 5, "read-only inspection sent a model request"
                os.write(master, b"\x7f" * len(b"blocked"))
                os.write(master, b'/session "session:A"\r')
                time.sleep(.5)
                os.write(master, b"fourth\r")
                until(b"answer-four")
                time.sleep(.5)
                output.clear()
                os.write(master, b'draft-b\x0fsession "session:B"\r')
                until(b"probe")
                output.clear()
                os.write(master, b"\x0fmodel alt\r\r")
                until(b"retained.")
                assert b"blocked" in output, "model selection did not warn about the incompatible continuation"
                assert len(requests) == 6, "incompatible continuation dispatched a request"
                os.write(master, b"\x0fmodel default\r\r")
                # Incremental redraw may reuse cells from an earlier row.
                until(b"five")
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
        run(b"first", "session:A", b"answer-one")
        run(b"foreign", "session:foreign", b"answer-foreign", selected_config=foreign_config)
        run(b"second", "session:B", b"answer-two", "session:A")
        thread.join(timeout=5)
        assert not thread.is_alive() and not errors, errors
        def text_content(message):
            content = message["content"]
            return content if isinstance(content, str) else "".join(part["text"] for part in content if part.get("type") == "output_text")

        second = requests[2]["input"]
        assert [text_content(message) for message in second] == ["second"], second
        probe = requests[3]["input"]
        assert [text_content(message) for message in probe] == ["second", "answer-two", "probe"], probe
        third = requests[4]["input"]
        assert [text_content(message) for message in third] == ["first", "answer-one", "third"], third
        fourth = requests[5]["input"]
        assert [text_content(message) for message in fourth] == ["first", "answer-one", "third", "answer-three", "fourth"], fourth
        fifth = requests[6]["input"]
        assert [text_content(message) for message in fifth] == ["second", "answer-two", "probe", "answer-probe", "draft-b"], fifth
        assert requests[5]["model"] == "gpt-fixture"
        assert requests[6]["model"] == "gpt-fixture"
        list_env = dict(os.environ)
        list_env.pop("MARLIN_TUI_FIXTURE_KEY", None)
        listing = subprocess.run([binary, config, "--list-sessions"], env=list_env, capture_output=True, check=True, timeout=10)
        assert json.loads(listing.stdout) == {"items": ["session:A", "session:B"], "next_cursor": None}, listing.stdout
        following = subprocess.run([binary, config, "--list-sessions", "--after", "session:A"], env=list_env, capture_output=True, check=True, timeout=10)
        assert json.loads(following.stdout) == {"items": ["session:B"], "next_cursor": None}, following.stdout
        missing_db = os.path.join(directory, "missing.db")
        missing_config = os.path.join(directory, "missing.toml")
        with open(config) as file:
            contents = file.read()
        with open(missing_config, "w") as file:
            file.write(contents.replace(json.dumps(database), json.dumps(missing_db)))
        missing = subprocess.run([binary, missing_config, "--list-sessions"], env=list_env, capture_output=True, timeout=10)
        assert missing.returncode == 1 and b"session database does not exist" in missing.stderr
        assert not os.path.exists(missing_db)
        print("PTY session selection passed: separate histories and resumed context were preserved")
    finally:
        listener.close()
