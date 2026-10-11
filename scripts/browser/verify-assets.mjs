import { createHash } from 'node:crypto';
import { existsSync, lstatSync, readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { configuredSourceRelease, sourceRelease } from './source-release.mjs';

const digest = (bytes) => createHash('sha256').update(bytes).digest('hex');
const record = (value) => value !== null && typeof value === 'object' && !Array.isArray(value);
const validHash = (value) => typeof value === 'string' && /^[a-f0-9]{64}$/.test(value);
const safePath = (value) => typeof value === 'string' && value.length < 512 && !value.startsWith('/') && !value.includes('\\') && value.split('/').every((part) => part && part !== '.' && part !== '..');
export function verifyBrowserAssets(directory, inputRoot, sourceDirectory) {
  const rootNames = new Set(['index.html', 'runtime.html', 'runtime.js', 'runtime.css', 'classic.html', 'classic.js', 'generated', 'assets', '.browser-release.json']);
  const generated = join(directory, 'generated');
  const receipt = JSON.parse(readFileSync(join(generated, 'build-receipt.json'), 'utf8'));
  if (!record(receipt)) throw new Error('Invalid browser source receipt');
  const generatedNames = new Set(['dragon_core.js', 'dragon_core_bg.wasm', 'xroar.js', 'xroar.wasm', 'COPYING.txt', 'NOTICE.txt', 'build-receipt.json', 'sources.html', 'c64.js', 'c64.wasm', 'mame-z80.js', 'mame-z80.wasm.gz', 'mame-bbc.js', 'mame-bbc.wasm.gz']);
  // Keep old receipts verifiable for archived deployments; new staging requires schema 2.
  if (receipt.schemaVersion === 1) for (const name of [...Array.from({ length: 8 }, (_, index) => `browser-source.part${String(index + 1).padStart(2, '0')}`), ...Array.from({ length: 9 }, (_, index) => `mame-source.part${String(index + 1).padStart(2, '0')}`)]) generatedNames.add(name);
  for (const name of readdirSync(directory)) if (!rootNames.has(name)) throw new Error('Unexpected browser asset; rebuild the UI before staging');
  for (const name of readdirSync(join(directory, 'generated'))) if (!generatedNames.has(name)) throw new Error('Unexpected generated browser asset');
  for (const name of readdirSync(join(directory, 'assets'))) if (!/^[A-Za-z0-9_-]+\.(js|css|ttf)$/.test(name)) throw new Error('Unexpected browser bundle asset');
  function scan(path) {
    const stat = lstatSync(path);
    if (stat.isSymbolicLink()) throw new Error('Browser assets refuse symlinks');
    if (stat.isDirectory()) for (const name of readdirSync(path)) scan(join(path, name));
    else if (!stat.isFile()) throw new Error('Unexpected browser asset');
    else if (stat.size > 25 * 1024 * 1024) throw new Error('Browser asset exceeds the Cloudflare 25 MiB file limit');
  }
  scan(directory);
  if (!record(receipt) || ![1, 2].includes(receipt.schemaVersion) || !record(receipt.binaries) || !record(receipt.inputs) || !Object.keys(receipt.inputs).length || !validHash(receipt.sourceArchiveSha256) || !validHash(receipt.sourceManifestSha256)) throw new Error('Invalid browser source receipt');
  for (const name of ['dragon_core.js', 'dragon_core_bg.wasm', 'xroar.js', 'xroar.wasm', 'c64.js', 'c64.wasm', 'mame-z80.js', 'mame-z80.wasm.gz']) {
    if (!validHash(receipt.binaries[name]) || digest(readFileSync(join(generated, name))) !== receipt.binaries[name]) throw new Error('Browser binary/source receipt mismatch');
  }
  // Archived pre-BBC receipts remain readable; a new BBC asset always needs
  // both binaries and their exact corresponding-source receipt.
  const bbc = ['mame-bbc.js', 'mame-bbc.wasm.gz'];
  if (bbc.some((name) => existsSync(join(generated, name)) || Object.hasOwn(receipt.binaries, name))) for (const name of bbc) {
    if (!validHash(receipt.binaries[name]) || digest(readFileSync(join(generated, name))) !== receipt.binaries[name]) throw new Error('BBC binary/source receipt mismatch');
  }
  if (!record(receipt.externalSources) || !validHash(receipt.externalSources.mameArchiveSha256)) throw new Error('Invalid external source receipt');
  if (receipt.schemaVersion === 2) {
    const release = sourceRelease({ schemaVersion: 1, ...receipt.sourceRelease });
    if (receipt.sourceRelease.url !== release.url || (inputRoot && release.url !== configuredSourceRelease().url)) throw new Error('Browser source release mismatch');
    if (sourceDirectory) {
      for (const [name, expected] of [['browser-source.tar.gz', receipt.sourceArchiveSha256], ['mame0289.tar.gz', receipt.externalSources.mameArchiveSha256]]) {
        const path = join(sourceDirectory, name);
        if (lstatSync(path).isSymbolicLink() || digest(readFileSync(path)) !== expected) throw new Error('GitHub corresponding source mismatch');
      }
    }
    if (!readFileSync(join(generated, 'sources.html'), 'utf8').includes(`href="${release.url}"`)) throw new Error('Missing matching GitHub source link');
  } else {
    if (!record(receipt.sourceArchiveParts) || Object.keys(receipt.sourceArchiveParts).length < 1 || Object.keys(receipt.sourceArchiveParts).length > 8) throw new Error('Invalid browser source parts');
    const sourceArchive = createHash('sha256');
    for (let index = 1; index <= Object.keys(receipt.sourceArchiveParts).length; index++) {
      const name = `browser-source.part${String(index).padStart(2, '0')}`;
      const bytes = readFileSync(join(generated, name));
      if (!validHash(receipt.sourceArchiveParts[name]) || digest(bytes) !== receipt.sourceArchiveParts[name]) throw new Error('Browser corresponding source mismatch');
      sourceArchive.update(bytes);
    }
    if (sourceArchive.digest('hex') !== receipt.sourceArchiveSha256) throw new Error('Browser corresponding source mismatch');
    const expectedParts = new Set([...Object.keys(receipt.sourceArchiveParts), ...Object.keys(receipt.externalSources?.parts ?? {})]);
    for (const name of readdirSync(generated)) if (name.includes('.part') && !expectedParts.has(name)) throw new Error('Unmatched corresponding source part');
    if (!record(receipt.externalSources) || !validHash(receipt.externalSources.mameArchiveSha256) || !record(receipt.externalSources.parts) || Object.keys(receipt.externalSources.parts).length !== 9) throw new Error('Invalid external source receipt');
    const whole = createHash('sha256');
    for (let index = 1; index <= 9; index++) {
      const name = `mame-source.part${String(index).padStart(2, '0')}`;
      const bytes = readFileSync(join(generated, name));
      if (!validHash(receipt.externalSources.parts[name]) || digest(bytes) !== receipt.externalSources.parts[name]) throw new Error('Emulator corresponding source part mismatch');
      whole.update(bytes);
    }
    if (whole.digest('hex') !== receipt.externalSources.mameArchiveSha256) throw new Error('Emulator corresponding source archive mismatch');
  }
  for (const name of ['COPYING.txt', 'NOTICE.txt']) if (!readFileSync(join(generated, name), 'utf8').trim()) throw new Error('Missing browser licence notice');
  if (inputRoot) for (const [path, expected] of Object.entries(receipt.inputs)) {
    if (!safePath(path) || !validHash(expected) || digest(readFileSync(join(inputRoot, path))) !== expected) throw new Error('Source changed after packaging; rebuild the browser source archive');
  }
  for (const name of ['index.html', 'runtime.html', 'runtime.js', 'runtime.css', 'classic.html', 'classic.js', 'generated/sources.html']) readFileSync(join(directory, name));
  return receipt;
}
