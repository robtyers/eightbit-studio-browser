import { cpSync, existsSync, mkdirSync, mkdtempSync, readFileSync, renameSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { verifyBrowserAssets } from './verify-assets.mjs';
import { configuredSourceRelease } from './source-release.mjs';
import { verifyPublicationProof } from './verify-source-release.mjs';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const source = join(root, 'apps/eightbit-studio-web/ui/dist');
const release = configuredSourceRelease();
const sourceDirectory = join(root, '.cache/xroar-web/github-release', release.tag);
const receipt = verifyBrowserAssets(source, root, sourceDirectory);
const proofPath = join(sourceDirectory, 'publication-proof.json');
if (!existsSync(proofPath)) throw new Error('Verify the matching public GitHub source release before staging');
verifyPublicationProof(readFileSync(join(source, 'generated/build-receipt.json')), JSON.parse(readFileSync(proofPath, 'utf8')), release);
const client = join(root, 'website/dist/client');
if (!existsSync(join(client, 'index.html'))) throw new Error('Build the existing website before staging the browser edition');
const target = join(client, 'studio');
const staging = mkdtempSync(join(client, '.studio-stage-'));
try {
  cpSync(source, staging, { recursive: true });
  writeFileSync(join(staging, '.browser-release.json'), `${JSON.stringify({ schemaVersion: 1, sourceManifestSha256: receipt.sourceManifestSha256 }, null, 2)}\n`);
  if (existsSync(target)) {
    const marker = JSON.parse(readFileSync(join(target, '.browser-release.json'), 'utf8'));
    if (marker.schemaVersion !== 1) throw new Error('Refuse to replace an unknown studio directory');
    // This is replaceable local build output with an ownership marker, never user data.
    rmSync(target, { recursive: true });
  }
  renameSync(staging, target);
  const headersPath = join(client, '_headers');
  const original = existsSync(headersPath) ? readFileSync(headersPath, 'utf8') : '';
  const browserHeaders = '/studio/*\n  X-Content-Type-Options: nosniff\n  Referrer-Policy: no-referrer\n  Cache-Control: public, max-age=0, must-revalidate\n';
  if (!original.includes('/studio/*\n')) writeFileSync(headersPath, `${original.trimEnd()}\n\n${browserHeaders}`);
  verifyBrowserAssets(target, root);
  console.log('Browser edition staged locally at website/dist/client/studio/. No deployment performed.');
} finally { rmSync(staging, { recursive: true, force: true }); }
