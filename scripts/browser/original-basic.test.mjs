import { test } from 'node:test';
import assert from 'node:assert/strict';
import { cpSync, mkdtempSync, readFileSync, rmSync, symlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { verifyOriginalBasic } from './original-basic.mjs';

test('the compiled original BASIC identity matches every original source input', () => {
  const root = resolve(import.meta.dirname, '../..');
  const manifest = JSON.parse(readFileSync(join(root, 'families/dragon/firmware/basic/manifest.json'), 'utf8'));
  assert.deepEqual(verifyOriginalBasic(root), manifest);
  const temporary = mkdtempSync(join(tmpdir(), 'eightbit-basic-receipt-'));
  const firmware = join(temporary, 'families/dragon/firmware/basic');
  try {
    cpSync(join(root, 'families/dragon/firmware/basic'), firmware, { recursive: true });
    cpSync(join(root, 'families/dragon/dialects/dragon32-basic'), join(temporary, 'families/dragon/dialects/dragon32-basic'), { recursive: true });
    for (const name of ['break.inc', 'numeric.inc', 'graphics-sprite.inc', 'tape-save.inc', 'syntax-open.inc', 'errors.inc', 'errors-data.inc']) {
      writeFileSync(join(firmware, name), `${readFileSync(join(firmware, name), 'utf8')}\n; changed source\n`);
      assert.throws(() => verifyOriginalBasic(temporary), /receipt mismatch/);
      cpSync(join(root, 'families/dragon/firmware/basic', name), join(firmware, name));
    }
    const image = readFileSync(join(firmware, 'basic.rom')); image[0] ^= 1;
    writeFileSync(join(firmware, 'basic.rom'), image);
    assert.throws(() => verifyOriginalBasic(temporary), /receipt mismatch/);
    cpSync(join(root, 'families/dragon/firmware/basic/basic.rom'), join(firmware, 'basic.rom'));
    writeFileSync(join(firmware, 'basic.sym'), `${readFileSync(join(firmware, 'basic.sym'), 'utf8')}\nCHANGED EQU $8000\n`);
    assert.throws(() => verifyOriginalBasic(temporary), /receipt mismatch/);
  } finally { rmSync(temporary, { recursive: true, force: true }); }
});

// Generated record fixtures exercise admission mechanics; they do not qualify the bundled image.
test('qualified identity requires the exact reviewed image, symbols and complete source receipt', () => {
  const root = resolve(import.meta.dirname, '../..');
  const temporary = mkdtempSync(join(tmpdir(), 'eightbit-basic-qualification-'));
  const firmware = join(temporary, 'families/dragon/firmware/basic');
  try {
    cpSync(join(root, 'families/dragon/firmware/basic'), firmware, { recursive: true });
    cpSync(join(root, 'families/dragon/dialects/dragon32-basic'), join(temporary, 'families/dragon/dialects/dragon32-basic'), { recursive: true });
    const receipt = { ...verifyOriginalBasic(temporary), id: 'eightbit-dragon32-basic-1', status: 'qualified' };
    rmSync(join(firmware, 'qualification.json'), { force: true });
    writeFileSync(join(firmware, 'manifest.json'), JSON.stringify(receipt));
    assert.throws(() => verifyOriginalBasic(temporary), /qualification is required/);
    const { layout: _layout, ...qualification } = receipt;
    const record = join(firmware, 'qualification.json');
    writeFileSync(record, JSON.stringify(qualification)); assert.equal(verifyOriginalBasic(temporary).status, 'qualified');
    for (const changes of [{ romSha256: 'd'.repeat(64) }, { symbolsSha256: 'd'.repeat(64) }, { sourceSha256: {} }, { extra: true }, { id: 'other' }, { bytes: 20292 }]) {
      writeFileSync(record, JSON.stringify({ ...qualification, ...changes })); assert.throws(() => verifyOriginalBasic(temporary), /exact build/);
    }
    writeFileSync(record, ' '.repeat(65537)); assert.throws(() => verifyOriginalBasic(temporary), /bounded regular/);
    rmSync(record); symlinkSync(join(firmware, 'missing'), record); assert.throws(() => verifyOriginalBasic(temporary), /bounded regular/);
  } finally { rmSync(temporary, { recursive: true, force: true }); }
});
