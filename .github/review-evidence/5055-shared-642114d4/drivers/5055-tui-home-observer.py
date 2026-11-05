#!/usr/bin/python3
"""Private actual Home observer. Importing this module starts no processes."""
import argparse
import hashlib
import json
import os
import pathlib
import shutil
import subprocess
import tempfile
import time
import urllib.request


def _until(predicate, label, seconds=35):
    deadline = time.monotonic() + seconds
    while True:
        value = predicate()
        if value:
            return value
        if time.monotonic() >= deadline:
            raise TimeoutError(label)
        time.sleep(0.05)


def _birth(pid):
    try:
        fields = pathlib.Path(f'/proc/{pid}/stat').read_text().rsplit(')', 1)[1].split()
        return None if fields[0] == 'Z' else fields[19]
    except FileNotFoundError:
        return None


def capture_home_stage(binary, env, cwd, title, output, expected, *, commit_fresh=False):
    """Observe an existing session; only explicit commit_fresh may confirm Fresh.

    `output` is an empty/new evidence directory. `expected` is a sequence of
    substrings required in the selected Home frame, not in the restart dialog.
    The caller owns Native birth/count assertions before and after this call.
    """
    binary = pathlib.Path(binary)
    output = pathlib.Path(output)
    cwd = pathlib.Path(cwd)
    assert binary.is_absolute() and binary.is_file(), binary
    assert cwd.is_absolute() and cwd.is_dir(), cwd
    assert output.is_absolute() and (not output.exists() or not any(output.iterdir())), output
    assert not isinstance(expected, str), 'expected must be a sequence, not a string'
    output.mkdir(parents=True, exist_ok=True)
    safe = {'PATH', 'HOME', 'XDG_CONFIG_HOME', 'XDG_DATA_HOME', 'XDG_STATE_HOME',
            'XDG_RUNTIME_DIR', 'TMPDIR', 'TMUX_TMPDIR', 'AOE_TMUX_SOCKET', 'SHELL',
            'LANG', 'LC_ALL', 'TERM', 'AOE_DAEMON_URL', 'AOE_NO_AUTOSTART'}
    child_env = {key: value for key, value in env.items() if key in safe}
    for key in ('HOME', 'XDG_CONFIG_HOME', 'XDG_DATA_HOME', 'TMPDIR'):
        assert key in child_env and pathlib.Path(child_env[key]).is_absolute(), key
        assert pathlib.Path(child_env[key]).is_dir(), (key, child_env[key])
    assert child_env.get('AOE_NO_AUTOSTART') == '1', 'observer must not autostart the daemon'
    base = child_env.get('AOE_DAEMON_URL', '')
    assert base.startswith('http://127.0.0.1:'), base
    child_env.pop('AOE_DAEMON_URL')
    local_sockets = list(pathlib.Path(child_env['XDG_CONFIG_HOME']).glob('agent-of-empires*/daemon/api.sock'))
    assert len(local_sockets) == 1, ('Home observer requires the existing isolated local daemon', local_sockets)
    child_env.update(TERM='xterm-256color', LANG='C.UTF-8')
    tmux = shutil.which('tmux', path='/usr/bin:/bin')
    assert tmux, 'real tmux unavailable'
    with urllib.request.urlopen(base + '/api/sessions', timeout=10) as response:
        rows = json.load(response)
    rows = rows['sessions'] if isinstance(rows, dict) else rows
    matches = [row for row in rows if row['title'] == title]
    assert len(matches) == 1 and len(rows) == 1, ('observer requires one isolated current session', rows)
    row = matches[0]
    ui_root = pathlib.Path(tempfile.mkdtemp(prefix='5055-ui-', dir=child_env['TMPDIR']))
    socket = ui_root / 'ui.sock'
    driver = [tmux, '-f', '/dev/null', '-S', str(socket)]
    with binary.open('rb') as binary_file:
        binary_sha256 = hashlib.file_digest(binary_file, 'sha256').hexdigest()
    result = {'scope': 'actual production Home dashboard; Fresh confirmation requires explicit commit_fresh',
              'binary': str(binary), 'binary_sha256': binary_sha256,
              'id': row['id'], 'title': title, 'expected': list(expected),
              'api_recovery_before': row.get('recovery'), 'ui_socket': str(socket),
              'local_daemon_socket': str(local_sockets[0]), 'remote_list_mode_disabled': True}
    ui_actor = None
    ui_app = None

    def command(*args, check=True):
        return subprocess.run([*driver, *args], env=child_env, cwd=cwd,
                              capture_output=True, text=True, check=check, timeout=15)

    def frame():
        return command('capture-pane', '-p', '-t', 'home-observer').stdout

    def normalized(text):
        return ' '.join(text.replace('│', ' ').replace('┃', ' ').split())

    def selected():
        text = frame()
        clean = normalized(text)
        return text if (title in text or str(cwd) in text) and 'Select a session to preview' not in text and all(
            normalized(value) in clean for value in expected) else False

    try:
        command('new-session', '-d', '-s', 'home-observer', '-x', '180', '-y', '56', str(binary))
        pid = int(command('display-message', '-p', '-t', 'home-observer', '#{pid}').stdout.strip())
        ui_actor = {'pid': pid, 'birth': _birth(pid)}
        assert ui_actor['birth'], ui_actor
        app_pid = int(command('display-message', '-p', '-t', 'home-observer', '#{pane_pid}').stdout.strip())
        ui_app = {'pid': app_pid, 'birth': _birth(app_pid)}
        assert ui_app['birth'], ui_app
        initial = _until(lambda: (text if title in text or str(cwd) in text else False) if (text := frame()) else False,
                         'actual Home lists the existing session')
        (output / 'home-initial.txt').write_text(initial)
        # A fresh dashboard starts on the project group. Down selects its sole row.
        if 'Select a session to preview' in initial or 'Welcome to Agent' in initial:
            command('send-keys', '-t', 'home-observer', 'Down')
        selected_frame = _until(selected, 'selected actual session contains every expected Home substring')
        (output / 'home-selected.txt').write_text(selected_frame)
        result['actual_home_observed'] = True
        result['home_frame'] = str(output / 'home-selected.txt')
        # Opening the dialog never selects or confirms a lifecycle action.
        command('send-keys', '-t', 'home-observer', 'F5')
        if row.get('recovery') is not None:
            dialog = _until(lambda: (text if 'Explicit Recovery' in text else False)
                            if (text := frame()) else False, 'actual explicit Recovery dialog')
            (output / 'recovery-dialog.txt').write_text(dialog)
            result['actual_recovery_dialog_observed'] = True
            assert 'Choose an offered recovery' in dialog, dialog
            result['no_implicit_recovery_selection'] = True
        else:
            # Active work may have no recovery projection. Do not invent one.
            dialog = _until(lambda: (text if text != selected_frame else False)
                            if (text := frame()) else False, 'nonmutating F5 UI outcome')
            (output / 'f5-outcome-no-recovery.txt').write_text(dialog)
            result['actual_recovery_dialog_observed'] = False
            result['capability_note'] = 'No shared recovery projection at this stage; captured F5 outcome only.'
        if commit_fresh:
            recovery = row.get('recovery')
            assert recovery and recovery['actions']['fresh'] and not recovery['actions']['restart_same'], recovery
            old_sid = row['agent_session_id']
            assert old_sid, row
            # Seeing Right's selected label proves the preceding empty Enter was processed.
            command('send-keys', '-t', 'home-observer', 'Enter', 'Right')
            chosen = _until(lambda: (text if 'Recovery: Launch without AoE restore flags' in text else False)
                            if (text := frame()) else False, 'explicit Fresh selected after inert empty Enter')
            (output / 'fresh-explicitly-selected.txt').write_text(chosen)
            with urllib.request.urlopen(base + '/api/sessions', timeout=10) as response:
                unchanged = next(value for value in json.load(response)['sessions'] if value['id'] == row['id'])
            assert unchanged['agent_session_id'] == old_sid and unchanged['recovery']['expected'] == recovery['expected'], unchanged
            result['empty_enter_did_not_replace_native'] = True
            command('send-keys', '-t', 'home-observer', 'Enter')
            result['fresh_confirm_key_sent'] = True
            def completed_fresh():
                with urllib.request.urlopen(base + '/api/sessions', timeout=10) as response:
                    current = next(value for value in json.load(response)['sessions'] if value['id'] == row['id'])
                sid = current.get('agent_session_id')
                return current if isinstance(sid, str) and sid and sid != old_sid else False
            result['fresh_api_after'] = _until(completed_fresh, 'actual TUI Fresh captures new controlled Native ID')
            (output / 'fresh-completed-home.txt').write_text(frame())
        else:
            command('send-keys', '-t', 'home-observer', 'Escape')
    except BaseException as error:
        result['failure'] = repr(error)
        try:
            (output / 'failure-frame.txt').write_text(frame())
        except Exception as capture_error:
            result['failure_capture_error'] = repr(capture_error)
        raise
    finally:
        cleanup_error = None
        killed = command('kill-server', check=False)
        actors = [actor for actor in (ui_actor, ui_app) if actor]
        try:
            _until(lambda: all(_birth(actor['pid']) != actor['birth'] for actor in actors),
                   'own UI tmux server and dashboard reaped', 15)
        except Exception as error:
            cleanup_error = repr(error)
        result['cleanup'] = {'ui_actor': ui_actor, 'ui_app': ui_app,
                             'kill_server_exit': killed.returncode,
                             'own_ui_processes_gone': all(_birth(actor['pid']) != actor['birth'] for actor in actors),
                             'native_lifecycle_commands_sent': result.get('fresh_confirm_key_sent', False), 'error': cleanup_error}
        if cleanup_error is None:
            shutil.rmtree(ui_root)
        (output / 'home-observer-result.json').write_text(json.dumps(result, indent=2))
        if cleanup_error:
            raise RuntimeError(cleanup_error)
    return result


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('binary')
    parser.add_argument('observer_env')
    parser.add_argument('cwd')
    parser.add_argument('title')
    parser.add_argument('output')
    parser.add_argument('--expected', action='append', default=[])
    parser.add_argument('--commit-fresh', action='store_true')
    args = parser.parse_args()
    result = capture_home_stage(args.binary, json.loads(pathlib.Path(args.observer_env).read_text()),
                                args.cwd, args.title, args.output, args.expected, commit_fresh=args.commit_fresh)
    print(json.dumps(result))


if __name__ == '__main__':
    main()
