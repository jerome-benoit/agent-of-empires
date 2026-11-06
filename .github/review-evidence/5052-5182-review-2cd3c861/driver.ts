import assert from 'node:assert/strict';
import { mkdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { createRequire } from 'node:module';
import { spawnAoeServe } from '/home/fraggle/src/agent-of-empires-git-worktrees/5052-5182-cursor-prefix-main431387c-20261105/web/tests/helpers/aoeServe.ts';
import { startAcpSession, replayFrames, script, endTurn, HOLD } from '/home/fraggle/src/agent-of-empires-git-worktrees/5052-5182-cursor-prefix-main431387c-20261105/web/tests/helpers/acp.ts';
const { chromium, expect } = createRequire('/home/fraggle/src/agent-of-empires-git-worktrees/5052-5182-cursor-prefix-main431387c-20261105/web/package.json')('@playwright/test');
const [binary, phase, output] = process.argv.slice(2);
assert(binary && ['before', 'after'].includes(phase) && output);
mkdirSync(output, { recursive: true });
const browser = await chromium.launch({ headless: true });
try {
  for (const promptId of [null, 'explicit-late-attach-turn']) {
    const label = `${phase}-${promptId ? 'explicit' : 'null'}`;
    process.env.AOE_E2E_BINARY = binary;
    let serve;
    const page = await browser.newPage({ viewport: { width: 1100, height: 800 } });
    const wire: any[] = [];
    let ordinal = 0;
    page.on('websocket', socket => {
      if (!socket.url().includes('/acp/ws')) return;
      const connection = ++ordinal;
      wire.push({ kind: 'open', connection, url: socket.url() });
      socket.on('framereceived', ({ payload }) => {
        const text = typeof payload === 'string' ? payload : payload.toString('utf8');
        try { wire.push({ connection, message: JSON.parse(text) }); }
        catch { wire.push({ connection, unparsed: text }); }
      });
    });
    try {
      const started = await startAcpSession(async options => {
        serve = await spawnAoeServe({ ...options, workerIndex: 0, parallelIndex: 36 });
        return serve;
      }, { title: `5181 ${label}`, fakeAcpScript: script(endTurn({ sessionUpdate: 'tool_call', toolCallId: 'late-attach-tool', title: 'LATE_ATTACH_NATIVE_TOOL', kind: 'read', status: 'pending' }, HOLD)) });
      const id = started.sessionId;
      const prompt = await fetch(`${serve.baseUrl}/api/sessions/${id}/acp/prompt`, {
        method: 'POST', headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ text: 'EXPLICIT_LATE_ATTACH_INPUT', ...(promptId ? { prompt_id: promptId } : {}) }),
      });
      assert(prompt.ok, `${label}: prompt ${prompt.status} ${await prompt.text()}`);
      await expect.poll(async () => JSON.stringify(await replayFrames(serve.baseUrl, id)), { timeout: 30000 }).toContain('LATE_ATTACH_NATIVE_TOOL');
      const replayBefore = await replayFrames(serve.baseUrl, id);
      await page.goto(`${serve.baseUrl}/session/${id}`);
      const card = page.getByRole('button', { name: 'LATE_ATTACH_NATIVE_TOOL', exact: true }).locator('xpath=ancestor::div[contains(@class,"rounded-md")][1]');
      await expect(card.getByText('running', { exact: true })).toBeVisible({ timeout: 15000 });
      await expect.poll(() => wire.some(frame => frame.message?.kind === 'transcript_snapshot'), { timeout: 15000 }).toBe(true);
      const initialWire = structuredClone(wire);
      const open = initialWire.find(frame => frame.kind === 'open');
      assert(open && Number(new URL(open.url).searchParams.get('since')) > 0, 'browser must attach after the tool start');
      const bootstrap = initialWire.find(frame => frame.message?.kind === 'transcript_snapshot');
      assert(bootstrap, 'cursor bootstrap must arrive before cancellation');
      assert.equal(bootstrap.message.rows.length, 0, 'cursor bootstrap must not recreate prefix rows');
      await page.screenshot({ path: join(output, `${label}-running.png`) });
      await page.getByRole('button', { name: 'Stop', exact: true }).click({ timeout: 15000 });
      await expect(page.getByRole('textbox', { name: /^Send a message/ })).toBeVisible({ timeout: 15000 });
      await expect.poll(async () => (await replayFrames(serve.baseUrl, id)).filter(frame => frame.event?.Stopped?.reason === 'cancelled').length, { timeout: 15000 }).toBe(1);
      const replayAfter = await replayFrames(serve.baseUrl, id);
      const terminalSeq = replayAfter.find(frame => frame.event?.Stopped?.reason === 'cancelled').seq;
      await expect.poll(() => wire.some(frame => frame.message?.kind === 'reduced_state' && frame.message.seq >= terminalSeq), { timeout: 15000 }).toBe(true);
      const liveStopped = () => wire.filter(frame => frame.message?.kind === 'transcript_delta').map(frame => frame.message.delta?.Append).filter(row => row?.kind === 'tool_stopped' && row?.tool_call_id === 'late-attach-tool');
      if (phase === 'after') {
        await expect(card.getByText('stopped', { exact: true })).toBeVisible({ timeout: 15000 });
        await expect(card.getByText('running', { exact: true })).toBeHidden();
        assert.equal(liveStopped().length, 1, 'one correlated live terminal');
      } else {
        const terminalIndex = wire.findIndex(frame => frame.message?.kind === 'reduced_state' && frame.message.seq >= terminalSeq);
        await expect.poll(() => wire.slice(terminalIndex + 1).some(frame => frame.message?.kind === 'heartbeat'), { timeout: 35000 }).toBe(true);
        await expect(card.getByText('running', { exact: true })).toBeVisible();
        assert.equal(liveStopped().length, 0, 'baseline lacks the correlated live terminal');
      }
      await page.screenshot({ path: join(output, `${label}-cancelled.png`) });
      const beforeReload = structuredClone(wire);
      await page.reload();
      await expect(card.getByText('stopped', { exact: true })).toBeVisible({ timeout: 15000 });
      await expect(card.getByText('running', { exact: true })).toBeHidden();
      const nativeLog = readFileSync(join(serve.home, 'fake-acp.log'), 'utf8');
      assert.equal([...nativeLog.matchAll(/session\/cancel sessionId=/g)].length, 1);
      console.log('LIFECYCLE_5181_RESULT', JSON.stringify({ binary, label, promptId, id, terminalSeq, initialWire, beforeReload, replayBefore, replayAfter, stoppedBeforeReload: phase === 'after', stoppedAfterReload: true, nativeCancel: 1, nativeLog }));
    } finally {
      await page.close();
      if (serve) await serve.stop();
      console.log('LIFECYCLE_5181_CLEANUP_GREEN', label);
    }
  }
} finally { await browser.close(); }
