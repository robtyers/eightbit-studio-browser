// Opt-in native/WASM acceptance of the maintained D64 development identities.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { spawnOwnedPeer } from './original-runtime-process.mjs';

const root = resolve(import.meta.dirname, '../..');
assert.deepEqual(process.argv.slice(2), ['--expect-development']);
execFileSync('python3', ['scripts/build-dragon64-basic.py', '--verify'], { cwd: root, stdio: 'pipe', timeout: 30_000 });
const manifest = JSON.parse(readFileSync(resolve(root, 'families/dragon/firmware/basic64/manifest.json')));
assert.equal(manifest.status, 'development');
const generated = resolve(root, 'apps/eightbit-studio-web/ui/public/generated');
const wasm = readFileSync(resolve(generated, 'dragon_core_bg.wasm'));
for (const name of ['compatibility', 'native']) assert.ok(wasm.includes(readFileSync(resolve(root, `families/dragon/firmware/basic64/${name}.rom`))), `WASM must contain exact ${name} firmware`);
const module = await import(pathToFileURL(resolve(generated, 'dragon_core.js')).href);
await module.default({ module_or_path: wasm });
let built;
try {
  built = execFileSync('cargo', ['build', '--offline', '--locked', '--example', 'original-runtime-peer', '-p', 'eightbit-browser', '--message-format=json'], { cwd: root, encoding: 'utf8', stdio: ['ignore', 'pipe', 'inherit'], timeout: 180_000, maxBuffer: 16 * 1024 * 1024 });
} catch { throw new Error('Native peer build failed; see Cargo diagnostics'); }
const executable = built.split('\n').filter(Boolean).map(line => JSON.parse(line)).find(value => value.reason === 'compiler-artifact' && value.target?.name === 'original-runtime-peer' && typeof value.executable === 'string')?.executable;
assert.equal(typeof executable, 'string');
const owner = spawnOwnedPeer(executable, [], { cwd: root });
let pending;
let buffer = '';
let sequence = 0;
let runtime;
const trace = createHash('sha256');
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
function compact(value) {
  if (!value || typeof value !== 'object') return value;
  if (value.session) value.session = compact(value.session);
  if (value.pixels) { value.pixelsSha256 = hash(Buffer.from(value.pixels)); delete value.pixels; }
  if (value.audio) {
    const bytes = Buffer.alloc(value.audio.length * 2);
    value.audio.forEach((sample, index) => bytes.writeInt16LE(sample, index * 2));
    value.audioSamples = value.audio.length; value.audioSha256 = hash(bytes); delete value.audio;
  }
  return value;
}
function fail(error) { if (pending) { const current = pending; pending = undefined; clearTimeout(current.timer); current.reject(error); } }
owner.child.on('error', fail);
owner.child.on('exit', () => fail(new Error('Native peer ended during acceptance')));
owner.child.stdout.setEncoding('utf8');
owner.child.stdout.on('data', chunk => {
  buffer += chunk;
  if (buffer.length > 4 * 1024 * 1024) return fail(new Error('Native response exceeds bound'));
  while (buffer.includes('\n')) {
    const end = buffer.indexOf('\n'); const line = buffer.slice(0, end); buffer = buffer.slice(end + 1);
    try {
      const reply = JSON.parse(line);
      assert.ok(pending && reply.id === pending.id, 'Unexpected native response');
      const current = pending; pending = undefined; clearTimeout(current.timer); current.resolve(reply);
    } catch (error) { fail(error); }
  }
});
function wasmRequest(command) {
  switch (command.operation) {
    case 'browser': return JSON.parse(module.browser_request(JSON.stringify(command.request), new Uint8Array()));
    case 'create': runtime = module.MonitorRuntime.fromOriginalSource(command.session_id, JSON.stringify(command.source_request)); return null;
    case 'monitor': return compact(JSON.parse(runtime.command(JSON.stringify(command.request))));
    case 'debug-state': { const value = runtime.debugState(); return value == null ? null : compact(JSON.parse(value)); }
    case 'debug': return compact(JSON.parse(runtime.debugCommand(JSON.stringify(command.request))));
    case 'close': runtime?.free(); runtime = undefined; return null;
    default: throw new Error('Unknown acceptance operation');
  }
}
async function action(command, rejected = false) {
  let actual;
  try { actual = { ok: true, result: wasmRequest(command) }; }
  catch (error) { actual = { ok: false, error: String(error).slice(0, 256) }; }
  assert.equal(pending, undefined);
  const expected = await new Promise((resolve, reject) => {
    const id = ++sequence;
    const timer = setTimeout(() => fail(new Error('Native request timed out')), 30_000);
    pending = { id, resolve, reject, timer };
    owner.child.stdin.write(`${JSON.stringify({ id, command })}\n`);
  });
  const { id: _, ...reply } = expected;
  assert.deepEqual(actual, reply, `Native/WASM mismatch at ${command.operation}`);
  assert.equal(actual.ok, !rejected, actual.error);
  trace.update(JSON.stringify(actual));
  return actual.result;
}
const browser = request => action({ operation: 'browser', request: { schemaVersion: 1, ...request }, bytes: [] });
const monitor = (id, command) => action({ operation: 'monitor', request: { schemaVersion: 1, sessionId: id, command } });
const source = '10 A=41\n20 PRINT A+1\n30 END\n';
const deadline = setTimeout(() => { fail(new Error('D64 acceptance deadline exceeded')); owner.child.kill(); }, 180_000);
try {
  const catalogue = await browser({ operation: 'machines' });
  assert.equal(catalogue.ok, true);
  for (const [target, name] of [['dragon64', 'compatibility'], ['dragon64-native', 'native']]) {
    assert.equal(catalogue.result.originalRuntimes[target].profile.firmwareSha256, manifest.profiles[name].sha256);
  }
  let id = 64;
  for (const target of ['dragon64', 'dragon64-native']) {
    console.error(`Checking ${target} native/WASM build, cassette execution and debugger`);
    const created = await browser({ operation: 'create-project', name: 'D64 parity', kind: 'basic', target, source });
    assert.equal(created.ok, true);
    const built = await browser({ operation: 'test-project', project: created.result.project });
    assert.equal(built.ok, true);
    const request = { schemaVersion: 1, target, source, sourceSha256: built.result.summary.sourceSha256, cassetteSha256: built.result.summary.cassetteSha256, breakpointLine: 20 };
    await action({ operation: 'create', session_id: id, source_request: request });
    const initial = await action({ operation: 'debug-state' });
    const cancelled = await action({ operation: 'debug', request: { schemaVersion: 1, revision: initial.revision, command: { operation: 'cancel' } } });
    assert.equal(cancelled.status.state, 'cancelled');
    const paused = await monitor(id, { operation: 'advance', cycles: 32768 });
    assert.equal(paused.cycles, cancelled.session.cycles);
    await action({ operation: 'debug', request: { schemaVersion: 1, revision: cancelled.revision, command: { operation: 'continue' } } });
    let stopped;
    for (let n = 0; n < 1024; n++) {
      const frame = await monitor(id, { operation: 'advance', cycles: 32768 });
      assert.notEqual(frame.state, 'failed');
      stopped = await action({ operation: 'debug-state' });
      if (stopped?.status?.state === 'stopped') break;
    }
    assert.equal(stopped?.status?.location?.basicLine, 20);
    assert.equal(stopped?.status?.variables?.find(value => value.name === 'A')?.numericValue, 41);
    const continued = await action({ operation: 'debug', request: { schemaVersion: 1, revision: stopped.revision, command: { operation: 'continue' } } });
    assert.ok(continued);
    await action({ operation: 'debug', request: { schemaVersion: 1, revision: stopped.revision, command: { operation: 'continue' } } }, true);
    let printed = false;
    for (let n = 0; n < 256; n++) {
      const frame = await monitor(id, { operation: 'advance', cycles: 32768 });
      if (frame.text.split('\n').some(line => line.trim() === '42')) { printed = true; break; }
    }
    assert.ok(printed, `${target} must execute the current guest programme`);
    await action({ operation: 'close' });
    await action({ operation: 'create', session_id: ++id, source_request: { ...request, source: '10 PRINT 99\n' } }, true);
    id++;
  }
  console.log(JSON.stringify({ schemaVersion: 1, status: 'passed', profiles: Object.fromEntries(Object.entries(manifest.profiles).map(([name, profile]) => [name, profile.sha256])), traceSha256: trace.digest('hex') }, null, 2));
} finally {
  clearTimeout(deadline);
  runtime?.free();
  const termination = await owner.stop();
  assert.equal(termination.error, undefined);
  assert.equal(termination.code, 0, 'Native peer must exit cleanly');
}
