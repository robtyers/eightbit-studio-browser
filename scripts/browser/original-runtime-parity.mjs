// Opt-in real compiled-runtime acceptance. Never creates/edits qualification.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { verifyOriginalBasicExtended } from './original-basic-extended.mjs';
import { spawnOwnedPeer } from './original-runtime-process.mjs';
import { acceptGames, readGameProjects } from './original-games-acceptance.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));
const [mode, ...extra] = process.argv.slice(2);
assert.ok(['--expect-development', '--expect-qualified'].includes(mode) &&
  (!extra.length || (mode === '--expect-qualified' && extra.length === 3 && extra[0] === '--games')),
  'Choose --expect-development or --expect-qualified, optionally followed by --games DragonDoom-folder Battlezone-folder');
const games = extra.length ? readGameProjects(extra.slice(1)) : undefined;
const receipt = verifyOriginalBasicExtended(root);
const qualified = mode === '--expect-qualified';
assert.equal(receipt.status, qualified ? 'qualified' : 'development', 'Requested acceptance requires the actual reviewed firmware identity');
// verifyOriginalBasicExtended also requires the real bounded source/symbol/image-bound
// qualification.json for this branch. No manifest/record substitution is allowed.
if (qualified) execFileSync('cargo', ['test', '--offline', '--locked', '--manifest-path',
  resolve(root, 'families/dragon/crates/eightbit-dragon-run/Cargo.toml'), '--lib',
  'original_runtime::tests::qualified_project_run_and_debug_acceptance', '--', '--ignored', '--exact'],
  { cwd: root, stdio: 'inherit', timeout: 180_000 });
const generated = resolve(root, 'apps/eightbit-studio-web/ui/public/generated');
const wasm = readFileSync(resolve(generated, 'dragon_core_bg.wasm'));
assert.ok(wasm.includes(readFileSync(resolve(root, 'families/dragon/firmware/basic-extended/basic.rom'))), 'WASM must contain the exact verified firmware image');
const module = await import(pathToFileURL(resolve(generated, 'dragon_core.js')).href);
await module.default({ module_or_path: wasm });
// Build first, then own the actual executable. Killing cargo-run does not
// reliably terminate its compiled child after a request deadline.
const built = execFileSync('cargo', ['build', '--offline', '--locked', '--release', '--message-format=json', '--manifest-path', resolve(root, 'apps/eightbit-studio-web/core/Cargo.toml'), '--example', 'original-runtime-peer'], { cwd: root, encoding: 'utf8', stdio: ['ignore', 'pipe', 'inherit'], timeout: 180_000, maxBuffer: 16 * 1024 * 1024 });
const executable = built.split('\n').filter(Boolean).map(line => JSON.parse(line)).find(value => value.reason === 'compiler-artifact' && value.target?.name === 'original-runtime-peer' && value.target.kind.includes('example') && typeof value.executable === 'string')?.executable;
assert.equal(typeof executable, 'string', 'Cargo must report the compiled native peer');
const owner = spawnOwnedPeer(executable, [], { cwd: root });
const native = owner.child;
let pending;
let buffer = '';
let next = 0;
let runtime;
let lastPixels;
let lastAudio;
let id = 0;
const deadline = setTimeout(() => fail('Acceptance exceeded its time budget'), games ? 600_000 : 180_000);
function fail(message) { const current = pending; pending = undefined; if (current) { clearTimeout(current.timer); current.reject(new Error(message)); } native.kill(); }
native.on('error', () => fail('Native acceptance peer could not start'));
native.on('exit', code => { if (pending) fail(`Native acceptance peer exited (${code})`); });
native.stdout.setEncoding('utf8');
native.stdout.on('data', chunk => {
  buffer += chunk;
  if (buffer.length > 4 * 1024 * 1024) return fail('Native response exceeds bound');
  while (buffer.includes('\n')) {
    const end = buffer.indexOf('\n'); const line = buffer.slice(0, end); buffer = buffer.slice(end + 1);
    try {
      const value = JSON.parse(line);
      if (!pending || value.id !== pending.id) return fail('Stale native response');
      const current = pending; pending = undefined; clearTimeout(current.timer); current.resolve(value);
    } catch { return fail('Malformed native response'); }
  }
});
function peer(command) {
  assert.equal(pending, undefined);
  return new Promise((resolve, reject) => {
    const request = ++next;
    const timer = setTimeout(() => fail('Native acceptance request timed out'), 120_000);
    pending = { id: request, resolve, reject, timer };
    native.stdin.write(`${JSON.stringify({ id: request, command })}\n`);
  });
}
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
function compact(value) {
  if (value === null || typeof value !== 'object') return value;
  if (value.session) value.session = compact(value.session);
  if (value.pixels) { value.pixelsSha256 = hash(Buffer.from(value.pixels)); delete value.pixels; }
  if (value.audio) {
    const bytes = Buffer.alloc(value.audio.length * 2);
    value.audio.forEach((sample, index) => bytes.writeInt16LE(sample, index * 2));
    value.audioSamples = value.audio.length; value.audioSha256 = hash(bytes); delete value.audio;
  }
  return value;
}
function wasmRequest(command) {
  switch (command.operation) {
    case 'catalog': return JSON.parse(module.browser_request('{"schemaVersion":1,"operation":"machines"}', new Uint8Array()));
    case 'build': return JSON.parse(module.browser_request(JSON.stringify({ schemaVersion: 1, operation: 'build', source: command.source }), new Uint8Array()));
    case 'browser': return JSON.parse(module.browser_request(JSON.stringify(command.request), Uint8Array.from(command.bytes)));
    case 'create': assert.equal(runtime, undefined); runtime = module.MonitorRuntime.fromOriginalSource(command.session_id, JSON.stringify(command.source_request)); return null;
    case 'create-assembly': assert.equal(runtime, undefined); runtime = module.MonitorRuntime.fromOriginalAssembly(command.session_id, JSON.stringify(command.source_request)); return null;
    case 'create-media': assert.equal(runtime, undefined); runtime = module.MonitorRuntime.fromOriginalMedia(command.session_id, JSON.stringify(command.media_request), Uint8Array.from(command.bytes)); return null;
    case 'monitor': {
      const frame = JSON.parse(runtime.command(JSON.stringify(command.request)));
      lastPixels = frame.pixels;
      lastAudio = frame.audio;
      return compact(frame);
    }
    case 'debug-state': { const state = runtime.debugState(); return state == null ? null : compact(JSON.parse(state)); }
    case 'debug': return compact(JSON.parse(runtime.debugCommand(JSON.stringify(command.request))));
    case 'close': runtime?.free(); runtime = undefined; return null;
    default: throw new Error('Unknown acceptance operation');
  }
}
const trace = createHash('sha256');
function sorted(value) { return Array.isArray(value) ? value.map(sorted) : value && typeof value === 'object' ? Object.fromEntries(Object.keys(value).sort().map(key => [key, sorted(value[key])])) : value; }
async function action(command, errorPrefix) {
  let actual;
  try { actual = { ok: true, result: wasmRequest(command) }; }
  catch (error) { actual = { ok: false, error: String(error).slice(0, 256) }; }
  const { id: _, ...expected } = await peer(command);
  assert.deepEqual(actual, expected, `Native/WASM disagreement at ${command.operation}`);
  trace.update(JSON.stringify(sorted(actual)));
  if (errorPrefix) { assert.equal(actual.ok, false); assert.ok(actual.error.startsWith(errorPrefix), actual.error); return actual.error; }
  assert.equal(actual.ok, true, actual.error); return actual.result;
}
const monitor = command => action({ operation: 'monitor', request: { schemaVersion: 1, sessionId: id, command } });
const debugState = () => action({ operation: 'debug-state' });
async function debug(operation) { const state = await debugState(); return action({ operation: 'debug', request: { schemaVersion: 1, revision: state.revision, command: { operation } } }); }
async function machineCommand(command) { const state = await debugState(); return action({ operation: 'debug', request: { schemaVersion: 1, revision: state.revision, command } }); }
const browserRequest = (request, bytes = []) => action({ operation: 'browser', request: { schemaVersion: 1, ...request }, bytes });
async function settle(count) { let frame; for (let n = 0; n < count; n++) { frame = await monitor({ operation: 'advance', cycles: 32768 }); assert.notEqual(frame.state, 'failed', frame.message); } return frame; }
async function until(check, limit = 1024) { let frame; for (let n = 0; n < limit; n++) { frame = await settle(1); if (await check(frame)) return frame; } throw new Error(`Guest acceptance exhausted its bounded frame budget: ${JSON.stringify({ pc: frame.pc, state: frame.state, text: frame.text })}`); }
async function build(source, breakpointLine) {
  const built = await action({ operation: 'build', source }); assert.equal(built.ok, true);
  return { schemaVersion: 1, source, sourceSha256: built.result.summary.sourceSha256, cassetteSha256: built.result.summary.cassetteSha256, ...(breakpointLine === undefined ? {} : { breakpointLine }) };
}
async function create(request) { id++; await action({ operation: 'create', session_id: id, source_request: request }); }
const output42 = frame => frame.text.split('\n').some(line => line.trim() === '42');
const atPrompt = frame => /(?:^|\n)OK>\s*$/.test(frame.text);
let result;
let termination;
let survey;
let assembly;
let gameAcceptance;
try {
  const identity = await peer({ operation: 'identity' });
  assert.equal(identity.ok, true, identity.error);
  assert.deepEqual(identity.result.manifest, receipt, 'Native peer must compile the exact verified manifest');
  assert.equal(identity.result.firmwareSha256, receipt.romSha256, 'Native peer must compile the exact verified image');
  const catalog = await action({ operation: 'catalog' }); assert.equal(catalog.ok, true);
  assert.equal(catalog.result.machines[0].defaultRuntime, 'original', 'Dragon 32 must default to its bundled runtime');
  assert.equal(catalog.result.machines[0].runtime, 'xroar', 'Explicit XRoar alternative must remain available');
  const profile = catalog.result.originalRuntime.profile;
  if (!qualified) {
    assert.equal(profile, null);
    for (const request of [{}, { schemaVersion: 1, source: '10 END\n' }]) await action({ operation: 'create', session_id: 1, source_request: request }, 'DME1400:');
  } else {
    assert.equal(profile.profileId, 'eightbit-dragon32-basic-extended-1'); assert.equal(profile.firmwareSha256, receipt.romSha256);
    if (games) {
      gameAcceptance = await acceptGames(games, {
        action, peer, browserRequest, monitor, debugState, machineCommand,
        createAssembly: async sourceRequest => { id++; await action({ operation: 'create-assembly', session_id: id, source_request: sourceRequest }); },
        getPixels: () => lastPixels, getAudio: () => lastAudio,
      });
      assert.deepEqual(readGameProjects(extra.slice(1)).map(game => game.inputs), games.map(game => game.inputs), 'Acceptance must leave original game inputs unchanged');
    }
    const source = '10 COUNT=COUNT+1\n20 PRINT COUNT+41\n30 END\n';
    const request = await build(source);
    await action({ operation: 'create', session_id: 1, source_request: { ...request, sourceSha256: '0'.repeat(64) } }, 'DME1303:');
    await create(request);
    assert.equal(await debugState(), null, 'Run has no fabricated debugger');
    await until(output42);
    await monitor({ operation: 'reset' });
    await settle(96);
    // Warm reset retains both screen and variables. A second automatic RUN
    // increments COUNT to 2; retained output42 alone proves neither outcome.
    await monitor({ operation: 'type', text: 'PRINT "RESETVALUE";COUNT' });
    await until(frame => frame.text.split('\n').some(line => /^RESETVALUE\s+1\s*$/.test(line)), 128);
    await monitor({ operation: 'stop' });
    await action({ operation: 'monitor', request: { schemaVersion: 1, sessionId: id, command: { operation: 'advance', cycles: 32768 } } }, 'DME1004:');
    await action({ operation: 'close' });

    const debugRequest = await build('10 INPUT COUNT\n20 COUNT=COUNT+1:PRINT COUNT\n30 END\n', 20);
    await create(debugRequest);
    const cancelled = await debug('cancel'); assert.equal(cancelled.status.state, 'cancelled');
    const before = await monitor({ operation: 'state' }); const paused = await settle(4); assert.equal(paused.cycles, before.cycles);
    await debug('continue');
    let replied = false;
    await until(async frame => {
      if (!replied && /\?\s*$/.test(frame.text)) { await monitor({ operation: 'type', text: '41' }); replied = true; }
      return (await debugState()).status.state === 'stopped';
    });
    assert.equal(replied, true);
    const stopped = await debugState();
    assert.equal(stopped.status.location.basicLine, 20); assert.equal(stopped.status.variables.find(value => value.name === 'COUNT').numericValue, 41);
    const continued = { schemaVersion: 1, revision: stopped.revision, command: { operation: 'continue' } };
    await action({ operation: 'debug', request: continued }); await action({ operation: 'debug', request: continued }, 'DME1303:');
    await until(async () => (await debugState()).status.state === 'stopped', 128);
    assert.equal((await debugState()).status.variables.find(value => value.name === 'COUNT').numericValue, 42);
    await debug('continue');
    await until(async () => (await debugState()).status.state === 'ready', 128);
    assert.equal((await debugState()).status.message, 'Programme returned to BASIC prompt');
    await monitor({ operation: 'reset' }); await monitor({ operation: 'stop' });
    await action({ operation: 'monitor', request: { schemaVersion: 1, sessionId: id, command: { operation: 'advance', cycles: 32768 } } }, 'DME1004:');
    await action({ operation: 'close' });
    // A later Run owns a newly constructed build/session, never the stopped owner.
    await create(await build(source)); await until(output42); await monitor({ operation: 'stop' }); await action({ operation: 'close' });

    // Source/settings are assembled afresh by both public hosts. No caller
    // supplies executable bytes or trusted source mappings to this route.
    const machineSource = 'START   lda #77\n        sta $0400\n        lda #42\nMARKER  sta $0600\nLOOP    bra LOOP\n';
    const created = await browserRequest({ operation: 'create-project', name: 'Machine acceptance', kind: 'assembly', source: machineSource, target: 'dragon32' });
    assert.equal(created.ok, true, created.message);
    const project = created.result.project;
    const builtMachine = await browserRequest({ operation: 'test-project', project });
    assert.equal(builtMachine.ok, true, builtMachine.message);
    const sourceRequest = { schemaVersion: 1, kind: 'assembly-source', project, buildId: builtMachine.result.assembly.buildId, cassetteSha256: builtMachine.result.summary.cassetteSha256 };
    const failedBuild = await browserRequest({ operation: 'test-project', project: { ...project, source: 'START illegal_opcode\n' } });
    assert.equal(failedBuild.ok, false);
    assert.ok(failedBuild.diagnostics.length > 0);
    await action({ operation: 'create-assembly', session_id: id + 1, source_request: { ...sourceRequest, project: { ...project, source: 'START rts\n' } } }, 'DME1401:');
    id++;
    await action({ operation: 'create-assembly', session_id: id, source_request: sourceRequest });
    assert.equal(await debugState(), null);
    await until(frame => frame.text.startsWith('M'));
    await monitor({ operation: 'stop' }); await action({ operation: 'close' });

    id++;
    await action({ operation: 'create-assembly', session_id: id, source_request: { ...sourceRequest, debug: { kind: 'source', sourceLine: 4 } } });
    await until(async () => (await debugState()).status.state === 'stopped');
    const machineStopped = await debugState();
    assert.equal(machineStopped.kind, 'machine');
    assert.equal(machineStopped.status.location.sourceLine, 4);
    assert.equal(machineStopped.status.registers.a, 42);
    assert.equal(machineStopped.status.registers.pc, 0x7007);
    assert.equal(machineStopped.status.disassembly[0].mnemonic, 'STA');
    const stepRequest = { schemaVersion: 1, revision: machineStopped.revision, command: { operation: 'step' } };
    await action({ operation: 'debug', request: stepRequest });
    await action({ operation: 'debug', request: stepRequest }, 'DME1501:');
    const memory = await machineCommand({ operation: 'memory', address: 0x0600, length: 1 });
    assert.deepEqual(memory.status.memory.bytes, [42]);
    await machineCommand({ operation: 'breakpoint', address: 0x7000 });
    await debug('continue'); await settle(4);
    const machinePaused = await debug('pause');
    assert.equal(machinePaused.status.state, 'cancelled');
    const machinePauseFrame = await monitor({ operation: 'state' });
    assert.equal((await settle(3)).cycles, machinePauseFrame.cycles, 'Pause must stop the only execution owner');
    await debug('continue'); await settle(2);
    await monitor({ operation: 'reset' });
    assert.equal((await debugState()).status.state, 'ready');
    assert.equal((await debugState()).status.valuesValid, false);
    await monitor({ operation: 'stop' });
    await action({ operation: 'monitor', request: { schemaVersion: 1, sessionId: id, command: { operation: 'advance', cycles: 32768 } } }, 'DME1004:');
    await action({ operation: 'close' });

    // Imported media stays immutable; reconstruction is an editable preview,
    // whereas Run/Debug read the exact selected cassette and checked identity.
    const machineCassette = builtMachine.result.cassette;
    const inspected = await browserRequest({ operation: 'decompile', input: { format: 'cassette' } }, machineCassette);
    assert.equal(inspected.ok, true, inspected.message);
    const mediaRequest = inspected.result.machineCode;
    const changedMedia = [...machineCassette]; changedMedia[0] ^= 1;
    await action({ operation: 'create-media', session_id: id + 1, media_request: mediaRequest, bytes: changedMedia }, 'DME1401:');
    id++;
    await action({ operation: 'create-media', session_id: id, media_request: { ...mediaRequest, breakpointAddress: 0x7000 }, bytes: machineCassette });
    await until(async () => (await debugState()).status.state === 'stopped');
    assert.equal((await debugState()).status.registers.pc, 0x7000);
    await debug('step');
    assert.equal((await debugState()).status.registers.a, 77);
    await monitor({ operation: 'stop' }); await action({ operation: 'close' });
    const cartridge = await browserRequest({ operation: 'decompile', input: { format: 'cartridge' } }, [0x39]);
    assert.equal(cartridge.ok, false); assert.match(cartridge.message, /cartridges are unsupported on the website/);
    assembly = { sourceSha256: hash(machineSource), buildId: sourceRequest.buildId, cassetteSha256: sourceRequest.cassetteSha256, sourceRun: true, sourceDebugStepMemory: true, revisionRejection: true, pauseResetStop: true, importedCassetteAddressDebug: true, changedInputRejection: true, cartridgeRejection: true };

    // This authored repository acceptance fixture crosses the real compiler,
    // physical keyboard, VDG/DAC and cassette pins. It is not a host evaluator.
    const surveySource = readFileSync(resolve(root, 'families/dragon/crates/eightbit-dragon-emulator/tests/fixtures/complete-basic/survey-acceptance.bas'), 'utf8');
    await create(await build(surveySource));
    await until(frame => frame.text.includes('SCALE 1 TO 4') && /\?\s*$/.test(frame.text));
    await monitor({ operation: 'type', text: '2' });
    let musicSamples = 0;
    let audible = false;
    const musicTrace = createHash('sha256');
    await until(frame => {
      musicSamples += frame.audioSamples;
      musicTrace.update(frame.audioSha256);
      audible ||= frame.audioSamples > 0 && frame.audioSha256 !== hash(Buffer.alloc(frame.audioSamples * 2));
      return audible;
    });
    // The short authored PLAY/SOUND sequence ends in a physical INKEY$ loop.
    // A bounded settle leaves the completed graphics visible before the key.
    const graphics = await settle(256);
    const colour = (x, y) => lastPixels.slice((y * 256 + x) * 4, (y * 256 + x) * 4 + 4);
    const background = colour(40, 160);
    const ink = colour(20, 160);
    assert.notDeepEqual(ink, background, 'LINE must draw the survey bars');
    for (const [x, y] of [[192, 152], [180, 50], [220, 60]]) assert.deepEqual(colour(x, y), ink, 'PUT, DRAW and PAINT must leave the expected visible pixels');
    await monitor({ operation: 'key', code: 'KeyQ', pressed: true });
    await settle(2); await monitor({ operation: 'key', code: 'KeyQ', pressed: false });
    const ready = await until(frame => frame.text.includes('RECORD DATA, THEN CONT') && atPrompt(frame));
    assert.match(ready.text, /TOTAL\s+64/);
    await monitor({ operation: 'start-recording' });
    await monitor({ operation: 'type', text: 'CONT' });
    await until(frame => frame.text.includes('MOUNT SAVED DATA, THEN CONT') && atPrompt(frame));
    const finished = await monitor({ operation: 'finish-recording' });
    assert.equal(finished.recording.state, 'ready', finished.recording.message);
    assert.equal(finished.recording.cassette.format, 'ascii-data');
    const exported = await monitor({ operation: 'export-recording' });
    const cassette = exported.recordingExport.bytes;
    assert.ok(cassette.length > 0 && cassette.length <= 8192, 'Authored recording must fit the bounded peer envelope');
    const recordedSha256 = hash(Buffer.from(cassette));
    assert.equal(finished.recording.cassette.sha256, recordedSha256);
    await monitor({ operation: 'stop' }); await action({ operation: 'close' });

    // Fresh normal construction proves that neither variables nor mounted media
    // from the first guest supply the read-back result.
    const reader = '10 PRINT"MOUNT SURVEY DATA, THEN CONT":STOP\n20 OPEN"I",#-1,"SURVEY":N=0\n30 FOR I=1 TO 4:INPUT#-1,A:N=N+A:NEXT I\n40 INPUT#-1,V,C$:CLOSE#-1\n50 IF N<>64 OR V<>64 OR C$<>"SURVEY 64" THEN90\n60 PRINT"FRESH DATA OK";N;C$:END\n90 PRINT"DATA MISMATCH":END\n';
    await create(await build(reader));
    await until(frame => frame.text.includes('MOUNT SURVEY DATA, THEN CONT') && atPrompt(frame));
    const mounted = await monitor({ operation: 'mount-cassette', bytes: cassette });
    assert.equal(mounted.cassette.sha256, recordedSha256);
    await monitor({ operation: 'type', text: 'CONT' });
    const recovered = await until(frame => frame.text.includes('FRESH DATA OK') && atPrompt(frame));
    assert.match(recovered.text, /FRESH DATA OK\s+64\s+SURVEY 64/);
    assert.ok(!recovered.text.includes('DATA MISMATCH'));
    await monitor({ operation: 'stop' }); await action({ operation: 'close' });
    survey = { sourceSha256: hash(surveySource), graphicsSha256: graphics.pixelsSha256, audible, musicSamples, musicTraceSha256: musicTrace.digest('hex'), recordedBytes: cassette.length, recordedSha256, freshGuestDataRead: true };
  }
  result = { schemaVersion: 1, expected: receipt.status, firmwareSha256: receipt.romSha256, wasmSha256: hash(wasm), nativeWasmParity: true, requests: next, traceSha256: trace.digest('hex'), normalRunDebugExecuted: qualified, ...(assembly && { assembly }), ...(survey && { survey }), ...(gameAcceptance && { games: gameAcceptance }), scope: qualified ? 'Actual admitted native/WASM BASIC and assembly Run/Debug, imported cassette address Debug, source and media identity rejection, INPUT, Continue, cancellation, reset, stop, fresh restart and authored graphics/music/data recording with fresh guest read-back; optional unchanged-game results are recorded separately; UI/package gates remain separate' : 'Real compiled development gate rejection only; no normal execution or qualification claimed' };
} finally {
  runtime?.free(); clearTimeout(deadline); if (pending) clearTimeout(pending.timer);
  termination = await owner.stop();
}
assert.equal(termination.code, 0, "Native acceptance peer must close cleanly before success");
console.log(JSON.stringify({ ...result, nativePeerTerminated: true }));
