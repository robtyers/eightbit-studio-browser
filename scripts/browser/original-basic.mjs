import { createHash } from 'node:crypto';
import { lstatSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

const hash = (bytes) => createHash('sha256').update(bytes).digest('hex');
const sources = ['../../dialects/dragon32-basic/tokens.toml', 'abi.inc', 'arrays.inc', 'basic.asm', 'break.inc', 'circle.inc', 'command-guards.inc', 'commands.inc', 'console.inc', 'control.inc', 'data.inc', 'delete.inc', 'devices.inc', 'draw.inc', 'errors-data.inc', 'errors.inc', 'exec.inc', 'expression.inc', 'extended-control.inc', 'format.inc', 'fp-program.inc', 'function-call.inc', 'function-definitions.inc', 'function-name.inc', 'function-return.inc', 'functions.inc', 'graphics-box.inc', 'graphics-cls.inc', 'graphics-copy-page.inc', 'graphics-copy.inc', 'graphics-fill-box.inc', 'graphics-line-core.inc', 'graphics-line.inc', 'graphics-low.inc', 'graphics-mode.inc', 'graphics-page.inc', 'graphics-paint.inc', 'graphics-point.inc', 'graphics-screen.inc', 'graphics-sprite.inc', 'input-end.inc', 'input-start.inc', 'input-store.inc', 'input-value.inc', 'input-variable.inc', 'keywords.inc', 'line-range.inc', 'math.inc', 'mid-assignment.inc', 'native-frame.inc', 'number.inc', 'numeric.inc', 'play.inc', 'print-channel.inc', 'print-device.inc', 'print-position.inc', 'print-tab.inc', 'random-init.inc', 'random.inc', 'scanner.inc', 'series.inc', 'sound.inc', 'statement-arguments.inc', 'storage.inc', 'string-functions.inc', 'strings-gc.inc', 'strings-roots.inc', 'strings.inc', 'syntax-open.inc', 'tape-ascii-in.inc', 'tape-ascii-out.inc', 'tape-basic.inc', 'tape-block-in.inc', 'tape-block-out.inc', 'tape-file.inc', 'tape-load.inc', 'tape-read.inc', 'tape-save.inc', 'tape-write.inc', 'tokenise.inc', 'trace.inc', 'unsigned.inc', 'value-functions.inc'];
export const originalBasicPath = 'families/dragon/firmware/basic/basic.rom';
/** The reviewed qualification is bound only to this maintained 16 KiB image. */
function verifyIdentity(directory, receipt) {
  const path = join(directory, 'qualification.json');
  // lstat also rejects a dangling symlink, which existsSync alone would miss.
  let stat;
  try { stat = lstatSync(path); } catch (error) { if (error.code !== 'ENOENT') throw error; }
  if (!stat) {
    if (receipt.id !== 'eightbit-dragon32-basic-0.1-dev' || receipt.status !== 'development') throw new Error('Original BASIC qualification is required');
    return;
  }
  if (!stat.isFile() || stat.isSymbolicLink() || stat.size > 65536) throw new Error('Original BASIC qualification must be a bounded regular file');
  const qualification = JSON.parse(readFileSync(path, 'utf8'));
  const fields = ['schemaVersion', 'licence', 'bytes', 'romSha256', 'symbolsSha256', 'sourceSha256', 'id', 'status'];
  if (!qualification || typeof qualification !== 'object' || Array.isArray(qualification) || Object.keys(qualification).sort().join(',') !== fields.sort().join(',') || qualification.id !== 'eightbit-dragon32-basic-1' || qualification.status !== 'qualified' || receipt.id !== qualification.id || receipt.status !== qualification.status || fields.filter(key => key !== 'sourceSha256').some(key => qualification[key] !== receipt[key]) || !qualification.sourceSha256 || Object.keys(qualification.sourceSha256).sort().join(',') !== sources.join(',') || sources.some(name => qualification.sourceSha256[name] !== receipt.sourceSha256[name])) throw new Error('Original BASIC qualification does not match this exact build');
}
/** Source may carry only this original image with its complete receipt. */
export function verifyOriginalBasic(root) {
  const directory = join(root, 'families/dragon/firmware/basic');
  for (const name of ['manifest.json', 'basic.rom', 'basic.sym', ...sources]) {
    const stat = lstatSync(join(directory, name));
    if (!stat.isFile() || stat.isSymbolicLink()) throw new Error('Original BASIC assets must be regular files');
  }
  const receipt = JSON.parse(readFileSync(join(directory, 'manifest.json'), 'utf8'));
  const image = readFileSync(join(directory, 'basic.rom'));
  if (receipt.symbolsSha256 !== hash(readFileSync(join(directory, 'basic.sym')))) throw new Error('Original BASIC source/firmware receipt mismatch');
  if (receipt.schemaVersion !== 1 || receipt.licence !== 'Apache-2.0' || receipt.bytes !== 16384 || image.length !== 16384 || receipt.romSha256 !== hash(image) || !receipt.sourceSha256 || Object.keys(receipt.sourceSha256).sort().join(',') !== sources.join(',') || sources.some((name) => receipt.sourceSha256[name] !== hash(readFileSync(join(directory, name))))) throw new Error('Original BASIC source/firmware receipt mismatch');
  verifyIdentity(directory, receipt);
  return receipt;
}
