import { test } from 'node:test';
import assert from 'node:assert/strict';
import { once } from 'node:events';
import { spawnOwnedPeer } from './original-runtime-process.mjs';

test('direct peer closes after EOF and repeated stop awaits the same owner', { timeout: 5_000 }, async () => {
  const owner = spawnOwnedPeer(process.execPath, ['-e', 'process.stdin.resume(); process.stdout.write("ready");'], {}, 500);
  try {
    await once(owner.child.stdout, 'data');
    const stopping = owner.stop();
    assert.equal(owner.stop(), stopping);
    const result = await stopping;
    assert.equal(result.code, 0);
    assert.equal(result.error, undefined);
    assert.equal(await owner.closed, result);
  } finally { await owner.stop(); }
});

test('non-cooperative timed-out peer is killed and reaped before recovery', { timeout: 5_000 }, async () => {
  const owner = spawnOwnedPeer(process.execPath, ['-e', 'process.on("SIGTERM", () => {}); process.stdin.resume(); setInterval(() => {}, 1000); process.stdout.write("ready");'], {}, 100);
  try {
    await once(owner.child.stdout, 'data');
    const result = await owner.stop();
    assert.ok(result.signal === 'SIGKILL' || (process.platform === 'win32' && result.signal === 'SIGTERM'));
    assert.throws(() => process.kill(owner.child.pid, 0), { code: 'ESRCH' });
  } finally { await owner.stop(); }
  const fresh = spawnOwnedPeer(process.execPath, ['-e', 'process.stdin.resume(); process.stdout.write("ready");'], {}, 500);
  try {
    await once(fresh.child.stdout, 'data');
    assert.equal((await fresh.stop()).code, 0);
  } finally { await fresh.stop(); }
});

test('spawn failure closes the owner without hanging teardown', { timeout: 5_000 }, async () => {
  const owner = spawnOwnedPeer('/eightbit-no-such-executable', [], {}, 100);
  assert.equal((await owner.closed).error.code, 'ENOENT');
  assert.equal((await owner.stop()).error.code, 'ENOENT');
});
