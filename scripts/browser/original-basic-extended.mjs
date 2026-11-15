import { createHash } from 'node:crypto';
import { lstatSync, readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';

const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const record = value => value !== null && typeof value === 'object' && !Array.isArray(value);
const sorted = value => Array.isArray(value) ? value.map(sorted) : record(value) ? Object.fromEntries(Object.keys(value).sort().map(key => [key, sorted(value[key])])) : value;
const equal = (a, b) => JSON.stringify(sorted(a)) === JSON.stringify(sorted(b));
const sourceRoot = 'families/dragon/firmware/basic-complete';
const directory = 'families/dragon/firmware/basic-extended';
const builders = ['build-dragon-basic-extended.py', 'qualify-dragon-basic.py'];
const fields = ['schemaVersion', 'id', 'status', 'licence', 'bytes', 'sourceRoot', 'sourceSha256', 'builderSha256', 'assemblyDefines', 'memoryMap', 'romSha256', 'symbolsSha256'];
export const originalBasicExtendedPath = `${directory}/basic.rom`;
export const originalBasicExtendedFiles = ['basic.rom', 'basic.sym', 'manifest.json', 'qualification.json'].map(name => `${directory}/${name}`);
const memoryMap = { ramStart: 0, ramEndExclusive: 32768, romStart: 32768, romEndExclusive: 65280, ioStart: 65280, ioEndExclusive: 65504, vectorAliasStart: 65504, vectorAliasEndExclusive: 65536, vectorAliasSourceStart: 49120, cartridges: false, samAllRam: 'unchanged' };

function regular(root, path, maximum) {
  // Check every parent: lstat of a final regular file does not reject directory aliases.
  const segments = path.split('/');
  for (let i = 1; i < segments.length; i++) {
    const stat = lstatSync(join(root, ...segments.slice(0, i)));
    if (!stat.isDirectory() || stat.isSymbolicLink()) throw new Error('Expanded BASIC assets must use regular directories');
  }
  const stat = lstatSync(join(root, path));
  if (!stat.isFile() || stat.isSymbolicLink() || stat.size > maximum) throw new Error('Expanded BASIC assets must be bounded regular files');
  return readFileSync(join(root, path));
}
export function originalBasicExtendedSources(receipt) {
  return Object.keys(receipt.sourceSha256).sort().map(name => `${sourceRoot}/${name}`);
}

/** This exact original-firmware exception never admits overflow qualification fixtures. */
export function verifyOriginalBasicExtended(root) {
  const receipt = JSON.parse(regular(root, `${directory}/manifest.json`, 65536));
  if (!record(receipt) || !equal(Object.keys(receipt).sort(), [...fields, 'layout'].sort())
    || receipt.schemaVersion !== 1 || receipt.id !== 'eightbit-dragon32-basic-extended-1'
    || !['development', 'qualified'].includes(receipt.status) || receipt.licence !== 'Apache-2.0'
    || receipt.bytes !== 32768 || receipt.sourceRoot !== '../basic-complete'
    || !equal(receipt.assemblyDefines, ['STUDIO_EXTENDED=1']) || !equal(receipt.memoryMap, memoryMap)) {
    throw new Error('Expanded BASIC profile/memory-map receipt mismatch');
  }
  const image = regular(root, originalBasicExtendedPath, 32768);
  if (image.length !== 32768 || receipt.romSha256 !== hash(image)
    || receipt.symbolsSha256 !== hash(regular(root, `${directory}/basic.sym`, 1024 * 1024))) throw new Error('Expanded BASIC image/symbol receipt mismatch');
  // The reserved hardware window is padding; the final vectors retain their original alias.
  if (!image.subarray(0x7f00, 0x7fe0).every(value => value === 0x12)
    || !image.subarray(0x7fe0).equals(image.subarray(0x3fe0, 0x4000))) throw new Error('Expanded BASIC hardware window mismatch');
  const names = readdirSync(join(root, sourceRoot, 'source')).filter(name => name.endsWith('.inc')).sort();
  if (!names.length || names.some(name => !/^[a-z0-9-]+\.inc$/.test(name))) throw new Error('Expanded BASIC source names are invalid');
  const sources = ['fixed-layout.asm', 'layout.asm', ...names.map(name => `source/${name}`)];
  if (!record(receipt.sourceSha256) || !equal(Object.keys(receipt.sourceSha256).sort(), sources.slice().sort())
    || sources.some(name => receipt.sourceSha256[name] !== hash(regular(root, `${sourceRoot}/${name}`, 1024 * 1024)))) throw new Error('Expanded BASIC source receipt mismatch');
  if (!record(receipt.builderSha256) || !equal(Object.keys(receipt.builderSha256).sort(), builders)
    || builders.some(name => receipt.builderSha256[name] !== hash(regular(root, `scripts/${name}`, 1024 * 1024)))) throw new Error('Expanded BASIC builder receipt mismatch');
  const qualification = JSON.parse(regular(root, `${directory}/qualification.json`, 65536));
  const identity = Object.fromEntries(fields.map(key => [key, receipt[key]]));
  if (!equal(qualification, identity)) throw new Error('Expanded BASIC qualification does not match this exact build');
  return receipt;
}
