import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const root = fileURLToPath(new URL('../../', import.meta.url));
const args = ['run', '--release', '--locked', '--quiet', '-p', 'eightbit-dragon-emulator', '--example', 'cassette-audio-acceptance'];
const native = extra => JSON.parse(execFileSync('cargo', [...args, ...extra], { cwd: root, encoding: 'utf8', timeout: 120000, maxBuffer: 5 * 1024 * 1024 }));
const fixture = native(['--', '--fixture']);
assert.ok(Array.isArray(fixture.bytes) && fixture.bytes.length <= 1048576);
const generated = resolve(root, 'apps/eightbit-studio-web/ui/public/generated');
const module = await import(pathToFileURL(resolve(generated, 'dragon_core.js')).href);
await module.default({ module_or_path: readFileSync(resolve(generated, 'dragon_core_bg.wasm')) });
const runtime = module.MonitorRuntime.basicDevelopment(1);
const states = createHash('sha256');
const request = command => {
  const snapshot = JSON.parse(runtime.command(JSON.stringify({ schemaVersion: 1, sessionId: 1, command })));
  states.update(JSON.stringify(snapshot));
  return snapshot;
};
try {
  for (let n = 0; n < 8; n++) request({ operation: 'advance', cycles: 32768 });
  const mounted = request({ operation: 'mount-wav', bytes: fixture.bytes });
  assert.equal(mounted.cassette.sha256, fixture.cassetteSha256);
  request({ operation: 'type', text: 'CLOADM"WAVTEST":EXEC:PRINT PEEK(&H6301)' });
  for (let n = 0; n < 256; n++) request({ operation: 'advance', cycles: 32768 });
  const frame = request({ operation: 'rewind-cassette' });
  assert.ok(frame.text.split('\n').some(line => line.trim() === '165'), 'the guest must execute the authored programme');
  assert.ok(!frame.text.includes('ERROR'));
  assert.equal(frame.cassette.pending, true);
  const actual = { schemaVersion: 1, stateTraceSha256: states.digest('hex'), cycles: frame.cycles, pc: frame.pc, cassetteSha256: frame.cassette.sha256 };
  assert.deepEqual(actual, native([]));
  console.log(JSON.stringify({ ...actual, nativeWasmParity: true, scope: 'bounded WAV mount and guest CLOADM/EXEC; development firmware' }));
} finally {
  runtime.free();
}
