import { test } from 'node:test';
import assert from 'node:assert/strict';
import { dirname, resolve, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { cpSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { verifyOriginalMonitor } from './original-monitor.mjs';
const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
test('only the checked original monitor has a matching source/binary receipt', () => {
  assert.equal(verifyOriginalMonitor(root).bytes, 16384);
  const temporary = mkdtempSync(join(tmpdir(), 'eightbit-monitor-source-'));
  try {
    const destination = join(temporary, 'families/dragon/firmware/monitor');
    cpSync(join(root, 'families/dragon/firmware/monitor'), destination, { recursive: true });
    const image = readFileSync(join(destination, 'monitor.rom')); image[0] ^= 1;
    writeFileSync(join(destination, 'monitor.rom'), image);
    assert.throws(() => verifyOriginalMonitor(temporary), /mismatch/);
  } finally { rmSync(temporary, { recursive: true }); }
});
