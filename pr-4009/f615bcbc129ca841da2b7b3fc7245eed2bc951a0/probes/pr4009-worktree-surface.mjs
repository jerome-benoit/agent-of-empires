import { spawnAoeServe } from '../web/tests/helpers/aoeServe.ts';
import { initWorkingRepo, writeFiles, commitAll, gitEnv } from '../web/tests/helpers/gitFixture.ts';
import { command, http } from './pr4009-live-smoke.mjs';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { createHash, randomUUID } from 'node:crypto';
import assert from 'node:assert/strict';
let repo, existing;
const handle = await spawnAoeServe({ workerIndex: 97, parallelIndex: 97, authMode: 'none', seedFn: async ({ home, env }) => {
  repo = join(home, 'worktree-repository');
  existing = join(home, 'pre-existing-checkout');
  initWorkingRepo(repo, env);
  writeFiles(repo, { 'tracked.txt': 'existing checkout must remain intact\n' });
  commitAll(repo, 'seed tracked fixture', env);
  await command(['git', '-C', repo, 'worktree', 'add', '-b', 'feature/already-checked-out', existing], gitEnv(env));
} });
let stopping = false;
async function stop() { if (stopping) return; stopping = true; await handle.stop(); process.exit(0); }
process.on('SIGINT', stop); process.on('SIGTERM', stop);
try {
  const socket = join(handle.env.XDG_CONFIG_HOME, 'agent-of-empires-dev/daemon/api.sock');
  const git = async args => (await command(['git', '-C', repo, ...args], gitEnv(handle.env))).stdout;
  const branchesBefore = await git(['branch', '--format=%(refname)']);
  const worktreesBefore = await git(['worktree', 'list', '--porcelain']);
  const trackedBefore = readFileSync(join(existing, 'tracked.txt'), 'utf8');
  const runtime = await http(handle.env, socket, '/api/runtime');
  assert.equal(runtime.status, 200);
  const listed = await http(handle.env, socket, `/api/git/worktrees?path=${encodeURIComponent(repo)}`);
  assert.equal(listed.status, 200, listed.text);
  assert(listed.json.some(row => row.branch === 'feature/already-checked-out' && row.path === existing));
  const created = await http(handle.env, socket, '/api/sessions', { path: repo, title: 'Native existing worktree smoke', tool: 'claude', worktree_enabled: true, worktree_branch: 'feature/already-checked-out', create_new_branch: false, trust_hooks: false, idempotency_key: randomUUID(), view: 'terminal' }, { 'aoe-runtime-epoch': runtime.json.epoch });
  assert.equal(created.status, 201, created.text);
  assert.equal(created.json.project_path, existing);
  assert.equal(await git(['branch', '--format=%(refname)']), branchesBefore);
  assert.equal(await git(['worktree', 'list', '--porcelain']), worktreesBefore);
  assert.equal(readFileSync(join(existing, 'tracked.txt'), 'utf8'), trackedBefore);
  const proof = { head: 'f615bcbc129ca841da2b7b3fc7245eed2bc951a0', binarySha256: createHash('sha256').update(readFileSync('target/debug/aoe')).digest('hex'), transport: 'actual owner Unix HTTP and real tmux-backed Daemon create', listedExisting: true, branch: 'feature/already-checked-out', projectPath: created.json.project_path, canonicalCreateStatus: created.status, worktreeInventoryUnchanged: true, branchesUnchanged: true, trackedFileUnchanged: true };
  await Bun.write('target/protocol4-interop/worktree-live-proof.json', JSON.stringify(proof, null, 2));
  await Bun.write('target/protocol4-interop/worktree-live-fixture.json', JSON.stringify({ url: handle.baseUrl, repo, existing, id: created.json.id }));
  console.log(JSON.stringify(proof));
  console.log(`PR4009_WORKTREE_READY ${handle.baseUrl}`);
  await new Promise(() => {});
} catch (error) { await handle.stop(); throw error; }
