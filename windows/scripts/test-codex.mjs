// Behavior tests run without Tauri, networking, API keys, or real approvals.
import { build } from 'esbuild';
import { mkdtemp, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';
import assert from 'node:assert/strict';

const dir = await mkdtemp(join(tmpdir(), 'coucou-codex-'));
try {
  const result = await build({
    stdin: { contents: `export { State } from './src/core/state.ts'; export { handleHook } from './src/island/hooks.ts'; export { Bridge } from './src/core/bridge.ts';`, resolveDir: process.cwd(), loader: 'ts' },
    bundle: true, format: 'esm', platform: 'node', write: false,
  });
  const file = join(dir, 'test-bundle.mjs');
  await writeFile(file, result.outputFiles[0].text);
  const timers = new Map(); let counter = 0;
  globalThis.window = { setTimeout: (fn) => { const id = ++counter; timers.set(id, fn); return id; }, clearTimeout: (id) => timers.delete(id) };
  const { State, handleHook, Bridge } = await import(pathToFileURL(file).href);
  const calls = [];
  Bridge.approvalAck = async id => calls.push(['ack', id]);
  Bridge.approvalDecline = async id => calls.push(['decline', id]);
  Bridge.approvalDecision = async (...args) => calls.push(['decision', ...args]);
  const island = { alert: v => { State.mode = 'expanded'; State.view = v; calls.push(['alert', v]); }, setView: v => { State.view = v; }, reveal() {}, dropPin() {} };
  State.loadIntegrationTasks();
  const event = (provider, sid, name, extra = {}) => handleHook(island, { provider, session_id: sid, cwd: `D:/projects/${sid}`, hook_event_name: name, ...extra });
  event('claude', 'claude-1', 'UserPromptSubmit', { prompt: 'Claude task' });
  event('codex', 'codex-1', 'UserPromptSubmit', { prompt: 'Codex task', turn_id: 'turn-1' });
  event('codex', 'codex-2', 'PreToolUse', { tool_name: 'apply_patch', tool_input: { command: 'patch two' } });
  const first = State.tasks.find(t => t.sessionId === 'codex-1');
  const second = State.tasks.find(t => t.sessionId === 'codex-2');
  assert.notEqual(first.id, second.id);
  assert.deepEqual(first.steps, ['Codex task']);
  assert(second.steps[0].includes('patch two'));
  assert.deepEqual(State.tasks.find(t => t.sessionId === 'claude-1').steps, ['Claude task']);
  console.log('PASS: Claude and simultaneous Codex sessions remain isolated');
  first.chatTitle = 'Coucou Codex Support';

  event('codex', 'codex-1', 'PreToolUse', {
    tool_name: 'Edit', tool_input: { file_path: 'invoice.ts', old_string: 'const TVA = 0.19;\n', new_string: 'const TVA = 0;\n' },
  });
  assert(first.activity.at(-1).detail.includes('invoice.ts'));
  assert.equal(first.name, 'Coucou Codex Support');
  assert(first.activity.at(-1).detail.includes('const TVA = 0.19;\n'));
  assert(first.activity.at(-1).detail.includes('const TVA = 0;\n'));
  assert(second.activity.at(-1).detail.includes('patch two'));

  event('codex', 'codex-1', 'Stop', { turn_id: 'turn-1', message: 'Finished one' });
  assert.equal(first.state, 'finished');
  event('codex', 'codex-1', 'UserPromptSubmit', { turn_id: 'turn-2', prompt: 'New task' });
  assert.deepEqual(first.activity, []);
  assert(second.activity.at(-1).detail.includes('patch two'));
  console.log('PASS: multiline activity fields survive and new turns do not clear another session');
  for (const fn of [...timers.values()]) fn();
  assert.equal(first.state, 'thinking');
  event('codex', 'codex-1', 'Stop', { turn_id: 'turn-1' });
  assert.equal(first.state, 'thinking');
  console.log('PASS: old completion events/timers cannot finish a newer turn');

  event('codex', 'codex-2', 'PermissionRequest', { request_id: 'request-1', tool_name: 'Bash', tool_input: { command: 'echo synthetic test' } });
  assert.equal(State.focusId, second.id);
  assert.equal(State.pendingApproval.taskId, second.id);
  assert.equal(State.view, 'approval');
  assert.deepEqual(calls.slice(-2), [['alert', 'approval'], ['ack', 'request-1']]);
  event('claude', 'claude-1', 'PermissionRequest', { request_id: 'request-2' });
  assert.equal(State.pendingApproval.requestId, 'request-1');
  assert(calls.some(c => c[0] === 'decline' && c[1] === 'request-2'));
  assert(!calls.some(c => c[0] === 'decision'));
  console.log('PASS: approval belongs to the displayed session; overlap falls back; no automatic decision');

  event('codex', 'codex-2', 'Interrupt');
  assert.equal(State.pendingApproval, null);
  assert.equal(second.state, 'idle');
  State.paused = true;
  event('codex', 'codex-1', 'PermissionRequest', { request_id: 'paused' });
  assert(calls.some(c => c[0] === 'decline' && c[1] === 'paused'));
  State.paused = false;
  event('codex', 'codex-2', 'SessionEnd');
  assert(!State.tasks.some(t => t.sessionId === 'codex-2'));
  assert(State.tasks.some(t => t.sessionId === 'codex-1'));
  console.log('PASS: interrupt, pause, and session end preserve other sessions');
} finally { await rm(dir, { recursive: true, force: true }); }
