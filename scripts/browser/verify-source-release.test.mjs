import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createHash } from 'node:crypto';
import { test } from 'node:test';
import { sourceRelease } from './source-release.mjs';
import { releaseAssetNames, verifyPublicSource, verifyPublicationProof } from './verify-source-release.mjs';
const hash = (bytes) => createHash('sha256').update(bytes).digest('hex');
const release = sourceRelease({ schemaVersion: 1, repository: 'robtyers/eightbit-studio-browser', tag: 'browser-0.5.2-20261010.1' });
async function fixture(run) {
  const directory = mkdtempSync(join(tmpdir(), 'eightbit-source-release-'));
  try {
    for (const name of releaseAssetNames) writeFileSync(join(directory, name), name);
    writeFileSync(join(directory, 'build-receipt.json'), JSON.stringify({ schemaVersion: 2, sourceRelease: { url: release.url }, sourceArchiveSha256: hash('browser-source.tar.gz'), externalSources: { mameArchiveSha256: hash('mame0289.tar.gz') } }));
    const assets = releaseAssetNames.map((name) => ({ name, size: readFileSync(join(directory, name)).length, browser_download_url: `${release.downloadBase}${name}` }));
    const fetcher = async (url, options) => {
      assert.equal(options.redirect, 'manual'); assert.ok(options.signal instanceof AbortSignal);
      if (url === `https://api.github.com/repos/${release.repository}`) return Response.json({ private: false, full_name: release.repository });
      if (url === `https://api.github.com/repos/${release.repository}/releases/tags/${release.tag}`) return Response.json({ draft: false, tag_name: release.tag, html_url: release.url, assets });
      const asset = assets.find((item) => item.browser_download_url === url);
      return asset ? new Response(readFileSync(join(directory, asset.name))) : new Response('missing', { status: 404 });
    };
    await run(directory, fetcher);
  } finally { rmSync(directory, { recursive: true, force: true }); }
}
test('publication verifies every public asset and binds staging to the exact build receipt', () => fixture(async (directory, fetcher) => {
  const proof = await verifyPublicSource(release, directory, fetcher);
  const bytes = readFileSync(join(directory, 'build-receipt.json'));
  verifyPublicationProof(bytes, proof, release);
  assert.equal(Object.keys(proof.verifiedAssets).length, 6);
  const stale = Buffer.from(bytes.toString() + '\n');
  assert.throws(() => verifyPublicationProof(stale, proof, release), /before staging/);
  assert.throws(() => verifyPublicationProof(bytes, { ...proof, sourceReleaseUrl: 'https://example.com' }, release), /before staging/);
}));
test('publication rejects private repositories, missing assets and corrupt public downloads', () => fixture(async (directory, fetcher) => {
  await assert.rejects(verifyPublicSource(release, directory, async () => Response.json({ private: true, full_name: release.repository })), /not public/);
  await assert.rejects(verifyPublicSource(release, directory, async (url, options) => url.endsWith('/browser-source.tar.gz') ? new Response('xxxxxxxxxxxxxxxxxxxxx') : fetcher(url, options)), /hash mismatch/);
  await assert.rejects(verifyPublicSource(release, directory, async (url, options) => url.endsWith('/mame0289.tar.gz') ? new Response('missing', { status: 404 }) : fetcher(url, options)), /HTTP 404/);
}));
test('publication rejects an off-host redirect before downloading its contents', () => fixture(async (directory, fetcher) => {
  await assert.rejects(verifyPublicSource(release, directory, async (url, options) => url.endsWith('/browser-source.tar.gz') ? new Response(null, { status: 302, headers: { location: 'https://attacker.invalid/source' } }) : fetcher(url, options)), /Unsafe GitHub source redirect/);
}));
