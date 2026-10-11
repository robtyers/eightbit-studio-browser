import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdtempSync, mkdirSync, readFileSync, readdirSync, rmSync, symlinkSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { test } from 'node:test';
import { verifyBrowserAssets } from './verify-assets.mjs';
import { configuredSourceRelease } from './source-release.mjs';
const hash = (text) => createHash('sha256').update(text).digest('hex');
function fixture(run) {
  const directory = mkdtempSync(join(tmpdir(), 'eightbit-browser-assets-'));
  try {
    mkdirSync(join(directory, 'generated')); mkdirSync(join(directory, 'assets'));
    for (const name of ['index.html', 'runtime.html', 'runtime.js', 'runtime.css', 'classic.html', 'classic.js', 'generated/sources.html']) writeFileSync(join(directory, name), 'original');
    const binaries = {};
    for (const name of ['dragon_core.js', 'dragon_core_bg.wasm', 'xroar.js', 'xroar.wasm', 'c64.js', 'c64.wasm', 'mame-z80.js', 'mame-z80.wasm.gz', 'COPYING.txt', 'NOTICE.txt', 'browser-source.part01']) { writeFileSync(join(directory, 'generated', name), 'fixture'); binaries[name] = hash('fixture'); }
    const parts = {};
    for (let index = 1; index <= 9; index++) { const name = `mame-source.part${String(index).padStart(2, '0')}`; writeFileSync(join(directory, 'generated', name), 'fixture'); parts[name] = hash('fixture'); }
    const receipt = { schemaVersion: 1, binaries, externalSources: { mameArchiveSha256: hash('fixture'.repeat(9)), parts }, inputs: { 'runtime.js': hash('original') }, sourceArchiveSha256: hash('fixture'), sourceArchiveParts: { 'browser-source.part01': hash('fixture') }, sourceManifestSha256: hash('manifest') };
    writeFileSync(join(directory, 'generated/build-receipt.json'), JSON.stringify(receipt));
    run(directory);
  } finally { rmSync(directory, { recursive: true, force: true }); }
}
test('browser staging accepts only matching source, binaries and notices', () => fixture((directory) => {
  assert.equal(verifyBrowserAssets(directory, directory).schemaVersion, 1);
  writeFileSync(join(directory, 'runtime.js'), 'changed');
  assert.throws(() => verifyBrowserAssets(directory, directory), /Source changed/);
}));
test('browser staging refuses corrupted source, runtime or symlink assets', () => fixture((directory) => {
  writeFileSync(join(directory, 'generated/browser-source.part01'), 'corrupt');
  assert.throws(() => verifyBrowserAssets(directory), /corresponding source mismatch/);
  writeFileSync(join(directory, 'generated/browser-source.part01'), 'fixture');
  writeFileSync(join(directory, 'generated/xroar.wasm'), 'corrupt');
  assert.throws(() => verifyBrowserAssets(directory), /binary\/source receipt mismatch/);
  rmSync(join(directory, 'runtime.js')); symlinkSync(join(directory, 'runtime.css'), join(directory, 'runtime.js'));
  assert.throws(() => verifyBrowserAssets(directory), /symlinks/);
}));
test('browser staging refuses disposable runtime harnesses and private additions', () => fixture((directory) => {
  writeFileSync(join(directory, 'runtime-smoke.html'), 'disposable');
  assert.throws(() => verifyBrowserAssets(directory), /Unexpected browser asset/);
}));
test('browser staging refuses declarations rejected by the combined website guard', () => fixture((directory) => {
  writeFileSync(join(directory, 'generated/dragon_core.d.ts'), 'export declare function browser_request(): string;');
  assert.throws(() => verifyBrowserAssets(directory), /Unexpected generated browser asset/);
}));

test('browser staging refuses an altered emulator source part', () => fixture((directory) => {
  writeFileSync(join(directory, 'generated/mame-source.part04'), 'corrupt');
  assert.throws(() => verifyBrowserAssets(directory), /source part mismatch/);
}));

test('BBC assets require both matching binaries in the corresponding-source receipt', () => fixture((directory) => {
  const generated = join(directory, 'generated');
  writeFileSync(join(generated, 'mame-bbc.js'), 'bbc fixture');
  assert.throws(() => verifyBrowserAssets(directory), /BBC binary\/source receipt mismatch/);
  const receipt = JSON.parse(readFileSync(join(generated, 'build-receipt.json'), 'utf8'));
  for (const name of ['mame-bbc.js', 'mame-bbc.wasm.gz']) {
    writeFileSync(join(generated, name), 'bbc fixture'); receipt.binaries[name] = hash('bbc fixture');
  }
  writeFileSync(join(generated, 'build-receipt.json'), JSON.stringify(receipt));
  assert.equal(verifyBrowserAssets(directory).schemaVersion, 1);
  writeFileSync(join(generated, 'mame-bbc.wasm.gz'), 'corrupt');
  assert.throws(() => verifyBrowserAssets(directory), /BBC binary\/source receipt mismatch/);
}));

test('GitHub source receipts verify local release archives and refuse Cloudflare source parts', () => fixture((directory) => {
  const generated = join(directory, 'generated');
  const release = configuredSourceRelease();
  const receipt = JSON.parse(readFileSync(join(generated, 'build-receipt.json'), 'utf8'));
  receipt.schemaVersion = 2; receipt.sourceRelease = { repository: release.repository, tag: release.tag, url: release.url };
  delete receipt.sourceArchiveParts; delete receipt.externalSources.parts;
  for (const name of readdirSync(generated)) if (name.includes('.part')) rmSync(join(generated, name));
  writeFileSync(join(generated, 'build-receipt.json'), JSON.stringify(receipt));
  writeFileSync(join(generated, 'sources.html'), `<a href="${release.url}">Source on GitHub</a>`);
  const archives = mkdtempSync(join(tmpdir(), 'eightbit-github-archives-'));
  try {
    writeFileSync(join(archives, 'browser-source.tar.gz'), 'fixture');
    writeFileSync(join(archives, 'mame0289.tar.gz'), 'fixture'.repeat(9));
    assert.equal(verifyBrowserAssets(directory, directory, archives).schemaVersion, 2);
    writeFileSync(join(generated, 'browser-source.part01'), 'fixture');
    assert.throws(() => verifyBrowserAssets(directory), /Unexpected generated browser asset/);
    rmSync(join(generated, 'browser-source.part01'));
    writeFileSync(join(archives, 'mame0289.tar.gz'), 'corrupt');
    assert.throws(() => verifyBrowserAssets(directory, undefined, archives), /GitHub corresponding source mismatch/);
    writeFileSync(join(generated, 'sources.html'), '<a href="https://example.com">Other source</a>');
    assert.throws(() => verifyBrowserAssets(directory), /Missing matching GitHub source link/);
  } finally { rmSync(archives, { recursive: true, force: true }); }
}));
