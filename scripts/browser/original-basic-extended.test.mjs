import { test } from 'node:test';
import assert from 'node:assert/strict';
import { cpSync, mkdirSync, mkdtempSync, readFileSync, renameSync, rmSync, symlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { originalBasicExtendedFiles, originalBasicExtendedSources, verifyOriginalBasicExtended } from './original-basic-extended.mjs';

const root = resolve(import.meta.dirname, '../..');
const firmware = 'families/dragon/firmware/basic-extended';
function fixture(run) {
  const receipt = verifyOriginalBasicExtended(root);
  const temporary = mkdtempSync(join(tmpdir(), 'eightbit-expanded-receipt-'));
  try {
    for (const path of [...originalBasicExtendedFiles, ...originalBasicExtendedSources(receipt), 'scripts/build-dragon-basic-extended.py', 'scripts/qualify-dragon-basic.py']) {
      mkdirSync(dirname(join(temporary, path)), { recursive: true }); cpSync(join(root, path), join(temporary, path));
    }
    run(temporary, receipt);
  } finally { rmSync(temporary, { recursive: true, force: true }); }
}
test('expanded source exception binds the actual image, source, symbols, map and builders', () => fixture((temporary, receipt) => {
  assert.deepEqual(verifyOriginalBasicExtended(temporary), receipt);
  for (const path of [`${firmware}/basic.rom`, `${firmware}/basic.sym`, ...originalBasicExtendedSources(receipt), 'scripts/build-dragon-basic-extended.py', 'scripts/qualify-dragon-basic.py']) {
    const original = readFileSync(join(temporary, path));
    const changed = Buffer.from(original); changed[0] ^= 1; writeFileSync(join(temporary, path), changed);
    assert.throws(() => verifyOriginalBasicExtended(temporary), /receipt mismatch/);
    writeFileSync(join(temporary, path), original);
  }
}));

test('expanded qualification requires the complete exact identity and cannot change the admitted memory map', () => fixture((temporary, receipt) => {
  const manifest = join(temporary, firmware, 'manifest.json');
  const record = join(temporary, firmware, 'qualification.json');
  const { layout: _layout, ...identity } = receipt;
  // These temporary generated records exercise admission mechanics; they do not qualify production.
  const qualified = { ...identity, status: 'qualified' };
  writeFileSync(record, JSON.stringify({ ...identity, status: 'development' }));
  writeFileSync(manifest, JSON.stringify({ ...receipt, status: 'qualified' }));
  assert.throws(() => verifyOriginalBasicExtended(temporary), /qualification does not match/);
  writeFileSync(record, JSON.stringify(qualified)); assert.equal(verifyOriginalBasicExtended(temporary).status, 'qualified');
  for (const change of [{ extra: true }, { status: 'development' }, { romSha256: 'a'.repeat(64) }, { builderSha256: {} }, { sourceSha256: {} }]) {
    writeFileSync(record, JSON.stringify({ ...qualified, ...change }));
    assert.throws(() => verifyOriginalBasicExtended(temporary), /qualification does not match/);
  }
  writeFileSync(record, JSON.stringify(qualified));
  for (const change of [{ id: 'eightbit-dragon32-basic-1' }, { bytes: 19811 }, { sourceRoot: '../basic' }, { assemblyDefines: [] }, { memoryMap: { ...receipt.memoryMap, cartridges: true } }, { sourceSha256: { ...receipt.sourceSha256, '../../outside': 'a'.repeat(64) } }]) {
    writeFileSync(manifest, JSON.stringify({ ...receipt, status: 'qualified', ...change }));
    assert.throws(() => verifyOriginalBasicExtended(temporary), /receipt mismatch/);
  }
}));

test('expanded receipt rejects missing, oversized, aliased and unexpected source inputs', () => fixture((temporary, receipt) => {
  const record = join(temporary, firmware, 'qualification.json');
  const original = readFileSync(record);
  writeFileSync(record, ' '.repeat(65537)); assert.throws(() => verifyOriginalBasicExtended(temporary), /bounded regular/);
  rmSync(record); assert.throws(() => verifyOriginalBasicExtended(temporary), /ENOENT/);
  symlinkSync(join(temporary, 'missing'), record); assert.throws(() => verifyOriginalBasicExtended(temporary), /bounded regular/);
  rmSync(record); writeFileSync(record, original);
  const extra = join(temporary, 'families/dragon/firmware/basic-complete/source/unreviewed.inc');
  writeFileSync(extra, '; authored extra'); assert.throws(() => verifyOriginalBasicExtended(temporary), /source receipt mismatch/); rmSync(extra);
  const path = join(temporary, originalBasicExtendedSources(receipt)[0]);
  const bytes = readFileSync(path); rmSync(path); symlinkSync(join(root, originalBasicExtendedSources(receipt)[0]), path);
  assert.throws(() => verifyOriginalBasicExtended(temporary), /bounded regular/); rmSync(path); writeFileSync(path, bytes);
  const directory = join(temporary, 'families/dragon/firmware/basic-complete/source');
  renameSync(directory, `${directory}-actual`); symlinkSync(`${directory}-actual`, directory);
  assert.throws(() => verifyOriginalBasicExtended(temporary), /regular directories/);
}));
