#!/usr/bin/env python3
import errno
import fcntl
import json
import os
import pty
import re
import select
import signal
import struct
import sys
import termios
import time
from pathlib import Path

binary, session_id, log_path = sys.argv[1:]
assert Path(binary).is_absolute() and Path(binary).is_file()
assert os.environ.get('HOME', '').startswith('/tmp/aoe-policy-')
ansi = re.compile(r'\x1b\[[0-?]*[ -/]*[@-~]|\x1b\][^\x07]*(?:\x07|\x1b\\)')
argv = [binary, 'session', 'attach', session_id]
pid, master = pty.fork()
if pid == 0:
    os.execvpe(binary, argv, dict(os.environ, TERM='xterm-256color', COLUMNS='132', LINES='42'))
fcntl.ioctl(master, termios.TIOCSWINSZ, struct.pack('HHHH', 42, 132, 0, 0))
output = bytearray()
status = None
forced = False
observed = False

def exited():
    global status
    if status is not None:
        return True
    child, result = os.waitpid(pid, os.WNOHANG)
    if child:
        status = result
    return bool(child)

def read_available(deadline):
    remaining = deadline - time.monotonic()
    if remaining <= 0:
        return
    readable, _, _ = select.select([master], [], [], min(remaining, 0.2))
    if readable:
        try:
            output.extend(os.read(master, 65536))
        except OSError as error:
            if error.errno != errno.EIO:
                raise

def await_exit(seconds):
    deadline = time.monotonic() + seconds
    while not exited() and time.monotonic() < deadline:
        read_available(deadline)
    return exited()

try:
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        read_available(deadline)
        visible = ansi.sub('', output.decode('utf-8', 'replace'))
        if 'PRIVATE FIXTURE READY' in visible:
            observed = True
            break
        if exited():
            raise AssertionError('CLI attach exited before the real Native surface: ' + visible[-5000:])
    assert observed, 'CLI attach did not render the real Native surface: ' + visible[-5000:]
    os.write(master, b'\x02d')
    if not await_exit(8):
        forced = True
        os.kill(pid, signal.SIGTERM)
        if not await_exit(5):
            os.kill(pid, signal.SIGKILL)
            _, status = os.waitpid(pid, 0)
finally:
    if status is None and not exited():
        os.kill(pid, signal.SIGTERM)
        if not await_exit(5):
            os.kill(pid, signal.SIGKILL)
            _, status = os.waitpid(pid, 0)
    os.close(master)
    Path(log_path).write_bytes(output)
assert not forced, 'CLI attach needed forced cleanup instead of normal tmux detach'
assert os.waitstatus_to_exitcode(status) == 0, status
print('TERMINAL_5040_ACTUAL_ATTACH_GREEN', json.dumps({'argv': argv, 'native_surface_observed': observed, 'exit': os.waitstatus_to_exitcode(status), 'forced': forced, 'ansi_log': log_path}))
