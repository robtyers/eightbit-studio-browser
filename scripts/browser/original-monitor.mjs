import { createHash } from 'node:crypto';
import { lstatSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

const hash = (bytes) => createHash('sha256').update(bytes).digest('hex');
export const originalMonitorPath = 'families/dragon/firmware/monitor/monitor.rom';
/** The original monitor exception; other binary/firmware exclusions remain. */
export function verifyOriginalMonitor(root) {
  const directory = join(root, 'families/dragon/firmware/monitor');
  for (const name of ['manifest.json', 'monitor.asm', 'monitor.rom']) if (!lstatSync(join(directory, name)).isFile() || lstatSync(join(directory, name)).isSymbolicLink()) throw new Error('Original monitor assets must be regular files');
  const receipt = JSON.parse(readFileSync(join(directory, 'manifest.json'), 'utf8'));
  const source = readFileSync(join(directory, 'monitor.asm'));
  const image = readFileSync(join(directory, 'monitor.rom'));
  if (receipt.schemaVersion !== 1 || receipt.id !== 'eightbit-dragon32-monitor-0.1' || receipt.licence !== 'Apache-2.0' || receipt.bytes !== 16384 || image.length !== 16384 || receipt.sourceSha256 !== hash(source) || receipt.romSha256 !== hash(image)) throw new Error('Original monitor source/firmware receipt mismatch');
  return receipt;
}
