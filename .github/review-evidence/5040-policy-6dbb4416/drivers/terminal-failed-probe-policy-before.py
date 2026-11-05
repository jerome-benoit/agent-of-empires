def run_terminal_failed_probe_policy_before(binary):
    global trace_binary
    root = prepare_epic_private_fixture('aoe-policy-before-', binary)
    ident = None
    native_pids = []
    proof = {'scope': 'real private AoE daemon/CLI/tmux and transcript-reading fake; resumed native exit is intentional, not the original OMP incident'}
    try:
        (root / 'home/.claude').mkdir(parents=True, exist_ok=True)
        trace_start_daemon()
        trace_cli('hooks', 'approve')
        ident = trace_add('failed-probe-policy-before', 'work-claude')
        mode_file = root / 'modes' / (ident + '.json')
        write(str(mode_file), json.dumps({'publish': True}))
        initial = trace_api('POST', '/api/sessions/' + ident + '/restart', {'wake_message': ''})
        assert initial[0] == 200, initial
        created = trace_until(lambda: next((e for e in trace_events(ident) if e['action'] == 'created'), None), 'original native transcript created')
        native_pids.append(created['pid'])
        sid = created['sid']
        trace_until(lambda: trace_row(ident).get('agent_session_id') == sid, 'original SID persisted')
        original_binding = trace_until(lambda: trace_row(ident).get('agent_session_binding'), 'initial declared execution/store binding qualified')
        transcript = pathlib.Path(created['transcript'])
        original_bytes = transcript.read_bytes()
        write(str(mode_file), json.dumps({'publish': True, 'exit_after_load': True}))
        failed = trace_api('POST', '/api/sessions/' + ident + '/restart', {'wake_message': ''})
        assert failed[0] == 409 and json.loads(failed[2]).get('error') == 'resume_failed', failed
        loaded = [e for e in trace_events(ident) if e['action'] == 'loaded']
        assert loaded and loaded[-1]['sid'] == sid and loaded[-1]['content'].encode() == original_bytes
        native_pids.append(loaded[-1]['pid'])
        marked = trace_row(ident)
        assert marked.get('agent_session_id') == sid
        assert marked.get('resume_probe_failed_sid') == sid
        marked_binding = marked.get('agent_session_binding')
        assert marked_binding and marked_binding['session_id'] == original_binding['session_id'] == sid, (original_binding, marked_binding)
        for field in ('agent', 'stores', 'configuration', 'cwd', 'cwd_filesystem', 'filesystem'):
            default = [] if field == 'configuration' else None
            assert (original_binding.get('execution') or {}).get(field, default) == (marked_binding.get('execution') or {}).get(field, default), (field, original_binding, marked_binding)
        assert transcript.read_bytes() == original_bytes
        write(str(mode_file), json.dumps({'publish': True}))
        sent = trace_api('POST', '/api/sessions/' + ident + '/send', {'message': 'PRIVATE_INPUT_AFTER_FAILED_PROBE'})
        assert sent[0] == 200, sent
        replacement = trace_until(lambda: next((e for e in trace_events(ident) if e['action'] == 'created' and e['sid'] != sid), None), 'baseline fresh association after failed resume')
        native_pids.append(replacement['pid'])
        trace_until(lambda: trace_row(ident).get('agent_session_id') == replacement['sid'], 'fresh association durable')
        delivery = trace_until(lambda: next((e for e in trace_events(ident) if e['action'] == 'input' and 'PRIVATE_INPUT_AFTER_FAILED_PROBE' in e.get('line', '')), None), 'explicit input delivered to new conversation')
        assert delivery['sid'] == replacement['sid']
        assert transcript.read_bytes() == original_bytes
        proof.update({'session': ident, 'original_sid': sid, 'replacement_sid': replacement['sid'], 'failed_resume_http': failed[0], 'next_send_http': sent[0], 'original_history_read_before_exit': True, 'original_history_bytes_unchanged': True, 'failure_marker_observed_before_next_action': marked['resume_probe_failed_sid'], 'next_input_recipient_sid': delivery['sid'], 'native_pids': native_pids})
        proof['binding_before_probe'] = original_binding
        proof['binding_after_probe'] = marked_binding
        proof['binding_identity_preserved'] = True
        print('BASELINE_FAILED_PROBE_SEND_SELECTS_FRESH', json.dumps(proof))
    except Exception as error:
        debug_log = trace_app / 'debug.log'
        if debug_log.exists():
            error.add_note(debug_log.read_text()[-8000:])
        raise
    finally:
        try:
            if ident:
                trace_cli('remove', ident, '--purge')
                trace_ids.remove(ident)
        finally:
            trace_stop_daemon()
            subprocess.run(['/usr/bin/tmux', '-S', str(root / 'tmux.sock'), 'kill-server'], capture_output=True, text=True, timeout=10)
        def gone():
            for pid in native_pids:
                try:
                    os.kill(pid, 0)
                    return False
                except ProcessLookupError:
                    pass
            return True
        trace_until(gone, 'private policy native processes reaped')
        shutil.rmtree(root)
        proof['cleanup'] = {'native_processes_gone': True, 'daemon_stopped': True, 'private_root_removed': True}
    return proof
