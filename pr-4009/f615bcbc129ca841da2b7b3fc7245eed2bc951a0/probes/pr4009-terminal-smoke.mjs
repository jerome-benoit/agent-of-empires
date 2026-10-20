import { mkdtempSync, mkdirSync, writeFileSync, readFileSync, existsSync, chmodSync, openSync, closeSync, writeSync, constants } from 'node:fs';
import { join } from 'node:path';
import assert from 'node:assert/strict';
import { command, environment, waitUntil, http, stopOwned, binary, output } from './pr4009-live-smoke.mjs';

const home = mkdtempSync('/tmp/aoe-terminal-');
const env = environment(home);
const app = join(env.XDG_CONFIG_HOME, 'agent-of-empires-dev');
const socket = join(app, 'daemon/api.sock');
const fifo = join(home, 'probe.fifo');
const input = join(home, 'received.input');
const probe = join(home, 'probe-agent');
const barrier = join(home, 'input-barrier');
const renderLog = join(home, 'render.log');
const title = 'freshness-probe';
const running = new Set([... '⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏']);
const waiting = new Set([... '⠃⠉⠘⠰⢠⣀⡄⠆']);
const samples = [];
let daemon;
let drains;
let fd;
let logs = '';
const tmux = (...args) => command(['tmux', '-S', env.AOE_TMUX_SOCKET, ...args], env);
const screen = async (name = 'proof-ui') => (await tmux('capture-pane', '-p', '-t', name)).stdout;
const paneText = async pane => (await tmux('capture-pane', '-p', '-t', pane)).stdout;
function glyph(text, want) {
  const frames = want === 'running' ? running : waiting;
  return text.split('\n').filter(line => line.includes(title) && !line.includes('Title:')).some(line => [...line].some(char => frames.has(char)));
}
async function syncInput(name = 'proof-ui', ack = barrier, log = renderLog) {
  const previous = existsSync(ack) ? Number(readFileSync(ack, 'utf8')) : 0;
  await tmux('send-keys', '-t', name, 'F12');
  const sequence = await waitUntil(() => existsSync(ack) && Number(readFileSync(ack, 'utf8')) > previous && Number(readFileSync(ack, 'utf8')), 'actual TUI input acknowledgement');
  await waitUntil(() => existsSync(log) && readFileSync(log).includes(Buffer.from(`\x1b]0;aoe-e2e-${sequence}\x07`)), 'actual terminal render acknowledgement');
  await tmux('display-message', '-p', '-t', name, '#{pane_id}');
}
async function spawnTui(name, args = [], extra = {}, log = renderLog) {
  writeFileSync(log, '');
  const fixtureEnv = { ...env, ...extra };
  const quote = value => `'${String(value).replaceAll("'", "'\\''")}'`;
  const injected = ['HOME', 'XDG_CONFIG_HOME', 'XDG_DATA_HOME', 'XDG_CACHE_HOME', 'XDG_RUNTIME_DIR', 'AOE_TMUX_SOCKET', 'AOE_E2E_INPUT_BARRIER'].map(key => `${key}=${quote(fixtureEnv[key])}`).join(' ');
  await command(['tmux', '-S', env.AOE_TMUX_SOCKET, 'new-session', '-d', '-s', name, '-x', '140', '-y', '44', `env -u NO_COLOR ${injected} ${quote(binary)} --profile default ${args.join(' ')}`, ';', 'set-option', '-s', 'exit-empty', 'off'], fixtureEnv);
  await tmux('pipe-pane', '-t', name, '-o', `cat >> '${log}'`);
}
async function rowStatus(id) {
  const response = await http(env, socket, '/api/sessions?state=all');
  assert.equal(response.status, 200);
  return response.json.sessions.find(row => row.id === id)?.status.toLowerCase();
}
try {
  mkdirSync(app, { recursive: true });
  writeFileSync(probe, '#!/bin/sh\ncat < /dev/tty > "$HOME/received.input" &\nreader=$!\ntrap \'kill "$reader" 2>/dev/null\' EXIT\nwhile IFS= read -r marker; do\n  printf "\\033[2J\\033[H%s\\n" "$marker"\n  tmux -S "$AOE_TMUX_SOCKET" clear-history -t "$TMUX_PANE" 2>/dev/null\ndone < "$HOME/probe.fifo"\n');
  chmodSync(probe, 0o755);
  writeFileSync(join(app, 'config.toml'), `[updates]\nupdate_check_mode = "off"\n[systemd]\nenabled = false\n[app_state]\nhas_seen_welcome = true\nhas_responded_to_telemetry = true\nhas_acknowledged_agent_hooks = true\nlast_seen_version = "1.20.0"\n[session.custom_agents]\nprobe = ${JSON.stringify(probe)}\n[[agents.probe.status_rules]]\nstatus = "waiting"\ncontains = "aoe_probe_waiting"\n[[agents.probe.status_rules]]\nstatus = "running"\ncontains = "aoe_probe_running"\n`);
  await command(['mkfifo', fifo], env);
  fd = openSync(fifo, constants.O_RDWR | constants.O_NONBLOCK);
  env.AOE_E2E_INPUT_BARRIER = barrier;
  await command([binary, 'profile', 'create', 'default'], env);
  await command([binary, '--profile', 'default', 'add', home, '--title', title, '--tool', 'probe'], env);
  const store = join(app, 'profiles/default/sessions.json');
  const storedRows = JSON.parse(readFileSync(store, 'utf8'));
  const row = storedRows.find(row => row.title === title);
  row.status = 'stopped';
  writeFileSync(store, JSON.stringify(storedRows));
  daemon = Bun.spawn([binary, 'serve', '--core-only'], { env, stdout: 'pipe', stderr: 'pipe' });
  drains = Promise.all([daemon.stdout, daemon.stderr].map(async stream => { for await (const bytes of stream) logs += Buffer.from(bytes).toString(); }));
  const initialInfo = await waitUntil(async () => { if (!existsSync(socket)) return false; const response = await http(env, socket, '/api/runtime'); return response.status === 200 && response.json; }, 'Core readiness', daemon);
  writeSync(fd, 'aoe_probe_running\n');
  const start = await http(env, socket, `/api/sessions/${row.id}/start`, {}, { 'aoe-runtime-epoch': initialInfo.epoch });
  assert.equal(start.status, 200, `native start: ${start.text}`);
  const agentPanes = (await tmux('list-panes', '-a', '-F', '#{session_name} #{pane_id} #{pane_pid}')).stdout.trim().split('\n');
  assert.equal(agentPanes.length, 1, 'one real terminal launched by the Daemon');
  const [agentSession, agentPane, firstPid] = agentPanes[0].split(' ');
  await spawnTui('proof-ui');
  await waitUntil(async () => (await rowStatus(row.id)) === 'running' && glyph(await screen(), 'running'), 'TUI and Daemon observe initial real running pane', daemon);
  await syncInput();
  writeFileSync(join(output, 'terminal-running-screen.txt'), await screen());
  for (let index = 0; index < 20; index++) {
    const want = index % 2 === 0 ? 'waiting' : 'running';
    const marker = `aoe_probe_${want}`;
    writeSync(fd, `${marker}\n`);
    await waitUntil(async () => (await paneText(agentPane)).includes(marker), 'actual pane marker transition', daemon);
    const t0 = performance.now();
    let daemonMs;
    let tuiMs;
    await waitUntil(async () => {
      if (daemonMs === undefined && await rowStatus(row.id) === want) daemonMs = performance.now() - t0;
      if (tuiMs === undefined && glyph(await screen(), want)) tuiMs = performance.now() - t0;
      return daemonMs !== undefined && tuiMs !== undefined;
    }, `terminal ${want} reaches canonical snapshot and actual TUI`, daemon);
    samples.push({ index, want, daemonMs, tuiMs, observedClientDeltaMs: tuiMs - daemonMs });
    assert.ok(tuiMs <= 2000, `predeclared healthy-local 2000ms target exceeded: ${JSON.stringify(samples.at(-1))}`);
  }
  writeFileSync(join(output, 'terminal-final-screen.txt'), await screen());
  await tmux('new-session', '-d', '-s', 'native-client-holder', `env -u TMUX tmux -S '${env.AOE_TMUX_SOCKET}' attach-session -t '=${agentSession}:'`);
  const client = await waitUntil(async () => (await tmux('list-clients', '-F', '#{client_pid} #{client_session}')).stdout.split('\n').find(line => line.endsWith(` ${agentSession}`)), 'pre-existing independent native tmux client');
  await stopOwned(daemon);
  await drains;
  await waitUntil(async () => { const text = await screen(); return text.includes('Runtime unavailable') && !text.includes(title); }, 'offline view hides all canonical row content');
  writeFileSync(join(output, 'terminal-offline-screen.txt'), await screen());
  assert.ok((await tmux('list-clients', '-F', '#{client_pid} #{client_session}')).stdout.includes(client), 'existing native client remains attached after Daemon loss');
  const diskBefore = readFileSync(store);
  writeFileSync(input, '');
  await tmux('respawn-pane', '-k', '-t', agentPane, probe);
  const secondPid = (await tmux('display-message', '-p', '-t', agentPane, '#{pane_pid}')).stdout.trim();
  assert.notEqual(secondPid, firstPid);
  writeSync(fd, 'replacement_ready\n');
  await waitUntil(async () => (await paneText(agentPane)).includes('replacement_ready'), 'same-pane replacement is alive before negative input check');
  await tmux('send-keys', '-t', 'proof-ui', '-l', 'ZXCVB97531');
  await tmux('send-keys', '-t', 'proof-ui', 'Enter', 'n', 'Enter');
  await syncInput();
  const windowEnds = performance.now() + 1000;
  let checkedFrames = 0;
  while (performance.now() < windowEnds) {
    await syncInput();
    const text = await screen();
    assert.ok(text.includes('Runtime unavailable') && !text.includes(title));
    assert.equal(readFileSync(input, 'utf8'), '', 'no offline send-keys fallback to replacement pane');
    checkedFrames++;
    await Bun.sleep(30);
  }
  assert.deepEqual(readFileSync(store), diskBefore, 'disconnected TUI performs no local persisted mutation');
  await tmux('send-keys', '-t', 'proof-ui', 'r');
  const newInfo = await waitUntil(async () => { if (!existsSync(socket)) return false; const response = await http(env, socket, '/api/runtime'); return response.status === 200 && response.json.epoch !== initialInfo.epoch && response.json; }, 'reconnect authenticates a new local runtime');
  await waitUntil(async () => { const text = await screen(); return text.includes(title) && !text.includes('Runtime unavailable'); }, 'fresh snapshot restores actual TUI');
  await syncInput();
  writeFileSync(join(output, 'terminal-reconnected-screen.txt'), await screen());
  await command([binary, 'serve', '--stop'], env);
  await waitUntil(() => !existsSync(socket), 'managed reconnect Daemon stops and unpublishes');
  await tmux('kill-session', '-t', 'proof-ui');
  const unavailableAck = join(home, 'unavailable-barrier');
  const unavailableLog = join(home, 'unavailable-render.log');
  await spawnTui('initial-unavailable', ['--daemon-url', 'http://127.0.0.1:1'], { AOE_E2E_INPUT_BARRIER: unavailableAck }, unavailableLog);
  await waitUntil(async () => { const text = await screen('initial-unavailable'); return text.includes('Could not reach daemon:') && text.includes('Press r to retry') && !text.includes(title); }, 'initial explicit-endpoint failure exposes only unavailable view');
  assert.equal(existsSync(socket), false, 'explicit endpoint failure publishes no local runtime');
  assert.equal(existsSync(join(app, 'serve.pid')), false, 'explicit endpoint failure starts no managed local Daemon');
  writeFileSync(join(output, 'terminal-initial-unavailable-screen.txt'), await screen('initial-unavailable'));
  const quantiles = key => {
    const values = samples.map(sample => sample[key]).sort((a, b) => a - b);
    return { min: values[0], p50: values[Math.ceil(values.length * 0.5) - 1], p95: values[Math.ceil(values.length * 0.95) - 1], max: values.at(-1) };
  };
  const proof = { targetMs: 2000, targetDeclaredBeforeMeasurement: true, measuredTransitions: samples.length, transport: 'real Core owner Unix API + native runtime WS + real tmux + actual TUI', startObservation: 'new marker first observed in real agent pane', resolution: '30ms predicate poll plus HTTP and terminal capture costs; client delta is observation order, not a strict propagation bound', daemonMs: quantiles('daemonMs'), tuiMs: quantiles('tuiMs'), samples, offline: { hiddenRows: true, noLocalStoreWrite: true, samePaneReplaced: true, noFallbackInput: true, causalRenderedFramesObserved: checkedFrames, negativeObservationMs: 1000, independentNativeClientSurvives: true, explicitReconnectNewEpoch: newInfo.epoch !== initialInfo.epoch, freshTuiRestored: true, initialUnavailableNoLocalSpawn: true }, official3900DeliveryClaimed: false };
  writeFileSync(join(output, 'terminal-live-proof.json'), JSON.stringify(proof, null, 2));
  console.log(JSON.stringify(proof, null, 2));
} catch (error) {
  const panes = await command(['tmux', '-S', env.AOE_TMUX_SOCKET, 'list-panes', '-a', '-F', '#{pane_id}'], env, true);
  const diagnostics = { error: String(error), home, panes: [] };
  for (const pane of panes.stdout.trim().split('\n').filter(Boolean)) {
    const captured = await command(['tmux', '-S', env.AOE_TMUX_SOCKET, 'capture-pane', '-p', '-t', pane], env, true);
    diagnostics.panes.push({ pane, screen: captured.stdout });
  }
  try { diagnostics.api = await http(env, socket, '/api/sessions?state=all'); } catch {}
  writeFileSync(join(output, 'terminal-failure.json'), JSON.stringify(diagnostics, null, 2));
  throw error;
} finally {
  await stopOwned(daemon);
  await drains;
  await command([binary, 'serve', '--stop'], env, true);
  await command(['tmux', '-S', env.AOE_TMUX_SOCKET, 'kill-server'], env, true);
  if (fd !== undefined) closeSync(fd);
  writeFileSync(join(output, 'terminal-daemon.log'), logs);
}
