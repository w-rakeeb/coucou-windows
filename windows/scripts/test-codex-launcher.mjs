// Check the installed hook command under both Windows shells, including a path
// containing spaces. Only Coucou's SessionEnd relay is run; no model is called.
import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';
import { copyFile, mkdtemp, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawn } from 'node:child_process';

if (process.platform !== 'win32') throw new Error('This launcher check requires Windows.');
const configDir = process.env.CODEX_HOME || join(process.env.USERPROFILE, '.codex');
const settings = JSON.parse(await readFile(join(configDir, 'hooks.json'), 'utf8'));
const handlers = (settings.hooks?.SessionEnd || []).flatMap(group => group.hooks || []);
const installed = handlers.map(handler => handler.command).find(command => /^cmd\.exe \/d \/c call "[^"\r\n]+\/coucou-hook\.exe" --codex SessionEnd$/.test(command));
assert(installed, 'Install the current Coucou Codex hooks before checking the launcher.');
const originalPath = installed.match(/"([^"]+)"/)[1];
const scratch = await mkdtemp(join(tmpdir(), "coucou launcher's test-"));
try {
  const helper = join(scratch, 'coucou-hook.exe');
  await copyFile(originalPath, helper);
  const command = installed.replace(originalPath, helper.replaceAll('\\', '/'));
  for (const shell of ['powershell', 'cmd']) {
    const executable = shell === 'powershell'
      ? join(process.env.SystemRoot, 'System32', 'WindowsPowerShell', 'v1.0', 'powershell.exe')
      : process.env.ComSpec;
    const args = shell === 'powershell'
      ? ['-NoProfile', '-NonInteractive', '-Command', command]
      : ['/C', `"${command}"`];
    const child = spawn(executable, args, { windowsHide: true, windowsVerbatimArguments: shell === 'cmd', stdio: ['pipe', 'pipe', 'pipe'] });
    let output = '', errors = '';
    child.stdout.on('data', bytes => { output += bytes; });
    child.stderr.on('data', bytes => { errors += bytes; });
    child.stdin.end(JSON.stringify({ session_id: `coucou-launcher-test-${randomUUID()}`, hook_event_name: 'SessionEnd', cwd: scratch }));
    const deadline = setTimeout(() => child.kill(), 6000);
    const code = await new Promise((resolve, reject) => { child.once('error', reject); child.once('close', resolve); });
    clearTimeout(deadline);
    assert.equal(code, 0, `${shell}: ${errors}`);
    assert.equal(output, '', 'SessionEnd must not inject context into Codex.');
    console.log(`${shell}: installed command launches the relay from a path containing spaces and an apostrophe`);
  }
} finally {
  assert(scratch.startsWith(tmpdir()), 'Only remove this test\'s temporary directory.');
  await rm(scratch, { recursive: true, force: true });
}
