#!/usr/bin/python3
"""Private serial production terminal proof. Parent alone runs this after gates.

Controlled real Native process, not a vendor compatibility test. Every Ready or
ownership claim must come from actual AoE state; printed Native text grants none.
"""
import argparse
import hashlib
import http.client
import json
import os
import pathlib
import select
import shutil
import signal
import socket
import subprocess
import sys
import time

S = pathlib.Path(__file__).resolve().parent

NATIVE = r'''#!/usr/bin/python3
import json,os,pathlib,re,select,subprocess,sys,uuid
ROOT=pathlib.Path(ROOT_VALUE)
if any(value in sys.argv[1:] for value in ('--help','--version')):
 print('private actual terminal peer');sys.exit(0)
ident=os.environ.get('AOE_INSTANCE_ID','unknown')
if ident=='unknown':
 name=subprocess.check_output(['/usr/bin/tmux','-S',str(ROOT/'tmux.sock'),'display-message','-p','-t',os.environ['TMUX_PANE'],'#{session_name}'],text=True).strip()
 suffix=name.rsplit('_',1)[-1]
 matches=[path.stem for path in (ROOT/'modes').glob(suffix+'*.json')] if re.fullmatch('[0-9a-f]{8}',suffix) else []
 if len(matches)!=1:raise RuntimeError('Native pane has no unique private control label: '+name)
 ident=matches[0]
def mode():return json.loads((ROOT/'modes'/(ident+'.json')).read_text())
def arg(flag):
 for index,value in enumerate(sys.argv[1:],1):
  if value==flag and index+1<len(sys.argv):return sys.argv[index+1]
  if value.startswith(flag+'='):return value[len(flag)+1:]
 return None
birth=pathlib.Path('/proc/self/stat').read_text().rsplit(')',1)[1].split()[19]
def event(action,**fields):
 with (ROOT/'events.jsonl').open('a') as output:
  output.write(json.dumps(dict(action=action,instance=ident,pid=os.getpid(),birth=birth,pane=os.environ.get('TMUX_PANE'),argv=sys.argv[1:],**fields))+'\n')
store=pathlib.Path(os.environ.get('CLAUDE_CONFIG_DIR',str(ROOT/'home/.claude')))
cwd=os.getcwd();encoded=''.join(value if value.isascii() and (value.isalnum() or value=='-') else '-' for value in cwd)
folder=store/'projects'/encoded
resume=arg('--resume');fork='--fork-session' in sys.argv
sid=(arg('--session-id') or str(uuid.uuid4())) if fork else (resume or arg('--session-id') or str(uuid.uuid4()))
transcript=folder/(sid+'.jsonl');loaded=transcript.exists()
if fork:
 parent=folder/(str(resume)+'.jsonl')
 if not parent.is_file():event('missing',sid=sid,parent=resume);sys.exit(19)
 folder.mkdir(parents=True,exist_ok=True)
 if not loaded:transcript.write_bytes(parent.read_bytes())
 event('forked',sid=sid,parent=resume)
elif resume and not loaded:
 event('missing',sid=sid);sys.exit(19)
elif not loaded:
 folder.mkdir(parents=True,exist_ok=True)
 transcript.write_text(json.dumps({'type':'user','message':{'role':'user','content':'PRIVATE ORIGINAL '+sid}})+'\n')
if mode().get('publish'):
 result=subprocess.run([os.environ['AOE_HOOK_BIN'],'__extract-session-id','--field','session-id','--agent','claude'],input=json.dumps({'session_id':sid}),text=True,capture_output=True)
 event('publication',sid=sid,returncode=result.returncode,stderr=result.stderr)
event('loaded' if loaded else 'created',sid=sid,transcript=str(transcript),content=transcript.read_text())
print('PRIVATE CONTROLLED NATIVE INPUT LOOP',flush=True)
# This input loop is a real OS process. Its control file only requests its own exit.
while True:
 if mode().get('exit_now'):
  event('exit17',sid=sid);os._exit(17)
 readable,_,_=select.select([sys.stdin],[],[],0.05)
 if readable:
  line=sys.stdin.readline()
  if not line:event('stdin_closed',sid=sid);sys.exit(0)
  event('input',sid=sid,line=line.rstrip('\n'))
'''


def birth(pid):
    try:
        fields = pathlib.Path(f'/proc/{pid}/stat').read_text().rsplit(')', 1)[1].split()
        return None if fields[0] == 'Z' else fields[19]
    except FileNotFoundError:
        return None


def native_lineage(native, pane_pid):
    """Require the exact Native birth to be the real pane process or its child."""
    assert birth(native['pid']) == native['birth'], native
    current = native['pid']
    observed = []
    for _ in range(32):
        fields = pathlib.Path(f'/proc/{current}/stat').read_text().rsplit(')', 1)[1].split()
        assert fields[0] != 'Z', (current, fields)
        observed.append({'pid': current, 'birth': fields[19], 'ppid': int(fields[1])})
        if current == pane_pid:
            return observed
        current = int(fields[1])
        assert current > 1, ('Native is not a child of the actual owned pane', observed, pane_pid)
    raise AssertionError(('Native pane ancestry exceeds bound', observed, pane_pid))


def run(binary, evidence):
    binary = pathlib.Path(binary)
    evidence = pathlib.Path(evidence)
    assert binary.is_absolute() and binary.is_file(), binary
    assert evidence.is_absolute() and evidence.resolve().is_relative_to(S), evidence
    assert not evidence.exists() or not any(evidence.iterdir()), 'use a new evidence directory'
    evidence.mkdir(parents=True, exist_ok=True)
    with binary.open('rb') as binary_file:
        binary_sha256 = hashlib.file_digest(binary_file, 'sha256').hexdigest()
    def fixture_write(path, text):
        target = pathlib.Path(path)
        target.parent.mkdir(parents=True, exist_ok=True)
        return target.write_text(text)

    ns = {'write': fixture_write}
    fixture = S / 'terminal-fixture.py'
    exec(compile(fixture.read_text(), str(fixture), 'exec'), ns)
    root = ns['prepare_epic_private_fixture']('5055-terminal-', str(binary))
    env = ns['trace_env']
    ns['trace_config'](auto=True, declared=False)
    # Both compiled app namespaces get the same private configuration. Discover
    # the active namespace from the real daemon socket, never from binary naming.
    config = ns['trace_app'].joinpath('config.toml').read_text()
    config += '\n[updates]\nupdate_check_mode="off"\n'
    for app_name in ('agent-of-empires-dev', 'agent-of-empires'):
        app = root / 'config' / app_name
        app.mkdir(parents=True, exist_ok=True)
        (app / 'config.toml').write_text(config)
        (app / 'state.toml').write_text('has_seen_welcome=true\nhas_responded_to_telemetry=true\nhas_acknowledged_agent_hooks=true\nlast_seen_version="1.21.0"\n')
    (root / 'bin/claude').write_text(NATIVE.replace('ROOT_VALUE', repr(str(root))))
    os.chmod(root / 'bin/claude', 0o755)
    # Existing work-claude exec wrapper from terminal-fixture remains undeclared.
    tmux = shutil.which('tmux', path='/usr/bin:/bin')
    assert tmux, 'real tmux required'
    with socket.socket() as reservation:
        reservation.bind(('127.0.0.1', 0))
        port = reservation.getsockname()[1]
    base = f'http://127.0.0.1:{port}'
    env['AOE_DAEMON_URL'] = base
    env['AOE_NO_AUTOSTART'] = '1'
    (evidence / 'observer-env-private.json').write_text(json.dumps(env, indent=2))
    proof = {'scope': 'actual production AoE HTTP gateway/CLI/Home/browser with a controlled real terminal Native; no vendor compatibility claim',
             'binary': str(binary), 'binary_sha256': binary_sha256,
             'fixture_sha256': hashlib.sha256(fixture.read_bytes()).hexdigest(),
             'baseUrl': base, 'private_root': str(root), 'cases': [], 'success': False}
    cli = ns['trace_cli']
    until = ns['trace_until']
    events = ns['trace_events']
    disk_row = ns['trace_row']
    native_socket = root / 'tmux.sock'
    daemon = None
    daemon_actor = None
    server_actor = None
    daemon_log = None

    def save(name, value):
        (evidence / name).write_text(json.dumps(value, indent=2))

    def api(method, path, body=None):
        connection = http.client.HTTPConnection('127.0.0.1', port, timeout=15)
        try:
            connection.request(method, path, body=None if body is None else json.dumps(body),
                               headers={'Content-Type': 'application/json'} if body is not None else {})
            response = connection.getresponse()
            return response.status, dict(response.getheaders()), response.read().decode()
        finally:
            connection.close()

    # Helpers keep their original isolation and row selection, but all requests
    # now use the actual normal production HTTP gateway, not --core-only.
    ns['trace_api'] = api

    def row(ident):
        status, _, text = api('GET', '/api/sessions')
        assert status == 200, (status, text)
        parsed = json.loads(text)
        return next((r for r in parsed['sessions'] if r['id'] == ident), None)

    def launches(ident):
        return [event for event in events(ident) if event['action'] in ('created', 'loaded')]

    def control(expected):
        deadline = time.monotonic() + 900
        while True:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise TimeoutError('parent command: ' + expected)
            readable, _, _ = select.select([sys.stdin], [], [], remaining)
            if not readable:
                raise TimeoutError('parent command: ' + expected)
            line = sys.stdin.readline()
            if not line:
                raise EOFError('parent stdin closed before ' + expected)
            assert line.strip() == expected, ('unexpected parent command', line, expected)
            return

    def selected(row_value):
        keys = ('id', 'tool', 'project_path', 'agent_session_id', 'agent_session_binding', 'resume_binding',
                'resume_intent', 'resume_probe_failed_sid', 'active_execution', 'capture_started_at',
                'started_at', 'omp_capture_generation', 'force_fresh_next_launch')
        return {key: row_value.get(key) for key in keys}

    def preserve(ident, before, transcript, original, launch_count):
        current = disk_row(ident)
        assert selected(current) == selected(before), (before, current)
        assert current['lifecycle_generation'] >= before['lifecycle_generation']
        assert transcript.read_bytes() == original, 'original Native history changed'
        assert len(launches(ident)) == launch_count, events(ident)
        assert not any(event['action'] == 'input' for event in events(ident)), events(ident)
        return current

    def recovery_assertions(current, confirmed_exit=True):
        recovery = current.get('recovery')
        assert recovery is not None, ('missing shared recovery projection', current)
        assert recovery['reason'] == ('runtime_exited' if confirmed_exit else 'identity_unattested'), recovery
        if confirmed_exit:
            assert isinstance(recovery['runtime'], dict) and 'pane_exited' in recovery['runtime'], recovery
        else:
            assert recovery['runtime'] == 'pane_alive', recovery
        assert recovery['work_in_progress'] is False, recovery
        assert recovery['actions'] == {'attach': True, 'retry_operation': False,
                                       'restart_same': False, 'stop': not confirmed_exit, 'fresh': True}, recovery
        assert isinstance(recovery['expected'], str) and recovery['expected'], recovery
        assert 'undeclared wrapper' in recovery['guidance'], recovery
        return recovery

    def shared_snapshot(ident, name, confirmed_exit=True):
        current = row(ident)
        recovery = recovery_assertions(current, confirmed_exit)
        human = cli('session', 'show', ident)
        machine = cli('session', 'show', ident, '--json')
        parsed = json.loads(machine.stdout)
        assert parsed['recovery'] == recovery, (parsed, current)
        runtime_label = 'Terminal pane observed exited' if confirmed_exit else 'Owned terminal pane observed alive'
        assert 'Observed runtime: ' + runtime_label in human.stdout, human.stdout
        labels = {'attach': 'Attach existing runtime',
                  'retry_operation': 'Retry operation with explicit input',
                  'restart_same': 'Restart same conversation', 'stop': 'Stop owned runtime',
                  'fresh': 'Launch without AoE restore flags'}
        for key, label in labels.items():
            availability = 'available' if recovery['actions'][key] else 'unavailable'
            assert f'{label}: {availability}' in human.stdout, (key, recovery, human.stdout)
        result = {'name': name, 'api': current, 'cli_human': human.stdout,
                  'cli_json': parsed, 'cli_stderr': [human.stderr, machine.stderr]}
        save(name + '-shared.json', result)
        return result

    def stage(name, ident, sid, title, snapshot):
        payload = {'name': name, 'baseUrl': base, 'id': ident, 'sid': sid, 'title': title,
                   'binary': str(binary), 'home': str(root / 'home'), 'project': str(root / 'project'),
                   'observerEnv': str(evidence / 'observer-env-private.json'), 'snapshot': snapshot,
                   'command': 'verify-' + name}
        save(name + '-ready.json', payload)
        print('TERMINAL_5055_STAGE_READY', json.dumps(payload), flush=True)
        control('verify-' + name)

    try:
        daemon_log = (root / 'daemon-output.log').open('ab', buffering=0)
        daemon = subprocess.Popen([str(binary), 'serve', '--host', '127.0.0.1', '--port', str(port), '--no-auth'],
                                  env=env, cwd=root / 'project', stdout=daemon_log, stderr=subprocess.STDOUT)
        daemon_actor = {'pid': daemon.pid, 'birth': birth(daemon.pid)}
        def gateway_ready():
            assert daemon.poll() is None, 'production server exited before readiness'
            return api('GET', '/api/about')[0] == 200
        until(gateway_ready, 'actual normal HTTP gateway ready', seconds=40)
        active_apps = list((root / 'config').glob('agent-of-empires*/daemon/api.sock'))
        assert len(active_apps) == 1, active_apps
        ns['trace_sock'] = active_apps[0]
        ns['trace_app'] = active_apps[0].parent.parent
        web = api('GET', '/')
        assert web[0] == 200 and '<html' in web[2].lower(), ('real embedded browser app unavailable', web)
        proof['web_gateway'] = {'status': web[0], 'html_sha256': hashlib.sha256(web[2].encode()).hexdigest()}
        cli('hooks', 'approve')
        for case_name in ('confirmed-exit', 'undeclared-wrapper'):
            title = '5055 terminal ' + case_name
            ident = ns['trace_add'](title, 'work-claude')
            mode_path = root / 'modes' / (ident + '.json')
            mode_path.write_text(json.dumps({'publish': True}))
            started = api('POST', f'/api/sessions/{ident}/restart', {'wake_message': ''})
            assert started[0] == 200, ('initial Native start refused', started)
            native = until(lambda: next(iter(launches(ident)), None), 'one real terminal Native birth')
            assert len(launches(ident)) == 1, events(ident)
            sid = native['sid']
            until(lambda: disk_row(ident).get('agent_session_id') == sid, 'actual hook-captured observed Native ID')
            publication = [event for event in events(ident) if event['action'] == 'publication']
            assert len(publication) == 1 and publication[0]['returncode'] == 0, publication
            assert birth(native['pid']) == native['birth'], native
            transcript = pathlib.Path(native['transcript'])
            original = transcript.read_bytes()
            initial = disk_row(ident)
            assert (initial.get('resume_intent') or {'kind': 'Default'})['kind'] == 'Default', initial
            # Retain the genuine corpse so tmux can report its actual exit status.
            pane = native['pane']
            assert pane, native
            def tmux_call(*args, check=True):
                return subprocess.run([tmux, '-f', '/dev/null', '-S', str(native_socket), *args],
                                      env=env, capture_output=True, text=True, check=check, timeout=15)
            tmux_call('set-option', '-p', '-t', pane, 'remain-on-exit', 'on')
            tmux_pid = int(tmux_call('display-message', '-p', '-t', pane, '#{pid}').stdout.strip())
            server_actor = {'pid': tmux_pid, 'birth': birth(tmux_pid)}
            # agent_pane is ephemeral and deliberately absent from disk/REST DTOs.
            # Observe the actual AoE-created pane and its hidden row label instead.
            # These observations do not grant Same/input permission; recovery
            # actions below must independently come from the production server.
            physical_before_exit = tmux_call(
                'display-message', '-p', '-t', pane,
                '#{session_name} #{pane_id} #{pane_pid} #{pane_dead}').stdout.strip()
            session_name, observed_pane, pane_pid, pane_dead = physical_before_exit.split()
            assert observed_pane == pane and pane_dead == '0', physical_before_exit
            ancestry = native_lineage(native, int(pane_pid))
            owner_label = tmux_call('show-environment', '-h', '-t', session_name, 'AOE_INSTANCE_ID').stdout.strip()
            assert owner_label == 'AOE_INSTANCE_ID=' + ident, owner_label
            death = None
            if case_name == 'confirmed-exit':
                mode_path.write_text(json.dumps({'publish': True, 'exit_now': True}))
                until(lambda: any(event['action'] == 'exit17' for event in events(ident)), 'controlled Native actually requests exit17')
                until(lambda: birth(native['pid']) != native['birth'], 'original Native birth really exited')
                death = until(lambda: (text if text == '1 17' else False)
                              if (text := tmux_call('display-message', '-p', '-t', pane, '#{pane_dead} #{pane_dead_status}').stdout.strip()) else False,
                              'owned terminal pane reports actual death and exit17')
                until(lambda: (value if value.get('recovery', {}).get('reason') == 'runtime_exited' else False)
                      if (value := row(ident)) and value.get('recovery') else False,
                      'current shared confirmed-exit recovery', seconds=35)
            before = disk_row(ident)
            preserve(ident, before, transcript, original, 1)
            refusals = []
            if case_name == 'undeclared-wrapper':
                refused = api('POST', f'/api/sessions/{ident}/restart', {'wake_message': ''})
                headers = {key.lower(): value for key, value in refused[1].items()}
                assert refused[0] == 409, refused
                assert headers.get('aoe-error-code') == 'managed_resume_refused', refused
                assert headers.get('aoe-managed-resume-cause') == 'wrapper_contract_required', refused
                assert 'wrapper contract missing' in refused[2].lower(), refused
                preserve(ident, before, transcript, original, 1)
                refusals.append({'action': 'api-default', 'response': refused})
                default_cli = cli('session', 'restart', ident, check=False)
                assert default_cli.returncode != 0 and 'wrapper contract missing' in (default_cli.stdout + default_cli.stderr).lower(), default_cli
                preserve(ident, before, transcript, original, 1)
                refusals.append({'action': 'cli-default', 'exit': default_cli.returncode, 'stdout': default_cli.stdout, 'stderr': default_cli.stderr})
                current_recovery = recovery_assertions(row(ident), False)
                same = api('POST', f'/api/sessions/{ident}/restart',
                           {'recovery': {'action': 'same', 'expected': current_recovery['expected']}})
                assert same[0] == 409, same
                preserve(ident, before, transcript, original, 1)
                refusals.append({'action': 'api-same', 'response': same})
                same_cli = cli('session', 'restart', ident, '--same', check=False)
                assert same_cli.returncode != 0, same_cli
                preserve(ident, before, transcript, original, 1)
                refusals.append({'action': 'cli-same', 'exit': same_cli.returncode, 'stdout': same_cli.stdout, 'stderr': same_cli.stderr})
            if case_name == 'undeclared-wrapper':
                assert birth(native['pid']) == native['birth'], 'refusal replaced or stopped the observed live Native'
            snapshot = shared_snapshot(ident, case_name, case_name == 'confirmed-exit')
            stage(case_name, ident, sid, title, snapshot)
            preserve(ident, before, transcript, original, 1)
            again = shared_snapshot(ident, case_name + '-after-observers', case_name == 'confirmed-exit')
            assert again['api']['recovery']['expected'] == snapshot['api']['recovery']['expected'], (snapshot, again)
            mode_path.write_text(json.dumps({'publish': True}))
            recovery = recovery_assertions(row(ident), case_name == 'confirmed-exit')
            consent_command = 'fresh-confirmed-exit' if case_name == 'confirmed-exit' else 'observe-web-fresh-undeclared-wrapper'
            print('TERMINAL_5055_EXPLICIT_FRESH_CONSENT_REQUIRED', json.dumps({'case': case_name, 'command': consent_command}), flush=True)
            control(consent_command)
            if case_name == 'confirmed-exit':
                fresh_cli = cli('session', 'restart', ident, '--fresh')
                fresh = {'surface': 'actual CLI --fresh', 'argv': fresh_cli.args,
                         'exit': fresh_cli.returncode, 'stdout': fresh_cli.stdout, 'stderr': fresh_cli.stderr}
            else:
                fresh = {'surface': 'parent-operated actual Chromium Fresh confirmation',
                         'expected_before_external_consent': recovery['expected'],
                         'driver_did_not_issue_another_mutation': True}
            replacement = until(lambda: launches(ident)[1] if len(launches(ident)) == 2 else False,
                                'one explicit Fresh Native invocation')
            assert not any(flag == '--resume' or flag.startswith('--resume=') or flag == '--session-id'
                           or flag.startswith('--session-id=') or flag == '--fork-session'
                           for flag in replacement['argv']), replacement
            assert transcript.read_bytes() == original, 'Fresh deleted or changed old Native history'
            assert not any(event['action'] == 'input' for event in events(ident)), events(ident)
            assert birth(replacement['pid']) == replacement['birth'], replacement
            assert birth(native['pid']) != native['birth'], 'explicit Fresh left the original Native alive'
            # This controlled exec wrapper chooses a new ID. Never generalize that
            # result into a promise that opaque wrappers cannot restore history.
            assert replacement['sid'] != sid and replacement['action'] == 'created', replacement
            until(lambda: disk_row(ident).get('agent_session_id') == replacement['sid'], 'Fresh callback captured without AoE restore flags')
            fresh_snapshot = {'api': row(ident), 'new_native': replacement,
                              'old_history_sha256': hashlib.sha256(original).hexdigest(),
                              'old_history_retained': transcript.read_bytes() == original,
                              'opaque_wrapper_history_guarantee': False, 'consent': fresh}
            stage(case_name + '-fresh', ident, replacement['sid'], title, fresh_snapshot)
            assert len(launches(ident)) == 2 and birth(replacement['pid']) == replacement['birth'], events(ident)
            assert transcript.read_bytes() == original and not any(e['action'] == 'input' for e in events(ident))
            final_replacement = replacement
            tui_fresh = None
            if case_name == 'confirmed-exit':
                second_history = pathlib.Path(replacement['transcript'])
                second_original = second_history.read_bytes()
                tmux_call('set-option', '-p', '-t', replacement['pane'], 'remain-on-exit', 'on')
                mode_path.write_text(json.dumps({'publish': True, 'exit_now': True}))
                until(lambda: any(event['action'] == 'exit17' and event['pid'] == replacement['pid']
                                  for event in events(ident)), 'second controlled Native actually exits17')
                until(lambda: birth(replacement['pid']) != replacement['birth'], 'second Native birth exited')
                until(lambda: tmux_call('display-message', '-p', '-t', replacement['pane'],
                                       '#{pane_dead} #{pane_dead_status}').stdout.strip() == '1 17',
                      'second owned pane confirms exit17')
                until(lambda: (value if value.get('recovery', {}).get('reason') == 'runtime_exited' else False)
                      if (value := row(ident)) and value.get('recovery') else False,
                      'current recovery for explicit Home TUI consent', seconds=35)
                tui_before = disk_row(ident)
                tui_snapshot = shared_snapshot(ident, 'confirmed-exit-tui-recovery')
                stage('confirmed-exit-tui-recovery', ident, replacement['sid'], title, tui_snapshot)
                preserve(ident, tui_before, transcript, original, 2)
                assert second_history.read_bytes() == second_original
                mode_path.write_text(json.dumps({'publish': True}))
                print('TERMINAL_5055_EXPLICIT_FRESH_CONSENT_REQUIRED',
                      json.dumps({'case': 'confirmed-exit-tui-recovery',
                                  'command': 'observe-tui-fresh-confirmed-exit'}), flush=True)
                control('observe-tui-fresh-confirmed-exit')
                third = until(lambda: launches(ident)[2] if len(launches(ident)) == 3 else False,
                              'explicit real Home TUI Fresh Native invocation')
                assert birth(third['pid']) == third['birth'] and third['sid'] != replacement['sid'], third
                assert not any(flag == '--resume' or flag.startswith('--resume=') or flag == '--session-id'
                               or flag.startswith('--session-id=') or flag == '--fork-session'
                               for flag in third['argv']), third
                assert transcript.read_bytes() == original and second_history.read_bytes() == second_original
                assert not any(event['action'] == 'input' for event in events(ident))
                until(lambda: disk_row(ident).get('agent_session_id') == third['sid'], 'TUI Fresh Native ID captured')
                tui_fresh = {'surface': 'parent-operated actual Home TUI explicit Fresh',
                             'before': tui_snapshot, 'after': row(ident), 'new_native': third,
                             'both_old_histories_retained': True, 'no_input_replay': True}
                stage('confirmed-exit-tui-fresh', ident, third['sid'], title, tui_fresh)
                assert len(launches(ident)) == 3 and birth(third['pid']) == third['birth']
                final_replacement = third
            case = {'name': case_name, 'original_native': native, 'pane_death': death,
                    'pane_before_exit': physical_before_exit, 'hidden_owner_label': owner_label,
                    'native_to_pane_ancestry': ancestry,
                    'lifecycle_attempt_generation': {'before': before['lifecycle_generation'],
                                                     'after_refusals': snapshot['api']['lifecycle_generation']},
                    'observed_id_retained_through_refusals': True, 'refusals': refusals,
                    'shared': snapshot, 'fresh': fresh_snapshot, 'tui_fresh': tui_fresh, 'events': events(ident)}
            proof['cases'].append(case)
            save(case_name + '-proof.json', case)
            cli('remove', ident, '--purge')
            ns['trace_ids'].remove(ident)
            until(lambda: birth(final_replacement['pid']) != final_replacement['birth'], 'serial case Native fully reaped')
        proof['success'] = True
    except BaseException as error:
        proof['failure'] = repr(error)
        proof['events_at_failure'] = events()
        try:
            proof['api_at_failure'] = api('GET', '/api/sessions')
            proof['disk_at_failure'] = json.loads((ns['trace_app'] / 'profiles/main/sessions.json').read_text())
        except Exception as diagnostic_error:
            proof['diagnostic_error'] = repr(diagnostic_error)
        save('terminal-failure.json', proof)
        raise
    finally:
        cleanup = {'commands': [], 'errors': []}
        native_actors = {(event['pid'], event['birth']) for event in events() if 'birth' in event}
        # CLI purge handles only IDs born in this isolated fixture.
        for ident in list(ns['trace_ids']):
            try:
                removed = cli('remove', ident, '--purge', check=False)
                cleanup['commands'].append({'action': 'remove-own-row', 'id': ident, 'exit': removed.returncode,
                                            'stdout': removed.stdout, 'stderr': removed.stderr})
            except Exception as error:
                cleanup['errors'].append(repr(error))
        if daemon and daemon.poll() is None:
            daemon.terminate()
            try:
                daemon.wait(timeout=20)
            except subprocess.TimeoutExpired:
                if birth(daemon.pid) == daemon_actor['birth']:
                    daemon.kill()
                daemon.wait(timeout=10)
        stopped = subprocess.run([tmux, '-f', '/dev/null', '-S', str(native_socket), 'kill-server'],
                                 env=env, capture_output=True, text=True, timeout=15)
        cleanup['commands'].append({'action': 'kill-only-own-tmux-server', 'socket': str(native_socket),
                                    'exit': stopped.returncode, 'stderr': stopped.stderr})
        for pid, original_birth in native_actors:
            if birth(pid) == original_birth:
                os.kill(pid, signal.SIGTERM)
                cleanup['commands'].append({'action': 'term-only-exact-owned-birth', 'pid': pid, 'birth': original_birth})
        actors = [{'pid': pid, 'birth': value} for pid, value in native_actors]
        actors += [actor for actor in (daemon_actor, server_actor) if actor and actor['birth']]
        try:
            until(lambda: all(birth(actor['pid']) != actor['birth'] for actor in actors),
                  'all exact owned Native/daemon/tmux births gone', seconds=20)
            cleanup['all_owned_births_gone'] = True
        except Exception as error:
            cleanup['all_owned_births_gone'] = False
            cleanup['errors'].append(repr(error))
        cleanup['observed_actors'] = actors
        if daemon_log:
            daemon_log.close()
        if (root / 'daemon-output.log').exists():
            shutil.copyfile(root / 'daemon-output.log', evidence / 'daemon-output.log')
        if (ns['trace_app'] / 'debug.log').exists():
            shutil.copyfile(ns['trace_app'] / 'debug.log', evidence / 'debug.log')
        save('native-events.json', events())
        if cleanup.get('all_owned_births_gone'):
            shutil.rmtree(root)
            cleanup['private_root_removed'] = True
        else:
            cleanup['private_root_removed'] = False
        proof['cleanup'] = cleanup
        proof['success'] = proof['success'] and not cleanup['errors'] and cleanup.get('all_owned_births_gone', False)
        save('terminal-proof.json', proof)
        print('TERMINAL_5055_COMPLETE', json.dumps({'success': proof['success'], 'proof': str(evidence / 'terminal-proof.json')}), flush=True)
    assert proof['success'], proof
    return proof


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('binary', help='absolute frozen production web-enabled binary')
    parser.add_argument('evidence', help='new absolute evidence directory under this private scaffold directory')
    arguments = parser.parse_args()
    run(arguments.binary, arguments.evidence)
