import { createHash } from 'node:crypto';
import { lstatSync, readFileSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { configuredSourceRelease } from './source-release.mjs';

const hash = (bytes) => createHash('sha256').update(bytes).digest('hex');
export const releaseAssetNames = ['browser-source.tar.gz', 'mame0289.tar.gz', 'COPYING.txt', 'NOTICE.txt', 'build-receipt.json', 'SHA256SUMS'];
const allowedHosts = new Set(['github.com', 'api.github.com', 'release-assets.githubusercontent.com', 'objects.githubusercontent.com']);
async function publicResponse(url, fetcher) {
  for (let redirects = 0; redirects < 5; redirects++) {
    const target = new URL(url);
    if (target.protocol !== 'https:' || !allowedHosts.has(target.hostname) || target.username || target.password) throw new Error('Unsafe GitHub source redirect');
    const response = await fetcher(target.href, { redirect: 'manual', signal: AbortSignal.timeout(180_000), headers: { 'User-Agent': 'EightbitStudio-source-verification' } });
    if ([301, 302, 303, 307, 308].includes(response.status)) {
      const location = response.headers.get('location');
      await response.body?.cancel();
      if (!location) throw new Error('Missing GitHub source redirect');
      url = new URL(location, target).href; continue;
    }
    if (!response.ok || !response.body) throw new Error(`Public GitHub source download failed (HTTP ${response.status})`);
    return response;
  }
  throw new Error('Too many GitHub source redirects');
}
async function publicJson(url, fetcher) {
  const response = await publicResponse(url, fetcher);
  const chunks = []; let size = 0;
  for await (const chunk of response.body) {
    size += chunk.length;
    if (size > 1024 * 1024) throw new Error('GitHub source metadata exceeds its limit');
    chunks.push(chunk);
  }
  return JSON.parse(Buffer.concat(chunks).toString('utf8'));
}
export async function verifyPublicSource(release, directory, fetcher = fetch, progress = () => {}) {
  const repository = await publicJson(`https://api.github.com/repos/${release.repository}`, fetcher);
  if (repository.private !== false || repository.full_name !== release.repository) throw new Error('Browser source repository is not public');
  const published = await publicJson(`https://api.github.com/repos/${release.repository}/releases/tags/${release.tag}`, fetcher);
  if (published.draft !== false || published.tag_name !== release.tag || published.html_url !== release.url || !Array.isArray(published.assets)) throw new Error('Matching GitHub source release is unavailable');
  const verifiedAssets = {};
  for (const name of releaseAssetNames) {
    const path = join(directory, name); const stat = lstatSync(path);
    if (!stat.isFile() || stat.size < 1 || stat.size > 512 * 1024 * 1024) throw new Error('Invalid local source release asset');
    const expected = hash(readFileSync(path));
    const asset = published.assets.find((item) => item.name === name);
    const url = `${release.downloadBase}${name}`;
    if (!asset || asset.size !== stat.size || asset.browser_download_url !== url) throw new Error(`Missing or mismatched GitHub source asset: ${name}`);
    const response = await publicResponse(url, fetcher);
    const digest = createHash('sha256'); let size = 0;
    for await (const chunk of response.body) {
      size += chunk.length;
      if (size > stat.size) throw new Error(`Oversized GitHub source asset: ${name}`);
      digest.update(chunk);
    }
    if (size !== stat.size || digest.digest('hex') !== expected) throw new Error(`GitHub source hash mismatch: ${name}`);
    verifiedAssets[name] = expected; progress(name);
  }
  return { schemaVersion: 1, sourceReleaseUrl: release.url, verifiedAssets };
}
export function verifyPublicationProof(receiptBytes, proof, release) {
  const receipt = JSON.parse(receiptBytes);
  if (receipt.schemaVersion !== 2 || receipt.sourceRelease?.url !== release.url || proof?.schemaVersion !== 1
    || proof.sourceReleaseUrl !== release.url || proof.verifiedAssets?.['build-receipt.json'] !== hash(receiptBytes)
    || proof.verifiedAssets?.['browser-source.tar.gz'] !== receipt.sourceArchiveSha256
    || proof.verifiedAssets?.['mame0289.tar.gz'] !== receipt.externalSources?.mameArchiveSha256
    || !releaseAssetNames.every((name) => /^[a-f0-9]{64}$/.test(proof.verifiedAssets?.[name] ?? ''))) {
    throw new Error('Verify the matching public GitHub source release before staging');
  }
}
if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  const release = configuredSourceRelease();
  const directory = new URL(`../../.cache/xroar-web/github-release/${release.tag}/`, import.meta.url);
  const path = fileURLToPath(directory);
  const proof = await verifyPublicSource(release, path, fetch, (name) => console.log(`Public source verified: ${name}`));
  verifyPublicationProof(readFileSync(join(path, 'build-receipt.json')), proof, release);
  writeFileSync(join(path, 'publication-proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`Matching source release verified: ${release.url}`);
}
