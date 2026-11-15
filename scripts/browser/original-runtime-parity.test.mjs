import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';
import { verifyOriginalBasicExtended } from './original-basic-extended.mjs';

const root = resolve(import.meta.dirname, '../..');
const script = resolve(import.meta.dirname, 'original-runtime-parity.mjs');
function rejected(args) {
  const result = spawnSync(process.execPath, [script, ...args], {
    cwd: root, encoding: 'utf8', timeout: 10_000, maxBuffer: 64 * 1024,
    // A preflight rejection must not depend on Cargo or execute a native peer.
    env: { ...process.env, PATH: '/eightbit-no-executables' },
  });
  assert.equal(result.error, undefined);
  assert.equal(result.status, 1);
  return result.stderr;
}
test('normal acceptance requires an explicit identity and a complete optional game pair', () => {
  for (const args of [[], ['--qualified'], ['--expect-development', 'extra'], ['--expect-qualified', '--games'], ['--expect-qualified', '--games', 'doom'], ['--expect-development', '--games', 'doom', 'battlezone']]) {
    assert.match(rejected(args), /Choose --expect-development or --expect-qualified/);
  }
});
test('opposite real qualification state fails before native or WASM startup', () => {
  const status = verifyOriginalBasicExtended(root).status;
  assert.ok(['development', 'qualified'].includes(status));
  const opposite = status === 'development' ? '--expect-qualified' : '--expect-development';
  assert.match(rejected([opposite]), /Requested acceptance requires the actual reviewed firmware identity/);
});
