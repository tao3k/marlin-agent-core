import fcntl, json, os, pty, re, select, socket, struct, subprocess, tempfile, termios, threading, time
binary = os.environ.get('MARLIN_TUI_BINARY', 'target/debug/marlin-tui')
listener = socket.socket()
listener.bind(('127.0.0.1', 0))
listener.listen(1)
port = listener.getsockname()[1]
requests = []
release = threading.Event()
def serve():
    try:
        conn, _ = listener.accept()
    except OSError:
        return
    with conn:
        buf = b''
        while b'\r\n\r\n' not in buf:
            buf += conn.recv(4096)
        headers, body = buf.split(b'\r\n\r\n', 1)
        length = int(next(line.split(b':', 1)[1] for line in headers.split(b'\r\n') if line.lower().startswith(b'content-length:')))
        while len(body) < length:
            body += conn.recv(4096)
        requests.append(json.loads(body[:length]))
        conn.sendall(b'HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n' + b'data: {"type":"response.output_text.delta","delta":"fixture-live"}\n\n')
        assert release.wait(10)
        conn.sendall(b'data: {"type":"response.output_text.delta","delta":"done"}\n\ndata: {"type":"response.completed","response":{"id":"fixture","model":"gpt-fixture","status":"completed","output":[{"type":"message","content":[{"type":"output_text","text":"fixture-livedone"}]}]}}\n\n')
threading.Thread(target=serve, daemon=True).start()
with tempfile.TemporaryDirectory() as temp:
    path = os.path.join(temp, 'config.toml')
    with open(path,'w') as f:
        f.write(f'provider = "openai"\nmodel = "gpt-fixture"\nendpoint = "http://127.0.0.1:{port}/v1/responses"\nprotocol = "open-ai-responses"\ncredential_env = "MARLIN_TUI_FIXTURE_KEY"\n')
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 30, 100, 0, 0))
    before = termios.tcgetattr(slave)
    env = dict(os.environ, MARLIN_TUI_FIXTURE_KEY='fixture')
    proc = subprocess.Popen([binary, path], stdin=slave, stdout=slave, stderr=slave, env=env)
    out = bytearray()
    def until(needle, seconds=15):
        deadline = time.monotonic() + seconds
        while needle not in out and time.monotonic() < deadline:
            if select.select([master], [], [], .2)[0]:
                try: out.extend(os.read(master, 65536))
                except OSError: break
        if needle not in out: raise AssertionError(f'missing {needle!r}; output tail {bytes(out[-1200:])!r}')
    def status_line():
        # Ratatui emits only changed cells, so rebuild the last terminal row
        # from cursor positions instead of searching raw output fragments.
        text = out.decode('utf-8', 'replace')
        csi = re.compile(r'\x1b\[([0-9;?]*)([A-Za-z])')
        row, col, index = 1, 1, 0
        cells = [' '] * 100
        while index < len(text):
            match = csi.match(text, index)
            if match:
                args, op = match.groups()
                nums = [int(n) if n else 0 for n in args.lstrip('?').split(';')]
                value = nums[0] or 1
                if op in ('H', 'f'):
                    row, col = value, (nums[1] or 1) if len(nums) > 1 else 1
                elif op == 'G': col = value
                elif op == 'A': row -= value
                elif op == 'B': row += value
                elif op == 'C': col += value
                elif op == 'D': col -= value
                elif op == 'K' and row == 30:
                    start = 0 if nums[0] in (1, 2) else max(0, col - 1)
                    end = min(100, col) if nums[0] == 1 else 100
                    cells[start:end] = [' '] * (end - start)
                index = match.end()
                continue
            ch = text[index]
            if ch == '\r': col = 1
            elif ch == '\n': row += 1
            elif ch >= ' ' and ch != '\x7f':
                if row == 30 and 1 <= col <= 100: cells[col - 1] = ch
                col += 1
            index += 1
        return ''.join(cells)
    def until_status(needle, seconds=15):
        deadline = time.monotonic() + seconds
        while needle not in status_line() and time.monotonic() < deadline:
            if select.select([master], [], [], .2)[0]:
                try: out.extend(os.read(master, 65536))
                except OSError: break
        assert needle in status_line(), f'missing status {needle!r}: {status_line()!r}'
    try:
        until(b'Marlin')
        assert b'GERBIL-BUILD' not in out, 'internal SDK module progress leaked into the terminal'
        os.write(master, b'\x1b[Z')
        until_status('Transcript: Up/Down scroll')
        os.write(master, b'\t')
        until_status('Message: Enter send')
        # Direct slash entry discovers and completes the Scheme-declared command.
        os.write(master, b'/hel')
        until_status('Command: Enter run')
        until(b'Keyboard shortcuts')
        os.write(master, b'\t\r')
        until_status('Message: Enter send')
        until(b'stream-open')
        assert not requests, 'command completion triggered a model request'
        os.write(master, b'\x1b[200~' + 'hello 中\r\nworld\r\nline-three\r\nline-four\r\nline-five'.encode() + b'\x1b[201~')
        until(b'line-five')
        # Scheme owns focus; command mode must retain the unsent draft.
        os.write(master, b'\t')
        until_status('Commands: Enter open')
        os.write(master, b'\r')
        until_status('Enter run command')
        until(b'Keyboard shortcuts')
        os.write(master, b'\x0f\t')
        until_status('Transcript: Up/Down scroll')
        os.write(master, b'\t')
        until_status('Message: Enter send')
        deadline = time.monotonic() + 5
        while b'\x1b[?25h' not in out[out.rfind(b'line-five'):] and time.monotonic() < deadline:
            if select.select([master], [], [], .2)[0]:
                out.extend(os.read(master, 65536))
        assert b'\x1b[?25h' in out[out.rfind(b'line-five'):], 'long draft hid the cursor'
        assert not requests, 'paste triggered a model request before Enter'
        os.write(master, b'\x1b[A!\x1b[B?\x01^\x05$\x1b[H<\x1b[F>')
        time.sleep(.3)
        os.write(master, b'\r')
        until(b'fixture-live')
        assert not release.is_set()
        release.set()
        until(b'done')
        os.write(master, b'\x11')
        assert proc.wait(timeout=5) == 0
        while select.select([master], [], [], 0)[0]:
            try: out.extend(os.read(master, 65536))
            except OSError: break
        assert requests and requests[0]['input'][0]['content'] == 'hello 中\nworld\nline-three\nline-four!\n<^line-five?$>' and requests[0]['stream'] is True, requests
        assert termios.tcgetattr(slave) == before, 'terminal mode was not restored'
        assert b'\x1b[?1049h' in out and b'\x1b[?1049l' in out, 'alternate screen not restored'
        assert b'\x1b[?2004h' in out and b'\x1b[?2004l' in out, 'paste mode not restored'
        print('PTY fixture passed: paste waited for Enter, Unicode multiline streamed, terminal and paste mode restored')
    finally:
        if proc.poll() is None: proc.kill(); proc.wait()
        os.close(master); os.close(slave); listener.close()
