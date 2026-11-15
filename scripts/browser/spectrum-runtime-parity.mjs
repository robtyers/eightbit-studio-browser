// Opt-in comparison of the actual compiled native and WASM Spectrum hosts.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { spawnOwnedPeer } from './original-runtime-process.mjs';

assert.equal(process.argv.length, 2, 'This acceptance has no input or admission overrides');
const root = resolve(import.meta.dirname, '../..');
const generated = resolve(root, 'apps/eightbit-studio-web/ui/public/generated');
const module = await import(pathToFileURL(resolve(generated, 'dragon_core.js')).href);
await module.default({ module_or_path: readFileSync(resolve(generated, 'dragon_core_bg.wasm')) });
const built = execFileSync('cargo', ['build', '--offline', '--locked', '--release', '--message-format=json', '-p', 'eightbit-browser', '--example', 'original-runtime-peer'], { cwd: root, encoding: 'utf8', stdio: ['ignore', 'pipe', 'inherit'], timeout: 180_000, maxBuffer: 16 * 1024 * 1024 });
const executable = built.split('\n').filter(Boolean).map(line => JSON.parse(line)).find(value => value.reason === 'compiler-artifact' && value.target?.name === 'original-runtime-peer' && typeof value.executable === 'string')?.executable;
assert.equal(typeof executable, 'string', 'Cargo must identify the exact native peer');
const owner = spawnOwnedPeer(executable, [], { cwd: root });
let pending;
let buffer = '';
let next = 0;
let runtime;
let sessionId = 0;
const trace = createHash('sha256');
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
function fail(message) {
  const current = pending; pending = undefined;
  if (current) { clearTimeout(current.timer); current.reject(new Error(message)); }
  owner.child.kill();
}
owner.child.on('error', () => fail('Native Spectrum peer failed to start'));
owner.child.on('exit', () => { if (pending) fail('Native Spectrum peer exited during a request'); });
owner.child.stdout.setEncoding('utf8');
owner.child.stdout.on('data', chunk => {
  buffer += chunk;
  if (buffer.length > 4 * 1024 * 1024) return fail('Native Spectrum response exceeds its bound');
  while (buffer.includes('\n')) {
    const end = buffer.indexOf('\n'); const line = buffer.slice(0, end); buffer = buffer.slice(end + 1);
    try {
      const value = JSON.parse(line);
      if (!pending || value.id !== pending.id) return fail('Stale Spectrum peer response');
      const current = pending; pending = undefined; clearTimeout(current.timer); current.resolve(value);
    } catch { return fail('Malformed Spectrum peer response'); }
  }
});
function peer(command) {
  assert.equal(pending, undefined);
  return new Promise((resolve, reject) => {
    const id = ++next;
    pending = { id, resolve, reject, timer: setTimeout(() => fail('Spectrum peer request timed out'), 30_000) };
    owner.child.stdin.write(`${JSON.stringify({ id, command })}\n`);
  });
}
function compact(value) {
  if (value.pixels) { value.pixelsSha256 = hash(Buffer.from(value.pixels)); delete value.pixels; }
  if (value.audio) {
    const bytes = Buffer.alloc(value.audio.length * 2);
    value.audio.forEach((sample, index) => bytes.writeInt16LE(sample, index * 2));
    value.audioSamples = value.audio.length; value.audioSha256 = hash(bytes); delete value.audio;
  }
  return value;
}
function wasm(command) {
  switch (command.operation) {
    case 'browser': return JSON.parse(module.browser_request(JSON.stringify(command.request), Uint8Array.from(command.bytes)));
    case 'spectrum-create':
      assert.equal(runtime, undefined);
      runtime = module.SpectrumDevelopmentRuntime.fromSource(command.session_id, JSON.stringify(command.source_request)); return null;
    case 'spectrum-monitor': return compact(JSON.parse(runtime.command(JSON.stringify(command.request))));
    case 'spectrum-close': runtime?.free(); runtime = undefined; return null;
    default: throw new Error('Invalid Spectrum acceptance operation');
  }
}
function sorted(value) { return Array.isArray(value) ? value.map(sorted) : value && typeof value === 'object' ? Object.fromEntries(Object.keys(value).sort().map(key => [key, sorted(value[key])])) : value; }
async function action(command, error = false) {
  let actual;
  try { actual = { ok: true, result: wasm(command) }; }
  catch (failure) { actual = { ok: false, error: String(failure).slice(0, 256) }; }
  const { id: _, ...expected } = await peer(command);
  assert.deepEqual(actual, expected, `Native/WASM Spectrum disagreement at ${command.operation}`);
  trace.update(JSON.stringify(sorted(actual)));
  assert.equal(actual.ok, !error, actual.error);
  return actual.result;
}
async function browser(request) {
  const response = await action({ operation: 'browser', request: { schemaVersion: 1, ...request }, bytes: [] });
  assert.equal(response.ok, true, response.message); return response.result;
}
async function launch(source, kind = 'basic', breakpointLine, target = 'spectrum48') {
  const { project, entryPath } = await browser({ operation: 'create-project', target, name: 'Original parity acceptance', kind, source });
  const build = await browser({ operation: 'test-project', project });
  return { schemaVersion: 1, target, kind, source, sourcePath: entryPath, revision: build.assembly?.buildId ?? hash(source), sourceSha256: build.summary.sourceSha256, cassetteSha256: build.summary.cassetteSha256, ...(breakpointLine === undefined ? {} : { breakpointLine }) };
}
const command = command => action({ operation: 'spectrum-monitor', request: { schemaVersion: 1, sessionId, command } });
async function create(request) { await action({ operation: 'spectrum-create', session_id: ++sessionId, source_request: request }); }
async function until(check, limit = 1600) {
  let frame;
  for (let n = 0; n < limit; n++) {
    frame = await command({ operation: 'advance', cycles: 350000 });
    assert.notEqual(frame.state, 'failed', frame.message);
    if (check(frame)) return frame;
  }
  throw new Error(`Spectrum acceptance frame budget exhausted: ${JSON.stringify({ message: frame.message, text: frame.text })}`);
}
const deadline = setTimeout(() => fail('Spectrum parity exceeded its total time budget'), 300_000);
let termination;
try {
  const catalog = await browser({ operation: 'machines' });
  const spectrum = catalog.machines.find(machine => machine.id === 'spectrum48');
  assert.equal(spectrum.defaultRuntime, 'original'); assert.equal(spectrum.runtime, 'mame-z80');
  const basic = await launch('10 BORDER 2:INK 7:PAPER 0:CLS\n20 LET A$="PARITY":LET N=21\n30 PLOT 20,30:DRAW 40,20:CIRCLE 100,80,15\n40 BEEP .02,0\n50 PRINT A$;N*2\n60 INPUT "VALUE? ";N\n70 PRINT "INPUT ";N\n80 STOP\n');
  await action({ operation: 'spectrum-create', session_id: 1, source_request: { ...basic, source: '10 STOP\n' } }, true);
  await create(basic);
  await until(frame => frame.text.includes('PARITY42') && frame.text.includes('VALUE?'));
  await command({ operation: 'type', text: '37' });
  await command({ operation: 'release' });
  await until(frame => frame.text.includes('INPUT 37') && frame.text.includes('STOP statement'));
  await command({ operation: 'type', text: 'POKE 49152,42:LET' });
  await until(frame => frame.text.includes('POKE 49152,42:LET'));
  await command({ operation: 'reset' });
  await command({ operation: 'stop' });
  await action({ operation: 'spectrum-monitor', request: { schemaVersion: 1, sessionId, command: { operation: 'advance', cycles: 350000 } } }, true);
  await action({ operation: 'spectrum-close' });

  await create(await launch('ORG 24576\nLD A,42\nLD (49152),A\nINC A\nLD (49153),A\nRET\n', 'assembly', 2));
  const first = await until(frame => frame.debug?.state === 'stopped');
  assert.equal(first.debug.sourceLine, 2); assert.equal(first.debug.valuesValid, true);
  const stepped = await command({ operation: 'step', stopId: first.debug.stopId });
  assert.equal(stepped.debug.sourceLine, 3); assert.equal(stepped.debug.registers.af >> 8, 42);
  await action({ operation: 'spectrum-monitor', request: { schemaVersion: 1, sessionId, command: { operation: 'step', stopId: first.debug.stopId } } }, true);
  const running = await command({ operation: 'continue', stopId: stepped.debug.stopId });
  assert.equal(running.debug.valuesValid, false);
  await until(frame => frame.text.includes('0 OK'));
  await command({ operation: 'start-recording' });
  await command({ operation: 'type', text: 'SAVE "PARITY" CODE 49152,2' });
  await until(frame => frame.recording.samples > frame.recording.sampleRate * 12 && frame.text.includes('0 OK'), 900);
  const ready = await command({ operation: 'finish-recording' });
  assert.equal(ready.recording.state, 'ready');
  const exported = await command({ operation: 'export-recording' });
  const tape = Buffer.from(exported.recordingExport.bytes);
  assert.equal(tape.length, 27); assert.deepEqual([...tape.subarray(-6)], [4, 0, 255, 42, 43, 254]);
  await command({ operation: 'stop' });
  await action({ operation: 'spectrum-close' });
  for (const target of ['spectrum128', 'spectrum-plus2']) {
    await create(await launch(`10 BORDER 3:POKE 49152,42:SAVE! "zeta" CODE 49152,1\n20 POKE 49152,43:SAVE! "alpha" CODE 49152,1:CAT!\n30 LOAD! "zeta" CODE:PLAY "T240O5N1C","O4N1E","O3N1G"\n40 PRINT "${target} READY"\n50 STOP\n`, 'basic', 5, target));
    const banked = await until(frame => frame.debug?.valuesValid);
    assert.equal(banked.debug.sourceLine, 5);
    assert.ok(banked.text.includes(`${target} READY`));
    assert.ok(banked.text.indexOf('alpha') < banked.text.indexOf('zeta'));
    assert.notEqual(banked.firmwareId, 'eightbit-spectrum48-basic-0.1-dev');
    await command({operation:'step', stopId:banked.debug.stopId});
    await command({operation:'stop'});
    await action({operation:'spectrum-close'});
    await create(await launch('ORG 24576\nDI\nLD BC,32765\nLD A,1\nOUT (C),A\nLD A,42\nLD (49152),A\nXOR A\nOUT (C),A\nEI\nRET\n', 'assembly', 2, target));
    const assembly = await until(frame => frame.debug?.valuesValid);
    assert.equal(assembly.debug.sourceLine, 2);
    const stepped = await command({operation:'step',stopId:assembly.debug.stopId});
    assert.equal(stepped.debug.sourceLine,3);
    await command({operation:'continue',stopId:stepped.debug.stopId});
    await command({operation:'advance',cycles:350000});
    await command({operation:'reset'});
    await command({operation:'stop'});
    await action({operation:'spectrum-close'});
  }
  console.log(JSON.stringify({ schemaVersion: 1, result: 'passed', comparedRequests: next, traceSha256: trace.digest('hex'), cassetteBytes: tape.length, cassetteSha256: hash(tape), checks: ['fresh BASIC/assembly builds', 'source mismatch', 'guest graphics/audio/text/input', 'release retains submitted input', 'reset/stop', 'instruction Step and stale stop', 'real MIC capture and checked TAP export', '128K and original +2 guest music/RAM disc', '128K and original +2 BASIC/assembly debugging'] }));
} finally {
  clearTimeout(deadline); runtime?.free();
  termination = await owner.stop();
  assert.equal(termination.error, undefined, 'Owned peer must start');
  assert.equal(termination.code, 0, 'Owned peer must finish cleanly');
}
