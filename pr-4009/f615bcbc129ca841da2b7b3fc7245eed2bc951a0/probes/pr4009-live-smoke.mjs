import { mkdtempSync, mkdirSync, writeFileSync, readFileSync, existsSync, readdirSync, readlinkSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { createServer } from 'node:net';
import assert from 'node:assert/strict';

const binary = resolve('target/debug/aoe');
const output = resolve('target/protocol4-interop');
const results = [];
async function command(args, env, allowFailure = false) {
  const p = Bun.spawn(args, { env, stdout: 'pipe', stderr: 'pipe' });
  const timer = setTimeout(() => p.kill('SIGKILL'), 30000);
  try {
    const [stdout, stderr, code] = await Promise.all([new Response(p.stdout).text(), new Response(p.stderr).text(), p.exited]);
    if (code && !allowFailure) throw new Error(`${args[0]} ${args.slice(1, 3).join(' ')} failed (${code}): ${stderr}`);
    return { stdout, stderr, code };
  } finally { clearTimeout(timer); }
}
function environment(home) {
  const env = { ...process.env };
  for (const key of Object.keys(env)) if (/^(AOE_|AGENT_OF_EMPIRES_)/.test(key) || ['TMUX', 'TMUX_PANE'].includes(key)) delete env[key];
  Object.assign(env, { HOME: home, XDG_CONFIG_HOME: join(home, '.config'), XDG_DATA_HOME: join(home, '.local/share'), XDG_CACHE_HOME: join(home, '.cache'), XDG_RUNTIME_DIR: join(home, 'runtime'), DBUS_SESSION_BUS_ADDRESS: 'unix:path=/dev/null', AOE_TMUX_SOCKET: join(home, 'tmux.sock'), TERM: 'xterm-256color', DO_NOT_TRACK: '1' });
  for (const key of ['HTTP_PROXY', 'HTTPS_PROXY', 'ALL_PROXY', 'http_proxy', 'https_proxy', 'all_proxy']) env[key] = '';
  mkdirSync(env.XDG_RUNTIME_DIR, { recursive: true });
  return env;
}
async function waitUntil(predicate, description, daemon) {
  const deadline = performance.now() + 30000;
  let last;
  while (performance.now() < deadline) {
    if (daemon && daemon.exitCode !== null) throw new Error(`${description}: owned daemon exited ${daemon.exitCode}`);
    try { const value = await predicate(); if (value) return value; } catch (e) { last = String(e); }
    await Bun.sleep(30);
  }
  throw new Error(`${description}: deadline, last=${last}`);
}
async function freePort() {
  const server = createServer();
  await new Promise((done, fail) => { server.once('error', fail); server.listen(0, '127.0.0.1', done); });
  const port = server.address().port;
  await new Promise(done => server.close(done));
  return port;
}
async function http(env, socket, path, body, headers = {}, tcp) {
  const args = ['curl', '--silent', '--show-error', '--noproxy', '*', '--max-time', '5', '--write-out', '\n%{http_code}'];
  if (socket) args.push('--unix-socket', socket);
  if (body !== undefined) args.push('--header', 'Content-Type: application/json', '--header', 'Accept: application/json, text/event-stream', '--data-binary', JSON.stringify(body));
  for (const [name, value] of Object.entries(headers)) args.push('--header', `${name}: ${value}`);
  args.push(`${tcp || 'http://localhost'}${path}`);
  const result = await command(args, env, true);
  if (result.code) throw new Error(`Owned fixture HTTP failed (${result.code})`);
  const boundary = result.stdout.lastIndexOf('\n');
  const text = result.stdout.slice(0, boundary);
  return { status: Number(result.stdout.slice(boundary + 1)), text, json: (() => { try { return JSON.parse(text); } catch { return null; } })() };
}
function tcpListenersFor(pid) {
  const owned = new Set(readdirSync(`/proc/${pid}/fd`).flatMap(fd => { try { const match = readlinkSync(`/proc/${pid}/fd/${fd}`).match(/^socket:\[(\d+)\]$/); return match ? [match[1]] : []; } catch { return []; } }));
  return ['tcp', 'tcp6'].flatMap(name => readFileSync(`/proc/${pid}/net/${name}`, 'utf8').trim().split('\n').slice(1).map(line => line.trim().split(/\s+/)).filter(parts => parts[3] === '0A' && owned.has(parts[9])));
}
async function stopOwned(daemon) {
  if (!daemon || daemon.exitCode !== null) return;
  daemon.kill('SIGINT');
  let killed = false;
  const timer = setTimeout(() => { killed = true; daemon.kill('SIGKILL'); }, 10000);
  const code = await daemon.exited;
  clearTimeout(timer);
  assert.equal(killed, false, 'owned daemon must drain without forced termination');
  assert.equal(code, 0, 'owned daemon clean shutdown');
}

export { command, environment, waitUntil, http, stopOwned, freePort, binary, output };

if (import.meta.main) {
for (const mode of ['enabled', 'disabled', 'read-only', 'cityhall', 'tcp-token']) {
  const home = mkdtempSync('/tmp/aoe-mcp-');
  const env = environment(home);
  const app = join(env.XDG_CONFIG_HOME, 'agent-of-empires-dev');
  mkdirSync(app, { recursive: true });
  writeFileSync(join(app, 'config.toml'), `[updates]\nupdate_check_mode = "off"\n[systemd]\nenabled = false\n[app_state]\nhas_seen_welcome = true\nhas_responded_to_telemetry = true\nhas_acknowledged_agent_hooks = true\nlast_seen_version = "1.20.0"\n[web]\nmcp_enabled = ${mode !== 'disabled'}\n`);
  await command([binary, 'profile', 'create', 'default'], env);
  for (const title of ['mcp-parent', 'mcp-child', 'mcp-trashed']) await command([binary, '--profile', 'default', 'add', home, '--title', title, '--tool', 'claude'], env);
  const store = join(app, 'profiles/default/sessions.json');
  const rows = JSON.parse(readFileSync(store, 'utf8'));
  const parent = rows.find(row => row.title === 'mcp-parent');
  const child = rows.find(row => row.title === 'mcp-child');
  const trashed = rows.find(row => row.title === 'mcp-trashed');
  parent.group_path = 'work'; child.group_path = 'work/sub'; child.parent_session_id = parent.id;
  trashed.group_path = 'work'; trashed.trashed_at = new Date().toISOString();
  writeFileSync(store, JSON.stringify(rows));
  const orderingPath = join(app, 'workspace-ordering.json');
  writeFileSync(orderingPath, '{"order":["manual-last"]}\n');
  const socket = join(app, 'daemon/api.sock');
  const port = mode === 'tcp-token' ? await freePort() : undefined;
  const args = [binary, 'serve', ...(port ? ['--port', String(port)] : ['--core-only'])];
  if (mode === 'read-only') args.push('--read-only');
  if (mode === 'cityhall') args.push('--cityhall');
  let daemon;
  let logs = '';
  let drains;
  try {
    daemon = Bun.spawn(args, { env, stdout: 'pipe', stderr: 'pipe' });
    drains = Promise.all([daemon.stdout, daemon.stderr].map(async stream => { for await (const bytes of stream) logs += Buffer.from(bytes).toString(); }));
    const ready = await waitUntil(async () => { if (!existsSync(socket)) return false; const response = await http(env, socket, '/api/runtime'); return response.status === 200 && response.json; }, 'native HTTP readiness', daemon);
    if (!port) assert.equal(tcpListenersFor(daemon.pid).length, 0, 'Core-only MCP must expose no TCP listener');
    const orderingBefore = readFileSync(orderingPath, 'utf8');
    const rpc = (name, params = {}) => http(env, socket, '/mcp', { jsonrpc: '2.0', id: 1, method: 'tools/call', params: { name, arguments: params } });
    if (mode === 'disabled') {
      const disabled = await rpc('list_sessions');
      assert.equal(disabled.status, 404);
      results.push({ mode, transport: 'real owner Unix HTTP', status: disabled.status, routeAbsent: true, tcpListeners: 0 });
    } else if (mode === 'cityhall') {
      const denied = await rpc('list_sessions');
      assert.equal(denied.status, 403);
      results.push({ mode, transport: 'real owner Unix HTTP', status: denied.status, tcpListeners: 0 });
    } else {
      const advertised = await http(env, socket, '/mcp', { jsonrpc: '2.0', id: 2, method: 'tools/list' });
      assert.equal(advertised.status, 200);
      assert.deepEqual(advertised.json.result.tools.map(tool => tool.name).sort(), ['list_sessions', 'whoami']);
      const listTool = advertised.json.result.tools.find(tool => tool.name === 'list_sessions');
      const listed = await rpc('list_sessions', { state: 'all' });
      assert.equal(listed.status, 200);
      assert.notEqual(listed.json.result.isError, true);
      const envelope = listed.json.result.structuredContent;
      const rest = await http(env, socket, '/api/sessions?state=all');
      assert.equal(rest.status, 200);
      assert.deepEqual(envelope, rest.json);
      const titles = envelope => envelope.sessions.map(row => row.title).sort();
      assert.deepEqual(titles(envelope), ['mcp-child', 'mcp-parent', 'mcp-trashed']);
      for (const [filter, expected] of [[{}, ['mcp-child', 'mcp-parent']], [{ group: 'work/sub' }, ['mcp-child']], [{ children_of: parent.id }, ['mcp-child']], [{ children_of: child.id }, []]]) {
        const response = await rpc('list_sessions', filter);
        assert.equal(response.status, 200);
        assert.deepEqual(titles(response.json.result.structuredContent), expected);
      }
      const who = await rpc('whoami');
      assert.equal(who.json.result.structuredContent, null);
      assert.equal(readFileSync(orderingPath, 'utf8'), orderingBefore);
      writeFileSync(join(output, `mcp-${mode}-schema.json`), JSON.stringify(listTool.outputSchema));
      writeFileSync(join(output, `mcp-${mode}-body.json`), JSON.stringify(envelope));
      let tcpAuth;
      if (port) {
        const tcp = `http://127.0.0.1:${port}`;
        const request = { jsonrpc: '2.0', id: 3, method: 'tools/list' };
        const denied = await http(env, null, '/mcp', request, {}, tcp);
        assert.equal(denied.status, 401);
        assert.ok(denied.json && !denied.text.includes('<html'));
        const published = new URL(readFileSync(join(app, 'serve.url'), 'utf8').trim());
        const token = published.searchParams.get('token');
        assert.ok(token, 'actual published fixture credential');
        const accepted = await http(env, null, '/mcp', request, { Authorization: `Bearer ${token}` }, tcp);
        assert.equal(accepted.status, 200);
        tcpAuth = { missingCredential: denied.status, validCredential: accepted.status, unauthorizedIsJson: true };
      }
      results.push({ mode, transport: 'real owner Unix HTTP', schemaAndBodySaved: true, canonicalRestEquality: true, filters: 4, rows: envelope.sessions.length, orderingBytesUnchanged: true, epochObserved: Boolean(ready.epoch), tcpListeners: port ? tcpListenersFor(daemon.pid).length : 0, tcpAuth });
    }
  } finally {
    await stopOwned(daemon);
    await drains;
    writeFileSync(join(output, `mcp-${mode}-daemon.log`), logs.replace(/token=[^&\s]+/g, 'token=[REDACTED]'));
    await command(['tmux', '-S', env.AOE_TMUX_SOCKET, 'kill-server'], env, true);
  }
}
writeFileSync(join(output, 'mcp-live-proof.json'), JSON.stringify(results, null, 2));
console.log(JSON.stringify(results, null, 2));
}
