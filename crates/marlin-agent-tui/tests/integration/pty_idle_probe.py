import fcntl, os, pty, select, struct, subprocess, sys, tempfile, termios, time
binary = sys.argv[1]
with tempfile.TemporaryDirectory() as tmp:
    config = os.path.join(tmp, 'config.toml')
    with open(config, 'w') as file:
        file.write('provider="openai"\nmodel="gpt-fixture"\nendpoint="http://127.0.0.1:9/v1/responses"\nprotocol="open-ai-responses"\ncredential_env="MARLIN_TUI_FIXTURE_KEY"\n')
    for trial in range(3):
        master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 30, 100, 0, 0))
        proc = subprocess.Popen([binary, config], stdin=slave, stdout=slave, stderr=slave, env=dict(os.environ, MARLIN_TUI_FIXTURE_KEY='fixture'))
        try:
            start = time.monotonic()
            startup = bytearray()
            while b'Marlin' not in startup and time.monotonic()-start < 15:
                if select.select([master], [], [], .1)[0]: startup.extend(os.read(master, 65536))
            assert b'Marlin' in startup, bytes(startup)
            start = time.monotonic()
            while time.monotonic()-start < .6:
                if select.select([master], [], [], .05)[0]: os.read(master, 65536)
            before = time.monotonic()
            count = 0
            while time.monotonic()-before < 1.0:
                if select.select([master], [], [], .05)[0]: count += len(os.read(master, 65536))
            os.write(master, b'\x11')
            assert proc.wait(timeout=5) == 0
            print('trial', trial+1, 'idle_bytes', count)
        finally:
            if proc.poll() is None: proc.kill(); proc.wait()
            os.close(master); os.close(slave)
