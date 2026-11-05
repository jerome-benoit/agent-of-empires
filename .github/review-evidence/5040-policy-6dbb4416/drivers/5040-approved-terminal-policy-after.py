import sys

def run_terminal_approved_probe_policy_after(binary, artifact_dir, pty_helper):
    root = prepare_epic_private_fixture('aoe-policy-after-', binary)
    evidence = pathlib.Path(artifact_dir)
    evidence.mkdir(parents=True, exist_ok=True)
    real_tmux = shutil.which('tmux')
    ui_driver = [real_tmux, '-f', '/dev/null', '-S', str(root / 'ui.sock')]
    proof = {'scope': 'actual isolated AoE CLI/core-daemon/tmux/TUI and transcript-reading controlled Native fixtures; no vendor-provider compatibility claim', 'cases': [], 'binary': str(binary), 'binary_sha256': hashlib.sha256(pathlib.Path(binary).read_bytes()).hexdigest()}
    peer = r'''#!/usr/bin/python3
import json,os,pathlib,re,subprocess,sys,uuid
ROOT=pathlib.Path(ROOT_VALUE)
AGENT=AGENT_VALUE
if any(value in sys.argv[1:] for value in ('--help','--version')):
 print('private terminal policy fixture');sys.exit(0)
ident=os.environ.get('AOE_INSTANCE_ID','unknown')
if ident=='unknown':
 name=subprocess.check_output(['tmux','-S',str(ROOT/'tmux.sock'),'display-message','-p','-t',os.environ['TMUX_PANE'],'#{session_name}'],text=True).strip()
 suffix=name.rsplit('_',1)[-1]
 matches=[path.stem for path in (ROOT/'modes').glob(suffix+'*.json')] if re.fullmatch('[0-9a-f]{8}',suffix) else []
 if len(matches)!=1:raise RuntimeError('private Native pane has no unique fixture control label: '+name)
 ident=matches[0]
modepath=ROOT/'modes'/(ident+'.json')
mode=json.loads(modepath.read_text()) if modepath.exists() else {}
def arg(flag):
 for index,value in enumerate(sys.argv[1:],1):
  if value==flag and index+1<len(sys.argv):return sys.argv[index+1]
  if value.startswith(flag+'='):return value[len(flag)+1:]
 return None
birth=pathlib.Path('/proc/self/stat').read_text().rsplit(')',1)[1].split()[19]
def event(action,**fields):
 with (ROOT/'events.jsonl').open('a') as output:output.write(json.dumps(dict(action=action,instance=ident,pid=os.getpid(),birth=birth,argv=sys.argv[1:],**fields))+'\n')
key='CLAUDE_CONFIG_DIR' if AGENT=='claude' else 'COPILOT_HOME'
store=pathlib.Path(os.environ.get(key,str(ROOT/'home'/('.claude' if AGENT=='claude' else '.copilot'))))
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
contents=transcript.read_text()
if AGENT=='claude' and mode.get('publish'):
 result=subprocess.run([os.environ['AOE_HOOK_BIN'],'__extract-session-id','--field','session-id','--agent','claude'],input=json.dumps({'session_id':sid}),text=True,capture_output=True)
 event('publication',sid=sid,returncode=result.returncode)
event('loaded' if loaded else 'created',sid=sid,transcript=str(transcript),content=contents,store=str(store))
if mode.get('exit_after_load') and loaded:
 event('exit_after_load',sid=sid);os._exit(17)
print('PRIVATE FIXTURE READY',flush=True)
for line in sys.stdin:event('input',sid=sid,line=line.rstrip('\n'))
'''
    for agent in ('claude', 'copilot'):
        script = peer.replace('ROOT_VALUE', repr(str(root))).replace('AGENT_VALUE', repr(agent))
        write(str(root / 'bin' / agent), script)
        os.chmod(root / 'bin' / agent, 0o755)
    def launches(ident):
        return [event for event in trace_events(ident) if event['action'] in ('created', 'loaded')]
    def intent(row):
        return (row.get('resume_intent') or {'kind': 'Default'})['kind']
    def mode(ident, **fields):
        write(str(root / 'modes' / (ident + '.json')), json.dumps(fields))
    def row_with_sid(ident, sid):
        row = trace_row(ident)
        return row if row.get('agent_session_id') == sid else None
    def preserved(ident, before, transcript, original):
        row = trace_row(ident)
        assert row.get('agent_session_id') == before.get('agent_session_id'), (before, row)
        assert row.get('agent_session_binding') == before.get('agent_session_binding'), (before, row)
        assert row.get('resume_binding') == before.get('resume_binding'), (before, row)
        assert (row.get('resume_intent') or {'kind': 'Default'}) == (before.get('resume_intent') or {'kind': 'Default'}), (before, row)
        assert row.get('resume_probe_failed_sid') == before.get('resume_probe_failed_sid'), (before, row)
        assert row['lifecycle_generation'] == before['lifecycle_generation'], (before, row)
        assert transcript.read_bytes() == original
        return row
    def refused(ident, label, method, path, body, before, transcript, original):
        binding = before.get('resume_binding') if intent(before) == 'Use' else before.get('agent_session_binding')
        assert binding and binding.get('execution') and binding.get('session_id') == before.get('resume_probe_failed_sid'), (label, before)
        previous = launches(ident)
        response = trace_api(method, path, body)
        assert response[0] == 409, (label, response)
        headers = {key.lower(): value for key, value in response[1].items()}
        assert headers.get('aoe-error-code') == 'managed_resume_refused', (label, response)
        assert headers.get('aoe-managed-resume-cause') == 'previous_probe_failed', (label, response)
        preserved(ident, before, transcript, original)
        assert launches(ident) == previous, (label, launches(ident))
        return {'action': label, 'http': response[0], 'error': headers['aoe-error-code'], 'cause': headers['aoe-managed-resume-cause'], 'native_launches_unchanged': True}
    def drop_session(ident):
        trace_cli('remove', ident, '--purge')
        trace_ids.remove(ident)
    def create_failed(title, publish=True, pin=False):
        ident = trace_add(title, 'work-claude')
        mode(ident, publish=publish)
        response = trace_api('POST', '/api/sessions/' + ident + '/restart', {'wake_message': ''})
        assert response[0] == 200, response
        first = trace_until(lambda: next(iter(launches(ident)), None), 'initial real Native creation')
        sid = first['sid']
        if not publish:
            trace_cli('session', 'set-session-id', ident, sid)
        else:
            trace_until(lambda: row_with_sid(ident, sid), 'original Native identity publication')
        if pin:
            trace_cli('session', 'set-session-id', ident, sid)
        transcript = pathlib.Path(first['transcript'])
        original = transcript.read_bytes()
        mode(ident, publish=publish, exit_after_load=True)
        failed = trace_api('POST', '/api/sessions/' + ident + '/restart', {'wake_message': ''})
        assert failed[0] == 409 and json.loads(failed[2]).get('error') == 'resume_failed', failed
        loaded = launches(ident)[-1]
        assert loaded['action'] == 'loaded' and loaded['sid'] == sid and loaded['content'].encode() == original, loaded
        before = trace_row(ident)
        assert before.get('agent_session_id') == sid and before.get('resume_probe_failed_sid') == sid, before
        assert transcript.read_bytes() == original
        mode(ident, publish=publish)
        return ident, sid, transcript, original, before
    def tui_refusal(ident, title):
        write(str(trace_app / 'config.toml'), (trace_app / 'config.toml').read_text() + '\n[updates]\nupdate_check_mode="off"\n')
        write(str(trace_app / 'state.toml'), 'has_seen_welcome=true\nhas_responded_to_telemetry=true\nhas_acknowledged_agent_hooks=true\nlast_seen_version="1.21.0"\n')
        subprocess.run([*ui_driver, 'new-session', '-d', '-s', 'policy-ui', '-x', '132', '-y', '42', str(binary)], env=trace_env, cwd=root / 'project', capture_output=True, text=True, check=True, timeout=15)
        def frame():
            return subprocess.run([*ui_driver, 'capture-pane', '-p', '-t', 'policy-ui'], env=trace_env, capture_output=True, text=True, check=True, timeout=10).stdout
        initial = trace_until(lambda: (text if title in text else False) if (text := frame()) else False, 'actual TUI home')
        write(str(evidence / 'tui-before.txt'), initial)
        subprocess.run([*ui_driver, 'send-keys', '-t', 'policy-ui', 'Down'], env=trace_env, capture_output=True, text=True, check=True, timeout=10)
        selected = trace_until(lambda: (text if title in text and 'Select a session to preview' not in text and 'Welcome to Agent' not in text else False) if (text := frame()) else False, 'actual session selected instead of its project group')
        write(str(evidence / 'tui-selected.txt'), selected)
        subprocess.run([*ui_driver, 'send-keys', '-t', 'policy-ui', 'Enter'], env=trace_env, capture_output=True, text=True, check=True, timeout=10)
        try:
            final = trace_until(lambda: (text if 'Managed resume refused' in text or 'previous probe failed' in text else False) if (text := frame()) else False, 'actual TUI refuses failed history without replacement', seconds=30)
        except BaseException:
            write(str(evidence / 'tui-failure-frame.txt'), frame())
            raise
        write(str(evidence / 'tui-refused.txt'), final)
        subprocess.run([*ui_driver, 'kill-server'], env=trace_env, capture_output=True, text=True, timeout=10)
        return {'actual_home_observed': True, 'actual_refusal_observed': True, 'frame': str(evidence / 'tui-refused.txt')}
    try:
        trace_start_daemon()
        trace_cli('hooks', 'approve')
        ident, sid, transcript, original, before = create_failed('policy-captured-spent-pin', pin=True)
        assert intent(before) == 'Default', before
        blocked_text = 'PRIVATE_5040_BLOCKED_MESSAGE_MUST_NEVER_REPLAY'
        actions = []
        for label, path, body in [('start', 'start', {}), ('ensure-attach', 'ensure', {}), ('send', 'send', {'message': blocked_text}), ('restart', 'restart', {'wake_message': blocked_text})]:
            actions.append(refused(ident, label, 'POST', '/api/sessions/' + ident + '/' + path, body, before, transcript, original))
        for action in ('start', 'restart'):
            count = launches(ident)
            result = trace_cli('session', action, ident, check=False)
            assert result.returncode != 0 and 'previous probe failed' in result.stderr, (action, result)
            preserved(ident, before, transcript, original)
            assert launches(ident) == count
            actions.append({'action': 'cli-' + action, 'exit': result.returncode, 'native_launches_unchanged': True})
        tui = tui_refusal(ident, 'policy-captured-spent-pin')
        preserved(ident, before, transcript, original)
        assert len(launches(ident)) == 2
        trace_cli('session', 'set-session-id', ident, sid)
        asserted = trace_row(ident)
        assert asserted.get('resume_probe_failed_sid') is None and intent(asserted) == 'Use', asserted
        response = trace_api('POST', '/api/sessions/' + ident + '/restart', {'wake_message': ''})
        assert response[0] == 200, response
        recovered = launches(ident)[-1]
        assert recovered['sid'] == sid and recovered['action'] == 'loaded'
        assert transcript.read_bytes() == original
        pty_result = subprocess.run([sys.executable, str(pty_helper), str(binary), ident, str(evidence / 'cli-attach.ansi')], env=trace_env, cwd=root / 'project', capture_output=True, text=True, timeout=50)
        assert pty_result.returncode == 0, (pty_result.stdout, pty_result.stderr)
        write(str(evidence / 'cli-attach-result.json'), pty_result.stdout)
        assert launches(ident)[-1]['pid'] == recovered['pid'] and len(launches(ident)) == 3
        same_input = 'PRIVATE_5040_EXPLICIT_SAME_RECOVERY_INPUT'
        sent = trace_api('POST', '/api/sessions/' + ident + '/send', {'message': same_input})
        assert sent[0] == 200, sent
        delivered = trace_until(lambda: next((event for event in trace_events(ident) if event['action'] == 'input' and same_input in event.get('line', '')), None), 'explicit same-ID input reaches recovered Native')
        assert delivered['pid'] == recovered['pid'] and delivered['sid'] == sid
        mode(ident, publish=True, exit_after_load=True)
        failed_again = trace_api('POST', '/api/sessions/' + ident + '/restart', {'wake_message': ''})
        assert failed_again[0] == 409 and trace_row(ident).get('resume_probe_failed_sid') == sid, failed_again
        before_optout = trace_row(ident)
        mode(ident, publish=True)
        trace_stop_daemon();trace_config(auto=False, declared=True);trace_start_daemon()
        optout_send = refused(ident, 'optout-send', 'POST', '/api/sessions/' + ident + '/send', {'message': blocked_text}, before_optout, transcript, original)
        manual = trace_api('POST', '/api/sessions/' + ident + '/restart', {'wake_message': ''})
        assert manual[0] == 200, manual
        replacement = trace_until(lambda: next((event for event in launches(ident) if event['action'] == 'created' and event['sid'] != sid), None), 'configured-Fresh manual restart')
        trace_until(lambda: row_with_sid(ident, replacement['sid']), 'configured-Fresh identity durable')
        assert transcript.read_bytes() == original
        assert not any(blocked_text in event.get('line', '') for event in trace_events(ident) if event['action'] == 'input')
        assert [event['sid'] for event in launches(ident)] == [sid, sid, sid, sid, replacement['sid']]
        proof['cases'].append({'name': 'captured-spent-pin-and-approved-recovery', 'original_sid': sid, 'configured_fresh_sid': replacement['sid'], 'initial_probe_preserves_history': True, 'spent_pin_is_Default': True, 'blocked_actions': actions, 'actual_tui': tui, 'actual_cli_attach': True, 'same_recovery_native_pid': recovered['pid'], 'same_recovery_input_pid': delivered['pid'], 'optout_send': optout_send, 'configured_fresh_manual_only': True, 'blocked_message_never_replayed': True, 'launches': 5})
        drop_session(ident)
        trace_stop_daemon();trace_config(auto=True, declared=True);trace_start_daemon()
        ident, sid, transcript, original, before = create_failed('policy-explicit-fresh')
        count = launches(ident)
        trace_cli('session', 'set-session-id', ident, '')
        assert intent(trace_row(ident)) == 'Cleared'
        assert launches(ident) == count
        trace_cli('session', 'start', ident)
        fresh = trace_until(lambda: next((event for event in launches(ident) if event['action'] == 'created' and event['sid'] != sid), None), 'explicit Fresh through direct CLI start')
        current = trace_until(lambda: row_with_sid(ident, fresh['sid']), 'explicit Fresh publication')
        assert intent(current) == 'Default' and current.get('resume_probe_failed_sid') is None
        assert transcript.read_bytes() == original
        assert not any(event['action'] == 'input' for event in trace_events(ident))
        proof['cases'].append({'name': 'explicit-fresh-direct-CLI', 'old_sid': sid, 'fresh_sid': fresh['sid'], 'recording_intent_does_not_launch': True, 'consumed_one_shot_intent': True, 'history_unchanged': True, 'no_implicit_message': True, 'launches': len(launches(ident))})
        drop_session(ident)
        ident = trace_add('policy-persistent-copilot-pin', 'copilot')
        mode(ident, publish=False)
        response = trace_api('POST', '/api/sessions/' + ident + '/restart', {'wake_message': ''})
        assert response[0] == 200, response
        native = trace_until(lambda: next(iter(launches(ident)), None), 'no-publisher Native creation')
        sid = native['sid'];transcript = pathlib.Path(native['transcript']);original = transcript.read_bytes()
        trace_cli('session', 'set-session-id', ident, sid)
        mode(ident, publish=False, exit_after_load=True)
        failed = trace_api('POST', '/api/sessions/' + ident + '/restart', {'wake_message': ''})
        assert failed[0] == 409, failed
        before = trace_row(ident)
        assert intent(before) == 'Use' and before.get('resume_probe_failed_sid') == sid, before
        actions = [refused(ident, action, 'POST', '/api/sessions/' + ident + '/' + action, {'message': 'PRIVATE_PERSISTENT_PIN_BLOCKED'} if action == 'send' else {}, before, transcript, original) for action in ('ensure', 'send')]
        repeated = trace_api('POST', '/api/sessions/' + ident + '/restart', {'wake_message': ''})
        assert repeated[0] == 409 and intent(trace_row(ident)) == 'Use', repeated
        assert len(launches(ident)) == 3 and all(event['sid'] == sid for event in launches(ident))
        mode(ident, publish=False)
        retry = trace_api('POST', '/api/sessions/' + ident + '/restart', {'wake_message': ''})
        assert retry[0] == 200, retry
        assert len(launches(ident)) == 4 and all(event['sid'] == sid for event in launches(ident))
        current = trace_row(ident)
        assert intent(current) == 'Use' and current.get('resume_probe_failed_sid') is None, current
        assert transcript.read_bytes() == original
        proof['cases'].append({'name': 'persistent-pin-explicit-same-only', 'sid': sid, 'pin_stays_Use': True, 'blocked_actions': actions, 'explicit_manual_retries_same_sid': 2, 'no_reassertion_needed_for_persistent_pin': True, 'launches': 4, 'no_fresh': True, 'history_unchanged': True})
        drop_session(ident)
        ident = trace_add('policy-fork-dispatch', 'work-claude')
        mode(ident, publish=True)
        assert trace_api('POST', '/api/sessions/' + ident + '/restart', {'wake_message': ''})[0] == 200
        parent_native = trace_until(lambda: next(iter(launches(ident)), None), 'fork parent Native')
        trace_until(lambda: row_with_sid(ident, parent_native['sid']), 'fork parent publication')
        parent_path = pathlib.Path(parent_native['transcript']);parent_bytes = parent_path.read_bytes()
        trace_cli('session', 'stop', ident)
        trace_cli('add', str(root / 'project'), '--title', 'policy-fork-child', '--fork-from', ident)
        child = next(row['id'] for row in json.loads((trace_app / 'profiles/main/sessions.json').read_text()) if row['title'] == 'policy-fork-child')
        trace_ids.append(child)
        trace_until(lambda: trace_listed(child), 'fork child CLI creation observed by daemon')
        mode(child, publish=True)
        started = trace_api('POST', '/api/sessions/' + child + '/start', {})
        assert started[0] == 200, started
        forked = trace_until(lambda: next((event for event in trace_events(child) if event['action'] == 'forked'), None), 'real Native fork dispatch')
        child_row = trace_until(lambda: row_with_sid(child, forked['sid']), 'fork child publication')
        assert forked['parent'] == parent_native['sid'] and forked['sid'] != parent_native['sid']
        assert intent(child_row) == 'Default' and parent_path.read_bytes() == parent_bytes
        restarted = trace_api('POST', '/api/sessions/' + child + '/restart', {'wake_message': ''})
        assert restarted[0] == 200, restarted
        assert len([event for event in trace_events(child) if event['action'] == 'forked']) == 1
        assert launches(child)[-1]['sid'] == forked['sid']
        proof['cases'].append({'name': 'fork-intent-spent-at-dispatch', 'parent_sid': parent_native['sid'], 'child_sid': forked['sid'], 'forks': 1, 'child_restart_resumes_child': True, 'parent_history_unchanged': True})
        drop_session(child);drop_session(ident)
        ident = trace_add('policy-automatic-optout', 'work-claude')
        mode(ident, publish=True)
        assert trace_api('POST', '/api/sessions/' + ident + '/restart', {'wake_message': ''})[0] == 200
        native = trace_until(lambda: next(iter(launches(ident)), None), 'automatic-case Native creation')
        sid = native['sid']
        before = trace_until(lambda: row_with_sid(ident, sid), 'automatic-case durable identity')
        transcript = pathlib.Path(native['transcript']);original = transcript.read_bytes()
        assert before.get('resume_probe_failed_sid') is None
        trace_stop_daemon()
        trace_config(auto=False, declared=True)
        process = pathlib.Path('/proc') / str(native['pid'])
        birth = (process / 'stat').read_text().rsplit(')', 1)[1].split()[19]
        assert birth == native['birth'], 'refuse to signal a different process birth'
        os.kill(native['pid'], 15)
        trace_until(lambda: not process.exists(), 'only the recorded Native birth exits')
        offset = (trace_app / 'debug.log').stat().st_size
        trace_start_daemon()
        def automatic_refused():
            text = (trace_app / 'debug.log').read_bytes()[offset:].decode('utf-8', errors='replace')
            return text if 'canonical recovery restart refused or failed' in text and ident in text else False
        automatic_log = trace_until(automatic_refused, 'actual automatic recovery completes with refusal', seconds=45)
        write(str(evidence / 'automatic-optout-refusal.log'), automatic_log)
        current = trace_row(ident)
        assert current.get('agent_session_id') == sid and current.get('resume_probe_failed_sid') is None, current
        assert len(launches(ident)) == 1 and transcript.read_bytes() == original
        assert not any(event['action'] == 'input' for event in trace_events(ident))
        trace_cli('session', 'start', ident)
        fresh = trace_until(lambda: next((event for event in launches(ident) if event['action'] == 'created' and event['sid'] != sid), None), 'manual configured-Fresh after automatic refusal')
        trace_until(lambda: row_with_sid(ident, fresh['sid']), 'manual configured-Fresh identity committed')
        assert len(launches(ident)) == 2 and transcript.read_bytes() == original
        assert not any(event['action'] == 'input' for event in trace_events(ident))
        proof['cases'].append({'name': 'automatic-optout-refuses-configured-fresh', 'original_sid': sid, 'manual_fresh_sid': fresh['sid'], 'recorded_native_birth_only_signalled': True, 'automatic_refusal_causally_observed': True, 'no_automatic_native_launch_or_wake': True, 'manual_configured_fresh_allowed': True, 'history_unchanged': True, 'launches': 2})
        drop_session(ident)
        write(str(evidence / 'proof-before-cleanup.json'), json.dumps(proof, indent=2))
        print('TERMINAL_5040_APPROVED_POLICY_ACTUAL_GREEN', json.dumps({'cases': [case['name'] for case in proof['cases']], 'binary_sha256': proof['binary_sha256']}))
    except BaseException as error:
        diagnostic = {'error': repr(error), 'events': trace_events(), 'sessions': json.loads((trace_app / 'profiles/main/sessions.json').read_text()) if (trace_app / 'profiles/main/sessions.json').exists() else [], 'daemon_output': (root / 'daemon-output.log').read_text() if (root / 'daemon-output.log').exists() else ''}
        diagnostic['configured_log'] = (trace_app / 'debug.log').read_text() if (trace_app / 'debug.log').exists() else ''
        write(str(evidence / 'failure.json'), json.dumps(diagnostic, indent=2))
        raise
    finally:
        native_pids = {int(event['pid']) for event in trace_events() if 'pid' in event}
        subprocess.run([*ui_driver, 'kill-server'], env=trace_env, capture_output=True, text=True, timeout=10)
        try:
            for ident in list(trace_ids):
                trace_cli('remove', ident, '--purge')
                trace_ids.remove(ident)
        finally:
            trace_stop_daemon()
            subprocess.run([real_tmux, '-S', str(root / 'tmux.sock'), 'kill-server'], capture_output=True, text=True, timeout=10)
        def gone():
            for pid in native_pids:
                try:os.kill(pid, 0);return False
                except ProcessLookupError:pass
            return True
        trace_until(gone, 'all owned terminal Native processes reaped')
        shutil.rmtree(root)
        proof['cleanup'] = {'native_processes_gone': True, 'daemon_stopped': True, 'owned_tmux_servers_stopped': True, 'private_root_removed': True}
        write(str(evidence / 'proof.json'), json.dumps(proof, indent=2))
    return proof
