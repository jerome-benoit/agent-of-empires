import { mkdtempSync, mkdirSync, writeFileSync, readFileSync, existsSync, watch } from 'node:fs';
import { resolve, join, dirname } from 'node:path';
import { createHash } from 'node:crypto';

const label = process.argv[2] || 'before';
const expectedField = process.argv[3] === 'present';
const base = resolve('target/protocol4-interop');
const home = mkdtempSync(join('/tmp', `aoe-v4-${label}-`));
const runtime = join(home, 'runtime');
mkdirSync(join(runtime, 'systemd'), { recursive: true });
// This is exactly the production bus-free visibility predicate, not a systemd service.
writeFileSync(join(runtime, 'systemd/private'), '');
const binary = resolve(process.env.AOE_PROTOCOL_PROBE_BINARY || 'target/debug/aoe');
const env = { ...process.env, HOME: home, XDG_CONFIG_HOME: join(home, '.config'),
  XDG_DATA_HOME: join(home, '.local/share'), XDG_RUNTIME_DIR: runtime,
  DBUS_SESSION_BUS_ADDRESS: `unix:path=${join(runtime, 'absent-bus')}`,
  AOE_TMUX_SOCKET: `aoe_proto4_${label}_${process.pid}`, DO_NOT_TRACK: '1',
  HTTP_PROXY: '', HTTPS_PROXY: '', ALL_PROXY: '', http_proxy: '', https_proxy: '', all_proxy: '' };
for (const key of ['AOE_DAEMON_URL', 'AOE_DAEMON_TOKEN', 'AOE_DAEMON_PASSPHRASE', 'AOE_SERVE_PASSPHRASE', 'AOE_CITYHALL_MODE', 'AGENT_OF_EMPIRES_PROFILE', 'TMUX', 'TMUX_PANE']) delete env[key];
async function finite(cmd) {
  const p = Bun.spawn({ cmd, env, stdout: 'pipe', stderr: 'pipe' });
  const timer = setTimeout(() => p.kill('SIGKILL'), 30000);
  try {
    const [stdout, stderr, exit] = await Promise.all([new Response(p.stdout).arrayBuffer(), new Response(p.stderr).text(), p.exited]);
    return { bytes: Buffer.from(stdout), stderr, exit };
  } finally { clearTimeout(timer); }
}
async function waitForSocket(path) {
  if (existsSync(path)) return;
  await new Promise((resolve, reject) => {
    const observer = watch(dirname(path), () => {
      if (existsSync(path)) { clearTimeout(deadline); observer.close(); resolve(); }
    });
    const deadline = setTimeout(() => { observer.close(); reject(new Error('Read publication socket not observed')); }, 30000);
    if (existsSync(path)) { clearTimeout(deadline); observer.close(); resolve(); }
  });
}
function frames(bytes) {
  const found = [];
  let offset = 0;
  let text = Buffer.alloc(0);
  while (offset < bytes.length) {
    if (bytes.length - offset < 2) throw new Error('Truncated websocket frame');
    const first = bytes[offset++], second = bytes[offset++];
    const fin = Boolean(first & 0x80), opcode = first & 0x0f;
    let length = second & 0x7f;
    if (length === 126) { length = bytes.readUInt16BE(offset); offset += 2; }
    else if (length === 127) { length = Number(bytes.readBigUInt64BE(offset)); offset += 8; }
    const masked = Boolean(second & 0x80);
    const mask = masked ? bytes.subarray(offset, offset + 4) : undefined;
    if (masked) offset += 4;
    if (offset + length > bytes.length) throw new Error('Truncated websocket payload');
    const payload = Buffer.from(bytes.subarray(offset, offset + length));
    offset += length;
    if (mask) for (let i = 0; i < payload.length; i++) payload[i] ^= mask[i % 4];
    if (opcode === 1) text = payload;
    else if (opcode === 0) text = Buffer.concat([text, payload]);
    if (fin && (opcode === 1 || opcode === 0)) { found.push(JSON.parse(text.toString())); text = Buffer.alloc(0); }
  }
  return found;
}
let daemon;
let logs = '';
let logReader;
try {
  const init = await finite([binary, 'profile', 'create', 'protocol-interop']);
  if (init.exit !== 0) throw new Error(`Profile preparation failed: ${init.stderr} ${init.bytes.toString()}`);
  let readyResolve, readyReject;
  const ready = new Promise((resolve, reject) => { readyResolve = resolve; readyReject = reject; });
  daemon = Bun.spawn({ cmd: [binary, 'serve', '--core-only'], env, stdout: 'pipe', stderr: 'pipe' });
  logReader = Promise.all([daemon.stdout, daemon.stderr].map(async stream => {
    for await (const chunk of stream) {
      logs += Buffer.from(chunk).toString();
      const match = logs.match(/aoe core daemon listening on ([^\r\n]+)/);
      if (match) readyResolve(match[1]);
    }
  }));
  const startupDeadline = setTimeout(() => readyReject(new Error('No observable Core readiness within 30 seconds')), 30000);
  const socket = await Promise.race([ready, daemon.exited.then(code => { throw new Error(`Core exited before readiness (${code}): ${logs}`); })]);
  clearTimeout(startupDeadline);
  const readSocket = join(dirname(dirname(socket)), 'runtime.sock');
  await waitForSocket(readSocket);
  const exchange = await finite(['curl', '--silent', '--show-error', '--http1.1', '--include', '--unix-socket', readSocket,
    '--header', 'Connection: Upgrade', '--header', 'Upgrade: websocket',
    '--header', 'Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==', '--header', 'Sec-WebSocket-Version: 13',
    'http://localhost/api/runtime/read/ws']);
  const split = exchange.bytes.indexOf('\r\n\r\n');
  if (split < 0 || !exchange.bytes.subarray(0, split).toString().startsWith('HTTP/1.1 101')) throw new Error(`No real websocket upgrade: ${exchange.bytes.toString()} ${exchange.stderr}`);
  const actual = frames(exchange.bytes.subarray(split + 4));
  const hello = actual.find(frame => frame.kind === 'hello');
  const snapshot = actual.find(frame => frame.kind === 'snapshot');
  if (!hello || !snapshot || hello.data.protocol_version !== 4 || !hello.data.local_owner) throw new Error(`Expected real local-owner protocol4 exchange: ${JSON.stringify(hello)}`);
  const fieldPresent = Object.hasOwn(snapshot.data, 'systemd_scope_namespace');
  if (fieldPresent !== expectedField) throw new Error(`Unexpected namespace property: present=${fieldPresent}, expected=${expectedField}`);
  if (fieldPresent && !/^[0-9a-f]{64}$/.test(snapshot.data.systemd_scope_namespace)) throw new Error('Invalid namespace payload');
  writeFileSync(join(base, `${label}-hello.json`), JSON.stringify(hello));
  writeFileSync(join(base, `${label}-snapshot.json`), JSON.stringify(snapshot));
  writeFileSync(join(base, `${label}-exchange.raw`), exchange.bytes);
  const oldRecords = readFileSync(resolve('tests/fixtures/cli-read/cases/uds-list-nominal/wire.raw'));
  let pos = 0;
  const controlFrames = [];
  while (pos < oldRecords.length) {
    const direction = oldRecords[pos];
    const length = oldRecords.readUInt32BE(pos + 2);
    const payload = oldRecords.subarray(pos + 6, pos + 6 + length);
    pos += 6 + length;
    if (direction === 2 && !payload.subarray(0, 5).equals(Buffer.from('HTTP/'))) controlFrames.push(...frames(payload));
  }
  const control = controlFrames.find(frame => frame.kind === 'snapshot');
  if (!control) throw new Error('Published control pack contains no snapshot');
  writeFileSync(join(base, 'published-control-snapshot.json'), JSON.stringify(control));
  const proof = { label, binary, binarySha256: createHash('sha256').update(readFileSync(binary)).digest('hex'), home,
    transport: 'real Unix websocket at runtime.sock, /api/runtime/read/ws',
    protocol: hello.data.protocol_version, localOwner: hello.data.local_owner, namespacePropertyPresent: fieldPresent,
    snapshotSha256: createHash('sha256').update(JSON.stringify(snapshot)).digest('hex'),
    curlExit: exchange.exit, applicationFrames: actual.map(frame => frame.kind),
    scope: 'Production Core and serializer. Private XDG visibility sentinel exercises scopes_expected; no physical systemd service is claimed.' };
  writeFileSync(join(base, `${label}-capture-proof.json`), JSON.stringify(proof, null, 2));
  console.log(JSON.stringify(proof, null, 2));
} finally {
  if (daemon && daemon.exitCode === null) {
    daemon.kill('SIGINT');
    let killed = false;
    const stopDeadline = setTimeout(() => { killed = true; daemon.kill('SIGKILL'); }, 10000);
    const exit = await daemon.exited;
    clearTimeout(stopDeadline);
    await logReader;
    if (killed) throw new Error('Owned Core did not finish its shutdown');
    console.log(JSON.stringify({ cleanup: 'owned Core stopped', exit }));
  }
  writeFileSync(join(base, `${label}-daemon.log`), logs);
}
